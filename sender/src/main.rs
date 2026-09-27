//! GPU Hardware-Accelerated Virtual Second Monitor Sender for Linux Wayland
//!
//! Clean, modular architecture separating CLI configuration, GNOME Screencast D-Bus IPC,
//! PipeWire graph linking, Streaming Pipeline builders, and Web Dashboard control.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

use std::env;
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

mod config;
mod control;
mod encoder;
mod i18n;
mod native_streamer;
mod pipeline;
mod pipewire;
mod screencast;
mod usb_transport;

use config::{SenderConfig, TransportKind};
use control::{ControlAction, ControlListener};
use pipeline::{PipelineBuilder, StreamerHandle};
use screencast::MutterScreenCastSession;

static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // 1. Parse configuration and handle --help
    let mut cfg = match SenderConfig::parse(&args)? {
        Some(c) => c,
        None => return Ok(()),
    };

    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ext-sender: AMD GPU Offload Virtual Second Monitor Sender v0.2.0      \x1b[0m");
    println!("\x1b[1;34m  100% Native Rust | PipeWire Zero-Copy | VA-API / NVENC / QSV Hardware  \x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m");

    // 2. Setup graceful signal handler
    let running = Arc::new(AtomicBool::new(true));
    setup_signal_handler(running.clone());

    // 3. Initialize Transport (USB Bulk or Network UDP)
    let mut pipe_fds = [0 as libc::c_int; 2];
    let usb_pipe_fd: Option<RawFd> = match cfg.transport {
        TransportKind::UsbBulk => {
            println!("\x1b[1;33m[*] Transport Mode: USB Bulk Direct (Zero Network Stack)\x1b[0m");
            let (handle, iface_num, ep_out) = usb_transport::open_usb_display_device()
                .map_err(|e| format!("Failed to open USB Display device: {}", e))?;

            unsafe {
                libc::pipe(pipe_fds.as_mut_ptr());
                const F_SETPIPE_SZ: libc::c_int = 1031;
                libc::fcntl(pipe_fds[1], F_SETPIPE_SZ, 1024 * 1024);
            }
            let read_fd = pipe_fds[0];
            let write_fd = pipe_fds[1];
            let _ = usb_transport::spawn_usb_bulk_writer(handle, read_fd, running.clone(), iface_num, ep_out);
            Some(write_fd)
        }
        TransportKind::Network { ref ip, port } => {
            println!("\x1b[1;34m[*] Transport Mode: Network IP (UDP RTP {}:{})\x1b[0m", ip, port);
            None
        }
    };

    println!("\x1b[1;34m[*] Target:\x1b[0m {}:{}", cfg.target_ip, cfg.target_port);
    println!("\x1b[1;34m[*] Bitrate:\x1b[0m {} kbps (Adaptive VBR, {} FPS)", cfg.bitrate, cfg.fps);
    println!("\x1b[1;34m[*] Color Profile:\x1b[0m {}", cfg.color_profile.name());
    println!("\x1b[1;34m[*] Display Mode:\x1b[0m {}", cfg.mode);
    println!("\x1b[1;34m[*] Encoder API:\x1b[0m {:?}", cfg.encoder);
    println!("\x1b[1;34m[*] Stream Engine:\x1b[0m {}", cfg.engine.name());

    let monitor_to_record = if cfg.mode == "clone" {
        "eDP-1"
    } else {
        pipewire::ensure_kernel_hdmi_connected();
        pipewire::ensure_gnome_displays();
        "HDMI-1"
    };

    println!("\x1b[1;34m[*] Recording Monitor:\x1b[0m {}", monitor_to_record);

    let ctrl_listener = ControlListener::bind(5001);
    let mut hud_hide_at = if cfg.hud {
        Some(Instant::now() + Duration::from_secs(60))
    } else {
        None
    };

    // 4. Main Supervisor Loop (Reconnects on Suspend/Resume or System Event)
    while running.load(Ordering::SeqCst) {
        println!("\x1b[1;34m[*] Connecting to GNOME Mutter ScreenCast via D-Bus...\x1b[0m");

        let session = match MutterScreenCastSession::create_and_start(monitor_to_record) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("\x1b[1;31m[!] Screencast session creation failed: {}. Retrying in 2s...\x1b[0m", e);
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        let node_id = session.node_id;

        let mut pipeline_builder = PipelineBuilder {
            node_id,
            target_ip: cfg.target_ip.clone(),
            target_port: cfg.target_port,
            bitrate: cfg.bitrate,
            encoder: cfg.encoder,
            fps: cfg.fps,
            hud: cfg.hud,
            color_profile: cfg.color_profile,
            drop_only: cfg.drop_only,
            skip_to_first: cfg.skip_to_first,
            key_int_max: cfg.key_int_max,
            usb_pipe_fd,
            engine: cfg.engine,
        };

        println!(
            "\x1b[1;33m[*] Starting {:?} hardware streaming pipeline via {} ({} FPS, HUD: {})...\x1b[0m",
            cfg.encoder, cfg.engine.name(), cfg.fps, cfg.hud
        );

        let mut child: StreamerHandle = match pipeline_builder.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("\x1b[1;31m[!] Failed to spawn streamer: {}. Retrying in 2s...\x1b[0m", e);
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        thread::sleep(Duration::from_millis(800));
        pipewire::link_monitor_port_to_sender(node_id, monitor_to_record);

        println!(
            "\x1b[1;32m[+] Monitor {} is streaming LIVE to Pi Zero at {} FPS ({})!\x1b[0m",
            monitor_to_record, cfg.fps, cfg.color_profile.name()
        );

        let mut last_node_check = Instant::now();
        let mut last_link_check = Instant::now();

        // 5. Watchdog and Web Hot-Apply loop
        while running.load(Ordering::SeqCst) {
            let mut restart_pipeline = false;

            // Check HUD auto-hide (60s)
            if let Some(hide_at) = hud_hide_at {
                if Instant::now() >= hide_at && cfg.hud {
                    println!("\x1b[1;33m[*] HUD auto-hide (60s): Hiding HUD for full screen display...\x1b[0m");
                    cfg.hud = false;
                    pipeline_builder.hud = false;
                    hud_hide_at = None;
                    restart_pipeline = true;
                }
            }

            // Poll UDP control actions
            for action in ctrl_listener.poll_actions() {
                match action {
                    ControlAction::TriggerHud => {
                        println!("\x1b[1;32m[+] Web Command: Re-triggering HUD on screen for 60 seconds!\x1b[0m");
                        cfg.hud = true;
                        pipeline_builder.hud = true;
                        hud_hide_at = Some(Instant::now() + Duration::from_secs(60));
                        restart_pipeline = true;
                    }
                    ControlAction::HideHud => {
                        println!("\x1b[1;33m[*] Web Command: Hiding HUD immediately on user request.\x1b[0m");
                        cfg.hud = false;
                        pipeline_builder.hud = false;
                        hud_hide_at = None;
                        restart_pipeline = true;
                    }
                    ControlAction::SetBitrate(b) => {
                        if b != cfg.bitrate {
                            println!("\x1b[1;34m[*] Web Bitrate Change: {} -> {} kbps\x1b[0m", cfg.bitrate, b);
                            cfg.bitrate = b;
                            pipeline_builder.bitrate = b;
                            restart_pipeline = true;
                        }
                    }
                    ControlAction::SetFps(f) => {
                        if f != cfg.fps {
                            println!("\x1b[1;34m[*] Web FPS Change: {} -> {} FPS\x1b[0m", cfg.fps, f);
                            cfg.fps = f;
                            pipeline_builder.fps = f;
                            cfg.bitrate = SenderConfig::recommended_bitrate(f);
                            pipeline_builder.bitrate = cfg.bitrate;
                            restart_pipeline = true;
                        }
                    }
                    ControlAction::SetColorProfile(cp) => {
                        if cp != cfg.color_profile {
                            println!("\x1b[1;34m[*] Web Color Change: {:?} -> {:?}\x1b[0m", cfg.color_profile, cp);
                            cfg.color_profile = cp;
                            pipeline_builder.color_profile = cp;
                            restart_pipeline = true;
                        }
                    }
                    ControlAction::SetDropOnly(d) => {
                        if d != cfg.drop_only {
                            cfg.drop_only = d;
                            pipeline_builder.drop_only = d;
                            restart_pipeline = true;
                        }
                    }
                    ControlAction::SetSkipToFirst(s) => {
                        if s != cfg.skip_to_first {
                            cfg.skip_to_first = s;
                            pipeline_builder.skip_to_first = s;
                            restart_pipeline = true;
                        }
                    }
                    ControlAction::SetKeyIntMax(k) => {
                        if k != cfg.key_int_max {
                            cfg.key_int_max = k;
                            pipeline_builder.key_int_max = k;
                            restart_pipeline = true;
                        }
                    }
                }
            }

            if restart_pipeline {
                println!("\x1b[1;36m[*] Hot-applying configuration (FPS: {}, Bitrate: {}k, Color: {:?})...\x1b[0m", cfg.fps, cfg.bitrate, cfg.color_profile);
                let _ = child.kill();
                let _ = child.wait();
                child = match pipeline_builder.spawn() {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("\x1b[1;31m[!] Failed to restart streamer: {}\x1b[0m", e);
                        break;
                    }
                };
                thread::sleep(Duration::from_millis(600));
                pipewire::link_monitor_port_to_sender(node_id, monitor_to_record);
                println!("\x1b[1;32m[+] Configuration hot-applied successfully.\x1b[0m");
                continue;
            }

            match child.try_wait() {
                Ok(Some(status)) => {
                    println!("\x1b[1;33m[*] Streamer exited with status: {}. Restarting...\x1b[0m", status);
                    break;
                }
                Ok(None) => thread::sleep(Duration::from_millis(200)),
                Err(e) => {
                    eprintln!("\x1b[1;31m[!] Error monitoring streamer: {}\x1b[0m", e);
                    break;
                }
            }

            // Health watchdog: Detect suspend/resume or PipeWire crash
            if last_node_check.elapsed() >= Duration::from_millis(1500) {
                last_node_check = Instant::now();
                if !pipewire::is_pipewire_node_alive(node_id) {
                    println!("\x1b[1;31m[!] Screencast node {} disappeared. Reconnecting...\x1b[0m", node_id);
                    let _ = child.kill();
                    let _ = child.wait();
                    thread::sleep(Duration::from_millis(1500));
                    break;
                }
            }

            // Link watchdog: Ensure pipewiresrc port connection
            if last_link_check.elapsed() >= Duration::from_millis(3000) {
                last_link_check = Instant::now();
                if !pipewire::is_sender_linked() {
                    println!("\x1b[1;33m[*] PipeWire link lost. Re-linking to node {}...\x1b[0m", node_id);
                    pipewire::link_monitor_port_to_sender(node_id, monitor_to_record);
                }
            }
        }

        let _ = child.kill();
        let _ = child.wait();

        if !running.load(Ordering::SeqCst) {
            break;
        }

        thread::sleep(Duration::from_secs(1));
    }

    println!("\x1b[1;32m[*] ext-sender terminated cleanly.\x1b[0m");
    Ok(())
}

fn setup_signal_handler(running: Arc<AtomicBool>) {
    let r = running.clone();
    unsafe {
        libc::signal(libc::SIGINT, signal_handler as *const () as usize);
        libc::signal(libc::SIGTERM, signal_handler as *const () as usize);
    }
    thread::spawn(move || {
        while RUNNING.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(100));
        }
        r.store(false, Ordering::SeqCst);
    });
}

extern "C" fn signal_handler(_: libc::c_int) {
    RUNNING.store(false, Ordering::SeqCst);
}

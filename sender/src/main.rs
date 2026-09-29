//! GPU Hardware-Accelerated Virtual Second Monitor Sender for Linux Wayland
//!
//! Dual-Engine Architecture:
//! - Engine 1: Kernel DRM/KMS Direct Hardware Scanout (Universal zero-copy, zero freeze, multi-GPU, no D-Bus/Mutter dependency)
//! - Engine 2: GNOME Mutter D-Bus ScreenCast + PipeWire Graph Linking
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
mod kms;
mod native_streamer;
mod pipeline;
mod pipewire;
mod screencast;
mod usb_transport;

use config::{CaptureEngine, SenderConfig, TransportKind};
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
    println!("\x1b[1;32m  ext-sender: Universal Virtual Second Monitor Sender v0.3.0            \x1b[0m");
    println!("\x1b[1;34m  Dual-Engine: Linux Kernel KMS Direct / GNOME Mutter Screencast        \x1b[0m");
    println!("\x1b[1;34m  Multi-GPU (AMD/Intel/NVIDIA) | Multi-Monitor | 100% Native Rust        \x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m");

    // 2. Setup graceful signal handler
    let running = Arc::new(AtomicBool::new(true));
    setup_signal_handler(running.clone());

    // 3. Initialize Transport (USB Bulk as Default, with Automatic Fallback to Network UDP)
    let mut pipe_fds = [0 as libc::c_int; 2];
    let usb_pipe_fd: Option<RawFd> = match cfg.transport {
        TransportKind::UsbBulk => {
            println!("\x1b[1;33m[*] Transport Mode: USB Bulk Direct (Default - Zero Network Stack)\x1b[0m");
            match usb_transport::open_usb_display_device() {
                Ok((handle, iface_num, ep_out)) => {
                    println!("\x1b[1;32m[*] USB Bulk Direct connected successfully! (480 Mbps FunctionFS Endpoint)\x1b[0m");
                    unsafe {
                        libc::pipe(pipe_fds.as_mut_ptr());
                        const F_SETPIPE_SZ: libc::c_int = 1031;
                        libc::fcntl(pipe_fds[1], F_SETPIPE_SZ, 65536);
                    }
                    let read_fd = pipe_fds[0];
                    let write_fd = pipe_fds[1];
                    let _ = usb_transport::spawn_usb_bulk_writer(handle, read_fd, running.clone(), iface_num, ep_out);
                    Some(write_fd)
                }
                Err(err) => {
                    println!("\x1b[1;33m[!] USB Bulk device/interface not available on USB bus: {}\x1b[0m", err);
                    println!(
                        "\x1b[1;36m[i] Automatically falling back to Network transport (UDP RTP {}:{})...\x1b[0m",
                        cfg.target_ip, cfg.target_port
                    );
                    None
                }
            }
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
    println!("\x1b[1;34m[*] Capture Engine:\x1b[0m {}", cfg.capture.name());
    if cfg.audio {
        println!("\x1b[1;34m[*] Audio Channel:\x1b[0m Enabled (UDP RTP Opus {}:{})", cfg.target_ip, cfg.audio_port);
    } else {
        println!("\x1b[1;33m[*] Audio Channel:\x1b[0m Disabled (--no-audio)");
    }

    let mut monitor_to_record = if cfg.mode == "clone" {
        "eDP-1".to_string()
    } else {
        pipewire::ensure_kernel_hdmi_connected();
        pipewire::ensure_gnome_displays();
        "HDMI-1".to_string()
    };

    println!("\x1b[1;34m[*] Recording Monitor:\x1b[0m {}", monitor_to_record);

    let ctrl_listener = ControlListener::bind(5001);
    let mut hud_hide_at = if cfg.hud {
        Some(Instant::now() + Duration::from_secs(60))
    } else {
        None
    };

    // 4. Inibe protetor de tela e suspensão de energia enquanto o streaming estiver ativo
    let _session_inhibitor = screencast::GnomeSessionInhibitor::inhibit(
        "ext-monitor",
        "Transmissao ativa para o monitor secundario (Pi Zero)",
    );

    // 5. Main Supervisor Loop (Reconnects on Suspend/Resume or System Event)
    while running.load(Ordering::SeqCst) {
        let (_screencast_session, node_id, kms_info) = match cfg.capture {
            CaptureEngine::Kms => {
                println!("\x1b[1;34m[*] Motor KMS Direct: Descobrindo conector DRM/KMS para '{}'...\x1b[0m", monitor_to_record);
                let info = match kms::KmsOutputInfo::discover(&monitor_to_record) {
                    Ok(i) => i,
                    Err(e) => {
                        eprintln!("\x1b[1;31m[!] DRM/KMS discovery failed: {}. Retrying in 2s...\x1b[0m", e);
                        thread::sleep(Duration::from_secs(2));
                        continue;
                    }
                };
                (None, 0, Some(info))
            }
            CaptureEngine::Mutter => {
                println!("\x1b[1;34m[*] Conectando ao GNOME Mutter ScreenCast via D-Bus...\x1b[0m");
                let session = match MutterScreenCastSession::create_and_start(&monitor_to_record) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("\x1b[1;31m[!] Screencast session creation failed: {}. Retrying in 2s...\x1b[0m", e);
                        thread::sleep(Duration::from_secs(2));
                        continue;
                    }
                };
                let nid = session.node_id;
                (Some(session), nid, None)
            }
        };

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
            capture: cfg.capture,
            kms_info: kms_info.clone(),
            audio: cfg.audio,
            audio_port: cfg.audio_port,
        };

        println!(
            "\x1b[1;33m[*] Starting {:?} hardware streaming pipeline via {} [Capture: {:?}] ({} FPS, Stream: {}, HUD: {})...\x1b[0m",
            cfg.encoder,
            cfg.engine.name(),
            cfg.capture,
            cfg.fps,
            if cfg.drop_only { "Economy (drop-only)" } else { "Continuous (CFR Anti-Freeze)" },
            cfg.hud
        );

        let mut child: StreamerHandle = match pipeline_builder.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("\x1b[1;31m[!] Failed to spawn streamer: {}. Retrying in 2s...\x1b[0m", e);
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        if cfg.capture == CaptureEngine::Mutter {
            thread::sleep(Duration::from_millis(500));
            pipewire::link_monitor_port_to_sender(node_id, &monitor_to_record);
        }

        println!(
            "\x1b[1;32m[+] Monitor {} is streaming LIVE to Pi Zero at {} FPS ({}) via {:?}!\x1b[0m",
            monitor_to_record, cfg.fps, cfg.color_profile.name(), cfg.capture
        );

        let mut last_node_check = Instant::now();
        let mut last_telemetry_check = Instant::now();

        // 6. Watchdog and Web Hot-Apply loop
        while running.load(Ordering::SeqCst) {
            let mut restart_pipeline = false;
            let mut switch_engine_or_monitor = false;

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
                    ControlAction::StartStreaming => {
                        println!("\x1b[1;32m[+] Web Command: Iniciar / Reiniciar Transmissão recebido!\x1b[0m");
                        restart_pipeline = true;
                    }
                    ControlAction::StopStreaming => {
                        println!("\x1b[1;33m[*] Web Command: Parar Transmissão recebido!\x1b[0m");
                        let _ = child.kill();
                    }
                    ControlAction::SetMode(m) => {
                        let target_mon = if m == "clone" { "eDP-1".to_string() } else { "HDMI-1".to_string() };
                        if target_mon != monitor_to_record || m != cfg.mode {
                            println!("\x1b[1;35m[*] Web Command: Troca de Modo '{}' -> '{}' (Monitor: {})\x1b[0m", cfg.mode, m, target_mon);
                            cfg.mode = m;
                            monitor_to_record = target_mon;
                            switch_engine_or_monitor = true;
                        }
                    }
                    ControlAction::SetAudio(a) => {
                        if a != cfg.audio {
                            println!("\x1b[1;35m[*] Web Command: Áudio {} -> {}\x1b[0m", cfg.audio, a);
                            cfg.audio = a;
                            pipeline_builder.audio = a;
                            restart_pipeline = true;
                        }
                    }
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
                    ControlAction::SetCapture(eng) => {
                        if eng != cfg.capture {
                            println!("\x1b[1;35m[*] Web Command: Troca de Motor de Captura {:?} -> {:?}\x1b[0m", cfg.capture, eng);
                            cfg.capture = eng;
                            switch_engine_or_monitor = true;
                        }
                    }
                    ControlAction::SetMonitor(mon) => {
                        if mon != monitor_to_record {
                            println!("\x1b[1;35m[*] Web Command: Troca de Monitor {} -> {}\x1b[0m", monitor_to_record, mon);
                            monitor_to_record = mon;
                            switch_engine_or_monitor = true;
                        }
                    }
                }
            }

            if switch_engine_or_monitor {
                println!("\x1b[1;33m[*] Reiniciando supervisor para nova engine/monitor...\x1b[0m");
                let _ = child.kill();
                let _ = child.wait();
                break;
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
                if cfg.capture == CaptureEngine::Mutter {
                    thread::sleep(Duration::from_millis(500));
                    pipewire::link_monitor_port_to_sender(node_id, &monitor_to_record);
                }
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

            // Health watchdog: Detect suspend/resume or PipeWire crash (Mutter mode only)
            if cfg.capture == CaptureEngine::Mutter {
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
            }

            // Telemetria Periódica de Diagnóstico (a cada 2.5s)
            if last_telemetry_check.elapsed() >= Duration::from_millis(2500) {
                last_telemetry_check = Instant::now();
                match cfg.capture {
                    CaptureEngine::Kms => {
                        let kms_detail = kms_info.as_ref().map(|k| {
                            format!("CRTC: {} | {}x{}@{}Hz | Render: {:?}", k.crtc_id, k.width, k.height, k.vrefresh, k.render_node)
                        }).unwrap_or_else(|| "KMS Direct".to_string());

                        println!(
                            "\x1b[1;34m[TELEMETRIA]\x1b[0m Motor: \x1b[1;32mKMS Direct\x1b[0m | Monitor: {} | {} | Encoder: {} FPS | Modo: Contínuo (Anti-Freeze Scanout)",
                            monitor_to_record,
                            kms_detail,
                            cfg.fps
                        );
                    }
                    CaptureEngine::Mutter => {
                        let is_alive = pipewire::is_pipewire_node_alive(node_id);
                        let is_linked = pipewire::is_sender_linked();
                        println!(
                            "\x1b[1;34m[TELEMETRIA]\x1b[0m Motor: \x1b[1;33mGNOME Mutter\x1b[0m | Monitor: {} | Nó Mutter: {} ({}) | Enlace PipeWire: {} | Encoder: {} FPS | Modo: {}",
                            monitor_to_record,
                            node_id,
                            if is_alive { "\x1b[1;32mATIVO\x1b[0m" } else { "\x1b[1;31mINATIVO\x1b[0m" },
                            if is_linked { "\x1b[1;32mCONECTADO\x1b[0m" } else { "\x1b[1;31mDESCONECTADO\x1b[0m" },
                            cfg.fps,
                            if cfg.drop_only { "Econômico (drop-only)" } else { "Contínuo (CFR Anti-Freeze)" }
                        );
                    }
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

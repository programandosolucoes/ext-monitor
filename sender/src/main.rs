//! GPU Hardware-Accelerated Virtual Second Monitor Sender for Linux Wayland
//!
//! Dual-Engine Architecture:
//! - Engine 1: Kernel DRM/KMS Direct Hardware Scanout (Universal zero-copy, zero freeze, multi-GPU, no D-Bus/Mutter dependency)
//! - Engine 2: GNOME Mutter D-Bus ScreenCast + PipeWire Graph Linking
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::env;
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

mod cast_cert;
pub mod audio_native;
mod cast_server;
mod config;
mod control;
pub mod damage_pacer;
mod discovery;
mod encoder;
mod i18n;
pub mod http_client;
pub mod dialog;
pub mod flow;
pub mod kms;
pub mod miracast;
pub mod miracast_launcher;
pub mod native_streamer;
pub mod pipeline;
pub mod pipewire;
pub mod screencast;
pub mod service;
mod time_sync;
mod usb_transport;

use config::{CaptureEngine, SenderConfig, TransportKind};
use control::{ControlAction, ControlListener};
use pipeline::{PipelineBuilder, StreamerHandle};
use screencast::MutterScreenCastSession;

static RUNNING: AtomicBool = AtomicBool::new(true);

struct UsbTransportHandle {
    write_fd: RawFd,
    stop_flag: Arc<AtomicBool>,
}

fn is_pi_zero_usb_present() -> bool {
    if let Ok(entries) = std::fs::read_dir("/sys/bus/usb/devices") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let (Ok(v), Ok(p)) = (
                std::fs::read_to_string(path.join("idVendor")),
                std::fs::read_to_string(path.join("idProduct")),
            ) {
                let v = v.trim();
                let p = p.trim();
                if (v == "1d50" || v == "1d6b") && (p == "614d" || p == "0104") {
                    return true;
                }
            }
        }
    }
    false
}

fn open_usb_pipe_transport(
    running: Arc<AtomicBool>,
    writer_alive: Arc<AtomicBool>,
) -> Option<UsbTransportHandle> {
    if !is_pi_zero_usb_present() {
        return None;
    }

    println!("\x1b[1;33m[*] Attempting USB Bulk Direct connection (480 Mbps FunctionFS Endpoint)...\x1b[0m");
    match usb_transport::open_usb_display_device() {
        Ok((handle, iface_num, ep_out)) => {
            println!("\x1b[1;32m[*] USB Bulk Direct connected successfully! (480 Mbps FunctionFS Endpoint)\x1b[0m");
            let mut pipe_fds = [0 as libc::c_int; 2];
            unsafe {
                libc::pipe2(pipe_fds.as_mut_ptr(), libc::O_CLOEXEC);
                const F_SETPIPE_SZ: libc::c_int = 1031;
                // Performance: 256KB pipe buffer to prevent stalls at high bitrate.
                // Matches fdsink blocksize=65536 with 4x headroom for burst I-frames.
                libc::fcntl(pipe_fds[1], F_SETPIPE_SZ, 262144);
            }
            let read_fd = pipe_fds[0];
            let write_fd = pipe_fds[1];
            let stop_flag = Arc::new(AtomicBool::new(false));
            let _ = usb_transport::spawn_usb_bulk_writer(
                handle,
                read_fd,
                running.clone(),
                writer_alive.clone(),
                stop_flag.clone(),
                iface_num,
                ep_out,
            );
            Some(UsbTransportHandle { write_fd, stop_flag })
        }
        Err(err) => {
            println!("\x1b[1;33m[!] USB Bulk device/interface not available on USB bus: {}\x1b[0m", err);
            None
        }
    }
}

fn close_usb_transport(
    transport: &mut Option<UsbTransportHandle>,
    writer_alive: &Arc<AtomicBool>,
) {
    if let Some(h) = transport.take() {
        h.stop_flag.store(true, Ordering::SeqCst);
        unsafe {
            libc::close(h.write_fd);
        }
        for _ in 0..15 {
            if !writer_alive.load(Ordering::SeqCst) {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
}

/// Prompts the user interactively on their GUI desktop to choose between Extend, Clone, Window or Tab
/// 100% Pure Rust via FreeDesktop D-Bus Notifications with Action buttons (Zero bash, zero zenity).
pub fn prompt_user_mode_selection() -> Option<String> {
    crate::dialog::prompt_user_mode_selection_native()
}

/// Signals the receiver to immediately return to Standby Splash screen upon stream stop
/// 100% Pure Rust via native TCP HTTP client (Zero curl subprocess).
pub fn notify_receiver_stop(target_ip: &str) {
    let url = format!("http://{}:8080/api/stream/stop", target_ip);
    let _ = crate::http_client::post_empty(&url);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // 1. Parse configuration and handle --help, service commands, background auto-install
    let mut cfg = match SenderConfig::parse(&args)? {
        Some(c) => c,
        None => return Ok(()),
    };

    // Debug mode: enabled via --debug flag or EXT_DEBUG=1 environment variable
    let debug_mode = cfg.debug || std::env::var("EXT_DEBUG").map(|v| v == "1").unwrap_or(false);
    if debug_mode {
        std::env::set_var("EXT_DEBUG", "1");
        println!("\x1b[1;35m[DEBUG] Debug mode ACTIVE — verbose logging enabled\x1b[0m");
        println!("\x1b[1;35m[DEBUG] Config: {:?}\x1b[0m", cfg);
    }

    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m  ext-sender: Universal Virtual Second Monitor Sender v0.3.0            \x1b[0m");
    println!("\x1b[1;34m  Dual-Engine: Linux Kernel KMS Direct / GNOME Mutter Screencast        \x1b[0m");
    println!("\x1b[1;34m  Multi-GPU (AMD/Intel/NVIDIA) | Multi-Monitor | 100% Native Rust        \x1b[0m");
    println!("\x1b[1;32m========================================================================\x1b[0m");

    // 2. Setup graceful signal handler and acquire single-instance lock
    let running = Arc::new(AtomicBool::new(true));
    setup_signal_handler(running.clone());

    let _pid_lock = match service::acquire_lock() {
        Ok(lock) => lock,
        Err(e) => {
            eprintln!("\x1b[1;31m[!] {}\x1b[0m", e);
            eprintln!("\x1b[1;33m    Para parar a instância existente, execute: ext-sender stop\x1b[0m");
            return Ok(());
        }
    };

    // Auto-Discovery: Se IP for o padrão USB (192.168.7.2) mas não houver resposta, busca via Wi-Fi/Rede
    if cfg.target_ip == "192.168.7.2" {
        if let Some(discovered_ip) = discovery::discover_receiver_ip(Duration::from_millis(1500)) {
            if discovered_ip != cfg.target_ip {
                println!("\x1b[1;32m[+] Auto-Discovery: Conectando ao appliance em {} via rede!\x1b[0m", discovered_ip);
                cfg.target_ip = discovered_ip.clone();
                cfg.transport = TransportKind::Network { ip: discovered_ip, port: cfg.target_port };
            }
        }
    }

    // Start local SSDP / DIAL bridge for instant Google Chrome casting discovery
    discovery::start_host_ssdp_bridge(running.clone(), cfg.target_ip.clone(), 8080);

    // Auto-provision Cast V2 developer certificates into Google Chrome
    if let Err(e) = cast_cert::auto_provision_chrome() {
        eprintln!("\x1b[1;33m[!] Aviso Cast Cert: {}\x1b[0m", e);
    }

    // Start Google Cast V2 (TLS 8009) Server for Chrome Tab and Screen Mirroring
    cast_server::start_cast_v2_server(running.clone());

    // Start background time synchronization with Pi Zero appliance
    time_sync::start_time_sync_daemon(running.clone(), cfg.target_ip.clone(), 8080);

    // Optimize USB interface txqueuelen directly via sysfs in pure Rust (Zero subprocesses)
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("enx") || name.starts_with("usb") {
                let tx_path = entry.path().join("tx_queue_len");
                let _ = std::fs::write(&tx_path, b"100");
            }
        }
    }

    // 3. Initialize Transport (USB Bulk as Default, with Automatic Fallback to Network UDP)
    let usb_writer_alive = Arc::new(AtomicBool::new(false));
    let mut current_usb_pipe: Option<UsbTransportHandle> = match cfg.transport {
        TransportKind::UsbBulk => {
            println!("\x1b[1;33m[*] Transport Mode: USB Bulk Direct (Default - Zero Network Stack)\x1b[0m");
            let pipe_handle = open_usb_pipe_transport(running.clone(), usb_writer_alive.clone());
            if pipe_handle.is_none() {
                println!(
                    "\x1b[1;36m[i] Automatically falling back to Network transport (UDP RTP {}:{})...\x1b[0m",
                    cfg.target_ip, cfg.target_port
                );
                cfg.transport = TransportKind::Network {
                    ip: cfg.target_ip.clone(),
                    port: cfg.target_port,
                };
            }
            pipe_handle
        }
        TransportKind::Network { ref ip, port } => {
            println!("\x1b[1;34m[*] Transport Mode: Network IP (UDP RTP {}:{})\x1b[0m", ip, port);
            None
        }
        TransportKind::Miracast => {
            println!("\x1b[1;32m[*] Transport Mode: Miracast / Wi-Fi Display (Pure-Rust Client)\x1b[0m");
            std::thread::spawn(|| {
                crate::miracast_launcher::launch_miracast_client(Some("192.168.7.2"));
            });
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
    println!("\x1b[1;34m[*] Scale Mode:\x1b[0m {}", cfg.scale.name());
    println!(
        "\x1b[1;34m[*] CAS Sharpening:\x1b[0m {} (Contrast: {:.2}, Saturation: {:.2})",
        if cfg.cas { "Enabled" } else { "Disabled" },
        cfg.contrast,
        cfg.saturation
    );
    let mut audio_tx_running = Arc::new(AtomicBool::new(cfg.audio));
    if cfg.audio {
        println!("\x1b[1;34m[*] Audio Subsystem:\x1b[0m Enabled (100% In-Process Native PCM {}:{} @ {} Hz Hi-Res, Spectrum: 5006)", cfg.target_ip, cfg.audio_port, cfg.audio_rate);
        let _ = audio_native::spawn_native_audio_subsystem(cfg.target_ip.clone(), cfg.audio_port, cfg.audio_rate, audio_tx_running.clone());
    } else {
        println!("\x1b[1;33m[*] Audio Subsystem:\x1b[0m Disabled (--no-audio)");
    }

    let mut is_paused = !cfg.auto_connect || cfg.transport == TransportKind::Miracast;

    let mut monitor_to_record = if cfg.mode == "clone" {
        "eDP-1".to_string()
    } else {
        "HDMI-1".to_string()
    };

    if !is_paused {
        if monitor_to_record == "HDMI-1" {
            pipewire::ensure_kernel_hdmi_connected();
            pipewire::ensure_gnome_displays(cfg.scale);
        } else {
            pipewire::collapse_gnome_displays();
        }
        println!("\x1b[1;34m[*] Recording Monitor:\x1b[0m {}", monitor_to_record);
    } else {
        println!("\x1b[1;33m[i] Ext-Monitor iniciado em modo Standby (auto-connect: desativado).\x1b[0m");
        println!("\x1b[1;36m    Aguardando ativação pelo Painel Web (http://192.168.7.2:8080) ou comando CLI.\x1b[0m");
        pipewire::collapse_gnome_displays();
    }

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

    // 4.1 Wayland Damage Pacer (Optional - only if explicitly enabled via --enable-damage-pacer and capture is Mutter):
    // By default KMS direct capture does not require X11 damage events and avoids dock window tracking glitches.
    let _pacer_handle = if cfg.enable_damage_pacer && cfg.capture == CaptureEngine::Mutter {
        let (pacer_x, pacer_y) = if monitor_to_record == "HDMI-1" {
            (1920 + 1280 - 2, 720 - 2)
        } else {
            (1280 - 2, 720 - 2)
        };
        damage_pacer::spawn_damage_pacer(running.clone(), pacer_x, pacer_y)
    } else {
        None
    };

    // 5. Main Supervisor Loop (Reconnects on Suspend/Resume or System Event)
    while running.load(Ordering::SeqCst) {
        if is_paused {
            // Idle Standby Loop: poll UDP control listener without starting screencast session or pipelines
            while running.load(Ordering::SeqCst) && is_paused {
                for action in ctrl_listener.poll_actions() {
                    match action {
                        ControlAction::StartStreaming => {
                            println!("\x1b[1;32m[+] Comando recebido: Iniciar Transmissão!\x1b[0m");
                            if monitor_to_record == "HDMI-1" {
                                pipewire::ensure_kernel_hdmi_connected();
                                pipewire::ensure_gnome_displays(cfg.scale);
                            } else {
                                pipewire::collapse_gnome_displays();
                            }
                            is_paused = false;
                            break;
                        }
                        ControlAction::ChromeCastLaunch => {
                            println!("\x1b[1;32m[+] Chrome Cast LAUNCH recebido: Seleção interativa de tela...\x1b[0m");
                            if let Some(selected) = prompt_user_mode_selection() {
                                if selected == "window" || selected == "tab" {
                                    println!("\x1b[1;36m[*] Abrindo Web Caster no navegador para transmissão de {}...\x1b[0m", selected);
                                    let _ = std::process::Command::new("xdg-open")
                                        .arg("http://192.168.7.2:8080/cast")
                                        .env("WAYLAND_DISPLAY", std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".to_string()))
                                        .env("DISPLAY", std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string()))
                                        .spawn();
                                    pipewire::collapse_gnome_displays();
                                    continue;
                                } else if selected == "clone" {
                                    monitor_to_record = "eDP-1".to_string();
                                    pipewire::collapse_gnome_displays();
                                } else {
                                    monitor_to_record = "HDMI-1".to_string();
                                    pipewire::ensure_kernel_hdmi_connected();
                                    pipewire::ensure_gnome_displays(cfg.scale);
                                }
                                is_paused = false;
                                break;
                            } else {
                                println!("\x1b[1;33m[!] Chrome Cast cancelado pelo usuário no diálogo.\x1b[0m");
                                continue;
                            }
                        }
                        ControlAction::StopStreaming => {
                            pipewire::collapse_gnome_displays();
                            notify_receiver_stop(&cfg.target_ip);
                        }
                        ControlAction::SetMode(ref m) => {
                            cfg.mode = m.clone();
                            if m == "clone" {
                                monitor_to_record = "eDP-1".to_string();
                                pipewire::collapse_gnome_displays();
                                is_paused = false;
                                break;
                            } else if m == "extend" {
                                monitor_to_record = "HDMI-1".to_string();
                                pipewire::ensure_kernel_hdmi_connected();
                                pipewire::ensure_gnome_displays(cfg.scale);
                                is_paused = false;
                                break;
                            } else if m == "ask" || m == "interactive" {
                                println!("\x1b[1;32m[+] Modo configurado para Perguntar ao Iniciar (Interativo).\x1b[0m");
                            }
                        }
                        ControlAction::SetTransport(new_trans) => {
                            println!("\x1b[1;35m[*] Standby: Transporte atualizado para {:?}\x1b[0m", new_trans);
                            cfg.transport = new_trans;
                        }
                        _ => {}
                    }
                }
                if is_paused {
                    thread::sleep(Duration::from_millis(150));
                }
            }
            if !running.load(Ordering::SeqCst) {
                break;
            }
        }

        let (_screencast_session, node_id, kms_info) = match cfg.capture {
            CaptureEngine::Kms => {
                println!("\x1b[1;34m[*] Motor KMS Direct: Descobrindo conector DRM/KMS para '{}'...\x1b[0m", monitor_to_record);
                let info = match kms::KmsOutputInfo::discover(&monitor_to_record) {
                    Ok(i) => {
                        println!("\x1b[1;32m[+] DRM/KMS Conector: {} | CRTC: {} | Resolução: {}x{}@{}Hz\x1b[0m", i.connector_name, i.crtc_id, i.width, i.height, i.vrefresh);
                        Some(i)
                    }
                    Err(e) => {
                        eprintln!("\x1b[1;33m[!] DRM/KMS aviso: {} (prosseguindo com captura Wayland)\x1b[0m", e);
                        None
                    }
                };

                println!("\x1b[1;34m[*] Conectando scanout via PipeWire D-Bus...\x1b[0m");
                let session = match MutterScreenCastSession::create_and_start(&monitor_to_record) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("\x1b[1;31m[!] Screencast session creation failed: {}. Retrying in 2s...\x1b[0m", e);
                        thread::sleep(Duration::from_secs(2));
                        continue;
                    }
                };
                let nid = session.node_id;
                (Some(session), nid, info)
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
            usb_pipe_fd: current_usb_pipe.as_ref().map(|h| h.write_fd),
            engine: cfg.engine,
            capture: cfg.capture,
            kms_info: kms_info.clone(),
            audio: cfg.audio,
            audio_port: cfg.audio_port,
            scale: cfg.scale,
            cas: cfg.cas,
            contrast: cfg.contrast,
            saturation: cfg.saturation,
        };

        let mut child: Option<StreamerHandle> = if !is_paused {
            println!(
                "\x1b[1;33m[*] Starting {:?} hardware streaming pipeline via {} [Capture: {:?}] ({} FPS, Stream: {}, HUD: {})...\x1b[0m",
                cfg.encoder,
                cfg.engine.name(),
                cfg.capture,
                cfg.fps,
                if cfg.drop_only { "Economy (drop-only)" } else { "Continuous (CFR Anti-Freeze)" },
                cfg.hud
            );

            match pipeline_builder.spawn() {
                Ok(c) => {
                    thread::sleep(Duration::from_millis(500));
                    pipewire::link_monitor_port_to_sender(node_id, &monitor_to_record);
                    println!(
                        "\x1b[1;32m[+] Monitor {} is streaming LIVE to Pi Zero at {} FPS ({}) via {:?}!\x1b[0m",
                        monitor_to_record, cfg.fps, cfg.color_profile.name(), cfg.capture
                    );
                    Some(c)
                }
                Err(e) => {
                    eprintln!("\x1b[1;31m[!] Failed to spawn streamer: {}. Retrying in 2s...\x1b[0m", e);
                    thread::sleep(Duration::from_secs(2));
                    continue;
                }
            }
        } else {
            println!("\x1b[1;33m[*] ext-sender em Standby / Pausa (aguardando 'Start' ou parâmetros via Painel Web http://192.168.7.2:8080)...\x1b[0m");
            None
        };

        let mut last_node_check = Instant::now();
        let mut last_usb_reconnect = Instant::now();
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
                        println!("\x1b[1;32m[+] Web Command: Iniciar / Retomar Transmissão recebido!\x1b[0m");
                        if monitor_to_record == "HDMI-1" {
                            pipewire::ensure_kernel_hdmi_connected();
                            pipewire::ensure_gnome_displays(cfg.scale);
                        } else {
                            pipewire::collapse_gnome_displays();
                        }
                        if cfg.transport == TransportKind::Miracast {
                            println!("\x1b[1;35m[*] Mode 2 Miracast ativo: Disparando cliente Miracast nativo pure-Rust...\x1b[0m");
                            is_paused = true;
                            close_usb_transport(&mut current_usb_pipe, &usb_writer_alive);
                            pipeline_builder.usb_pipe_fd = None;
                            std::thread::spawn(|| {
                                crate::miracast_launcher::launch_miracast_client(Some("192.168.7.2"));
                            });
                        } else {
                            is_paused = false;
                            restart_pipeline = true;
                        }
                    }
                    ControlAction::ChromeCastLaunch => {
                        println!("\x1b[1;32m[+] Chrome Cast LAUNCH recebido durante execução: Seleção de tela...\x1b[0m");
                        if let Some(selected) = prompt_user_mode_selection() {
                            if selected == "window" || selected == "tab" {
                                println!("\x1b[1;36m[*] Abrindo Web Caster no navegador para transmissão de {}...\x1b[0m", selected);
                                let _ = std::process::Command::new("xdg-open")
                                    .arg("http://192.168.7.2:8080/cast")
                                    .env("WAYLAND_DISPLAY", std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".to_string()))
                                    .env("DISPLAY", std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string()))
                                    .spawn();
                                pipewire::collapse_gnome_displays();
                                is_paused = true;
                                break;
                            }
                            let target_mon = if selected == "clone" { "eDP-1".to_string() } else { "HDMI-1".to_string() };
                            if target_mon == "HDMI-1" {
                                pipewire::ensure_kernel_hdmi_connected();
                                pipewire::ensure_gnome_displays(cfg.scale);
                            } else {
                                pipewire::collapse_gnome_displays();
                            }
                            if target_mon != monitor_to_record {
                                monitor_to_record = target_mon;
                                switch_engine_or_monitor = true;
                            }
                            is_paused = false;
                            restart_pipeline = true;
                        } else {
                            println!("\x1b[1;33m[!] Chrome Cast cancelado pelo usuário no diálogo.\x1b[0m");
                        }
                    }
                    ControlAction::StopStreaming => {
                        println!("\x1b[1;33m[*] Web Command: Parar Transmissão / Standby recebido! Encerrando transmissores e recolhendo display...\x1b[0m");
                        crate::miracast_launcher::stop_miracast_client();
                        if let Some(mut c) = child.take() {
                            let _ = c.kill();
                            let _ = c.wait();
                        }
                        let _ = std::process::Command::new("pkill")
                            .arg("-f")
                            .arg("gst-launch-1.0.*192.168.7.2")
                            .output();
                        close_usb_transport(&mut current_usb_pipe, &usb_writer_alive);
                        pipeline_builder.usb_pipe_fd = None;
                        is_paused = true;
                        pipewire::collapse_gnome_displays();
                        notify_receiver_stop(&cfg.target_ip);
                        println!("\x1b[1;32m[*] Todos os transmissores de vídeo do Host foram finalizados e tela estendida recolhida do GNOME Mutter. Modo Standby ativo.\x1b[0m");
                        break;
                    }
                    ControlAction::SetMode(m) => {
                        if m == "miracast" || m == "wfd" || m == "mode2" || m == "2" {
                            println!("\x1b[1;35m[*] Web Command: Troca de Modo para Miracast (Mode 2)\x1b[0m");
                            is_paused = true;
                            cfg.transport = TransportKind::Miracast;
                            close_usb_transport(&mut current_usb_pipe, &usb_writer_alive);
                            pipeline_builder.usb_pipe_fd = None;
                            restart_pipeline = true;
                            std::thread::spawn(|| {
                                crate::miracast_launcher::launch_miracast_client(Some("192.168.7.2"));
                            });
                        } else if m == "ask" || m == "interactive" {
                            cfg.mode = "ask".to_string();
                            println!("\x1b[1;32m[+] Modo configurado: Perguntar ao Iniciar (Interativo)\x1b[0m");
                        } else {
                            let target_mon = if m == "clone" { "eDP-1".to_string() } else { "HDMI-1".to_string() };
                            if target_mon == "eDP-1" {
                                pipewire::collapse_gnome_displays();
                            } else {
                                pipewire::ensure_kernel_hdmi_connected();
                                pipewire::ensure_gnome_displays(cfg.scale);
                            }
                            if target_mon != monitor_to_record || m != cfg.mode {
                                println!("\x1b[1;35m[*] Web Command: Troca de Modo '{}' -> '{}' (Monitor: {})\x1b[0m", cfg.mode, m, target_mon);
                                cfg.mode = m;
                                monitor_to_record = target_mon;
                                switch_engine_or_monitor = true;
                            } else {
                                println!("\x1b[1;35m[*] Web Command: Reativando Modo '{}' (Monitor: {})\x1b[0m", m, target_mon);
                                is_paused = false;
                                restart_pipeline = true;
                            }
                        }
                    }
                    ControlAction::SetAudio(a) => {
                        if a != cfg.audio {
                            println!("\x1b[1;35m[*] Web Command: Áudio {} -> {}\x1b[0m", cfg.audio, a);
                            cfg.audio = a;
                            pipeline_builder.audio = a;
                            audio_tx_running.store(false, Ordering::SeqCst);
                            if a {
                                audio_tx_running = Arc::new(AtomicBool::new(true));
                                let _ = audio_native::spawn_native_audio_subsystem(cfg.target_ip.clone(), cfg.audio_port, cfg.audio_rate, audio_tx_running.clone());
                            }
                        }
                    }
                    ControlAction::SetAudioRate(rate) => {
                        if [44100, 48000, 88200, 96000, 192000].contains(&rate) && rate != cfg.audio_rate {
                            println!("\x1b[1;35m[*] Web Command: Switching Audio Sample Rate {} Hz -> {} Hz\x1b[0m", cfg.audio_rate, rate);
                            cfg.audio_rate = rate;
                            if cfg.audio {
                                audio_tx_running.store(false, Ordering::SeqCst);
                                thread::sleep(Duration::from_millis(80));
                                audio_tx_running = Arc::new(AtomicBool::new(true));
                                let _ = audio_native::spawn_native_audio_subsystem(cfg.target_ip.clone(), cfg.audio_port, cfg.audio_rate, audio_tx_running.clone());
                            }
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
                    ControlAction::LaunchMiracast => {
                        println!("\x1b[1;35m[*] Web Command: Lançar Miracast (cliente nativo pure-Rust)\x1b[0m");
                        is_paused = true;
                        cfg.transport = TransportKind::Miracast;
                        close_usb_transport(&mut current_usb_pipe, &usb_writer_alive);
                        pipeline_builder.usb_pipe_fd = None;
                        restart_pipeline = true;

                        std::thread::spawn(|| {
                            crate::miracast_launcher::launch_miracast_client(Some("192.168.7.2"));
                        });
                    }
                    ControlAction::SetTransport(new_trans) => {
                        println!("\x1b[1;35m[*] Web Command: Troca de Transporte {:?} -> {:?}\x1b[0m", cfg.transport, new_trans);
                        match new_trans {
                            TransportKind::Miracast => {
                                is_paused = true;
                                cfg.transport = TransportKind::Miracast;
                                close_usb_transport(&mut current_usb_pipe, &usb_writer_alive);
                                pipeline_builder.usb_pipe_fd = None;
                                restart_pipeline = true;

                                // Auto-launch native pure-Rust Miracast client with GPU hardware acceleration
                                std::thread::spawn(|| {
                                    crate::miracast_launcher::launch_miracast_client(Some("192.168.7.2"));
                                });
                            }
                            TransportKind::UsbBulk => {
                                crate::miracast_launcher::stop_miracast_client();
                                is_paused = false;
                                cfg.transport = TransportKind::UsbBulk;
                                close_usb_transport(&mut current_usb_pipe, &usb_writer_alive);
                                pipeline_builder.usb_pipe_fd = None;
                                restart_pipeline = true;
                            }
                            TransportKind::Network { ref ip, port } => {
                                crate::miracast_launcher::stop_miracast_client();
                                is_paused = false;
                                cfg.transport = new_trans.clone();
                                close_usb_transport(&mut current_usb_pipe, &usb_writer_alive);
                                pipeline_builder.usb_pipe_fd = None;
                                pipeline_builder.target_ip = ip.clone();
                                pipeline_builder.target_port = port;
                                restart_pipeline = true;
                            }
                        }
                    }
                    ControlAction::SetScale(new_scale) => {
                        if new_scale != cfg.scale {
                            println!("\x1b[1;35m[*] Web Command: Troca de Escala {:?} -> {:?}\x1b[0m", cfg.scale, new_scale);
                            cfg.scale = new_scale;
                            pipeline_builder.scale = new_scale;
                            pipewire::ensure_gnome_displays(new_scale);
                            restart_pipeline = true;
                        }
                    }
                    ControlAction::SetCas(new_cas) => {
                        if new_cas != cfg.cas {
                            println!("\x1b[1;35m[*] Web Command: Realce de Nitidez CAS {} -> {}\x1b[0m", cfg.cas, new_cas);
                            cfg.cas = new_cas;
                            pipeline_builder.cas = new_cas;
                            cfg.contrast = if new_cas { 1.16 } else { 1.0 };
                            cfg.saturation = if new_cas { 1.08 } else { 1.0 };
                            pipeline_builder.contrast = cfg.contrast;
                            pipeline_builder.saturation = cfg.saturation;
                            restart_pipeline = true;
                        }
                    }
                    ControlAction::SetAutoConnect(ac) => {
                        println!("\x1b[1;35m[*] Web Command: Configuração Auto-Connect {} -> {}\x1b[0m", cfg.auto_connect, ac);
                        cfg.auto_connect = ac;
                    }
                }
            }

            // Health watchdog: Detect suspend/resume or PipeWire crash (when using PipeWire capture)
            let uses_pipewire = cfg.capture == CaptureEngine::Mutter || kms_info.is_none();
            if !is_paused && uses_pipewire {
                if last_node_check.elapsed() >= Duration::from_millis(1500) {
                    last_node_check = Instant::now();
                    if !pipewire::is_pipewire_node_alive(node_id) {
                        println!("\x1b[1;31m[!] Screencast node {} disappeared. Reconnecting...\x1b[0m", node_id);
                        if let Some(mut c) = child.take() {
                            let _ = c.kill();
                            let _ = c.wait();
                        }
                        close_usb_transport(&mut current_usb_pipe, &usb_writer_alive);
                        thread::sleep(Duration::from_millis(1500));
                        break;
                    }
                }
            }

            // Health watchdog: Detect USB Bulk disconnect / Pi Zero reboot
            if !is_paused && cfg.transport == TransportKind::UsbBulk && !usb_writer_alive.load(Ordering::SeqCst) {
                if last_usb_reconnect.elapsed() >= Duration::from_millis(2000) {
                    last_usb_reconnect = Instant::now();
                    close_usb_transport(&mut current_usb_pipe, &usb_writer_alive);
                    if let Some(new_pipe) = open_usb_pipe_transport(running.clone(), usb_writer_alive.clone()) {
                        println!("\x1b[1;32m[*] USB Bulk reconectado com sucesso! Reiniciando pipeline...\x1b[0m");
                        pipeline_builder.usb_pipe_fd = Some(new_pipe.write_fd);
                        current_usb_pipe = Some(new_pipe);
                        restart_pipeline = true;
                    } else if let Some(mut c) = child.take() {
                        let _ = c.kill();
                        let _ = c.wait();
                        pipeline_builder.usb_pipe_fd = None;
                    }
                }
            }

            if switch_engine_or_monitor {
                println!("\x1b[1;33m[*] Reiniciando supervisor para nova engine/monitor...\x1b[0m");
                if let Some(mut c) = child.take() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                close_usb_transport(&mut current_usb_pipe, &usb_writer_alive);
                break;
            }

            if restart_pipeline {
                if !is_paused {
                    println!("\x1b[1;36m[*] Hot-applying configuration (FPS: {}, Bitrate: {}k, Color: {:?})...\x1b[0m", cfg.fps, cfg.bitrate, cfg.color_profile);
                    if let Some(mut c) = child.take() {
                        let _ = c.kill();
                        let _ = c.wait();
                    }
                    close_usb_transport(&mut current_usb_pipe, &usb_writer_alive);
                    if cfg.transport == TransportKind::UsbBulk {
                        if current_usb_pipe.is_none() {
                            current_usb_pipe = open_usb_pipe_transport(running.clone(), usb_writer_alive.clone());
                        }
                        pipeline_builder.usb_pipe_fd = current_usb_pipe.as_ref().map(|h| h.write_fd);
                        if pipeline_builder.usb_pipe_fd.is_none() {
                            println!("\x1b[1;33m[!] USB Bulk indisponível no momento. Aguardando reconexão...\x1b[0m");
                            child = None;
                            continue;
                        }
                    } else {
                        pipeline_builder.usb_pipe_fd = None;
                    }

                    child = match pipeline_builder.spawn() {
                        Ok(c) => Some(c),
                        Err(e) => {
                            eprintln!("\x1b[1;31m[!] Failed to restart streamer: {}\x1b[0m", e);
                            break;
                        }
                    };
                    if uses_pipewire {
                        thread::sleep(Duration::from_millis(500));
                        pipewire::link_monitor_port_to_sender(node_id, &monitor_to_record);
                    }
                    println!("\x1b[1;32m[+] Configuration hot-applied successfully.\x1b[0m");
                }
                continue;
            }

            if let Some(ref mut c) = child {
                match c.try_wait() {
                    Ok(Some(status)) => {
                        if !is_paused {
                            println!("\x1b[1;33m[*] Streamer exited with status: {}. Waiting 1s before reconnecting...\x1b[0m", status);
                            thread::sleep(Duration::from_secs(1));
                            break;
                        }
                    }
                    Ok(None) => thread::sleep(Duration::from_millis(200)),
                    Err(e) => {
                        eprintln!("\x1b[1;31m[!] Error monitoring streamer: {}\x1b[0m", e);
                        break;
                    }
                }
            } else {
                // Em standby / pausa
                thread::sleep(Duration::from_millis(200));
            }

            // Telemetria Periódica de Diagnóstico (a cada 2.5s)
            if !is_paused && last_telemetry_check.elapsed() >= Duration::from_millis(2500) {
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

        if let Some(mut c) = child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        close_usb_transport(&mut current_usb_pipe, &usb_writer_alive);

        if !running.load(Ordering::SeqCst) {
            break;
        }

        thread::sleep(Duration::from_secs(1));
    }

    pipewire::collapse_gnome_displays();
    crate::miracast_launcher::stop_miracast_client();
    close_usb_transport(&mut current_usb_pipe, &usb_writer_alive);
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

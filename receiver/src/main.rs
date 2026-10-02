//! Raspberry Pi Zero GPU Hardware Display Receiver
//!
//! A unified, 100% native Rust video display receiver designed for Raspberry Pi Zero W / 2.
//! Renders H.264 video streams directly to HDMI via Broadcom VideoCore IV (V4L2 M2M & KMS DRM).
//!
//! Core Subsystems:
//! 1. Mode 1 (Network + Miracast Hybrid):
//!    - Linux Wayland streaming via UDP port 5000 (RTP H.264).
//!    - Windows 10/11 native wireless casting via RTSP port 7236 (Win + K).
//!    - Embedded HTTP Web Dashboard on port 8080 with multilingual UI (EN, PT, IT, ZH).
//! 2. Mode 2 (USB Bulk Direct):
//!    - Direct USB FunctionFS (`f_fs`) streaming bypassing TCP/IP and UDP.
//! 3. Built-in Headless DRM Guard:
//!    - Automatically forces DRM HDMI status 'on' when no physical monitor is connected.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

mod decoder;
pub mod discovery;
mod display;
mod drm;
pub mod audio;
pub mod dhcp;
mod i18n;
mod ingress;
mod native_v4l2;
mod pipeline;
pub mod service;
mod stream;
mod usb_bulk;
pub mod swagger;
mod web;
mod web_ui;
mod wfd;
pub mod media_renderer;
pub mod mdns;
pub mod web_cast;
pub mod cast_proxy;
pub mod flow;

use i18n::Language;
use pipeline::{PipelineBackend, PipelineKind, PipelineManager};
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check for help flag
    if args.iter().any(|a| a == "--help" || a == "-h") {
        let lang = args
            .iter()
            .find(|a| a.starts_with("--lang="))
            .map(|a| Language::from_str(&a[7..]))
            .unwrap_or_else(Language::detect);
        i18n::print_help(lang);
        return;
    }

    // Subcomandos de controle de serviço para máquinas x86 / PC não-Pi Zero
    if let Some(cmd) = args.get(1) {
        match cmd.as_str() {
            "stop" | "--stop" => {
                let _ = service::stop_all();
                return;
            }
            "start" | "--start" => {
                if let Err(e) = service::start_service() {
                    eprintln!("\x1b[1;31m[!] {}\x1b[0m", e);
                }
                return;
            }
            "install" | "--install" => {
                match service::install_service() {
                    Ok(p) => println!("\x1b[1;32m[+] ext-receiver serviço instalado em {:?}!\x1b[0m", p),
                    Err(e) => eprintln!("\x1b[1;31m[!] {}\x1b[0m", e),
                }
                return;
            }
            _ => {}
        }
    }

    // Anti-duplicação via PID Lock
    let _pid_lock = match service::acquire_lock() {
        Ok(lock) => lock,
        Err(e) => {
            eprintln!("\x1b[1;31m[!] {}\x1b[0m", e);
            eprintln!("\x1b[1;33m    Para parar a instância existente: ext-receiver stop\x1b[0m");
            return;
        }
    };

    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m[ext-receiver]\x1b[0m Raspberry Pi Zero GPU Display Receiver v0.2.0");
    println!("\x1b[1;34m[ext-receiver]\x1b[0m 100% Native Rust | VideoCore IV V4L2 M2M | KMS DRM Output");
    println!("\x1b[1;32m========================================================================\x1b[0m");

    // 0. Immediately display hardware loading splash ("Aguarde carregando..." in 4 languages)
    display::SplashEngine::show_loading();

    let is_usb_bulk_mode = args.iter().any(|arg| arg == "--mode=usb-bulk" || arg == "-b");
    let udp_port = args
        .get(1)
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(5000);

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    setup_signal_handler(r);

    // Ensure DRM KMS connector is forced 'on' for headless operation
    drm::ensure_drm_hdmi_connected();

    let pipeline_mgr = Arc::new(PipelineManager::new());
    if let Some(backend_arg) = args.iter().find(|a| a.starts_with("--backend=") || a.starts_with("--engine=")) {
        let val = backend_arg.strip_prefix("--backend=").or_else(|| backend_arg.strip_prefix("--engine=")).unwrap_or("");
        pipeline_mgr.set_backend(PipelineBackend::from_str(val));
    }
    if let Some(rate_arg) = args.iter().find(|a| a.starts_with("--audio-rate=")) {
        if let Some(rate_str) = rate_arg.strip_prefix("--audio-rate=") {
            if let Ok(r) = rate_str.parse::<u32>() {
                pipeline_mgr.set_audio_rate(r);
            }
        }
    }

    // 1. Start embedded Web Control Server on HTTP port 8080 (always available for telemetry and control)
    if let Err(e) = web::start_web_server(running.clone(), pipeline_mgr.clone(), udp_port) {
        eprintln!("\x1b[1;31m[ext-receiver]\x1b[0m Failed to start embedded web server: {}", e);
    }

    // 2. Start Wi-Fi Display (Miracast / MS-MICE) RTSP server on TCP port 7236
    if let Err(e) = wfd::start_wfd_server(running.clone(), pipeline_mgr.clone()) {
        eprintln!("\x1b[1;31m[ext-receiver]\x1b[0m Failed to start Miracast server: {}", e);
    }

    // 3. Start native pure-Rust Zero-Gateway DHCP server for usb0
    dhcp::start_dhcp_server(running.clone());

    // 4. Start UPnP / DLNA SSDP auto-discovery daemon on UDP 1900
    media_renderer::start_ssdp_responder(running.clone(), 8080);

    // 4.1 Start Ext-Monitor Wi-Fi/Ethernet auto-discovery beacon on UDP 5005
    discovery::start_discovery_beacon(running.clone(), 8080, udp_port);

    // 4.2 Start pure-Rust mDNS Google Cast / Miracast responder on UDP 5353
    mdns::start_mdns_responder(running.clone());

    // 4.3 Start Google Cast V2 TCP forwarder on port 8009 -> 192.168.7.1:8009
    cast_proxy::start_cast_forwarder(running.clone(), "192.168.7.1", 8009);

    // 5. Start HDMI dynamic visualizer engine (active when playing audio, multiplexed with video)
    media_renderer::VisualizerEngine::start(running.clone(), pipeline_mgr.clone());
    media_renderer::start_audio_telemetry_listener(running.clone());
    pipeline_mgr.start_audio();

    if let Ok(mut cfg) = web::CONFIG.lock() {
        cfg.mode3 = is_usb_bulk_mode;
        cfg.mode1 = !is_usb_bulk_mode;
        cfg.mode2 = !is_usb_bulk_mode;
        cfg.active_transport = if is_usb_bulk_mode {
            "mode3_usb_bulk".to_string()
        } else {
            "mode1_udp".to_string()
        };
    }

    if is_usb_bulk_mode {
        println!("\x1b[1;33m[ext-receiver]\x1b[0m Active Mode: MODE 3 (Direct USB Bulk via FunctionFS)");
        if let Err(e) = usb_bulk::activate_usb_bulk(running.clone(), pipeline_mgr.clone()) {
            eprintln!("\x1b[1;31m[ext-receiver]\x1b[0m Failed to activate initial USB Bulk: {}", e);
        }
    } else {
        println!("\x1b[1;33m[ext-receiver]\x1b[0m Active Mode: MODE 1 (Network + Miracast Hybrid)");
        println!("\x1b[1;34m[ext-receiver]\x1b[0m Linux Channel: UDP port {}", udp_port);
        println!("\x1b[1;34m[ext-receiver]\x1b[0m Windows Channel: RTSP port 7236 (Win + K)");

        let default_kind = PipelineKind::RawH264Rtp { port: udp_port };
        if let Err(e) = pipeline_mgr.start(default_kind) {
            eprintln!("\x1b[1;31m[ext-receiver]\x1b[0m Failed to start initial UDP pipeline: {}", e);
        }

        // Show Ready splash screen (waiting for incoming stream)
        display::SplashEngine::show_ready();
    }

    // Supervisor loop: restores active decode pipeline if session ends or pipeline exits
    while running.load(Ordering::SeqCst) {
        if !pipeline_mgr.is_paused() {
            let active_trans = web::CONFIG.lock().map(|c| c.active_transport.clone()).unwrap_or_default();
            // In Miracast mode (Mode 2), pipeline kind is None until a client connects via RTSP; do NOT auto-switch to UDP!
            if active_trans != "mode2_miracast" {
                let is_idle = pipeline_mgr.current_kind().is_none();
                let has_crashed = pipeline_mgr.has_exited();

                if is_idle || has_crashed {
                    let want_bulk = if !active_trans.is_empty() {
                        active_trans == "mode3_usb_bulk"
                    } else {
                        web::CONFIG.lock().map(|c| c.mode3).unwrap_or(is_usb_bulk_mode)
                    };

                    if want_bulk {
                        println!("\x1b[1;33m[ext-receiver]\x1b[0m Restoring USB Bulk pipeline...");
                        if let Err(e) = usb_bulk::activate_usb_bulk(running.clone(), pipeline_mgr.clone()) {
                            eprintln!("\x1b[1;31m[ext-receiver]\x1b[0m Failed to restore USB Bulk: {} (retrying in 2s)", e);
                            thread::sleep(Duration::from_secs(2));
                        }
                    } else {
                        let default_kind = PipelineKind::RawH264Rtp { port: udp_port };
                        println!("\x1b[1;33m[ext-receiver]\x1b[0m Restoring default Linux UDP pipeline (port {})...", udp_port);
                        if let Err(e) = pipeline_mgr.start(default_kind) {
                            eprintln!("\x1b[1;31m[ext-receiver]\x1b[0m Pipeline start failed: {} (retrying in 2s)", e);
                            thread::sleep(Duration::from_secs(2));
                        }
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(500));
    }

    pipeline_mgr.stop();

    println!("\x1b[1;32m[ext-receiver]\x1b[0m Receiver terminated cleanly.");
}

/// Set up OS signal handlers for graceful shutdown (SIGINT & SIGTERM)
fn setup_signal_handler(running: Arc<AtomicBool>) {
    unsafe {
        register_libc_signal(libc::SIGINT, signal_handler);
        register_libc_signal(libc::SIGTERM, signal_handler);
    }
    let r = running.clone();
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

unsafe fn register_libc_signal(sig: libc::c_int, handler: extern "C" fn(libc::c_int)) {
    libc::signal(sig, handler as usize);
}

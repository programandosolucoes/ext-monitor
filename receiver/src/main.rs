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
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

mod drm;
mod i18n;
mod native_v4l2;
mod pipeline;
mod usb_bulk;
mod web;
mod web_ui;
mod wfd;

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

    println!("\x1b[1;32m========================================================================\x1b[0m");
    println!("\x1b[1;32m[ext-receiver]\x1b[0m Raspberry Pi Zero GPU Display Receiver v0.2.0");
    println!("\x1b[1;34m[ext-receiver]\x1b[0m 100% Native Rust | VideoCore IV V4L2 M2M | KMS DRM Output");
    println!("\x1b[1;32m========================================================================\x1b[0m");

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
        let val = if backend_arg.starts_with("--backend=") { &backend_arg[10..] } else { &backend_arg[9..] };
        pipeline_mgr.set_backend(PipelineBackend::from_str(val));
    }

    if is_usb_bulk_mode {
        println!("\x1b[1;33m[ext-receiver]\x1b[0m Active Mode: MODE 2 (Direct USB Bulk via FunctionFS)");
        if let Err(e) = usb_bulk::run_usb_bulk_receiver(running.clone(), pipeline_mgr.clone()) {
            eprintln!("\x1b[1;31m[ext-receiver]\x1b[0m Fatal USB Bulk error: {}", e);
        }
    } else {
        println!("\x1b[1;33m[ext-receiver]\x1b[0m Active Mode: MODE 1 (Network + Miracast Hybrid)");
        println!("\x1b[1;34m[ext-receiver]\x1b[0m Linux Channel: UDP port {}", udp_port);
        println!("\x1b[1;34m[ext-receiver]\x1b[0m Windows Channel: RTSP port 7236 (Win + K)");

        // 1. Start embedded Web Control Server on HTTP port 8080
        if let Err(e) = web::start_web_server(running.clone(), pipeline_mgr.clone(), udp_port) {
            eprintln!("\x1b[1;31m[ext-receiver]\x1b[0m Failed to start embedded web server: {}", e);
        }

        // 2. Start Wi-Fi Display (Miracast / MS-MICE) RTSP server on TCP port 7236
        if let Err(e) = wfd::start_wfd_server(running.clone(), pipeline_mgr.clone()) {
            eprintln!("\x1b[1;31m[ext-receiver]\x1b[0m Failed to start Miracast server: {}", e);
        }

        // 3. Start default Linux Wayland video decode pipeline
        let default_kind = PipelineKind::RawH264Rtp { port: udp_port };
        if let Err(e) = pipeline_mgr.start(default_kind) {
            eprintln!("\x1b[1;31m[ext-receiver]\x1b[0m Failed to start initial UDP pipeline: {}", e);
        }

        // 4. Supervisor loop: restores default Linux channel if Miracast session ends or pipeline restarts
        while running.load(Ordering::SeqCst) {
            if !pipeline_mgr.is_paused() {
                let is_idle = pipeline_mgr.current_kind().is_none();
                let has_crashed = pipeline_mgr.has_exited();

                if is_idle || has_crashed {
                    println!("\x1b[1;33m[ext-receiver]\x1b[0m Restoring default Linux UDP pipeline (port {})...", udp_port);
                    let _ = pipeline_mgr.start(default_kind);
                }
            }
            thread::sleep(Duration::from_millis(500));
        }

        pipeline_mgr.stop();
    }

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

//! Embedded Web Dashboard & Control Server in 100% Pure Rust
//!
//! Serves the embedded HTML/CSS/JS control panel directly from binary memory on HTTP port 8080.
//! Eliminates external static files and Python web servers completely.
//!
//! Supports:
//! - Multilingual UI (English, Portuguese, Italian, Chinese)
//! - Real-time hardware telemetry API (`GET /api/status`)
//! - Hot-apply stream configuration via UDP 5001 (`POST /api/config`)
//! - Display pause/resume controls (`POST /api/stream/stop`, `POST /api/stream/start`)
//! - Graceful panel & service shutdown (`POST /api/service/stop`)
//! - Dual-mode switching (`POST /api/mode`)
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use crate::pipeline::{PipelineKind, PipelineManager};
use crate::web_ui::DASHBOARD_HTML;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub const HTTP_PORT: u16 = 8080;

/// Represents Configstate configuration and operational state.
pub struct ConfigState {
    pub fps: u32,
    pub bitrate: u32,
    pub color: String,
    pub drop_only: bool,
    pub skip_to_first: bool,
    pub key_int_max: u32,
    pub capture: String,
    pub monitor: String,
    pub mode1: bool,
    pub mode2: bool,
    pub mode3: bool,
    pub active_transport: String,
}

pub static CONFIG: Mutex<ConfigState> = Mutex::new(ConfigState {
    fps: 30,
    bitrate: 400,
    color: String::new(),
    drop_only: false,
    skip_to_first: true,
    key_int_max: 30,
    capture: String::new(),
    monitor: String::new(),
    mode1: true,
    mode2: true,
    mode3: false,
    active_transport: String::new(),
});

/// Start the embedded HTTP dashboard server in a background thread
pub fn start_web_server(
    running: Arc<AtomicBool>,
    pipeline_mgr: Arc<PipelineManager>,
    default_udp_port: u16,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", HTTP_PORT))?;
    listener.set_nonblocking(true)?;

    println!(
        "\x1b[1;32m[web-server]\x1b[0m Embedded Web Dashboard active at http://0.0.0.0:{}",
        HTTP_PORT
    );

    // Optional Google Cast port 8008 listener
    if let Ok(cast_listener) = TcpListener::bind("0.0.0.0:8008") {
        let _ = cast_listener.set_nonblocking(true);
        println!("\x1b[1;32m[web-server]\x1b[0m Google Cast HTTP service active on http://0.0.0.0:8008");
        let pipe_cast = pipeline_mgr.clone();
        let run_cast = running.clone();
        thread::spawn(move || {
            while run_cast.load(Ordering::SeqCst) {
                match cast_listener.accept() {
                    Ok((stream, _addr)) => {
                        let pipe = pipe_cast.clone();
                        let run = run_cast.clone();
                        thread::spawn(move || {
                            handle_http_client(stream, pipe, run, default_udp_port);
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(500));
                    }
                }
            }
        });
    }

    let run_loop = running.clone();
    thread::spawn(move || {
        while run_loop.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _addr)) => {
                    let pipe = pipeline_mgr.clone();
                    let run = run_loop.clone();
                    thread::spawn(move || {
                        handle_http_client(stream, pipe, run, default_udp_port);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    eprintln!("\x1b[1;31m[web-server]\x1b[0m Accept error: {}", e);
                    thread::sleep(Duration::from_millis(500));
                }
            }
        }
        println!("\x1b[1;33m[web-server]\x1b[0m Web server stopped.");
    });

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Enumerates supported options for Modeswitchresult.
pub enum ModeSwitchResult {
    EnterStandby,
    SwitchTransport(String),
    DisableListener(String),
    NoChange,
}

/// Executes `evaluate_mode_switch` operational routine.
pub fn evaluate_mode_switch(
    body: &str,
    current_active_transport: &str,
    mut mode1: bool,
    mut mode2: bool,
    mut mode3: bool,
) -> (bool, bool, bool, ModeSwitchResult) {
    let mut m1_change = None;
    let mut m3_change = None;
    let active_transport = extract_json_str(body, "active_transport")
        .or_else(|| extract_json_str(body, "transport"));

    if let Some(m1) = extract_json_bool(body, "mode1") {
        mode1 = m1;
        m1_change = Some(m1);
    }
    if let Some(m2) = extract_json_bool(body, "mode2") {
        mode2 = m2;
    }
    if let Some(m3) = extract_json_bool(body, "mode3") {
        mode3 = m3;
        m3_change = Some(m3);
    }

    let is_standby_requested = active_transport == Some("standby");
    let all_disabled = is_standby_requested || (active_transport.is_none() && !mode1 && !mode2 && !mode3);

    if all_disabled {
        return (false, false, false, ModeSwitchResult::EnterStandby);
    }

    let inferred_transport = active_transport.map(|s| s.to_string())
        .or_else(|| {
            if let Some(true) = m3_change {
                Some("mode3_usb_bulk".to_string())
            } else if let Some(true) = m1_change {
                Some("mode1_udp".to_string())
            } else if let Some(true) = extract_json_bool(body, "mode2") {
                Some("mode2_miracast".to_string())
            } else {
                if (current_active_transport.contains("mode1") && m1_change == Some(false))
                    || (current_active_transport.contains("mode3") && m3_change == Some(false))
                    || (current_active_transport.contains("mode2") && extract_json_bool(body, "mode2") == Some(false))
                {
                    if mode1 {
                        Some("mode1_udp".to_string())
                    } else if mode3 {
                        Some("mode3_usb_bulk".to_string())
                    } else if mode2 {
                        Some("mode2_miracast".to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        });

    if let Some(transport) = inferred_transport {
        (mode1, mode2, mode3, ModeSwitchResult::SwitchTransport(transport))
    } else if let Some(false) = m3_change {
        (mode1, mode2, mode3, ModeSwitchResult::DisableListener("mode3".to_string()))
    } else if let Some(false) = m1_change {
        (mode1, mode2, mode3, ModeSwitchResult::DisableListener("mode1".to_string()))
    } else {
        (mode1, mode2, mode3, ModeSwitchResult::NoChange)
    }
}

/// Executes `transition_to_standby_or_visualizer` operational routine.
pub fn transition_to_standby_or_visualizer(pipeline_mgr: Arc<PipelineManager>) {
    let pipe = pipeline_mgr.clone();
    thread::spawn(move || {
        pipe.pause();
        crate::wfd::terminate_active_sessions();
        if let Ok(mut lock) = crate::decoder::v4l2_m2m::LATEST_SCREENSHOT_FRAME.lock() {
            *lock = None;
        }
        let vis = crate::flow::ARBITER.is_visualizer_user_enabled();
        if vis {
            println!("\x1b[1;36m[web-server]\x1b[0m Transitioning to Level 2 (Audio Soundbox + FFT Visualizer)...");
            let _ = crate::flow::ARBITER.request_level2(true);
        } else {
            println!("\x1b[1;33m[web-server]\x1b[0m Transitioning to Level 3 (Standby Splash Screen)...");
            let _ = crate::flow::ARBITER.request_standby();
            crate::display::SplashEngine::show_ready();
        }
    });
}

/// Executes `handle_http_client` operational routine.
fn handle_http_client(
    mut stream: TcpStream,
    pipeline_mgr: Arc<PipelineManager>,
    running: Arc<AtomicBool>,
    default_udp_port: u16,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));

    let mut buffer = [0u8; 4096];
    let n = match stream.read(&mut buffer) {
        Ok(n) if n > 0 => n,
        _ => return,
    };

    let req_str = String::from_utf8_lossy(&buffer[..n]);
    let mut lines = req_str.lines();
    let first_line = lines.next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    if parts.len() < 2 {
        return;
    }

    let method = parts[0];
    let path = parts[1];

    // 0. WebSocket Stream for Web Cast (Option C)
    if path == "/api/stream/ws" && req_str.to_ascii_lowercase().contains("upgrade: websocket") {
        crate::web_cast::handle_websocket_stream(stream, &req_str, pipeline_mgr, default_udp_port);
        return;
    }

    match (method, path) {
        ("GET", "/cast") | ("GET", "/cast/") | ("HEAD", "/cast") | ("HEAD", "/cast/") => {
            // Web Cast removed in favor of native Google Cast / Chromecast: redirect cleanly to dashboard
            send_response(&mut stream, "301 Moved Permanently\r\nLocation: /", "text/plain", b"Redirecting to /");
        }
        ("GET", "/") | ("GET", "/index.html") | ("HEAD", "/") | ("HEAD", "/index.html") => {
            let body = if method == "HEAD" {
                &[][..]
            } else {
                DASHBOARD_HTML.as_bytes()
            };
            send_response(&mut stream, "200 OK", "text/html; charset=utf-8", body);
        }
        ("GET", "/connect.sh") | ("HEAD", "/connect.sh") => {
            let body = if method == "HEAD" { &[][..] } else { CONNECT_SCRIPT.as_bytes() };
            send_response(&mut stream, "200 OK", "text/x-shellscript", body);
        }
        ("GET", "/download/ext-sender") | ("HEAD", "/download/ext-sender") => {
            serve_file_or_fallback(&mut stream, "/var/www/download/ext-sender", "application/octet-stream", method == "HEAD");
        }
        ("GET", "/download/client.tar.gz") | ("HEAD", "/download/client.tar.gz") => {
            serve_file_or_fallback(&mut stream, "/var/www/download/client.tar.gz", "application/gzip", method == "HEAD");
        }
        ("GET", "/download/99-ext-monitor.rules") | ("HEAD", "/download/99-ext-monitor.rules") => {
            let body = if method == "HEAD" { &[][..] } else { UDEV_RULES.as_bytes() };
            send_response(&mut stream, "200 OK", "text/plain", body);
        }
        ("GET", "/swagger") | ("GET", "/swagger/") | ("GET", "/docs") | ("GET", "/docs/") |
        ("HEAD", "/swagger") | ("HEAD", "/swagger/") | ("HEAD", "/docs") | ("HEAD", "/docs/") => {
            let body = if method == "HEAD" { &[][..] } else { crate::swagger::SWAGGER_HTML.as_bytes() };
            send_response(&mut stream, "200 OK", "text/html; charset=utf-8", body);
        }
        ("GET", "/api/openapi.json") | ("GET", "/swagger.json") |
        ("HEAD", "/api/openapi.json") | ("HEAD", "/swagger.json") => {
            let body = if method == "HEAD" { &[][..] } else { crate::swagger::OPENAPI_JSON.as_bytes() };
            send_response(&mut stream, "200 OK", "application/json; charset=utf-8", body);
        }
        ("GET", "/api/status") => {
            let is_paused = pipeline_mgr.is_paused();
            let cur_kind = pipeline_mgr.current_kind();
            let audio_st = pipeline_mgr.audio_status();
            let status_json = get_system_telemetry_json(is_paused, cur_kind, &audio_st);
            send_response(&mut stream, "200 OK", "application/json", status_json.as_bytes());
        }
        ("POST", "/api/time/sync") => {
            if let Some(idx) = req_str.find("\r\n\r\n") {
                let body = &req_str[idx + 4..];
                if let Some(epoch_secs) = extract_json_u64(body, "unix_epoch_secs") {
                    let cur_now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    // Only invoke settimeofday if the clock is uninitialized (< 2024) or drift exceeds 5 seconds.
                    // Routine time calls during live playback disrupt monotonic timer wheels and cause video flickers.
                    if cur_now < 1704067200 || (cur_now as i64 - epoch_secs as i64).abs() > 5 {
                        let tv = libc::timeval {
                            tv_sec: epoch_secs as _,
                            tv_usec: 0,
                        };
                        let res = unsafe { libc::settimeofday(&tv, std::ptr::null()) };
                        if res == 0 {
                            println!("\x1b[1;32m[time-sync]\x1b[0m System clock set to Unix timestamp {}", epoch_secs);
                        } else {
                            eprintln!("\x1b[1;31m[time-sync]\x1b[0m settimeofday failed: {}", std::io::Error::last_os_error());
                        }
                    }
                    let resp = format!("{{\"status\":\"synchronized\",\"unix_epoch_secs\":{}}}", epoch_secs);
                    send_response(&mut stream, "200 OK", "application/json", resp.as_bytes());
                    return;
                }
            }
            send_response(&mut stream, "400 Bad Request", "text/plain", b"Invalid unix_epoch_secs");
        }
        ("GET", "/api/time") => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let resp = format!("{{\"unix_epoch_secs\":{}}}", now);
            send_response(&mut stream, "200 OK", "application/json", resp.as_bytes());
        }
        ("GET", "/api/audio/status") => {
            let audio_st = pipeline_mgr.audio_status();
            send_response(&mut stream, "200 OK", "application/json", audio_st.to_json().as_bytes());
        }
        ("POST", "/api/audio/volume") => {
            if let Some(idx) = req_str.find("\r\n\r\n") {
                let body = &req_str[idx + 4..];
                if let Some(vol) = extract_json_u32(body, "volume") {
                    pipeline_mgr.set_audio_volume(vol);
                    let audio_st = pipeline_mgr.audio_status();
                    send_response(&mut stream, "200 OK", "application/json", audio_st.to_json().as_bytes());
                    return;
                }
            }
            send_response(&mut stream, "400 Bad Request", "text/plain", b"Missing volume");
        }
        ("POST", "/api/audio/mute") => {
            if let Some(idx) = req_str.find("\r\n\r\n") {
                let body = &req_str[idx + 4..];
                if let Some(muted) = extract_json_bool(body, "muted") {
                    pipeline_mgr.set_audio_muted(muted);
                    let audio_st = pipeline_mgr.audio_status();
                    send_response(&mut stream, "200 OK", "application/json", audio_st.to_json().as_bytes());
                    return;
                }
            }
            send_response(&mut stream, "400 Bad Request", "text/plain", b"Missing muted");
        }
        ("POST", "/api/audio/rate") => {
            if let Some(idx) = req_str.find("\r\n\r\n") {
                let body = &req_str[idx + 4..];
                if let Some(rate) = extract_json_u32(body, "rate") {
                    pipeline_mgr.set_audio_rate(rate);
                    forward_config_to_sender(&format!("{{\"audio_rate\":{}}}", rate));
                    let audio_st = pipeline_mgr.audio_status();
                    send_response(&mut stream, "200 OK", "application/json", audio_st.to_json().as_bytes());
                    return;
                }
            }
            send_response(&mut stream, "400 Bad Request", "text/plain", b"Missing rate");
        }
        ("POST", "/api/audio/transport") => {
            if let Some(idx) = req_str.find("\r\n\r\n") {
                let body = &req_str[idx + 4..];
                if let Some(mode) = extract_json_str(body, "transport") {
                    let transport = crate::audio::AudioTransport::from_str(mode);
                    pipeline_mgr.set_audio_transport(transport);
                    let audio_st = pipeline_mgr.audio_status();
                    send_response(&mut stream, "200 OK", "application/json", audio_st.to_json().as_bytes());
                    return;
                }
            }
            send_response(&mut stream, "400 Bad Request", "text/plain", b"Missing transport");
        }
        ("GET", "/api/config") => {
            let json = if let Ok(cfg) = CONFIG.lock() {
                let col = if cfg.color.is_empty() { "full" } else { &cfg.color };
                let cap = if cfg.capture.is_empty() { "kms" } else { &cfg.capture };
                let mon = if cfg.monitor.is_empty() { "HDMI-1" } else { &cfg.monitor };
                format!(
                    "{{\"fps\":{},\"bitrate\":{},\"color\":\"{}\",\"drop_only\":{},\"skip_to_first\":{},\"key_int_max\":{},\"capture\":\"{}\",\"monitor\":\"{}\",\"mode1\":{},\"mode2\":{},\"mode3\":{}}}",
                    cfg.fps, cfg.bitrate, col, cfg.drop_only, cfg.skip_to_first, cfg.key_int_max, cap, mon, cfg.mode1, cfg.mode2, cfg.mode3
                )
            } else {
                "{\"fps\":30,\"bitrate\":400,\"color\":\"full\",\"drop_only\":false,\"skip_to_first\":true,\"key_int_max\":30,\"capture\":\"kms\",\"monitor\":\"HDMI-1\",\"mode1\":true,\"mode2\":true,\"mode3\":true}".to_string()
            };
            send_response(&mut stream, "200 OK", "application/json", json.as_bytes());
        }
        ("POST", "/api/config") => {
            // Find body after empty line
            if let Some(idx) = req_str.find("\r\n\r\n") {
                let body = &req_str[idx + 4..];
                if let Ok(mut cfg) = CONFIG.lock() {
                    if let Some(fps) = extract_json_u32(body, "fps") { cfg.fps = fps; }
                    if let Some(bitrate) = extract_json_u32(body, "bitrate") { cfg.bitrate = bitrate; }
                    if let Some(color) = extract_json_str(body, "color") { cfg.color = color.to_string(); }
                    if let Some(drop_only) = extract_json_bool(body, "drop_only") { cfg.drop_only = drop_only; }
                    if let Some(skip_to_first) = extract_json_bool(body, "skip_to_first") { cfg.skip_to_first = skip_to_first; }
                    if let Some(key_int_max) = extract_json_u32(body, "key_int_max") { cfg.key_int_max = key_int_max; }
                    if let Some(capture) = extract_json_str(body, "capture") { cfg.capture = capture.to_string(); }
                    if let Some(monitor) = extract_json_str(body, "monitor") { cfg.monitor = monitor.to_string(); }
                    if let Some(m1) = extract_json_bool(body, "mode1") { cfg.mode1 = m1; }
                    if let Some(m2) = extract_json_bool(body, "mode2") { cfg.mode2 = m2; }
                    if let Some(m3) = extract_json_bool(body, "mode3") { cfg.mode3 = m3; }
                }
                forward_config_to_sender(body);
            }
            send_response(&mut stream, "200 OK", "application/json", b"{\"status\":\"ok\"}");
        }
        ("POST", "/api/modes") | ("POST", "/api/transport/active") => {
            if let Some(idx) = req_str.find("\r\n\r\n") {
                let body = &req_str[idx + 4..];

                let (cur_trans, cur_m1, cur_m2, cur_m3) = if let Ok(cfg) = CONFIG.lock() {
                    (cfg.active_transport.clone(), cfg.mode1, cfg.mode2, cfg.mode3)
                } else {
                    (String::new(), false, false, false)
                };

                let (new_m1, new_m2, new_m3, result) = evaluate_mode_switch(body, &cur_trans, cur_m1, cur_m2, cur_m3);

                if let Ok(mut cfg) = CONFIG.lock() {
                    cfg.mode1 = new_m1;
                    cfg.mode2 = new_m2;
                    cfg.mode3 = new_m3;
                }

                match result {
                    ModeSwitchResult::EnterStandby => {
                        println!("\x1b[1;33m[web-server]\x1b[0m All video modes disabled via Web UI. Entering Standby / Audio Visualizer...");
                        if let Ok(mut cfg) = CONFIG.lock() {
                            cfg.active_transport = "standby".to_string();
                        }
                        forward_config_to_sender("{\"action\":\"stop\"}");
                        transition_to_standby_or_visualizer(pipeline_mgr.clone());
                    }
                    ModeSwitchResult::SwitchTransport(transport) => {
                        if let Ok(mut lock) = crate::decoder::v4l2_m2m::LATEST_SCREENSHOT_FRAME.lock() {
                            *lock = None;
                        }
                        println!("\x1b[1;36m[web-server]\x1b[0m Direct Active Transport switch requested: {}", transport);
                        let audio_flag = extract_json_bool(body, "audio");
                        let mode_name = extract_json_str(body, "mode").unwrap_or("extend");
                        match transport.as_str() {
                            "mode3_usb_bulk" | "usb_bulk" | "mode3" => {
                                let is_already_usb = cur_trans == "mode3_usb_bulk" && crate::flow::ARBITER.is_level0_active() && !pipeline_mgr.is_paused() && pipeline_mgr.current_kind().is_some();
                                let _ = crate::flow::ARBITER.request_level0(crate::flow::DesktopMode::Mode3UsbBulk, 86);
                                if let Ok(mut cfg) = CONFIG.lock() {
                                    cfg.active_transport = "mode3_usb_bulk".to_string();
                                    cfg.mode3 = true;
                                    cfg.mode1 = false;
                                    cfg.mode2 = false;
                                }
                                let fwd_payload = format!(
                                    "{{\"action\":\"start\",\"transport\":\"usb_bulk\",\"mode\":\"{}\"{}}}",
                                    mode_name,
                                    audio_flag.map(|a| format!(",\"audio\":{}", a)).unwrap_or_default()
                                );
                                forward_config_to_sender(&fwd_payload);
                                if !is_already_usb {
                                    let run = running.clone();
                                    let pipe = pipeline_mgr.clone();
                                    thread::spawn(move || {
                                        if let Err(e) = crate::usb_bulk::activate_usb_bulk(run, pipe) {
                                            eprintln!("\x1b[1;31m[web-server]\x1b[0m Failed to activate USB Bulk: {}", e);
                                        }
                                    });
                                } else {
                                    println!("\x1b[1;32m[web-server]\x1b[0m USB Bulk receiver endpoint already active. Skipping duplicate activation.");
                                }
                            }
                            "mode1_udp" | "network" | "udp" | "mode1" => {
                                let is_already_udp = cur_trans == "mode1_udp" && crate::flow::ARBITER.is_level0_active() && !pipeline_mgr.is_paused() && pipeline_mgr.current_kind().is_some();
                                let _ = crate::flow::ARBITER.request_level0(crate::flow::DesktopMode::Mode1NetworkUdp, 86);
                                if let Ok(mut cfg) = CONFIG.lock() {
                                    cfg.active_transport = "mode1_udp".to_string();
                                    cfg.mode1 = true;
                                    cfg.mode2 = false;
                                    cfg.mode3 = false;
                                }
                                let fwd_payload = format!(
                                    "{{\"action\":\"start\",\"transport\":\"network\",\"mode\":\"{}\"{}}}",
                                    mode_name,
                                    audio_flag.map(|a| format!(",\"audio\":{}", a)).unwrap_or_default()
                                );
                                forward_config_to_sender(&fwd_payload);
                                if !is_already_udp {
                                    let pipe = pipeline_mgr.clone();
                                    thread::spawn(move || {
                                        let default_kind = PipelineKind::RawH264Rtp { port: default_udp_port };
                                        let _ = pipe.resume(default_kind);
                                    });
                                } else {
                                    println!("\x1b[1;32m[web-server]\x1b[0m UDP Network receiver already active. Skipping duplicate activation.");
                                }
                            }
                            "mode2_miracast" | "miracast" | "mode2" => {
                                let _ = crate::flow::ARBITER.request_level0(crate::flow::DesktopMode::Mode2Miracast, 86);
                                if let Ok(mut cfg) = CONFIG.lock() {
                                    cfg.active_transport = "mode2_miracast".to_string();
                                    cfg.mode1 = false;
                                    cfg.mode2 = true;
                                    cfg.mode3 = false;
                                }
                                forward_config_to_sender("{\"action\":\"launch_miracast\",\"transport\":\"miracast\"}");
                                let pipe = pipeline_mgr.clone();
                                thread::spawn(move || {
                                    pipe.stop();
                                    crate::display::SplashEngine::show_miracast();
                                    println!("\x1b[1;32m[web-server]\x1b[0m Miracast connection guide splash displayed. Listening on RTSP 7236 / MS-MICE 7250.");
                                });
                            }
                            _ => {}
                        }
                    }
                    ModeSwitchResult::DisableListener(l) => {
                        println!("\x1b[1;33m[web-server]\x1b[0m Mode listener {} disabled.", l);
                    }
                    ModeSwitchResult::NoChange => {}
                }
            }
            send_response(&mut stream, "200 OK", "application/json", b"{\"status\":\"ok\"}");
        }
        ("POST", "/api/host/control") => {
            if let Some(idx) = req_str.find("\r\n\r\n") {
                let body = req_str[idx + 4..].trim();
                println!("\x1b[1;36m[web-server]\x1b[0m Encaminhando comando ao Host (192.168.7.1:5001): {}", body);
                if let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") {
                    let _ = sock.send_to(body.as_bytes(), "192.168.7.1:5001");
                    if let Ok(peer) = stream.peer_addr() {
                        let peer_ip = peer.ip().to_string();
                        if peer_ip != "192.168.7.1" && peer_ip != "127.0.0.1" {
                            let _ = sock.send_to(body.as_bytes(), format!("{}:5001", peer.ip()));
                        }
                    }
                }

                if body.contains("\"action\":\"stop\"") {
                    if let Ok(mut cfg) = CONFIG.lock() {
                        cfg.mode1 = false;
                        cfg.mode2 = false;
                        cfg.mode3 = false;
                        cfg.active_transport = "standby".to_string();
                    }
                    transition_to_standby_or_visualizer(pipeline_mgr.clone());
                } else if body.contains("\"action\":\"start\"") || body.contains("\"action\":\"launch_miracast\"") {
                    let pipe = pipeline_mgr.clone();
                    let run = running.clone();
                    let body_str = body.to_string();
                    thread::spawn(move || {
                        let req_trans = if body_str.contains("\"transport\":\"usb_bulk\"") || body_str.contains("\"transport\":\"usb\"") {
                            Some("mode3_usb_bulk".to_string())
                        } else if body_str.contains("\"transport\":\"network\"") || body_str.contains("\"transport\":\"udp\"") {
                            Some("mode1_udp".to_string())
                        } else if body_str.contains("\"transport\":\"miracast\"") || body_str.contains("\"transport\":\"mode2\"") || body_str.contains("\"mode2\"") || body_str.contains("launch_miracast") {
                            Some("mode2_miracast".to_string())
                        } else {
                            None
                        };

                        let trans = if let Ok(mut cfg) = CONFIG.lock() {
                            if let Some(ref rt) = req_trans {
                                cfg.active_transport = rt.clone();
                                if rt == "mode3_usb_bulk" {
                                    cfg.mode3 = true;
                                    cfg.mode1 = false;
                                    cfg.mode2 = false;
                                } else if rt == "mode2_miracast" {
                                    cfg.mode2 = true;
                                    cfg.mode1 = false;
                                    cfg.mode3 = false;
                                } else {
                                    cfg.mode1 = true;
                                    cfg.mode2 = false;
                                    cfg.mode3 = false;
                                }
                            }
                            if cfg.active_transport.is_empty() { "mode3_usb_bulk".to_string() } else { cfg.active_transport.clone() }
                        } else {
                            req_trans.unwrap_or_else(|| "mode3_usb_bulk".to_string())
                        };

                        if trans == "mode3_usb_bulk" {
                            let _ = crate::usb_bulk::activate_usb_bulk(run, pipe);
                        } else if trans == "mode1_udp" {
                            let default_kind = PipelineKind::RawH264Rtp { port: default_udp_port };
                            let _ = pipe.resume(default_kind);
                        } else if trans == "mode2_miracast" {
                            pipe.stop();
                            crate::display::SplashEngine::show_miracast();
                        }
                    });
                }
            }
            send_response(&mut stream, "200 OK", "application/json", b"{\"status\":\"forwarded_to_host\"}");
        }
        ("GET", "/api/host/status") => {
            let (active_trans, is_live) = if let Ok(cfg) = CONFIG.lock() {
                (cfg.active_transport.clone(), cfg.mode1 || cfg.mode2 || cfg.mode3)
            } else {
                ("standby".to_string(), false)
            };
            let json = format!(
                "{{\"connected\":true,\"ip\":\"192.168.7.1\",\"active_transport\":\"{}\",\"is_streaming\":{}}}",
                active_trans, is_live
            );
            send_response(&mut stream, "200 OK", "application/json", json.as_bytes());
        }
        ("POST", "/api/bluetooth/discoverable") => {
            println!("\x1b[1;34m[web-server]\x1b[0m Ativando Bluetooth A2DP Sink (pareável por 60s)...");
            let _ = std::process::Command::new("/bin/sh")
                .args(["-c", "bluetoothctl discoverable on && bluetoothctl pairable on 2>/dev/null || true"])
                .output();
            send_response(&mut stream, "200 OK", "application/json", b"{\"status\":\"discoverable_on\",\"timeout\":60}");
        }
        ("GET", "/upnp/desc.xml") => {
            let xml = crate::media_renderer::get_upnp_desc_xml("192.168.7.2", 8080);
            send_response(&mut stream, "200 OK", "text/xml; charset=utf-8", xml.as_bytes());
        }
        ("GET", "/dial/dd.xml") => {
            let xml = crate::media_renderer::get_dial_dd_xml("192.168.7.2", 8080);
            send_dial_response(&mut stream, "200 OK", "text/xml; charset=utf-8", xml.as_bytes(), "http://192.168.7.2:8080/apps/");
        }
        ("GET", "/setup/eureka_info") => {
            let eureka_json = r#"{"name":"RaspCast","version":8,"build_info":{"build_type":2,"system_build_number":"1.56.275994","cast_build_revision":"1.56.275994"},"device_info":{"model_name":"Eureka Dongle","manufacturer":"Carlos Alberto / Ext-Monitor Project","mac_address":"12:22:33:44:55:67","ssdp_udn":"uuid:ext-monitor-dial-device","capabilities":{"audio_in":false,"audio_out":true,"video_in":false,"video_out":true,"display_supported":true}},"net":{"ip_address":"192.168.7.2","online":true}}"#;
            send_response(&mut stream, "200 OK", "application/json", eureka_json.as_bytes());
        }
        ("GET", "/setup/icon.png") => {
            let fallback_png: [u8; 67] = [
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
                0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
                0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
                0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
                0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
                0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
            ];
            let data = fs::read("/var/www/icon.png").unwrap_or_else(|_| fallback_png.to_vec());
            send_response(&mut stream, "200 OK", "image/png", &data);
        }
        ("GET", p) if p.starts_with("/apps") => {
            let app_name = p.strip_prefix("/apps/").unwrap_or("YouTube").split('/').next().unwrap_or("YouTube");
            let xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<service xmlns="urn:dial-multiscreen-org:schemas:dial">
  <name>{}</name>
  <options allowStop="true"/>
  <state>running</state>
  <link rel="run" href="run"/>
</service>"#,
                app_name
            );
            send_dial_response(&mut stream, "200 OK", "text/xml; charset=utf-8", xml.as_bytes(), "http://192.168.7.2:8080/apps/");
        }
        ("POST", p) if p.starts_with("/apps") => {
            let run_url = format!("http://192.168.7.2:8080{}/run", p);
            let header = format!(
                "HTTP/1.1 201 Created\r\nLocation: {}\r\nContent-Length: 0\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Expose-Headers: Location\r\nConnection: close\r\n\r\n",
                run_url
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.flush();
        }
        ("DELETE", p) if p.starts_with("/apps") => {
            send_response(&mut stream, "200 OK", "text/plain", b"OK");
        }
        ("GET", "/api/media/status") => {
            let st = crate::media_renderer::get_media_state();
            let spec_arc = crate::media_renderer::get_audio_spectrum();
            let (bands, peaks, rms_db, is_audio_active) = {
                let s = spec_arc.lock().unwrap();
                let active = s.is_active && s.last_update.elapsed() < Duration::from_millis(800);
                if active {
                    (s.bands, s.peaks, s.rms_db, true)
                } else {
                    ([0.0; 24], [0.0; 24], -60.0, false)
                }
            };
            let is_video_active = pipeline_mgr.current_kind().is_some();
            let bars_json: Vec<String> = bands.iter().map(|b| format!("{:.2}", b)).collect();
            let peaks_json: Vec<String> = peaks.iter().map(|p| format!("{:.2}", p)).collect();

            let json = {
                let m = st.lock().unwrap();
                format!(
                    "{{\"title\":\"{}\",\"artist\":\"{}\",\"album\":\"{}\",\"source\":\"{}\",\"state\":\"{}\",\"volume\":{},\"visualizer_enabled\":{},\"video_active\":{},\"audio_active\":{},\"rms_db\":{:.1},\"bars\":[{}],\"peaks\":[{}]}}",
                    m.title.replace('"', "\\\""),
                    m.artist.replace('"', "\\\""),
                    m.album.replace('"', "\\\""),
                    m.source,
                    m.state,
                    m.volume,
                    m.visualizer_enabled,
                    is_video_active,
                    is_audio_active,
                    rms_db,
                    bars_json.join(","),
                    peaks_json.join(",")
                )
            };
            send_response(&mut stream, "200 OK", "application/json", json.as_bytes());
        }
        ("POST", "/api/media/control") => {
            if let Some(idx) = req_str.find("\r\n\r\n") {
                let body = req_str[idx + 4..].trim();
                let st = crate::media_renderer::get_media_state();
                if let Ok(mut m) = st.lock() {
                    if body.contains("\"action\":\"play\"") {
                        m.state = "playing".to_string();
                    } else if body.contains("\"action\":\"pause\"") {
                        m.state = "paused".to_string();
                    } else if body.contains("\"action\":\"stop\"") {
                        m.state = "idle".to_string();
                    }
                    if let Some(title) = extract_json_str(body, "title") {
                        m.title = title.to_string();
                    }
                    if let Some(artist) = extract_json_str(body, "artist") {
                        m.artist = artist.to_string();
                    }
                    if let Some(album) = extract_json_str(body, "album") {
                        m.album = album.to_string();
                    }
                };
            }
            send_response(&mut stream, "200 OK", "application/json", b"{\"status\":\"ok\"}");
        }
        ("POST", "/api/media/visualizer") => {
            if let Some(idx) = req_str.find("\r\n\r\n") {
                let body = req_str[idx + 4..].trim();
                let st = crate::media_renderer::get_media_state();
                if let Ok(mut m) = st.lock() {
                    let vis_opt = extract_json_bool(body, "enabled")
                        .or_else(|| extract_json_bool(body, "visualizer"));
                    if let Some(vis) = vis_opt {
                        m.visualizer_enabled = vis;
                        crate::flow::ARBITER.set_visualizer_user_enabled(vis);
                        if !crate::flow::ARBITER.is_level0_active() {
                            if vis {
                                println!("\x1b[1;36m[web-server]\x1b[0m Visualizer enabled: requesting Level 2 Audio Soundbox...");
                                let _ = crate::flow::ARBITER.request_level2(true);
                            } else {
                                println!("\x1b[1;33m[web-server]\x1b[0m Visualizer disabled: entering Standby Splash...");
                                let _ = crate::flow::ARBITER.request_standby();
                                crate::display::SplashEngine::show_ready();
                            }
                        }
                    }
                };
            }
            send_response(&mut stream, "200 OK", "application/json", b"{\"status\":\"visualizer_updated\"}");
        }
        ("POST", "/api/media/realtime") => {
            if let Some(idx) = req_str.find("\r\n\r\n") {
                let body = req_str[idx + 4..].trim();
                if let Some(bars_idx) = body.find("\"bars\":[") {
                    let rest = &body[bars_idx + 8..];
                    if let Some(end_idx) = rest.find(']') {
                        let nums_str = &rest[..end_idx];
                        let mut bands = [0.0f32; 24];
                        for (i, part) in nums_str.split(',').enumerate() {
                            if i < 24 {
                                if let Ok(val) = part.trim().parse::<f32>() {
                                    bands[i] = val.clamp(0.0, 1.0);
                                }
                            }
                        }
                        let rms_val = extract_json_f32(body, "rms").unwrap_or(-18.0);
                        crate::media_renderer::update_audio_spectrum(&bands, rms_val);
                        send_response(&mut stream, "200 OK", "application/json", b"{\"status\":\"spectrum_updated\"}");
                        return;
                    }
                }
            }
            send_response(&mut stream, "400 Bad Request", "text/plain", b"Invalid body");
        }
        ("POST", "/api/media/test_sound") => {
            let bands = [
                0.35, 0.55, 0.75, 0.90, 0.98, 0.92, 0.80, 0.65,
                0.45, 0.40, 0.55, 0.70, 0.85, 0.75, 0.60, 0.45,
                0.35, 0.30, 0.25, 0.40, 0.55, 0.45, 0.30, 0.20
            ];
            crate::media_renderer::update_audio_spectrum(&bands, -8.5);
            send_response(&mut stream, "200 OK", "application/json", b"{\"status\":\"test_sound_triggered\"}");
        }
        ("POST", "/api/system/reboot") => {
            println!("\x1b[1;31m[web-server]\x1b[0m System REBOOT requested via Web UI.");
            send_response(&mut stream, "200 OK", "application/json", b"{\"status\":\"rebooting\"}");
            thread::spawn(|| {
                thread::sleep(Duration::from_millis(500));
                let _ = std::process::Command::new("/sbin/reboot").arg("-f").output();
                unsafe {
                    libc::sync();
                    libc::reboot(libc::RB_AUTOBOOT);
                }
            });
        }
        ("GET", "/api/screenshot") => {
            crate::decoder::v4l2_m2m::SCREENSHOT_REQUESTED.store(true, Ordering::SeqCst);
            for _ in 0..100 {
                if !crate::decoder::v4l2_m2m::SCREENSHOT_REQUESTED.load(Ordering::SeqCst) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            if let Ok(lock) = crate::decoder::v4l2_m2m::LATEST_SCREENSHOT_FRAME.lock() {
                if let Some((ref data, w, h)) = *lock {
                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nX-Frame-Width: {}\r\nX-Frame-Height: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        w, h, data.len()
                    );
                    let _ = stream.write_all(header.as_bytes());
                    let _ = stream.write_all(data);
                    let _ = stream.flush();
                    return;
                }
            }

            if let Ok(mut fb) = std::fs::File::open("/dev/fb0") {
                let mut data = vec![0u8; 1280 * 720 * 2];
                if fb.read_exact(&mut data).is_ok() {
                    send_response(&mut stream, "200 OK", "application/octet-stream", &data);
                    return;
                }
            }
            send_response(&mut stream, "500 Internal Server Error", "text/plain", b"Failed to capture fb0");
        }
        ("GET", "/api/dmesg") => {
            let out = std::process::Command::new("dmesg").output().map(|o| o.stdout).unwrap_or_default();
            send_response(&mut stream, "200 OK", "text/plain; charset=utf-8", &out);
        }
        ("GET", "/api/logs") => {
            let logs = std::fs::read("/var/log/ext-receiver.log").unwrap_or_else(|_| b"No /var/log/ext-receiver.log found".to_vec());
            send_response(&mut stream, "200 OK", "text/plain; charset=utf-8", &logs);
        }

        // ======================================================================
        // Debug / Diagnostics API — Structured endpoints for system inspection
        // Replaces ad-hoc /api/exec calls with safe, typed REST responses
        // ======================================================================

        ("GET", "/api/debug/clocks") => {
            // Read real SoC clocks from debugfs (requires mount -t debugfs)
            let read_clk = |path: &str| -> String {
                fs::read_to_string(path)
                    .ok()
                    .and_then(|s| s.trim().parse::<u64>().ok())
                    .map(|hz| format!("{}", hz / 1_000_000))
                    .unwrap_or_else(|| "null".to_string())
            };
            let arm_mhz = fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq")
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(|khz| format!("{}", khz / 1000))
                .unwrap_or_else(|| read_clk("/sys/kernel/debug/clk/fw-clk-arm/clk_rate"));
            let arm_max = fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq")
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(|khz| format!("{}", khz / 1000))
                .unwrap_or_else(|| "null".to_string());
            let governor = fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
                .unwrap_or_else(|_| "unknown".to_string()).trim().to_string();
            let debugfs_mounted = fs::metadata("/sys/kernel/debug/clk/fw-clk-core/clk_rate").is_ok();

            let json = format!(
                "{{\"debugfs_mounted\":{},\"arm_mhz\":{},\"arm_max_mhz\":{},\"governor\":\"{}\",\
                \"core_mhz\":{},\"vpu_mhz\":{},\"h264_mhz\":{},\"v3d_mhz\":{},\
                \"sdram_mhz\":{},\"hevc_mhz\":{},\"pixel_mhz\":{}}}",
                debugfs_mounted,
                arm_mhz, arm_max,
                governor,
                read_clk("/sys/kernel/debug/clk/fw-clk-core/clk_rate"),
                read_clk("/sys/kernel/debug/clk/vpu/clk_rate"),
                read_clk("/sys/kernel/debug/clk/h264/clk_rate"),
                read_clk("/sys/kernel/debug/clk/fw-clk-v3d/clk_rate"),
                read_clk("/sys/kernel/debug/clk/sdram/clk_rate"),
                read_clk("/sys/kernel/debug/clk/fw-clk-hevc/clk_rate"),
                read_clk("/sys/kernel/debug/clk/fw-clk-pixel/clk_rate"),
            );
            send_response(&mut stream, "200 OK", "application/json", json.as_bytes());
        }

        ("GET", "/api/debug/pipeline") => {
            // V4L2 M2M decoder state and process statistics
            let v4l2_devices: Vec<String> = fs::read_dir("/dev")
                .map(|entries| {
                    entries.filter_map(|e| e.ok())
                        .filter(|e| e.file_name().to_str().map(|n| n.starts_with("video")).unwrap_or(false))
                        .map(|e| format!("\"/dev/{}\"", e.file_name().to_string_lossy()))
                        .collect()
                })
                .unwrap_or_default();
            let uptime = fs::read_to_string("/proc/uptime")
                .unwrap_or_else(|_| "0".to_string());
            let uptime_secs = uptime.split_whitespace().next().unwrap_or("0");

            let json = format!(
                "{{\"v4l2_devices\":[{}],\"uptime_secs\":{}}}",
                v4l2_devices.join(","),
                uptime_secs
            );
            send_response(&mut stream, "200 OK", "application/json", json.as_bytes());
        }

        ("GET", "/api/debug/usb") => {
            // USB gadget and FunctionFS endpoint status
            let gadget_udc = fs::read_to_string(
                "/sys/kernel/config/usb_gadget/ext_composite/UDC"
            ).unwrap_or_else(|_| "not_bound".to_string()).trim().to_string();
            let ffs_mounted = fs::metadata("/dev/usb-ffs/display").is_ok();
            let usb0_up = fs::metadata("/sys/class/net/usb0").is_ok();
            let usb0_mac = fs::read_to_string("/sys/class/net/usb0/address")
                .unwrap_or_else(|_| "unknown".to_string()).trim().to_string();

            let json = format!(
                "{{\"udc\":\"{}\",\"functionfs_display_mounted\":{},\"usb0_up\":{},\"usb0_mac\":\"{}\"}}",
                gadget_udc, ffs_mounted, usb0_up, usb0_mac
            );
            send_response(&mut stream, "200 OK", "application/json", json.as_bytes());
        }

        ("GET", "/api/debug/memory") => {
            // Detailed memory breakdown from /proc/meminfo
            let meminfo = fs::read_to_string("/proc/meminfo").unwrap_or_default();
            let parse_kb = |key: &str| -> u64 {
                meminfo.lines()
                    .find(|l| l.starts_with(key))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0)
            };
            let json = format!(
                "{{\"total_kb\":{},\"free_kb\":{},\"available_kb\":{},\
                \"buffers_kb\":{},\"cached_kb\":{}}}",
                parse_kb("MemTotal:"), parse_kb("MemFree:"), parse_kb("MemAvailable:"),
                parse_kb("Buffers:"), parse_kb("Cached:")
            );
            send_response(&mut stream, "200 OK", "application/json", json.as_bytes());
        }

        ("GET", "/api/debug/config") => {
            // Read boot config.txt from mounted boot partition
            let config = fs::read_to_string("/boot/config.txt")
                .or_else(|_| fs::read_to_string("/mnt/boot/config.txt"))
                .unwrap_or_else(|_| "# config.txt not readable (boot partition not mounted)".to_string());
            send_response(&mut stream, "200 OK", "text/plain; charset=utf-8", config.as_bytes());
        }
        ("POST", "/api/exec") => {
            if let Some(idx) = req_str.find("\r\n\r\n") {
                let cmd_str = req_str[idx + 4..].trim();
                let out = std::process::Command::new("/bin/sh")
                    .args(&["-c", cmd_str])
                    .output();
                let resp = match out {
                    Ok(o) => {
                        let mut res = o.stdout;
                        res.extend_from_slice(&o.stderr);
                        res
                    }
                    Err(e) => format!("Error: {}", e).into_bytes(),
                };
                send_response(&mut stream, "200 OK", "text/plain; charset=utf-8", &resp);
                return;
            }
            send_response(&mut stream, "400 Bad Request", "text/plain", b"Missing body");
        }
        ("GET", "/api/sdcard/status") => {
            let is_inserted = fs::metadata("/dev/mmcblk0").is_ok();
            let is_mounted = fs::read_to_string("/proc/mounts")
                .map(|s| s.contains("mmcblk0"))
                .unwrap_or(false);
            let json = format!(
                "{{\"inserted\":{},\"mounted\":{},\"device\":\"/dev/mmcblk0\",\"partition\":\"/dev/mmcblk0p1\"}}",
                is_inserted, is_mounted
            );
            send_response(&mut stream, "200 OK", "application/json", json.as_bytes());
        }
        ("POST", "/api/sdcard/mount") => {
            let _ = std::process::Command::new("mkdir").args(&["-p", "/mnt/boot"]).output();
            let res = std::process::Command::new("mount")
                .args(&["-t", "vfat", "/dev/mmcblk0p1", "/mnt/boot"])
                .output();
            let status = match res {
                Ok(o) if o.status.success() => "mounted",
                _ => "error",
            };
            send_response(&mut stream, "200 OK", "application/json", format!("{{\"status\":\"{}\"}}", status).as_bytes());
        }
        ("POST", "/api/sdcard/unmount") => {
            let res = std::process::Command::new("umount").arg("/mnt/boot").output();
            let status = match res {
                Ok(o) if o.status.success() => "unmounted",
                _ => "error",
            };
            send_response(&mut stream, "200 OK", "application/json", format!("{{\"status\":\"{}\"}}", status).as_bytes());
        }
        ("POST", "/api/stream/stop") => {
            let was_running = if let Ok(mut cfg) = CONFIG.lock() {
                let r = cfg.mode1 || cfg.mode2 || cfg.mode3 || cfg.active_transport != "standby";
                cfg.mode1 = false;
                cfg.mode2 = false;
                cfg.mode3 = false;
                cfg.active_transport = "standby".to_string();
                r
            } else {
                true
            };
            if was_running {
                println!("\x1b[1;33m[web-server]\x1b[0m User requested stream STOP / STANDBY via Web UI. Stopping all 3 video services...");
                forward_config_to_sender("{\"action\":\"stop\"}");
            }
            transition_to_standby_or_visualizer(pipeline_mgr.clone());
            send_response(
                &mut stream,
                "200 OK",
                "application/json",
                b"{\"status\":\"paused\"}",
            );
        }
        ("POST", "/api/stream/start") => {
            println!("\x1b[1;32m[web-server]\x1b[0m User requested stream RESUME via Web UI.");
            let trans = if let Ok(cfg) = CONFIG.lock() {
                if cfg.active_transport.is_empty() {
                    "mode3_usb_bulk".to_string()
                } else {
                    cfg.active_transport.clone()
                }
            } else {
                "mode3_usb_bulk".to_string()
            };
            match trans.as_str() {
                "mode3_usb_bulk" => {
                    forward_config_to_sender("{\"action\":\"start\",\"transport\":\"usb_bulk\"}");
                    let run = running.clone();
                    let pipe = pipeline_mgr.clone();
                    thread::spawn(move || {
                        if let Err(e) = crate::usb_bulk::activate_usb_bulk(run, pipe.clone()) {
                            eprintln!("\x1b[1;31m[web-server]\x1b[0m USB Bulk failed: {}. Falling back to UDP...", e);
                            forward_config_to_sender("{\"action\":\"start\",\"transport\":\"network\"}");
                            let default_kind = PipelineKind::RawH264Rtp {
                                port: default_udp_port,
                            };
                            let _ = pipe.resume(default_kind);
                        }
                    });
                }
                "mode2_miracast" => {
                    crate::display::SplashEngine::show_miracast();
                }
                _ => {
                    forward_config_to_sender("{\"action\":\"start\",\"transport\":\"network\"}");
                    let default_kind = PipelineKind::RawH264Rtp {
                        port: default_udp_port,
                    };
                    let pipe = pipeline_mgr.clone();
                    thread::spawn(move || {
                        let _ = pipe.resume(default_kind);
                    });
                }
            }
            send_response(
                &mut stream,
                "200 OK",
                "application/json",
                b"{\"status\":\"resumed\"}",
            );
        }
        ("POST", "/api/service/stop") => {
            println!(
                "\x1b[1;31m[web-server]\x1b[0m User requested graceful SHUTDOWN of panel and receiver service."
            );
            send_response(
                &mut stream,
                "200 OK",
                "application/json",
                b"{\"status\":\"stopping\",\"message\":\"Service is stopping now\"}",
            );
            // Spawn clean shutdown on a background thread after sending HTTP response
            let r = running.clone();
            let p = pipeline_mgr.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(300));
                p.stop();
                r.store(false, Ordering::SeqCst);
            });
        }
        ("POST", "/api/mode") => {
            println!("\x1b[1;33m[web-server]\x1b[0m Received mode switch request via Web UI.");
            if let Ok(mut lock) = crate::decoder::v4l2_m2m::LATEST_SCREENSHOT_FRAME.lock() {
                *lock = None;
            }
            if let Some(idx) = req_str.find("\r\n\r\n") {
                let body = &req_str[idx + 4..];
                if body.contains("usb-bulk") || body.contains("bulk") || body.contains("\"mode\":3") || body.contains("\"mode\":\"3\"") {
                    println!("\x1b[1;32m[web-server]\x1b[0m Switching to Mode 3 (USB Bulk Direct)...");
                    let _ = crate::flow::ARBITER.request_level0(crate::flow::DesktopMode::Mode3UsbBulk, 86);
                    if let Ok(mut cfg) = CONFIG.lock() {
                        cfg.mode3 = true;
                        cfg.mode1 = false;
                        cfg.mode2 = false;
                        cfg.active_transport = "mode3_usb_bulk".to_string();
                    }
                    let run = running.clone();
                    let pipe = pipeline_mgr.clone();
                    thread::spawn(move || {
                        if let Err(e) = crate::usb_bulk::activate_usb_bulk(run, pipe) {
                            eprintln!("\x1b[1;31m[web-server]\x1b[0m Failed to activate USB Bulk: {}", e);
                        }
                    });
                } else if body.contains("miracast") || body.contains("wfd") || body.contains("\"mode\":2") || body.contains("\"mode\":\"2\"") {
                    println!("\x1b[1;32m[web-server]\x1b[0m Switching to Mode 2 (Windows Miracast WFD)...");
                    let _ = crate::flow::ARBITER.request_level0(crate::flow::DesktopMode::Mode2Miracast, 86);
                    if let Ok(mut cfg) = CONFIG.lock() {
                        cfg.mode3 = false;
                        cfg.mode1 = false;
                        cfg.mode2 = true;
                        cfg.active_transport = "mode2_miracast".to_string();
                    }
                    crate::display::SplashEngine::show_miracast();
                    let pipe = pipeline_mgr.clone();
                    thread::spawn(move || {
                        pipe.stop();
                        let _ = pipe.start(PipelineKind::MiracastMp2t { port: crate::wfd::WFD_RTP_PORT });
                    });
                } else if body.contains("network") || body.contains("udp") || body.contains("\"mode\":1") || body.contains("\"mode\":\"1\"") {
                    println!("\x1b[1;32m[web-server]\x1b[0m Switching to Mode 1 (Network UDP 5000)...");
                    let _ = crate::flow::ARBITER.request_level0(crate::flow::DesktopMode::Mode1NetworkUdp, 86);
                    if let Ok(mut cfg) = CONFIG.lock() {
                        cfg.mode3 = false;
                        cfg.mode2 = false;
                        cfg.mode1 = true;
                        cfg.active_transport = "mode1_udp".to_string();
                    }
                    let pipe = pipeline_mgr.clone();
                    thread::spawn(move || {
                        let default_kind = PipelineKind::RawH264Rtp { port: default_udp_port };
                        let _ = pipe.resume(default_kind);
                    });
                }
            }
            send_response(
                &mut stream,
                "200 OK",
                "application/json",
                b"{\"status\":\"switched\"}",
            );
        }
        ("POST", "/api/system/update") => {
            println!("\x1b[1;34m[web-server]\x1b[0m OTA Firmware/Appliance update requested via Web API.");
            let update_url = if let Some(idx) = req_str.find("\r\n\r\n") {
                extract_json_str(&req_str[idx + 4..], "url").unwrap_or("http://192.168.7.1:8000/initramfs.cpio.gz")
            } else {
                "http://192.168.7.1:8000/initramfs.cpio.gz"
            };
            let url_clone = update_url.to_string();
            thread::spawn(move || {
                println!("[ota-update] Mounting /mnt/boot...");
                let _ = std::process::Command::new("mkdir").args(&["-p", "/mnt/boot"]).output();
                let _ = std::process::Command::new("mount").args(&["-t", "vfat", "/dev/mmcblk0p1", "/mnt/boot"]).output();
                println!("[ota-update] Fetching {}...", url_clone);
                let res = std::process::Command::new("wget")
                    .args(&["-q", "-O", "/mnt/boot/initramfs.cpio.gz", &url_clone])
                    .output();
                if let Ok(st) = res {
                    if st.status.success() {
                        println!("[ota-update] Update written successfully! Syncing and rebooting in 1s...");
                        let _ = std::process::Command::new("sync").output();
                        let _ = std::process::Command::new("umount").arg("/mnt/boot").output();
                        thread::sleep(Duration::from_secs(1));
                        let _ = std::process::Command::new("/sbin/reboot").arg("-f").output();
                        return;
                    }
                }
                eprintln!("[ota-update] Update download failed!");
            });
            send_response(&mut stream, "200 OK", "application/json", b"{\"status\":\"updating\"}");
        }
        ("GET", "/api/network") => {
            let net_json = get_network_status_json();
            send_response(&mut stream, "200 OK", "application/json", net_json.as_bytes());
        }
        ("POST", "/api/network") => {
            if let Some(idx) = req_str.find("\r\n\r\n") {
                let body = &req_str[idx + 4..];
                apply_network_config(body);
            }
            send_response(&mut stream, "200 OK", "application/json", b"{\"status\":\"ok\"}");
        }
        _ => {
            send_response(&mut stream, "404 Not Found", "text/plain", b"404 Not Found");
        }
    }
}

/// Executes `send_response` operational routine.
fn send_response(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-cache, no-store, must-revalidate, max-age=0\r\nPragma: no-cache\r\nExpires: 0\r\n\r\n",
        status,
        content_type,
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

/// Executes `send_dial_response` operational routine.
fn send_dial_response(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8], app_url: &str) {
    let header = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nApplication-URL: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Expose-Headers: Location, Application-URL\r\nCache-Control: no-cache, no-store, must-revalidate, max-age=0\r\nPragma: no-cache\r\nExpires: 0\r\n\r\n",
        status,
        content_type,
        body.len(),
        app_url
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

/// Query real-time SoC telemetry (temperature, CPU load, RAM, active streaming mode, and HDMI screen)
fn get_system_telemetry_json(
    is_paused: bool,
    cur_kind: Option<crate::pipeline::PipelineKind>,
    audio_st: &crate::audio::AudioStatus,
) -> String {
    // 1. Temperature
    let temp_str = fs::read_to_string("/sys/class/thermal/thermal_zone0/temp")
        .unwrap_or_else(|_| "45000".to_string());
    let temp_val = temp_str.trim().parse::<f64>().unwrap_or(45000.0) / 1000.0;

    // 2. CPU load
    let load_str =
        fs::read_to_string("/proc/loadavg").unwrap_or_else(|_| "0.15 0.10 0.05".to_string());
    let cpu_load = load_str.split_whitespace().next().unwrap_or("0.15");

    // 3. Free RAM
    let mem_str = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut mem_free_mb = 250;
    for line in mem_str.lines() {
        if line.starts_with("MemAvailable:") || line.starts_with("MemFree:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(kb) = parts[1].parse::<u64>() {
                    mem_free_mb = kb / 1024;
                    break;
                }
            }
        }
    }

    let is_active = cur_kind.is_some();
    let pipeline_state = if is_paused {
        "paused"
    } else if is_active {
        "active"
    } else {
        "idle"
    };

    let active_trans = if let Ok(cfg) = CONFIG.lock() {
        if cfg.active_transport.is_empty() {
            "mode3_usb_bulk".to_string()
        } else {
            cfg.active_transport.clone()
        }
    } else {
        "mode3_usb_bulk".to_string()
    };

    let (mode_id, mode_name, mode_icon, mode_proto, mode_port, mode_details) = match cur_kind {
        Some(crate::pipeline::PipelineKind::RawH264Rtp { port }) => (
            "mode1_udp",
            "Mode 1: UDP Network (Linux Wayland / X11)",
            "🐧",
            "RTP H.264 / RFC 4571",
            port,
            format!("UDP Port {} • Latency < 15ms • VA-API/M2M Pipeline", port),
        ),
        Some(crate::pipeline::PipelineKind::MiracastMp2t { port }) => (
            "mode2_miracast",
            "Mode 2: Windows Miracast (Wi-Fi Display)",
            "🪟",
            "MPEG-TS / RTSP WFD",
            port,
            format!("RTSP Port {} • Windows Win+K • V4L2 M2M Hardware Decode", port),
        ),
        Some(crate::pipeline::PipelineKind::UsbBulkPipe { fd: _ }) => (
            "mode3_usb_bulk",
            "Mode 3: USB Bulk Direct (480 Mbps)",
            "⚡",
            "USB FunctionFS Bulk Raw H.264",
            0,
            "USB 2.0 High-Speed • Zero-Network • Latency < 1ms".to_string(),
        ),
        None => {
            if is_paused {
                (
                    "standby",
                    "Extension Disabled (Standby)",
                    "⏹",
                    "Standby",
                    0,
                    "Screen extension is turned off. Ready to resume.".to_string(),
                )
            } else if active_trans == "mode2_miracast" {
                (
                    "mode2_miracast",
                    "Mode 2: Windows Miracast (Ready)",
                    "🪟",
                    "RTSP WFD / TCP 7236",
                    7236,
                    "Ready for connection: press Win + K on Windows or cast via GNOME Displays".to_string(),
                )
            } else {
                (
                    "idle",
                    "Awaiting Stream (Standby / Splash)",
                    "⏳",
                    "No Active Stream",
                    0,
                    "Receiver ready displaying splash screen with IP & QR Code".to_string(),
                )
            }
        }
    };

    let all_displays = crate::display::MonitorInfo::read_all_realtime();
    let mon = all_displays
        .iter()
        .find(|m| m.connected)
        .cloned()
        .or_else(|| all_displays.first().cloned())
        .unwrap_or_else(crate::display::MonitorInfo::read_realtime);

    let displays_json: Vec<String> = all_displays.iter().map(|d| {
        format!(
            "{{\"connector\":\"{}\",\"connector_friendly\":\"{}\",\"hardware_model\":\"{}\",\"connected\":{},\"name\":\"{}\",\"active_mode\":\"{}\",\"preferred_mode\":\"{}\",\"vpu\":\"{}\",\"has_audio\":{}}}",
            d.connector, d.connector_friendly, d.hardware_model, d.connected, d.name, d.active_mode, d.preferred_mode, d.vpu, d.has_audio
        )
    }).collect();
    let displays_str = format!("[{}]", displays_json.join(","));

    // 4. Subhardware Clocks (SoC Broadcom BCM2835 / VideoCore IV)
    let read_clk_mhz = |path: &str, def_hz: u64| -> u64 {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(def_hz) / 1_000_000
    };

    let h264_mhz = read_clk_mhz("/sys/kernel/debug/clk/h264/clk_rate", 200_000_000);
    let vpu_mhz = read_clk_mhz("/sys/kernel/debug/clk/vpu/clk_rate", 400_000_000);
    let v3d_mhz = read_clk_mhz("/sys/kernel/debug/clk/fw-clk-v3d/clk_rate", 250_000_000);
    let arm_mhz = fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|khz| khz / 1000)
        .unwrap_or_else(|| read_clk_mhz("/sys/kernel/debug/clk/fw-clk-arm/clk_rate", 1_000_000_000));
    let sdram_mhz = read_clk_mhz("/sys/kernel/debug/clk/sdram/clk_rate", 166_666_668);
    let core_mhz = read_clk_mhz("/sys/kernel/debug/clk/fw-clk-core/clk_rate", 400_000_000);

    // 5. Power Estimation
    let cpu_pct = cpu_load.parse::<f64>().unwrap_or(0.15) * 100.0;
    let est_watts = 0.55 + (cpu_pct / 100.0) * 0.35 + (if is_active { 0.35 } else { 0.05 });
    let est_ma = (est_watts / 5.0) * 1000.0;

    let (h_lvl, h_name, h_owner, h_vis) = {
        let arb = &crate::flow::ARBITER;
        let lvl = arb.current_level();
        let lvl_num = match lvl {
            crate::flow::ServiceLevel::Level0Desktop(_) => 0u8,
            crate::flow::ServiceLevel::Level1Media(_) => 1u8,
            crate::flow::ServiceLevel::Level2AudioOnly { .. } => 2u8,
            crate::flow::ServiceLevel::Level3Standby => 3u8,
        };
        let lvl_name = match lvl {
            crate::flow::ServiceLevel::Level0Desktop(m) => format!("Level 0: Desktop Streaming ({})", m.as_str()),
            crate::flow::ServiceLevel::Level1Media(_) => "Level 1: Dedicated Media Cast".to_string(),
            crate::flow::ServiceLevel::Level2AudioOnly { .. } => "Level 2: Standalone PC Audio".to_string(),
            crate::flow::ServiceLevel::Level3Standby => "Level 3: Standby / Ready Splash".to_string(),
        };
        let owner = match arb.display_owner() {
            crate::flow::DisplayOwner::KmsPlane(id) => format!("KMS Plane {}", id),
            crate::flow::DisplayOwner::Framebuffer => "Framebuffer (/dev/fb0)".to_string(),
            crate::flow::DisplayOwner::Unassigned => "Unassigned".to_string(),
        };
        (lvl_num, lvl_name, owner, arb.is_visualizer_user_enabled())
    };

    format!(
        "{{\"temp\":\"{:.1}\",\"cpu\":\"{}%\",\"ram\":{},\"stream_state\":\"{}\",\"active_transport\":\"{}\",\"active_mode\":{{\"id\":\"{}\",\"name\":\"{}\",\"icon\":\"{}\",\"protocol\":\"{}\",\"port\":{},\"details\":\"{}\"}},\"hierarchy\":{{\"level\":{},\"level_name\":\"{}\",\"display_owner\":\"{}\",\"visualizer_enabled\":{}}},\"displays\":{},\"hdmi\":{{\"connector\":\"{}\",\"connector_friendly\":\"{}\",\"hardware_model\":\"{}\",\"connected\":{},\"name\":\"{}\",\"active_mode\":\"{}\",\"preferred_mode\":\"{}\",\"vpu\":\"{}\",\"has_audio\":{}}},\"monitor\":{{\"connected\":{},\"name\":\"{}\",\"preferred_mode\":\"{}\",\"active_mode\":\"{}\",\"vpu\":\"{}\",\"connector\":\"{}\",\"connector_friendly\":\"{}\",\"hardware_model\":\"{}\",\"has_audio\":{}}},\"audio\":{},\"clocks\":{{\"h264_mhz\":{},\"vpu_mhz\":{},\"arm_mhz\":{},\"v3d_mhz\":{},\"core_mhz\":{},\"sdram_mhz\":{}}},\"power\":{{\"estimated_watts\":{:.2},\"current_ma\":{:.0},\"voltage_core_volts\":1.20}}}}",
        temp_val,
        cpu_load,
        mem_free_mb,
        pipeline_state,
        active_trans,
        mode_id,
        mode_name,
        mode_icon,
        mode_proto,
        mode_port,
        mode_details,
        h_lvl,
        h_name,
        h_owner,
        h_vis,
        displays_str,
        mon.connector,
        mon.connector_friendly,
        mon.hardware_model,
        mon.connected,
        mon.name,
        mon.active_mode,
        mon.preferred_mode,
        mon.vpu,
        mon.has_audio,
        mon.connected,
        mon.name,
        mon.preferred_mode,
        mon.active_mode,
        mon.vpu,
        mon.connector,
        mon.connector_friendly,
        mon.hardware_model,
        mon.has_audio,
        audio_st.to_json(),
        h264_mhz,
        vpu_mhz,
        arm_mhz,
        v3d_mhz,
        core_mhz,
        sdram_mhz,
        est_watts,
        est_ma
    )
}

/// Forward JSON configuration to ext-sender on UDP port 5001
pub fn forward_config_to_sender(payload: &str) {
    let data = payload.as_bytes().to_vec();
    thread::spawn(move || {
        if let Ok(sock) = UdpSocket::bind("0.0.0.0:0") {
            let _ = sock.set_broadcast(true);
            let _ = sock.set_write_timeout(Some(Duration::from_millis(50)));
            // Send to host USB IP (192.168.7.1), directed broadcast (192.168.7.255), and local loopback
            let _ = sock.send_to(&data, "192.168.7.1:5001");
            let _ = sock.send_to(&data, "192.168.7.255:5001");
            let _ = sock.send_to(&data, "127.0.0.1:5001");
        }
    });
}

/// Retrieve network interfaces status (IPs, DHCP server, gateway)
fn read_network_conf() -> (String, String, String, String, String, String) {
    let mut iface = "eth0".to_string();
    let mut mode = "static".to_string();
    let mut ip = "192.168.1.50".to_string();
    let mut netmask = "255.255.255.0".to_string();
    let mut gateway = "192.168.1.1".to_string();
    let mut dns = "1.1.1.1, 8.8.8.8".to_string();

    let conf_path = if fs::metadata("/boot/network.conf").is_ok() {
        "/boot/network.conf"
    } else if fs::metadata("/etc/network.conf").is_ok() {
        "/etc/network.conf"
    } else if fs::metadata("/mnt/boot/network.conf").is_ok() {
        "/mnt/boot/network.conf"
    } else {
        ""
    };

    if !conf_path.is_empty() {
        if let Ok(content) = fs::read_to_string(conf_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') || !trimmed.contains('=') {
                    continue;
                }
                let mut parts = trimmed.splitn(2, '=');
                let key = parts.next().unwrap_or("").trim();
                let val = parts.next().unwrap_or("").trim().trim_matches('"');
                match key {
                    "INTERFACE" => iface = val.to_string(),
                    "MODE" => mode = val.to_string(),
                    "IP" => ip = val.to_string(),
                    "NETMASK" => netmask = val.to_string(),
                    "GATEWAY" => gateway = val.to_string(),
                    "DNS" => dns = val.to_string(),
                    _ => {}
                }
            }
        }
    }

    (iface, mode, ip, netmask, gateway, dns)
}

/// Retrieves the network status json.
fn get_network_status_json() -> String {
    let (usb0_ipv4, usb0_ipv6) = get_interface_addrs("usb0");
    let (eth0_ipv4, eth0_ipv6) = get_interface_addrs("eth0");
    let (wlan0_ipv4, wlan0_ipv6) = get_interface_addrs("wlan0");

    let usb0_detected = fs::metadata("/sys/class/net/usb0").is_ok();
    let eth0_detected = fs::metadata("/sys/class/net/eth0").is_ok();
    let wlan0_detected = fs::metadata("/sys/class/net/wlan0").is_ok();

    let lease = crate::dhcp::get_active_lease();
    let host_mac = lease.as_ref().map(|l| l.mac.as_str()).unwrap_or("pending");
    let dhcp_running = true; // Native pure-Rust DHCP server active in-process

    let (c_iface, c_mode, c_ip, c_netmask, c_gw, c_dns) = read_network_conf();

    format!(
        concat!(
            "{{",
            "\"usb0\":{{\"detected\":{},\"ipv4\":\"{}\",\"ipv6\":\"{}\",\"dhcp_server\":{},\"host_ip\":\"192.168.7.1\",\"host_mac\":\"{}\",\"gateway\":\"none\"}},",
            "\"eth0\":{{\"detected\":{},\"ipv4\":\"{}\",\"ipv6\":\"{}\"}},",
            "\"wlan0\":{{\"detected\":{},\"ipv4\":\"{}\",\"ipv6\":\"{}\"}},",
            "\"config\":{{\"interface\":\"{}\",\"mode\":\"{}\",\"ip\":\"{}\",\"netmask\":\"{}\",\"gateway\":\"{}\",\"dns\":\"{}\"}}",
            "}}"
        ),
        usb0_detected,
        usb0_ipv4.unwrap_or_else(|| "192.168.7.2".to_string()),
        usb0_ipv6.unwrap_or_else(|| "none".to_string()),
        dhcp_running,
        host_mac,
        eth0_detected,
        eth0_ipv4.unwrap_or_else(|| "disconnected".to_string()),
        eth0_ipv6.unwrap_or_else(|| "none".to_string()),
        wlan0_detected,
        wlan0_ipv4.unwrap_or_else(|| "disconnected".to_string()),
        wlan0_ipv6.unwrap_or_else(|| "none".to_string()),
        c_iface,
        c_mode,
        c_ip,
        c_netmask,
        c_gw,
        c_dns
    )
}

/// Retrieves the interface addrs.
fn get_interface_addrs(iface: &str) -> (Option<String>, Option<String>) {
    let output = match std::process::Command::new("ifconfig").arg(iface).output() {
        Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
        Err(_) => return (None, None),
    };
    let mut ipv4 = None;
    let mut ipv6 = None;
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("inet addr:") {
            if let Some(ip) = trimmed["inet addr:".len()..].split_whitespace().next() {
                ipv4 = Some(ip.to_string());
            }
        } else if trimmed.starts_with("inet ") && !trimmed.starts_with("inet6") {
            if let Some(ip) = trimmed["inet ".len()..].split_whitespace().next() {
                ipv4 = Some(ip.to_string());
            }
        }

        if trimmed.starts_with("inet6 addr:") {
            if let Some(ip) = trimmed["inet6 addr:".len()..].split_whitespace().next() {
                ipv6 = Some(ip.to_string());
            }
        } else if trimmed.starts_with("inet6 ") {
            if let Some(ip) = trimmed["inet6 ".len()..].split_whitespace().next() {
                ipv6 = Some(ip.to_string());
            }
        }
    }
    (ipv4, ipv6)
}

/// Executes `extract_json_str` operational routine.
fn extract_json_str<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let pattern = format!("\"{}\"", key);
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let colon_idx = rest.find(':')?;
    let after_colon = rest[colon_idx + 1..].trim_start();
    if after_colon.starts_with('"') {
        let end_quote = after_colon[1..].find('"')?;
        Some(&after_colon[1..1 + end_quote])
    } else {
        None
    }
}

/// Executes `extract_json_u32` operational routine.
fn extract_json_u32(json: &str, key: &str) -> Option<u32> {
    let pattern = format!("\"{}\"", key);
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let colon_idx = rest.find(':')?;
    let after_colon = rest[colon_idx + 1..].trim_start();
    let num_str: String = after_colon.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse().ok()
}

/// Executes `extract_json_u64` operational routine.
fn extract_json_u64(json: &str, key: &str) -> Option<u64> {
    let pattern = format!("\"{}\"", key);
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let colon_idx = rest.find(':')?;
    let after_colon = rest[colon_idx + 1..].trim_start();
    let num_str: String = after_colon.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse().ok()
}

/// Executes `extract_json_f32` operational routine.
fn extract_json_f32(json: &str, key: &str) -> Option<f32> {
    let pattern = format!("\"{}\"", key);
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let colon_idx = rest.find(':')?;
    let after_colon = rest[colon_idx + 1..].trim_start();
    let num_str: String = after_colon.chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();
    num_str.parse().ok()
}

/// Executes `extract_json_bool` operational routine.
fn extract_json_bool(json: &str, key: &str) -> Option<bool> {
    let pattern = format!("\"{}\"", key);
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let colon_idx = rest.find(':')?;
    let after_colon = rest[colon_idx + 1..].trim_start();
    if after_colon.starts_with("true") {
        Some(true)
    } else if after_colon.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

/// Executes `apply_network_config` operational routine.
fn apply_network_config(payload: &str) {
    println!("\x1b[1;34m[web-server]\x1b[0m Applying network config: {}", payload);
    let iface = extract_json_str(payload, "interface")
        .or_else(|| extract_json_str(payload, "iface"))
        .unwrap_or("eth0");
    let mode = extract_json_str(payload, "mode").unwrap_or("static");
    let ip = extract_json_str(payload, "ip").unwrap_or("192.168.1.50");
    let netmask = extract_json_str(payload, "netmask").unwrap_or("255.255.255.0");
    let gateway = extract_json_str(payload, "gateway").unwrap_or("192.168.1.1");
    let dns = extract_json_str(payload, "dns").unwrap_or("1.1.1.1,8.8.8.8");
    let ipv6_mode = extract_json_str(payload, "ipv6").unwrap_or("auto");

    // Persist to configuration files
    let conf_content = format!(
        "# ExtMonitor Network Configuration\nINTERFACE={}\nMODE={}\nIP={}\nNETMASK={}\nGATEWAY={}\nDNS={}\n",
        iface, mode, ip, netmask, gateway, dns
    );
    let _ = fs::write("/etc/network.conf", &conf_content);
    if fs::metadata("/boot").is_ok() {
        let _ = fs::write("/boot/network.conf", &conf_content);
        let _ = std::process::Command::new("sync").output();
    } else if fs::metadata("/mnt/boot").is_ok() {
        let _ = fs::write("/mnt/boot/network.conf", &conf_content);
        let _ = std::process::Command::new("sync").output();
    }

    // Only allow known network interfaces for security
    if iface != "usb0" && iface != "eth0" && iface != "wlan0" {
        eprintln!("\x1b[1;31m[web-server]\x1b[0m Invalid network interface: {}", iface);
        return;
    }

    // Configure IPv6
    let ipv6_proc = format!("/proc/sys/net/ipv6/conf/{}/disable_ipv6", iface);
    if ipv6_mode == "disable" {
        let _ = fs::write(&ipv6_proc, "1");
    } else {
        let _ = fs::write(&ipv6_proc, "0");
    }

    if mode == "dhcp" {
        if iface == "usb0" {
            // For usb0, restore default OTG IP and restart udhcpd zero-gateway server
            let _ = std::process::Command::new("ifconfig")
                .args(&["usb0", "192.168.7.2", "netmask", "255.255.255.0", "up"])
                .output();
            let _ = std::process::Command::new("killall").arg("udhcpd").output();
            let _ = std::process::Command::new("udhcpd").arg("/etc/udhcpd.conf").spawn();
        } else {
            // Physical interface (eth0/wlan0): run DHCP client
            let _ = std::process::Command::new("pkill")
                .args(&["-f", &format!("udhcpc -i {}", iface)])
                .output();
            let _ = std::process::Command::new("udhcpc")
                .args(&["-i", iface, "-b", "-q"])
                .spawn();
        }
    } else if mode == "static" && !ip.is_empty() {
        // Kill dhcp client on this interface
        let _ = std::process::Command::new("pkill")
            .args(&["-f", &format!("udhcpc -i {}", iface)])
            .output();

        // Apply static IP
        let _ = std::process::Command::new("ifconfig")
            .args(&[iface, ip, "netmask", netmask, "up"])
            .output();

        // Apply Gateway if specified and not "none"
        if !gateway.is_empty() && gateway != "none" {
            let _ = std::process::Command::new("route")
                .args(&["add", "default", "gw", gateway, iface])
                .output();
        }

        // Apply DNS
        if !dns.is_empty() {
            let mut resolv = String::new();
            for server in dns.split(',') {
                let s = server.trim();
                if !s.is_empty() {
                    resolv.push_str(&format!("nameserver {}\n", s));
                }
            }
            let _ = fs::write("/etc/resolv.conf", resolv);
        }
    }

    // Call apply-network.sh helper if present
    if fs::metadata("/usr/local/bin/apply-network.sh").is_ok() {
        let _ = std::process::Command::new("/usr/local/bin/apply-network.sh").output();
    }
}

const CONNECT_SCRIPT: &str = include_str!("../../scripts/connect.sh");
const UDEV_RULES: &str = include_str!("../../scripts/99-ext-monitor.rules");

/// Executes `serve_file_or_fallback` operational routine.
fn serve_file_or_fallback(stream: &mut TcpStream, path: &str, content_type: &str, is_head: bool) {
    if let Ok(mut f) = fs::File::open(path) {
        let mut buf = Vec::new();
        if f.read_to_end(&mut buf).is_ok() {
            let body = if is_head { &[][..] } else { &buf };
            send_response(stream, "200 OK", content_type, body);
            return;
        }
    }
    let fallback = format!("File {} not available directly on Pi Zero flash yet. Please use /connect.sh to fetch or build directly.", path);
    send_response(stream, "404 Not Found", "text/plain", fallback.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_all_modes_disabled_enters_standby() {
        let (m1, m2, m3, res) = evaluate_mode_switch(
            r#"{"mode1":false,"mode2":false,"mode3":false}"#,
            "mode1_udp",
            true,
            false,
            false,
        );
        assert_eq!(m1, false);
        assert_eq!(m2, false);
        assert_eq!(m3, false);
        assert_eq!(res, ModeSwitchResult::EnterStandby);
    }

    #[test]
    fn test_evaluate_explicit_transport_activation_mode1() {
        let (m1, _m2, m3, res) = evaluate_mode_switch(
            r#"{"active_transport":"mode1_udp"}"#,
            "mode3_usb_bulk",
            true,
            false,
            true,
        );
        assert_eq!(m1, true);
        assert_eq!(m3, true);
        assert_eq!(res, ModeSwitchResult::SwitchTransport("mode1_udp".to_string()));
    }

    #[test]
    fn test_evaluate_explicit_transport_activation_mode3() {
        let (_m1, _m2, _m3, res) = evaluate_mode_switch(
            r#"{"transport":"mode3_usb_bulk"}"#,
            "mode1_udp",
            true,
            false,
            true,
        );
        assert_eq!(res, ModeSwitchResult::SwitchTransport("mode3_usb_bulk".to_string()));
    }

    #[test]
    fn test_evaluate_explicit_transport_activation_mode2() {
        let (_m1, _m2, _m3, res) = evaluate_mode_switch(
            r#"{"active_transport":"mode2_miracast"}"#,
            "mode1_udp",
            true,
            true,
            false,
        );
        assert_eq!(res, ModeSwitchResult::SwitchTransport("mode2_miracast".to_string()));
    }

    #[test]
    fn test_evaluate_mode_toggle_transfers_active_stream() {
        // Turning on mode3 switches active transport to mode3_usb_bulk
        let (_m1, _m2, m3, res) = evaluate_mode_switch(
            r#"{"mode3":true}"#,
            "mode1_udp",
            true,
            false,
            false,
        );
        assert_eq!(m3, true);
        assert_eq!(res, ModeSwitchResult::SwitchTransport("mode3_usb_bulk".to_string()));

        // Turning on mode1 switches active transport to mode1_udp
        let (m1, _m2, _m3, res) = evaluate_mode_switch(
            r#"{"mode1":true}"#,
            "mode3_usb_bulk",
            false,
            false,
            true,
        );
        assert_eq!(m1, true);
        assert_eq!(res, ModeSwitchResult::SwitchTransport("mode1_udp".to_string()));
    }

    #[test]
    fn test_evaluate_active_mode_disabled_fallback_to_other_mode() {
        // Mode1 was active, but mode1 is toggled off while mode3 is enabled -> fallback to mode3
        let (m1, _m2, m3, res) = evaluate_mode_switch(
            r#"{"mode1":false}"#,
            "mode1_udp",
            true,
            false,
            true,
        );
        assert_eq!(m1, false);
        assert_eq!(m3, true);
        assert_eq!(res, ModeSwitchResult::SwitchTransport("mode3_usb_bulk".to_string()));

        // Mode3 was active, but mode3 is toggled off while mode1 is enabled -> fallback to mode1
        let (m1, _m2, m3, res) = evaluate_mode_switch(
            r#"{"mode3":false}"#,
            "mode3_usb_bulk",
            true,
            false,
            true,
        );
        assert_eq!(m3, false);
        assert_eq!(m1, true);
        assert_eq!(res, ModeSwitchResult::SwitchTransport("mode1_udp".to_string()));
    }

    #[test]
    fn test_evaluate_secondary_mode_disabled_does_not_switch_active() {
        // Mode1 is active, and mode3 is turned off -> listener disabled, active remains mode1
        let (_m1, _m2, m3, res) = evaluate_mode_switch(
            r#"{"mode3":false}"#,
            "mode1_udp",
            true,
            false,
            true,
        );
        assert_eq!(m3, false);
        assert_eq!(res, ModeSwitchResult::DisableListener("mode3".to_string()));
    }

    #[test]
    fn test_evaluate_explicit_standby_transport() {
        let (m1, m2, m3, res) = evaluate_mode_switch(
            r#"{"active_transport":"standby"}"#,
            "mode1_udp",
            true,
            false,
            false,
        );
        assert_eq!(m1, false);
        assert_eq!(m2, false);
        assert_eq!(m3, false);
        assert_eq!(res, ModeSwitchResult::EnterStandby);
    }
}


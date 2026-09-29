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
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

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

    match (method, path) {
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
        ("POST", "/api/modes") => {
            if let Some(idx) = req_str.find("\r\n\r\n") {
                let body = &req_str[idx + 4..];
                let mut m1_change = None;
                let mut m3_change = None;
                if let Ok(mut cfg) = CONFIG.lock() {
                    if let Some(m1) = extract_json_bool(body, "mode1") {
                        cfg.mode1 = m1;
                        m1_change = Some(m1);
                    }
                    if let Some(m2) = extract_json_bool(body, "mode2") { cfg.mode2 = m2; }
                    if let Some(m3) = extract_json_bool(body, "mode3") {
                        cfg.mode3 = m3;
                        m3_change = Some(m3);
                    }
                }
                if let Some(true) = m3_change {
                    println!("\x1b[1;32m[web-server]\x1b[0m Mode 3 (USB Bulk Direct) requested via Web UI!");
                    let run = running.clone();
                    let pipe = pipeline_mgr.clone();
                    thread::spawn(move || {
                        if let Err(e) = crate::usb_bulk::activate_usb_bulk(run, pipe) {
                            eprintln!("\x1b[1;31m[web-server]\x1b[0m Failed to activate USB Bulk: {}", e);
                        }
                    });
                } else if let Some(false) = m3_change {
                    println!("\x1b[1;33m[web-server]\x1b[0m Mode 3 (USB Bulk Direct) disabled. Returning to Miracast / Standby.");
                    let pipe = pipeline_mgr.clone();
                    thread::spawn(move || {
                        pipe.stop();
                        let _ = pipe.start(PipelineKind::MiracastMp2t { port: crate::wfd::WFD_RTP_PORT });
                    });
                } else if let Some(m1) = m1_change {
                    let pipe = pipeline_mgr.clone();
                    thread::spawn(move || {
                        if !m1 {
                            println!("\x1b[1;33m[web-server]\x1b[0m Mode 1 (Linux UDP) turned OFF by user flag.");
                            pipe.pause();
                        } else {
                            println!("\x1b[1;32m[web-server]\x1b[0m Mode 1 (Linux UDP) turned ON by user flag.");
                            let default_kind = PipelineKind::RawH264Rtp { port: default_udp_port };
                            let _ = pipe.resume(default_kind);
                        }
                    });
                }
            }
            send_response(&mut stream, "200 OK", "application/json", b"{\"status\":\"ok\"}");
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
            println!("\x1b[1;33m[web-server]\x1b[0m User requested stream PAUSE via Web UI.");
            pipeline_mgr.pause();
            crate::display::SplashEngine::clear();
            send_response(
                &mut stream,
                "200 OK",
                "application/json",
                b"{\"status\":\"paused\"}",
            );
        }
        ("POST", "/api/stream/start") => {
            println!("\x1b[1;32m[web-server]\x1b[0m User requested stream RESUME via Web UI.");
            let default_kind = PipelineKind::RawH264Rtp {
                port: default_udp_port,
            };
            let _ = pipeline_mgr.resume(default_kind);
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
            if let Some(idx) = req_str.find("\r\n\r\n") {
                let body = &req_str[idx + 4..];
                if body.contains("usb-bulk") || body.contains("bulk") || body.contains("\"mode\":3") || body.contains("\"mode\":\"3\"") {
                    println!("\x1b[1;32m[web-server]\x1b[0m Switching to Mode 3 (USB Bulk Direct)...");
                    if let Ok(mut cfg) = CONFIG.lock() {
                        cfg.mode3 = true;
                        cfg.mode1 = false;
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
                    if let Ok(mut cfg) = CONFIG.lock() {
                        cfg.mode3 = false;
                        cfg.mode2 = true;
                    }
                    let pipe = pipeline_mgr.clone();
                    thread::spawn(move || {
                        pipe.stop();
                        let _ = pipe.start(PipelineKind::MiracastMp2t { port: crate::wfd::WFD_RTP_PORT });
                    });
                } else if body.contains("network") || body.contains("udp") || body.contains("\"mode\":1") || body.contains("\"mode\":\"1\"") {
                    println!("\x1b[1;32m[web-server]\x1b[0m Switching to Mode 1 (Network UDP 5000)...");
                    if let Ok(mut cfg) = CONFIG.lock() {
                        cfg.mode3 = false;
                        cfg.mode1 = true;
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
                        let _ = std::process::Command::new("/sbin/reboot").output();
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

fn send_response(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
        status,
        content_type,
        body.len()
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

    let (mode_id, mode_name, mode_icon, mode_proto, mode_port, mode_details) = match cur_kind {
        Some(crate::pipeline::PipelineKind::RawH264Rtp { port }) => (
            "mode1_udp",
            "Modo 1: Rede UDP (Linux Wayland / X11)",
            "🐧",
            "RTP H.264 / RFC 4571",
            port,
            format!("Porta UDP {} • Latência < 15ms • Pipeline VA-API/M2M", port),
        ),
        Some(crate::pipeline::PipelineKind::MiracastMp2t { port }) => (
            "mode2_miracast",
            "Modo 2: Windows Miracast (Wi-Fi Display)",
            "🪟",
            "MPEG-TS / RTSP WFD",
            port,
            format!("Porta RTSP {} • Windows Win+K • Decodificação V4L2 M2M", port),
        ),
        Some(crate::pipeline::PipelineKind::UsbBulkPipe { fd: _ }) => (
            "mode3_usb_bulk",
            "Modo 3: USB Bulk Direct (480 Mbps)",
            "⚡",
            "USB FunctionFS Bulk Raw H.264",
            0,
            "Barramento USB 2.0 High-Speed • Zero-Network • Latência < 1ms".to_string(),
        ),
        None => (
            "idle",
            "Aguardando Transmissão (Standby / Splash)",
            "⏳",
            "Nenhum Fluxo Ativo",
            0,
            "Receptor em prontidão exibindo tela de splash com IP e QR Code".to_string(),
        ),
    };

    let mon = crate::display::MonitorInfo::read_realtime();

    format!(
        "{{\"temp\":\"{:.1}\",\"cpu\":\"{}%\",\"ram\":{},\"stream_state\":\"{}\",\"active_mode\":{{\"id\":\"{}\",\"name\":\"{}\",\"icon\":\"{}\",\"protocol\":\"{}\",\"port\":{},\"details\":\"{}\"}},\"hdmi\":{{\"connector\":\"{}\",\"connector_friendly\":\"{}\",\"hardware_model\":\"{}\",\"connected\":{},\"name\":\"{}\",\"active_mode\":\"{}\",\"preferred_mode\":\"{}\",\"vpu\":\"{}\"}},\"monitor\":{{\"connected\":{},\"name\":\"{}\",\"preferred_mode\":\"{}\",\"active_mode\":\"{}\",\"vpu\":\"{}\",\"connector\":\"{}\",\"connector_friendly\":\"{}\",\"hardware_model\":\"{}\"}},\"audio\":{}}}",
        temp_val,
        cpu_load,
        mem_free_mb,
        pipeline_state,
        mode_id,
        mode_name,
        mode_icon,
        mode_proto,
        mode_port,
        mode_details,
        mon.connector,
        mon.connector_friendly,
        mon.hardware_model,
        mon.connected,
        mon.name,
        mon.active_mode,
        mon.preferred_mode,
        mon.vpu,
        mon.connected,
        mon.name,
        mon.preferred_mode,
        mon.active_mode,
        mon.vpu,
        mon.connector,
        mon.connector_friendly,
        mon.hardware_model,
        audio_st.to_json()
    )
}

/// Forward JSON configuration to ext-sender on UDP port 5001
fn forward_config_to_sender(payload: &str) {
    if let Ok(sock) = UdpSocket::bind("0.0.0.0:0") {
        // Send to host IP (192.168.7.1) and local loopback
        let _ = sock.send_to(payload.as_bytes(), "192.168.7.1:5001");
        let _ = sock.send_to(payload.as_bytes(), "127.0.0.1:5001");
    }
}

/// Retrieve network interfaces status (IPs, DHCP server, gateway)
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

    format!(
        concat!(
            "{{",
            "\"usb0\":{{\"detected\":{},\"ipv4\":\"{}\",\"ipv6\":\"{}\",\"dhcp_server\":{},\"host_ip\":\"192.168.7.1\",\"host_mac\":\"{}\",\"gateway\":\"none\"}},",
            "\"eth0\":{{\"detected\":{},\"ipv4\":\"{}\",\"ipv6\":\"{}\"}},",
            "\"wlan0\":{{\"detected\":{},\"ipv4\":\"{}\",\"ipv6\":\"{}\"}}",
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
        wlan0_ipv6.unwrap_or_else(|| "none".to_string())
    )
}

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

fn extract_json_u32(json: &str, key: &str) -> Option<u32> {
    let pattern = format!("\"{}\"", key);
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let colon_idx = rest.find(':')?;
    let after_colon = rest[colon_idx + 1..].trim_start();
    let num_str: String = after_colon.chars().take_while(|c| c.is_ascii_digit()).collect();
    num_str.parse().ok()
}

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

fn apply_network_config(payload: &str) {
    println!("\x1b[1;34m[web-server]\x1b[0m Applying network config: {}", payload);
    let iface = extract_json_str(payload, "iface").unwrap_or("eth0");
    let mode = extract_json_str(payload, "mode").unwrap_or("dhcp");
    let ip = extract_json_str(payload, "ip").unwrap_or("");
    let netmask = extract_json_str(payload, "netmask").unwrap_or("255.255.255.0");
    let gateway = extract_json_str(payload, "gateway").unwrap_or("");
    let dns = extract_json_str(payload, "dns").unwrap_or("");
    let ipv6_mode = extract_json_str(payload, "ipv6").unwrap_or("auto");

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
}

const CONNECT_SCRIPT: &str = include_str!("../../scripts/connect.sh");
const UDEV_RULES: &str = include_str!("../../scripts/99-ext-monitor.rules");

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

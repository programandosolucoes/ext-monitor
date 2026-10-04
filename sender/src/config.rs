//! Configuration and CLI argument parser for ext-sender
//!
//! Follows Clean Code and Single Responsibility Principle (SRP).
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use crate::i18n::{self, Language};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Enumerates supported options for Colorprofile.
pub enum ColorProfile {
    TrueColor,  // Standard 24-bit color (VBR 75% avg, dynamic QP)
    Economy256, // Emulated 256-color coarse quantization (min-qp 30, max-qp 44, VBR 50%)
    Grayscale,  // Monochrome terminal mode (saturation=0.0)
}

impl ColorProfile {
    /// Executes `name` operational routine.
    pub fn name(&self) -> &'static str {
        match self {
            ColorProfile::TrueColor => "24-bit TrueColor (Full)",
            ColorProfile::Economy256 => "256-Color (Coarse QP 30-44)",
            ColorProfile::Grayscale => "Monochrome (Grayscale)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Enumerates supported options for Scalemode.
pub enum ScaleMode {
    Native720p,    // 1:1 Pixel Mapping (1280x720) - Zero downscaling blur, recommended for 720p receiver
    Scale1600x900, // 1600x900 canvas scaled to 720p stream
    Off,           // Passthrough without forced dimensions
}

impl ScaleMode {
    /// Executes `name` operational routine.
    pub fn name(&self) -> &'static str {
        match self {
            ScaleMode::Native720p => "Native 720p (1:1 Direct - Anti-Blur)",
            ScaleMode::Scale1600x900 => "1600x900 (GPU Scaled to 720p)",
            ScaleMode::Off => "Native Passthrough (Scale Off)",
        }
    }

    /// Parses an audio transport selector from a string slice.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "off" | "none" | "no-scale" | "disabled" | "false" => ScaleMode::Off,
            "1600x900" | "900p" | "scale" | "upscale" => ScaleMode::Scale1600x900,
            _ => ScaleMode::Native720p,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Enumerates supported options for Captureengine.
pub enum CaptureEngine {
    Mutter, // GNOME Mutter ScreenCast via D-Bus & PipeWire
    Kms,    // Kernel DRM/KMS Direct Hardware Scanout via DMA-BUF
}

impl CaptureEngine {
    /// Executes `name` operational routine.
    pub fn name(&self) -> &'static str {
        match self {
            CaptureEngine::Mutter => "GNOME Mutter ScreenCast (PipeWire D-Bus)",
            CaptureEngine::Kms => "Kernel Direct DRM/KMS (Hardware Scanout DMA-BUF)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Enumerates supported options for Encoderapi.
pub enum EncoderApi {
    Vaapi,    // AMD & Intel hardware encoding via VA-API
    Nvenc,    // NVIDIA hardware encoding via NVENC
    Qsv,      // Intel QuickSync hardware encoding
    Software, // CPU / x264 zerolatency
}

impl EncoderApi {
    /// Executes `detect` operational routine.
    pub fn detect() -> Self {
        // 1. Check for NVIDIA NVENC
        if let Ok(out) = Command::new("gst-inspect-1.0").arg("nvh264enc").output() {
            if out.status.success() {
                return EncoderApi::Nvenc;
            }
        }
        // 2. Check for VA-API (AMD / Intel)
        if let Ok(out) = Command::new("gst-inspect-1.0").arg("vah264enc").output() {
            if out.status.success() {
                return EncoderApi::Vaapi;
            }
        }
        // 3. Check for Intel QSV
        if let Ok(out) = Command::new("gst-inspect-1.0").arg("qsvh264enc").output() {
            if out.status.success() {
                return EncoderApi::Qsv;
            }
        }
        EncoderApi::Software
    }

    /// Parses an audio transport selector from a string slice.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "vaapi" | "va" | "amd" | "intel" => EncoderApi::Vaapi,
            "nvenc" | "nvidia" | "nv" => EncoderApi::Nvenc,
            "qsv" | "quicksync" => EncoderApi::Qsv,
            "x264" | "software" | "cpu" => EncoderApi::Software,
            _ => Self::detect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Enumerates supported options for Transportkind.
pub enum TransportKind {
    Network { ip: String, port: u16 },
    UsbBulk,
    Miracast,
}

#[derive(Debug, Clone)]
/// Represents Senderconfig configuration and operational state.
pub struct SenderConfig {
    pub target_ip: String,
    pub target_port: u16,
    pub bitrate: u32,
    pub mode: String, // "extend" or "clone"
    pub encoder: EncoderApi,
    pub fps: u32,
    pub hud: bool,
    pub color_profile: ColorProfile,
    pub drop_only: bool,
    pub skip_to_first: bool,
    pub key_int_max: u32,
    pub transport: TransportKind,
    pub capture: CaptureEngine,
    pub audio: bool,
    pub audio_port: u16,
    pub audio_rate: u32,
    pub scale: ScaleMode,
    pub cas: bool,
    pub contrast: f32,
    pub saturation: f32,
    pub debug: bool,
    pub auto_connect: bool,
    pub enable_damage_pacer: bool,
}

impl SenderConfig {
    /// Auto-scales recommended bitrate in kbps based on framerate for sub-15ms latency
    pub fn recommended_bitrate(fps: u32) -> u32 {
        match fps {
            f if f >= 50 => 6000,
            f if f >= 25 => 3000,
            f if f >= 15 => 1800,
            f if f >= 10 => 1200,
            _ => 800,
        }
    }

    /// Parses command line arguments cleanly without nested conditionals
    pub fn parse(args: &[String]) -> Result<Option<Self>, String> {
        // Handle help request first
        if args.iter().any(|a| a == "--help" || a == "-h") {
            let lang = args
                .iter()
                .find(|a| a.starts_with("--lang="))
                .map(|a| Language::from_str(&a[7..]))
                .unwrap_or_else(Language::detect);
            i18n::print_help(lang);
            return Ok(None);
        }

/// Executes `notify_daemon_and_receiver` operational routine.
fn notify_daemon_and_receiver(payload: &str, pi_api_path: Option<(&str, &str)>) {
    if let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") {
        let _ = sock.send_to(payload.as_bytes(), "127.0.0.1:5001");
    }
    if let Some((path, body)) = pi_api_path {
        let url = format!("http://192.168.7.2:8080{}", path);
        let _ = crate::http_client::post_json(&url, body);
    }
}

        // Subcomandos de controle de serviço e processo
        if let Some(cmd) = args.get(1) {
            let extract_mode = |slice: &[String]| -> Option<String> {
                for a in slice {
                    let l = a.to_lowercase();
                    if l == "extend" || l == "clone" || l == "ask" {
                        return Some(l);
                    } else if let Some(m) = l.strip_prefix("--mode=") {
                        return Some(m.to_string());
                    }
                }
                None
            };

            match cmd.as_str() {
                "switch" => {
                    let target = args.get(2).map(|s| s.as_str()).unwrap_or("status");
                    let mode_opt = extract_mode(&args[2..]);
                    let mode_str = mode_opt.as_deref().unwrap_or("extend");

                    match target {
                        "usb-bulk" | "bulk" | "3" | "usb" => {
                            let payload = format!(r#"{{"action":"start","transport":"usb_bulk","mode":"{}"}}"#, mode_str);
                            let pi_body = format!(r#"{{"active_transport":"mode3_usb_bulk","transport":"usb_bulk","action":"start","mode1":false,"mode2":false,"mode3":true,"mode":"{}"}}"#, mode_str);
                            notify_daemon_and_receiver(&payload, Some(("/api/transport/active", &pi_body)));
                            println!("\x1b[1;32m[+] Modo 3 (USB Bulk Direto) ativado com sucesso! [modo: {}]\x1b[0m", mode_str);
                            println!("    - Protocolo: USB FunctionFS Bulk 480 Mbps (< 1ms)");
                            println!("    - Painel Web: http://192.168.7.2:8080");
                            return Ok(None);
                        }
                        "network" | "udp" | "1" | "net" => {
                            let payload = format!(r#"{{"action":"start","transport":"network","mode":"{}"}}"#, mode_str);
                            let pi_body = format!(r#"{{"active_transport":"mode1_udp","transport":"network","action":"start","mode1":true,"mode2":false,"mode3":false,"mode":"{}"}}"#, mode_str);
                            notify_daemon_and_receiver(&payload, Some(("/api/transport/active", &pi_body)));
                            println!("\x1b[1;32m[+] Modo 1 (Rede UDP) ativado com sucesso! [modo: {}]\x1b[0m", mode_str);
                            println!("    - Protocolo: UDP H.264 / RFC 4571 (< 15ms)");
                            println!("    - Painel Web: http://192.168.7.2:8080");
                            return Ok(None);
                        }
                        "miracast" | "wfd" | "2" => {
                            let payload = r#"{"action":"stop"}"#;
                            let pi_body = r#"{"active_transport":"mode2_miracast","transport":"miracast","action":"stop","mode1":false,"mode2":true,"mode3":false}"#;
                            notify_daemon_and_receiver(payload, Some(("/api/transport/active", pi_body)));
                            println!("\x1b[1;32m[+] Modo 2 (Windows Miracast) ativado com sucesso!\x1b[0m");
                            println!("    - Windows Miracast: Porta RTSP 7236 (Win + K)");
                            println!("    - Painel Web: http://192.168.7.2:8080");
                            return Ok(None);
                        }
                        "standby" | "stop" | "pause" | "off" => {
                            notify_daemon_and_receiver(
                                r#"{"action":"stop"}"#,
                                Some(("/api/host/control", r#"{"action":"stop"}"#)),
                            );
                            println!("\x1b[1;33m[+] Modo Standby ativado: Transmissão pausada. TV exibindo Splash Screen.\x1b[0m");
                            return Ok(None);
                        }
                        _ => {
                            crate::service::show_status();
                            return Ok(None);
                        }
                    }
                }
                "standby" | "pause" => {
                    notify_daemon_and_receiver(
                        r#"{"action":"stop"}"#,
                        Some(("/api/host/control", r#"{"action":"stop"}"#)),
                    );
                    println!("\x1b[1;33m[+] Modo Standby ativado: Transmissão pausada. TV exibindo Splash Screen.\x1b[0m");
                    return Ok(None);
                }
                "usb-bulk" | "bulk" => {
                    let mode_str = extract_mode(&args[2..]).unwrap_or_else(|| "extend".to_string());
                    let payload = format!(r#"{{"action":"start","transport":"usb_bulk","mode":"{}"}}"#, mode_str);
                    let pi_body = format!(r#"{{"active_transport":"mode3_usb_bulk","transport":"usb_bulk","action":"start","mode1":false,"mode2":false,"mode3":true,"mode":"{}"}}"#, mode_str);
                    notify_daemon_and_receiver(
                        &payload,
                        Some(("/api/transport/active", &pi_body)),
                    );
                    println!("\x1b[1;32m[+] Modo 3 (USB Bulk Direto) ativado com sucesso! [modo: {}]\x1b[0m", mode_str);
                    println!("    - Protocolo: USB FunctionFS Bulk 480 Mbps (< 1ms)");
                    println!("    - Painel Web: http://192.168.7.2:8080");
                    return Ok(None);
                }
                "network" | "udp" => {
                    let mode_str = extract_mode(&args[2..]).unwrap_or_else(|| "extend".to_string());
                    let payload = format!(r#"{{"action":"start","transport":"network","mode":"{}"}}"#, mode_str);
                    let pi_body = format!(r#"{{"active_transport":"mode1_udp","transport":"network","action":"start","mode1":true,"mode2":false,"mode3":false,"mode":"{}"}}"#, mode_str);
                    notify_daemon_and_receiver(
                        &payload,
                        Some(("/api/transport/active", &pi_body)),
                    );
                    println!("\x1b[1;32m[+] Modo 1 (Rede UDP) ativado com sucesso! [modo: {}]\x1b[0m", mode_str);
                    println!("    - Protocolo: UDP H.264 / RFC 4571 (< 15ms)");
                    println!("    - Painel Web: http://192.168.7.2:8080");
                    return Ok(None);
                }
                "miracast" | "wfd" => {
                    notify_daemon_and_receiver(
                        r#"{"action":"stop"}"#,
                        Some(("/api/transport/active", r#"{"active_transport":"mode2_miracast","transport":"miracast","action":"stop","mode1":false,"mode2":true,"mode3":false}"#)),
                    );
                    println!("\x1b[1;32m[+] Modo 2 (Windows Miracast) ativado com sucesso!\x1b[0m");
                    println!("    - Windows Miracast: Porta RTSP 7236 (Win + K)");
                    println!("    - Painel Web: http://192.168.7.2:8080");
                    return Ok(None);
                }
                "mode" => {
                    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("status");
                    match sub {
                        "extend" | "ext" => {
                            let payload = r#"{"action":"start","mode":"extend"}"#;
                            notify_daemon_and_receiver(payload, Some(("/api/host/control", payload)));
                            println!("\x1b[1;32m[+] Topologia alterada para Estender Tela (HDMI-1 TV)\x1b[0m");
                            return Ok(None);
                        }
                        "clone" | "cln" | "mirror" => {
                            let payload = r#"{"action":"start","mode":"clone"}"#;
                            notify_daemon_and_receiver(payload, Some(("/api/host/control", payload)));
                            println!("\x1b[1;32m[+] Topologia alterada para Clonar Tela (eDP-1 Notebook)\x1b[0m");
                            return Ok(None);
                        }
                        "ask" => {
                            let payload = r#"{"mode":"ask"}"#;
                            notify_daemon_and_receiver(payload, Some(("/api/host/control", payload)));
                            println!("\x1b[1;36m[+] Modo interativo configurado para Google Cast (Perguntar na Transmissão)\x1b[0m");
                            return Ok(None);
                        }
                        "usb-bulk" | "bulk" | "3" => {
                            let mode_str = extract_mode(&args[3..]).unwrap_or_else(|| "extend".to_string());
                            let payload = format!(r#"{{"action":"start","transport":"usb_bulk","mode":"{}"}}"#, mode_str);
                            let pi_body = format!(r#"{{"active_transport":"mode3_usb_bulk","transport":"usb_bulk","action":"start","mode1":false,"mode2":false,"mode3":true,"mode":"{}"}}"#, mode_str);
                            notify_daemon_and_receiver(&payload, Some(("/api/transport/active", &pi_body)));
                            println!("\x1b[1;32m[+] Modo 3 (USB Bulk Direto) ativado com sucesso! [modo: {}]\x1b[0m", mode_str);
                        }
                        "network" | "udp" | "1" => {
                            let mode_str = extract_mode(&args[3..]).unwrap_or_else(|| "extend".to_string());
                            let payload = format!(r#"{{"action":"start","transport":"network","mode":"{}"}}"#, mode_str);
                            let pi_body = format!(r#"{{"active_transport":"mode1_udp","transport":"network","action":"start","mode1":true,"mode2":false,"mode3":false,"mode":"{}"}}"#, mode_str);
                            notify_daemon_and_receiver(&payload, Some(("/api/transport/active", &pi_body)));
                            println!("\x1b[1;32m[+] Modo 1 (Rede UDP) ativado com sucesso! [modo: {}]\x1b[0m", mode_str);
                        }
                        "miracast" | "wfd" | "2" => {
                            notify_daemon_and_receiver(
                                r#"{"action":"stop"}"#,
                                Some(("/api/transport/active", r#"{"active_transport":"mode2_miracast","transport":"miracast","action":"stop","mode1":false,"mode2":true,"mode3":false}"#)),
                            );
                            println!("\x1b[1;32m[+] Modo 2 (Windows Miracast) ativado com sucesso!\x1b[0m");
                        }
                        "standby" | "stop" | "off" => {
                            notify_daemon_and_receiver(
                                r#"{"action":"stop"}"#,
                                Some(("/api/host/control", r#"{"action":"stop"}"#)),
                            );
                            println!("\x1b[1;33m[+] Modo Standby ativado: TV repousando.\x1b[0m");
                        }
                        _ => {
                            crate::service::show_status();
                        }
                    }
                    return Ok(None);
                }
                "stop" | "--stop" => {
                    crate::service::stop_all().map_err(|e| e.to_string())?;
                    return Ok(None);
                }
                "status" | "--status" => {
                    crate::service::show_status();
                    return Ok(None);
                }
                "start" | "--start" => {
                    crate::service::start_service().map_err(|e| e.to_string())?;
                    return Ok(None);
                }
                "logs" | "--logs" => {
                    crate::service::follow_logs();
                    return Ok(None);
                }
                "install" | "--install" => {
                    let path = crate::service::install_service().map_err(|e| e.to_string())?;
                    println!("\x1b[1;32m[+] Serviço ext-monitor-sender instalado em {:?}!\x1b[0m", path);
                    return Ok(None);
                }
                "uninstall" | "--uninstall" => {
                    crate::service::uninstall_service().map_err(|e| e.to_string())?;
                    println!("\x1b[1;32m[+] Serviço ext-monitor-sender desinstalado com sucesso.\x1b[0m");
                    return Ok(None);
                }
                "service" => {
                    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("status");
                    match sub {
                        "install" => {
                            let path = crate::service::install_service().map_err(|e| e.to_string())?;
                            println!("\x1b[1;32m[+] Serviço ext-monitor-sender instalado em {:?}!\x1b[0m", path);
                        }
                        "uninstall" => {
                            crate::service::uninstall_service().map_err(|e| e.to_string())?;
                            println!("\x1b[1;32m[+] Serviço ext-monitor-sender desinstalado.\x1b[0m");
                        }
                        "start" => {
                            crate::service::start_service().map_err(|e| e.to_string())?;
                        }
                        "stop" => {
                            crate::service::stop_all().map_err(|e| e.to_string())?;
                        }
                        "status" | _ => {
                            crate::service::show_status();
                        }
                    }
                    return Ok(None);
                }
                _ => {}
            }
        }

        // Se invocado sem flags de primeiro plano e sem ser daemon interno, ativa o serviço em background
        let is_foreground = args.iter().any(|a| a == "--foreground" || a == "-f" || a == "run" || a == "--direct");
        let is_daemon = args.iter().any(|a| a == "--service-daemon");

        if !is_foreground && !is_daemon {
            // Anti-duplicação inteligente: Se já está rodando, encaminha o comando de start/modo/transporte para o daemon via UDP 5001!
            if let Some(pid) = crate::service::get_running_pid() {
                println!("\x1b[1;32m[i] ext-sender já está ativo em segundo plano (PID: {})\x1b[0m", pid);

                // Analisa argumentos para direcionar o comando correto para o daemon
                let mut action_json = serde_json::json!({
                    "action": "start"
                });

                for arg in args.iter().skip(1) {
                    let lower = arg.to_lowercase();
                    if lower == "extend" || lower == "clone" {
                        action_json["mode"] = serde_json::Value::String(lower);
                    } else if let Some(m) = lower.strip_prefix("--mode=") {
                        action_json["mode"] = serde_json::Value::String(m.to_string());
                    } else if lower == "usb-bulk" || lower == "bulk" || lower == "usb" || lower == "mode3" || lower == "--usb" {
                        action_json["transport"] = serde_json::Value::String("usb_bulk".to_string());
                    } else if lower == "network" || lower == "net" || lower == "udp" || lower == "mode1" || lower == "--network" {
                        action_json["transport"] = serde_json::Value::String("network".to_string());
                    } else if let Some(fps_val) = lower.strip_prefix("--fps=").and_then(|f| f.parse::<u32>().ok()) {
                        action_json["fps"] = serde_json::Value::Number(fps_val.into());
                    } else if let Ok(fps_val) = lower.parse::<u32>() {
                        if (10..=60).contains(&fps_val) {
                            action_json["fps"] = serde_json::Value::Number(fps_val.into());
                        }
                    }
                }

                if let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") {
                    let payload = action_json.to_string();
                    let _ = sock.send_to(payload.as_bytes(), "127.0.0.1:5001");
                    println!("\x1b[1;32m[+] Comando de início/retomada enviado com sucesso ao daemon: {}\x1b[0m", payload);
                    println!("\x1b[1;36m    Segunda tela ativa em http://192.168.7.2:8080\x1b[0m");
                }
                return Ok(None);
            }

            // Se não está rodando, sobe como serviço systemd --user liberando o terminal
            println!("\x1b[1;34m[*] Iniciando ext-sender como serviço de usuário em segundo plano (systemd --user)...\x1b[0m");
            crate::service::start_service().map_err(|e| e.to_string())?;
            return Ok(None);
        }

        // Defaults de Máxima Velocidade e Fluidez (60 FPS, VBR 6000k, Modo Extend, VA-API)
        let mut target_ip = "192.168.7.2".to_string();
        let mut target_port: u16 = 5000;
        let mut raw_bitrate: u32 = 0;
        let mut mode = "ask".to_string();
        let mut encoder = EncoderApi::detect();
        let mut fps: u32 = 60; // 60 FPS por padrão para máxima fluidez do mouse e tela

        // Checa se é chamada posicional legada (quando o primeiro argumento é IP numérico com 4 octetos)
        let is_legacy_ip = args.get(1).map(|s| s.contains('.') && s.split('.').count() == 4 && s.chars().all(|c| c.is_ascii_digit() || c == '.')).unwrap_or(false);

        if is_legacy_ip {
            target_ip = args.get(1).unwrap().clone();
            target_port = args.get(2).and_then(|p| p.parse::<u16>().ok()).unwrap_or(5000);
            raw_bitrate = args.get(3).and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);
            if let Some(m) = args.get(4) {
                if m == "extend" || m == "clone" || m == "ask" || m == "interactive" { mode = m.to_lowercase(); }
            }
            if let Some(enc) = args.get(5) {
                encoder = EncoderApi::from_str(enc);
            }
            if let Some(f) = args.get(6).and_then(|p| p.parse::<u32>().ok()) {
                fps = f;
            }
        } else {
            // Parser flexível e inteligente: suporta 'ext-sender', 'ext-sender extend', 'ext-sender 60', 'ext-sender --fps=60'
            for arg in args.iter().skip(1) {
                let lower = arg.to_lowercase();
                if lower == "extend" || lower == "clone" || lower == "ask" || lower == "interactive" {
                    mode = lower;
                } else if let Some(m) = lower.strip_prefix("--mode=") {
                    mode = m.to_string();
                } else if lower == "vaapi" || lower == "nvenc" || lower == "qsv" || lower == "cpu" || lower == "software" || lower == "auto" {
                    encoder = EncoderApi::from_str(&lower);
                } else if let Some(enc) = lower.strip_prefix("--encoder=") {
                    encoder = EncoderApi::from_str(enc);
                } else if let Some(ip_str) = lower.strip_prefix("--ip=") {
                    target_ip = ip_str.to_string();
                } else if let Some(p_str) = lower.strip_prefix("--port=") {
                    if let Ok(p) = p_str.parse::<u16>() { target_port = p; }
                } else if let Some(b_str) = lower.strip_prefix("--bitrate=").or_else(|| lower.strip_prefix("-b=")) {
                    if let Ok(b) = b_str.parse::<u32>() { raw_bitrate = b; }
                } else if let Some(f_str) = lower.strip_prefix("--fps=") {
                    if let Ok(f) = f_str.parse::<u32>() { fps = f; }
                } else if lower.contains('.') && lower.split('.').count() == 4 && lower.chars().all(|c| c.is_ascii_digit() || c == '.') {
                    target_ip = lower;
                } else if let Ok(num) = lower.parse::<u32>() {
                    if num <= 120 {
                        fps = num;
                    } else {
                        raw_bitrate = num;
                    }
                }
            }
        }

        let bitrate = if raw_bitrate == 0 || raw_bitrate == 8000 {
            Self::recommended_bitrate(fps)
        } else {
            raw_bitrate
        };

        let hud = args.iter().any(|a| {
            let s = a.to_lowercase();
            s == "hud" || s == "--hud" || s == "true" || s == "1"
        });

        let audio = !args.iter().any(|a| a == "--no-audio" || a == "--audio=off" || a == "--audio=false");
        let audio_port = args
            .iter()
            .find_map(|a| {
                if let Some(val) = a.strip_prefix("--audio-port=") {
                    val.parse::<u16>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(5004);

        let audio_rate = args
            .iter()
            .find_map(|a| {
                if let Some(val) = a.strip_prefix("--audio-rate=") {
                    match val.to_lowercase().as_str() {
                        "high" | "96k" | "96000" => Some(96000),
                        "max" | "192k" | "192000" => Some(192000),
                        "standard" | "48k" | "48000" => Some(48000),
                        "cd" | "44.1k" | "44100" => Some(44100),
                        _ => val.parse::<u32>().ok(),
                    }
                } else {
                    None
                }
            })
            .unwrap_or(48000);

        let color_profile = if args.iter().any(|a| {
            let s = a.to_lowercase();
            s == "256" || s == "--256" || s == "--colors=256" || s == "economy" || s == "--economy"
        }) {
            ColorProfile::Economy256
        } else if args.iter().any(|a| {
            let s = a.to_lowercase();
            s == "gray" || s == "--gray" || s == "bw" || s == "--bw" || s == "mono"
        }) {
            ColorProfile::Grayscale
        } else {
            ColorProfile::TrueColor
        };

        let drop_only = if args.iter().any(|a| a == "--no-drop-only" || a == "--continuous" || a == "--cfr" || a == "--drop-only=false") {
            false
        } else {
            args.iter().any(|a| a == "--drop-only" || a == "--drop-only=true" || a == "--economy" || a == "--battery-saver")
        };
        let skip_to_first = !args.iter().any(|a| a == "--no-skip-to-first" || a == "--skip-to-first=false");

        let key_int_max = args
            .iter()
            .find_map(|a| {
                if let Some(v) = a.strip_prefix("--key-int-max=") {
                    v.parse::<u32>().ok()
                } else if let Some(v) = a.strip_prefix("--idr=") {
                    v.parse::<u32>().ok()
                } else if let Some(v) = a.strip_prefix("--key-int=") {
                    v.parse::<u32>().ok()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| fps.max(15));

        let is_explicit_net = args.iter().any(|a| a == "--transport=network" || a == "--network" || a == "--udp" || a == "--net");
        let transport = if is_explicit_net {
            TransportKind::Network {
                ip: target_ip.clone(),
                port: target_port,
            }
        } else {
            // Padrão: USB Bulk Direto (com fallback automático para UDP caso dispositivo USB não esteja acessível)
            TransportKind::UsbBulk
        };

        // Padrão de fábrica: KMS Direct (Kernel DRM/KMS scanout de ultra-baixa latência)
        // GNOME Mutter/Wayland é ativado apenas se explicitamente solicitado via CLI (--capture=mutter) ou painel web
        let capture = if args.iter().any(|a| a == "--capture=mutter" || a == "--mutter" || a == "--gnome") {
            CaptureEngine::Mutter
        } else {
            CaptureEngine::Kms
        };

        let scale = if args.iter().any(|a| a == "--no-scale" || a == "--scale=off" || a == "--scale=none" || a == "--scale=false") {
            ScaleMode::Off
        } else if args.iter().any(|a| a == "--scale=1600x900" || a == "--scale=900p" || a == "--scale=on" || a == "--scale=upscale") {
            ScaleMode::Scale1600x900
        } else if let Some(val) = args.iter().find_map(|a| a.strip_prefix("--scale=")) {
            ScaleMode::from_str(val)
        } else {
            ScaleMode::Native720p
        };

        let cas = if args.iter().any(|a| a == "--no-cas" || a == "--cas=off" || a == "--cas=false" || a == "--no-sharpen") {
            false
        } else if args.iter().any(|a| a == "--cas" || a == "--cas=on" || a == "--cas=true" || a == "--sharpen" || a == "--sharp") {
            true
        } else {
            false
        };

        let contrast = args
            .iter()
            .find_map(|a| a.strip_prefix("--contrast="))
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(if cas { 1.16 } else { 1.0 });

        let saturation = args
            .iter()
            .find_map(|a| a.strip_prefix("--saturation="))
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(if cas { 1.08 } else { 1.0 });

        let debug = args.iter().any(|a| a == "--debug" || a == "-d");

        let auto_connect = if args.iter().any(|a| a == "--no-auto-connect" || a == "--standby" || a == "--no-auto-start") {
            false
        } else if args.iter().any(|a| a == "--auto-connect" || a == "--auto-start" || a == "--connect") {
            true
        } else {
            std::env::var("EXT_AUTO_CONNECT").map(|v| v == "1" || v == "true").unwrap_or(false)
        };

        let enable_damage_pacer = args.iter().any(|a| a == "--enable-damage-pacer" || a == "--pacer");

        Ok(Some(Self {
            target_ip,
            target_port,
            bitrate,
            mode,
            encoder,
            fps,
            hud,
            color_profile,
            drop_only,
            skip_to_first,
            key_int_max,
            transport,
            capture,
            audio,
            audio_port,
            audio_rate,
            scale,
            cas,
            contrast,
            saturation,
            debug,
            auto_connect,
            enable_damage_pacer,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_capture_is_kms() {
        let args = vec!["ext-sender".to_string(), "--direct".to_string()];
        let cfg = SenderConfig::parse(&args).unwrap().unwrap();
        assert_eq!(cfg.capture, CaptureEngine::Kms);
        assert_eq!(cfg.transport, TransportKind::UsbBulk);
    }

    #[test]
    fn test_mutter_capture_opt_in() {
        let args = vec!["ext-sender".to_string(), "--direct".to_string(), "--capture=mutter".to_string()];
        let cfg = SenderConfig::parse(&args).unwrap().unwrap();
        assert_eq!(cfg.capture, CaptureEngine::Mutter);
    }

    #[test]
    fn test_scale_and_cas_options() {
        // Default: Native720p, CAS off, contrast 1.0, saturation 1.0
        let args = vec!["ext-sender".to_string(), "--direct".to_string()];
        let cfg = SenderConfig::parse(&args).unwrap().unwrap();
        assert_eq!(cfg.scale, ScaleMode::Native720p);
        assert!(!cfg.cas);
        assert_eq!(cfg.contrast, 1.0);
        assert_eq!(cfg.saturation, 1.0);

        // Explicit --no-scale and --cas with custom contrast
        let args2 = vec![
            "ext-sender".to_string(),
            "--direct".to_string(),
            "--no-scale".to_string(),
            "--cas".to_string(),
            "--contrast=1.20".to_string(),
        ];
        let cfg2 = SenderConfig::parse(&args2).unwrap().unwrap();
        assert_eq!(cfg2.scale, ScaleMode::Off);
        assert!(cfg2.cas);
        assert_eq!(cfg2.contrast, 1.20);
        assert_eq!(cfg2.saturation, 1.08);

        // Explicit 1600x900 scaling
        let args3 = vec!["ext-sender".to_string(), "--direct".to_string(), "--scale=1600x900".to_string()];
        let cfg3 = SenderConfig::parse(&args3).unwrap().unwrap();
        assert_eq!(cfg3.scale, ScaleMode::Scale1600x900);
    }

    #[test]
    fn test_auto_connect_default_and_flags() {
        // By default, auto_connect is FALSE (Standby on boot)
        let args_def = vec!["ext-sender".to_string(), "--direct".to_string()];
        let cfg_def = SenderConfig::parse(&args_def).unwrap().unwrap();
        assert!(!cfg_def.auto_connect);

        // Opt-in via --auto-connect
        let args_auto = vec!["ext-sender".to_string(), "--direct".to_string(), "--auto-connect".to_string()];
        let cfg_auto = SenderConfig::parse(&args_auto).unwrap().unwrap();
        assert!(cfg_auto.auto_connect);

        // Explicit --standby
        let args_standby = vec!["ext-sender".to_string(), "--direct".to_string(), "--standby".to_string()];
        let cfg_standby = SenderConfig::parse(&args_standby).unwrap().unwrap();
        assert!(!cfg_standby.auto_connect);
    }

    #[test]
    fn test_switch_subcommand_usb_bulk_and_network() {
        let args_usb = vec!["ext-sender".to_string(), "switch".to_string(), "usb-bulk".to_string(), "--mode=clone".to_string()];
        let res_usb = SenderConfig::parse(&args_usb);
        assert!(res_usb.is_ok());
        assert!(res_usb.unwrap().is_none());

        let args_net = vec!["ext-sender".to_string(), "switch".to_string(), "network".to_string(), "--mode=extend".to_string()];
        let res_net = SenderConfig::parse(&args_net);
        assert!(res_net.is_ok());
        assert!(res_net.unwrap().is_none());
    }

    #[test]
    fn test_switch_subcommand_miracast_and_standby() {
        let args_mira = vec!["ext-sender".to_string(), "switch".to_string(), "miracast".to_string()];
        let res_mira = SenderConfig::parse(&args_mira);
        assert!(res_mira.is_ok());
        assert!(res_mira.unwrap().is_none());

        let args_std = vec!["ext-sender".to_string(), "switch".to_string(), "standby".to_string()];
        let res_std = SenderConfig::parse(&args_std);
        assert!(res_std.is_ok());
        assert!(res_std.unwrap().is_none());
    }

    #[test]
    fn test_mode_subcommand_extend_clone_and_ask() {
        let args_ext = vec!["ext-sender".to_string(), "mode".to_string(), "extend".to_string()];
        let res_ext = SenderConfig::parse(&args_ext);
        assert!(res_ext.is_ok());
        assert!(res_ext.unwrap().is_none());

        let args_cln = vec!["ext-sender".to_string(), "mode".to_string(), "clone".to_string()];
        let res_cln = SenderConfig::parse(&args_cln);
        assert!(res_cln.is_ok());
        assert!(res_cln.unwrap().is_none());

        let args_ask = vec!["ext-sender".to_string(), "mode".to_string(), "ask".to_string()];
        let res_ask = SenderConfig::parse(&args_ask);
        assert!(res_ask.is_ok());
        assert!(res_ask.unwrap().is_none());
    }
}


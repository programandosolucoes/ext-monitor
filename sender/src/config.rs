//! Configuration and CLI argument parser for ext-sender
//!
//! Follows Clean Code and Single Responsibility Principle (SRP).
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use crate::i18n::{self, Language};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorProfile {
    TrueColor,  // Standard 24-bit color (VBR 75% avg, dynamic QP)
    Economy256, // Emulated 256-color coarse quantization (min-qp 30, max-qp 44, VBR 50%)
    Grayscale,  // Monochrome terminal mode (saturation=0.0)
}

impl ColorProfile {
    pub fn name(&self) -> &'static str {
        match self {
            ColorProfile::TrueColor => "24-bit TrueColor (Full)",
            ColorProfile::Economy256 => "256-Color (Coarse QP 30-44)",
            ColorProfile::Grayscale => "Monochrome (Grayscale)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureEngine {
    Mutter, // GNOME Mutter ScreenCast via D-Bus & PipeWire
    Kms,    // Kernel DRM/KMS Direct Hardware Scanout via DMA-BUF
}

impl CaptureEngine {
    pub fn name(&self) -> &'static str {
        match self {
            CaptureEngine::Mutter => "GNOME Mutter ScreenCast (PipeWire D-Bus)",
            CaptureEngine::Kms => "Kernel Direct DRM/KMS (Hardware Scanout DMA-BUF)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamEngine {
    NativeRust, // 100% Pure Rust Native In-Process GPU Pipeline
    GStreamer,  // GStreamer 1.0 (Hardware)
}

impl StreamEngine {
    pub fn name(&self) -> &'static str {
        match self {
            StreamEngine::NativeRust => "100% Native Rust (In-Process GPU Pipeline)",
            StreamEngine::GStreamer => "GStreamer 1.0 (Hardware)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderApi {
    Vaapi,    // AMD & Intel hardware encoding via VA-API
    Nvenc,    // NVIDIA hardware encoding via NVENC
    Qsv,      // Intel QuickSync hardware encoding
    Software, // CPU / x264 zerolatency
}

impl EncoderApi {
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
pub enum TransportKind {
    Network { ip: String, port: u16 },
    UsbBulk,
}

#[derive(Debug, Clone)]
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
    pub engine: StreamEngine,
    pub capture: CaptureEngine,
    pub audio: bool,
    pub audio_port: u16,
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

        // Subcomandos de controle de serviço e processo
        if let Some(cmd) = args.get(1) {
            match cmd.as_str() {
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
            // Anti-duplicação: Se já está rodando, apenas relata o status e não duplica processo
            if let Some(pid) = crate::service::get_running_pid() {
                println!("\x1b[1;32m[i] ext-sender já está ativo em segundo plano (PID: {})\x1b[0m", pid);
                println!("\x1b[1;36m    Controle Web: http://192.168.7.2:8080\x1b[0m");
                println!("\x1b[1;36m    Para ver logs:  ext-sender logs\x1b[0m");
                println!("\x1b[1;36m    Para parar:     ext-sender stop\x1b[0m");
                println!("\x1b[1;36m    Para status:    ext-sender status\x1b[0m");
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
        let mut mode = "extend".to_string();
        let mut encoder = EncoderApi::detect();
        let mut fps: u32 = 60; // 60 FPS por padrão para máxima fluidez do mouse e tela

        // Checa se é chamada posicional legada (quando o primeiro argumento é IP numérico com 4 octetos)
        let is_legacy_ip = args.get(1).map(|s| s.contains('.') && s.split('.').count() == 4 && s.chars().all(|c| c.is_ascii_digit() || c == '.')).unwrap_or(false);

        if is_legacy_ip {
            target_ip = args.get(1).unwrap().clone();
            target_port = args.get(2).and_then(|p| p.parse::<u16>().ok()).unwrap_or(5000);
            raw_bitrate = args.get(3).and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);
            if let Some(m) = args.get(4) {
                if m == "extend" || m == "clone" { mode = m.to_lowercase(); }
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
                if lower == "extend" || lower == "clone" {
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

        let engine = if args.iter().any(|a| a == "--engine=native" || a == "--native" || a == "--rust") {
            StreamEngine::NativeRust
        } else {
            StreamEngine::GStreamer
        };

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
            engine,
            capture,
            audio,
            audio_port,
        }))
    }
}

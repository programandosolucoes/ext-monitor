//! Configuration and CLI argument parser for ext-sender
//!
//! Follows Clean Code and Single Responsibility Principle (SRP).
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

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

        let target_ip = args.get(1).map(|s| s.as_str()).unwrap_or("192.168.7.2").to_string();
        let target_port = args.get(2).and_then(|p| p.parse::<u16>().ok()).unwrap_or(5000);
        let raw_bitrate = args.get(3).and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);
        let mode = args.get(4).map(|s| s.to_lowercase()).unwrap_or_else(|| "extend".to_string());
        let encoder_arg = args.get(5).map(|s| s.as_str()).unwrap_or("auto");
        let encoder = EncoderApi::from_str(encoder_arg);
        let fps = args.get(6).and_then(|p| p.parse::<u32>().ok()).unwrap_or(30);

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
            // Default: USB Bulk Direct (with auto-fallback to Network if device is absent)
            TransportKind::UsbBulk
        };

        let capture = if args.iter().any(|a| a == "--capture=kms" || a == "--kms" || a == "--drm") {
            CaptureEngine::Kms
        } else {
            CaptureEngine::Mutter
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

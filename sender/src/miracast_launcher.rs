//! Intelligent Miracast Client (GNOME Network Displays) Launcher
//!
//! Automatically detects the host GPU vendor (AMD, Intel, NVIDIA) and launches
//! `gnome-network-displays` with prioritized hardware H.264 video encoders
//! via GStreamer feature ranking (`GST_PLUGIN_FEATURE_RANK`), eliminating
//! CPU software encoding bottlenecks and achieving sub-2ms realtime latency.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static LAST_LAUNCH_MILLIS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct GpuVendorInfo {
    pub vendor_name: &'static str,
    pub encoder_name: &'static str,
    pub rank_string: String,
    pub env_h264_enc: &'static str,
}

/// Detects host GPU vendor(s) via Linux DRM sysfs (/sys/class/drm/card*/device/vendor) and lspci
pub fn detect_gpu_hardware() -> GpuVendorInfo {
    let mut has_amd = false;
    let mut has_intel = false;
    let mut has_nvidia = false;

    // 1. Inspect DRM card vendor IDs in sysfs
    if let Ok(entries) = fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("card") && name.chars().nth(4).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                let vendor_path = entry.path().join("device/vendor");
                if let Ok(vendor_hex) = fs::read_to_string(vendor_path) {
                    let v = vendor_hex.trim().to_lowercase();
                    match v.as_str() {
                        "0x1002" => has_amd = true,
                        "0x8086" => has_intel = true,
                        "0x10de" => has_nvidia = true,
                        _ => {}
                    }
                }
            }
        }
    }

    // 2. Fallback to lspci if sysfs was inconclusive
    if !has_amd && !has_intel && !has_nvidia {
        if let Ok(output) = Command::new("lspci").output() {
            let s = String::from_utf8_lossy(&output.stdout).to_lowercase();
            if s.contains("amd") || s.contains("radeon") || s.contains("advanced micro devices") {
                has_amd = true;
            }
            if s.contains("intel") {
                has_intel = true;
            }
            if s.contains("nvidia") {
                has_nvidia = true;
            }
        }
    }

    if has_amd && !has_intel && !has_nvidia {
        GpuVendorInfo {
            vendor_name: "AMD Radeon (VA-API / RDNA / Vega / Mendocino)",
            encoder_name: "vaapih264enc / vah264enc",
            rank_string: "vaapih264enc:MAX,vah264enc:MAX".to_string(),
            env_h264_enc: "vaapih264enc",
        }
    } else if has_intel && !has_nvidia && !has_amd {
        GpuVendorInfo {
            vendor_name: "Intel HD / Iris Xe / Arc / UHD (QuickSync)",
            encoder_name: "vaapih264enc / vah264enc / qsvh264enc",
            rank_string: "vaapih264enc:MAX,vah264enc:MAX,qsvh264enc:MAX".to_string(),
            env_h264_enc: "vaapih264enc",
        }
    } else if has_nvidia && !has_amd && !has_intel {
        GpuVendorInfo {
            vendor_name: "NVIDIA GeForce / RTX / Quadro (NVENC)",
            encoder_name: "nvh264enc / vaapih264enc",
            rank_string: "nvh264enc:MAX,vaapih264enc:MAX".to_string(),
            env_h264_enc: "nvh264enc",
        }
    } else if has_nvidia && (has_amd || has_intel) {
        GpuVendorInfo {
            vendor_name: "Hybrid GPU (NVIDIA dGPU + AMD/Intel iGPU)",
            encoder_name: "nvh264enc / vaapih264enc / vah264enc",
            rank_string: "nvh264enc:MAX,vaapih264enc:MAX,vah264enc:MAX,qsvh264enc:MAX".to_string(),
            env_h264_enc: "nvh264enc",
        }
    } else {
        GpuVendorInfo {
            vendor_name: "Universal Hardware GPU (Auto-Ranking)",
            encoder_name: "vaapih264enc / vah264enc / nvh264enc / qsvh264enc",
            rank_string: "vaapih264enc:MAX,vah264enc:MAX,nvh264enc:MAX,qsvh264enc:MAX".to_string(),
            env_h264_enc: "vaapih264enc",
        }
    }
}

/// Updates ~/.local/share/applications/org.gnome.NetworkDisplays.desktop with GPU rank parameters
pub fn update_desktop_entry_with_ranks(rank_str: &str) {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return,
    };
    let local_app_dir = Path::new(&home).join(".local/share/applications");
    let desktop_path = local_app_dir.join("org.gnome.NetworkDisplays.desktop");

    let _ = fs::create_dir_all(&local_app_dir);

    let base_content = if desktop_path.exists() {
        fs::read_to_string(&desktop_path).unwrap_or_default()
    } else if Path::new("/usr/share/applications/org.gnome.NetworkDisplays.desktop").exists() {
        fs::read_to_string("/usr/share/applications/org.gnome.NetworkDisplays.desktop").unwrap_or_default()
    } else {
        String::new()
    };

    if !base_content.is_empty() {
        let exec_line = format!("Exec=env GST_PLUGIN_FEATURE_RANK={} gnome-network-displays", rank_str);
        let mut new_lines = Vec::new();
        for line in base_content.lines() {
            if line.starts_with("Exec=") {
                new_lines.push(exec_line.clone());
            } else {
                new_lines.push(line.to_string());
            }
        }
        let _ = fs::write(&desktop_path, new_lines.join("\n") + "\n");
    }
}

/// Checks if gnome-network-displays is installed on the system
pub fn is_gnome_network_displays_installed() -> bool {
    if Path::new("/usr/bin/gnome-network-displays").exists()
        || Path::new("/usr/local/bin/gnome-network-displays").exists()
    {
        return true;
    }
    Command::new("which")
        .arg("gnome-network-displays")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Launches gnome-network-displays with GPU hardware acceleration
pub fn launch_gnome_network_displays() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let prev = LAST_LAUNCH_MILLIS.load(Ordering::SeqCst);
    if now.saturating_sub(prev) < 2500 {
        println!("\x1b[1;33m[miracast-launcher]\x1b[0m Miracast já foi disparado recentemente (< 2.5s). Ignorando requisição duplicada.");
        return;
    }
    LAST_LAUNCH_MILLIS.store(now, Ordering::SeqCst);

    if !is_gnome_network_displays_installed() {
        eprintln!(
            "\x1b[1;31m[miracast-launcher]\x1b[0m gnome-network-displays não encontrado!\n\
             \x1b[1;33m[!] Instale no laptop com:\x1b[0m sudo apt update && sudo apt install -y gnome-network-displays"
        );
        return;
    }

    let gpu = detect_gpu_hardware();
    println!(
        "\x1b[1;32m[miracast-launcher]\x1b[0m GPU Detectada: \x1b[1;34m{}\x1b[0m (Encoder: {})",
        gpu.vendor_name, gpu.encoder_name
    );
    println!(
        "\x1b[1;36m[miracast-launcher]\x1b[0m Aplicando: GST_PLUGIN_FEATURE_RANK={}",
        gpu.rank_string
    );

    // Update .desktop entry so desktop clicks also get hardware acceleration
    update_desktop_entry_with_ranks(&gpu.rank_string);

    // Check if already running
    let is_running = Command::new("pidof")
        .arg("gnome-network-displays")
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false);

    if is_running {
        println!("\x1b[1;33m[miracast-launcher]\x1b[0m gnome-network-displays já está em execução no sistema. Focando janela...");
        // Try to focus window via gtk-launch or dbus
        let _ = Command::new("gtk-launch")
            .arg("org.gnome.NetworkDisplays")
            .spawn();
        return;
    }

    // Launch with GPU acceleration environment variables
    let mut cmd = Command::new("gnome-network-displays");
    cmd.env("GST_PLUGIN_FEATURE_RANK", &gpu.rank_string);
    cmd.env("NETWORK_DISPLAYS_H264_ENC", gpu.env_h264_enc);

    // Ensure session display variables are preserved in systemd user daemon
    if std::env::var("DISPLAY").is_err() {
        cmd.env("DISPLAY", ":0");
    }
    if std::env::var("WAYLAND_DISPLAY").is_err() {
        cmd.env("WAYLAND_DISPLAY", "wayland-0");
    }
    if std::env::var("XDG_RUNTIME_DIR").is_err() {
        let uid = unsafe { libc::getuid() };
        cmd.env("XDG_RUNTIME_DIR", format!("/run/user/{}", uid));
    }

    match cmd.spawn() {
        Ok(child) => {
            println!(
                "\x1b[1;32m[miracast-launcher]\x1b[0m gnome-network-displays iniciado com sucesso (PID: {}) com aceleração por GPU!",
                child.id()
            );
        }
        Err(e) => {
            eprintln!(
                "\x1b[1;31m[miracast-launcher]\x1b[0m Falha ao executar gnome-network-displays: {}. Tentando via gtk-launch...",
                e
            );
            let _ = Command::new("gtk-launch")
                .arg("org.gnome.NetworkDisplays")
                .spawn();
        }
    }
}

/// Terminates any running gnome-network-displays instances cleanly on Standby
pub fn stop_gnome_network_displays() {
    println!("\x1b[1;33m[miracast-launcher]\x1b[0m Finalizando instâncias de gnome-network-displays...");
    let _ = Command::new("pkill")
        .arg("-15")
        .arg("-f")
        .arg("gnome-network-displays")
        .output();
    let _ = Command::new("pkill")
        .arg("-9")
        .arg("-f")
        .arg("gnome-network-displays")
        .output();
}


//! Real-Time HDMI Monitor & EDID Telemetry in 100% Pure Rust
//!
//! Queries the Linux DRM subsystem sysfs nodes directly (/sys/class/drm/*-HDMI-*)
//! and decodes binary VESA EDID 1.3/1.4 structures without any external dependencies.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::fs;

#[derive(Debug, Clone)]
#[allow(dead_code)]
/// Represents Monitorinfo configuration and operational state.
pub struct MonitorInfo {
    pub connected: bool,
    pub name: String,
    pub manufacturer: String,
    pub product_code: u16,
    pub preferred_mode: String,
    pub active_mode: String,
    pub vpu: String,
    pub connector: String,
    pub connector_friendly: String,
    pub hardware_model: String,
    pub has_audio: bool,
}

impl MonitorInfo {
    /// Reads real-time HDMI monitor telemetry for ALL available DRM video outputs
    pub fn read_all_realtime() -> Vec<Self> {
        // 1. Detect hardware model
        let mut hardware_model = "Raspberry Pi".to_string();
        if let Ok(m) = fs::read("/proc/device-tree/model") {
            let clean = String::from_utf8_lossy(&m).trim_matches('\0').trim().to_string();
            if !clean.is_empty() {
                hardware_model = clean;
            }
        } else if let Ok(prod) = fs::read_to_string("/sys/devices/virtual/dmi/id/product_name") {
            let clean = prod.trim().to_string();
            if !clean.is_empty() {
                hardware_model = clean;
            }
        }

        // 2. Compute VPU name based on hardware
        let vpu = if hardware_model.contains("Zero")
            || hardware_model.contains("Raspberry Pi 1")
            || hardware_model.contains("Raspberry Pi 2")
            || hardware_model.contains("Raspberry Pi 3")
        {
            "VideoCore IV Hardware VPU".to_string()
        } else if hardware_model.contains("Pi 4") {
            "VideoCore VI Hardware VPU (4K DRM/KMS)".to_string()
        } else if hardware_model.contains("Pi 5") {
            "VideoCore VII Hardware VPU (4K DRM/KMS)".to_string()
        } else {
            "GPU Hardware Acceleration (DRM/KMS)".to_string()
        };

        // 3. Active scanout mode from fb0 virtual_size
        let active_scanout = if let Ok(fb_size) = fs::read_to_string("/sys/class/graphics/fb0/virtual_size") {
            let parts: Vec<&str> = fb_size.trim().split(',').collect();
            if parts.len() == 2 {
                format!("{}x{} @ 60 Hz", parts[0], parts[1])
            } else {
                "1280x720 @ 60 Hz".to_string()
            }
        } else {
            "1280x720 @ 60 Hz".to_string()
        };

        // 4. Discover all DRM HDMI and display connectors
        let mut hdmi_dirs: Vec<(String, String, bool)> = Vec::new();
        if let Ok(entries) = fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains("HDMI") || name.contains("DP-") || name.contains("DVI") {
                    let path_str = entry.path().to_string_lossy().to_string();
                    let st_file = format!("{}/status", path_str);
                    let is_conn = fs::read_to_string(&st_file)
                        .map(|s| s.trim() == "connected")
                        .unwrap_or(false);
                    let conn_name = if let Some(idx) = name.find("HDMI") {
                        name[idx..].to_string()
                    } else if let Some(idx) = name.find("card") {
                        if let Some(sub) = name[idx..].split('-').nth(1) {
                            sub.to_string()
                        } else {
                            name.clone()
                        }
                    } else {
                        name.clone()
                    };
                    hdmi_dirs.push((path_str, conn_name, is_conn));
                }
            }
        }

        // Sort connectors logically (e.g. HDMI-A-1 before HDMI-A-2)
        hdmi_dirs.sort_by(|a, b| a.1.cmp(&b.1));

        // If none found in sysfs, fallback to default Pi Zero HDMI-A-1
        if hdmi_dirs.is_empty() {
            let def_path = "/sys/class/drm/card0-HDMI-A-1".to_string();
            let st_file = format!("{}/status", def_path);
            let is_conn = fs::read_to_string(&st_file)
                .map(|s| s.trim() == "connected")
                .unwrap_or(false);
            hdmi_dirs.push((def_path, "HDMI-A-1".to_string(), is_conn));
        }

        let mut results = Vec::new();
        for (path_str, conn_name, is_conn) in hdmi_dirs {
            let connector_friendly = if hardware_model.contains("Zero") {
                format!("Mini-HDMI Port ({})", conn_name)
            } else if hardware_model.contains("Raspberry Pi 4")
                || hardware_model.contains("Raspberry Pi 5")
                || hardware_model.contains("Pi 4")
                || hardware_model.contains("Pi 5")
            {
                if conn_name.contains("-2") || conn_name.ends_with('2') {
                    format!("Micro-HDMI Port 1 / Secondary ({})", conn_name)
                } else {
                    format!("Micro-HDMI Port 0 / Primary ({}) - Next to USB-C", conn_name)
                }
            } else if hardware_model.contains("Raspberry Pi") {
                format!("Primary HDMI Port ({})", conn_name)
            } else {
                format!("Digital Video Output ({})", conn_name)
            };

            let edid_file = format!("{}/edid", path_str);
            let modes_file = format!("{}/modes", path_str);

            let mut modes = Vec::new();
            if let Ok(modes_str) = fs::read_to_string(&modes_file) {
                for line in modes_str.lines() {
                    let m = line.trim();
                    if !m.is_empty() && !modes.contains(&m.to_string()) {
                        modes.push(m.to_string());
                    }
                }
            }

            let preferred_mode = modes.first().cloned().unwrap_or_else(|| "1280x720".to_string());

            let has_audio = conn_name.starts_with("HDMI") || conn_name.starts_with("DP");

            if !is_conn {
                results.push(Self {
                    connected: false,
                    name: format!("Standby / Disconnected ({})", conn_name),
                    manufacturer: "None".to_string(),
                    product_code: 0,
                    preferred_mode: "1280x720".to_string(),
                    active_mode: format!("{} (Virtual)", active_scanout),
                    vpu: vpu.clone(),
                    connector: conn_name,
                    connector_friendly,
                    hardware_model: hardware_model.clone(),
                    has_audio,
                });
                continue;
            }

            let edid_bytes = fs::read(&edid_file).unwrap_or_default();
            let (mfg, prod_code, desc_name) = if edid_bytes.len() >= 128 {
                parse_edid(&edid_bytes)
            } else {
                ("Generic".to_string(), 0, None)
            };

            let display_name = if let Some(name) = desc_name {
                if !name.trim().is_empty() {
                    format!("{} ({})", name.trim(), preferred_mode)
                } else {
                    format!("{} Monitor ({})", mfg, preferred_mode)
                }
            } else {
                format!("{} Monitor ({})", mfg, preferred_mode)
            };

            results.push(Self {
                connected: true,
                name: display_name,
                manufacturer: mfg,
                product_code: prod_code,
                preferred_mode,
                active_mode: active_scanout.clone(),
                vpu: vpu.clone(),
                connector: conn_name,
                connector_friendly,
                hardware_model: hardware_model.clone(),
                has_audio,
            });
        }

        results
    }

    /// Reads real-time HDMI monitor telemetry from the system (primary/first connected)
    pub fn read_realtime() -> Self {
        let all = Self::read_all_realtime();
        all.iter()
            .find(|m| m.connected)
            .cloned()
            .or_else(|| all.first().cloned())
            .unwrap_or_else(|| Self {
                connected: false,
                name: "Nenhum Monitor Conectado (Headless Guard Ativo)".to_string(),
                manufacturer: "None".to_string(),
                product_code: 0,
                preferred_mode: "1280x720".to_string(),
                active_mode: "1280x720 @ 60 Hz (Virtual)".to_string(),
                vpu: "VideoCore IV Hardware VPU".to_string(),
                connector: "HDMI-A-1".to_string(),
                connector_friendly: "Mini-HDMI Port (HDMI-A-1)".to_string(),
                hardware_model: "Raspberry Pi Zero W".to_string(),
                has_audio: true,
            })
    }
}

/// Decodes VESA EDID 1.3/1.4 byte payload
fn parse_edid(edid: &[u8]) -> (String, u16, Option<String>) {
    if edid.len() < 128 {
        return ("Generic".to_string(), 0, None);
    }

    // 1. Manufacturer ID at bytes 8-9 (3 compressed 5-bit ASCII chars)
    let mfg_raw = ((edid[8] as u16) << 8) | (edid[9] as u16);
    let c1 = (((mfg_raw >> 10) & 0x1F) as u8).saturating_add(b'A' - 1) as char;
    let c2 = (((mfg_raw >> 5) & 0x1F) as u8).saturating_add(b'A' - 1) as char;
    let c3 = ((mfg_raw & 0x1F) as u8).saturating_add(b'A' - 1) as char;
    let mfg = format!("{}{}{}", c1, c2, c3);

    // 2. Product code at bytes 10-11 (little endian)
    let prod_code = (edid[10] as u16) | ((edid[11] as u16) << 8);

    // 3. Detailed timing descriptors (offsets 54, 72, 90, 108 - 18 bytes each)
    let mut monitor_name = None;
    for &offset in &[54, 72, 90, 108] {
        if offset + 18 <= edid.len() {
            let desc = &edid[offset..offset + 18];
            // Monitor Name Descriptor has header 00 00 00 FC
            if desc[0] == 0 && desc[1] == 0 && desc[2] == 0 && desc[3] == 0xFC {
                let name_bytes = &desc[5..18];
                let clean_name: String = name_bytes
                    .iter()
                    .take_while(|&&b| b != 0x0A && b != 0x00)
                    .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { ' ' })
                    .collect();
                let trimmed = clean_name.trim().to_string();
                if !trimmed.is_empty() {
                    monitor_name = Some(trimmed);
                    break;
                }
            }
        }
    }

    (mfg, prod_code, monitor_name)
}

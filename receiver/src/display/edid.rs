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
}

impl MonitorInfo {
    /// Reads real-time HDMI monitor telemetry from the system
    pub fn read_realtime() -> Self {
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

        // 2. Discover all DRM HDMI connectors dynamically
        let mut chosen_path = "/sys/class/drm/card0-HDMI-A-1".to_string();
        let mut chosen_connector = "HDMI-A-1".to_string();
        let mut connected = false;

        if let Ok(entries) = fs::read_dir("/sys/class/drm") {
            let mut hdmi_dirs: Vec<(String, String, bool)> = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains("HDMI") {
                    let path_str = entry.path().to_string_lossy().to_string();
                    let st_file = format!("{}/status", path_str);
                    let is_conn = fs::read_to_string(&st_file)
                        .map(|s| s.trim() == "connected")
                        .unwrap_or(false);
                    let conn_name = if let Some(idx) = name.find("HDMI") {
                        name[idx..].to_string()
                    } else {
                        name.clone()
                    };
                    hdmi_dirs.push((path_str, conn_name, is_conn));
                }
            }
            // Prefer connected connector, fallback to first available
            if let Some((p, c, _)) = hdmi_dirs.iter().find(|(_, _, conn)| *conn) {
                chosen_path = p.clone();
                chosen_connector = c.clone();
                connected = true;
            } else if let Some((p, c, _)) = hdmi_dirs.first() {
                chosen_path = p.clone();
                chosen_connector = c.clone();
                connected = false;
            }
        } else {
            // Fallback for standard Pi Zero path
            let st_file = format!("{}/status", chosen_path);
            if let Ok(s) = fs::read_to_string(&st_file) {
                connected = s.trim() == "connected";
            }
        }

        // 3. Compute friendly physical socket name
        let connector_friendly = if hardware_model.contains("Zero") {
            format!("Mini-HDMI Port ({})", chosen_connector)
        } else if hardware_model.contains("Raspberry Pi 4")
            || hardware_model.contains("Raspberry Pi 5")
            || hardware_model.contains("Pi 4")
            || hardware_model.contains("Pi 5")
        {
            if chosen_connector.contains("-2") {
                format!("Micro-HDMI Port 1 / Secondary ({})", chosen_connector)
            } else {
                format!("Micro-HDMI Port 0 / Primary ({}) - Next to USB-C", chosen_connector)
            }
        } else if hardware_model.contains("Raspberry Pi") {
            format!("Primary HDMI Port ({})", chosen_connector)
        } else {
            format!("Digital Video Output ({})", chosen_connector)
        };

        // 4. Compute VPU name based on hardware
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

        let edid_file = format!("{}/edid", chosen_path);
        let modes_file = format!("{}/modes", chosen_path);

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

        // 5. Active scanout mode from fb0 virtual_size
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

        if !connected {
            return Self {
                connected: false,
                name: "Nenhum Monitor Conectado (Headless Guard Ativo)".to_string(),
                manufacturer: "None".to_string(),
                product_code: 0,
                preferred_mode: "1280x720".to_string(),
                active_mode: format!("{} (Virtual)", active_scanout),
                vpu,
                connector: chosen_connector,
                connector_friendly,
                hardware_model,
            };
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

        Self {
            connected: true,
            name: display_name,
            manufacturer: mfg,
            product_code: prod_code,
            preferred_mode,
            active_mode: active_scanout,
            vpu,
            connector: chosen_connector,
            connector_friendly,
            hardware_model,
        }
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

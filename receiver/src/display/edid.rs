//! Real-Time HDMI Monitor & EDID Telemetry in 100% Pure Rust
//!
//! Queries the Linux DRM subsystem sysfs nodes directly (/sys/class/drm/*-HDMI-*)
//! and decodes binary VESA EDID 1.3/1.4 structures without any external dependencies.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

use std::fs;
use std::path::Path;

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
}

impl MonitorInfo {
    /// Reads real-time HDMI monitor telemetry from the system
    pub fn read_realtime() -> Self {
        let status_paths = [
            "/sys/class/drm/card0-HDMI-A-1",
            "/sys/class/drm/card1-HDMI-A-1",
        ];

        let base_path = status_paths
            .iter()
            .find(|p| Path::new(p).exists())
            .unwrap_or(&status_paths[0]);

        let status_file = format!("{}/status", base_path);
        let edid_file = format!("{}/edid", base_path);
        let modes_file = format!("{}/modes", base_path);

        let status_raw = fs::read_to_string(&status_file)
            .unwrap_or_else(|_| "disconnected".to_string());
        let connected = status_raw.trim() == "connected";

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

        if !connected {
            return Self {
                connected: false,
                name: "Nenhum Monitor Conectado (Headless Guard Ativo)".to_string(),
                manufacturer: "None".to_string(),
                product_code: 0,
                preferred_mode: "1280x720".to_string(),
                active_mode: "1280x720 @ 60 Hz (Virtual)".to_string(),
                vpu: "VideoCore IV Hardware VPU".to_string(),
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
            active_mode: "1280x720 @ 60 Hz".to_string(),
            vpu: "VideoCore IV Hardware VPU".to_string(),
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

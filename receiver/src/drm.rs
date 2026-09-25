//! DRM KMS Connector Management & Headless Display Guard
//!
//! Monitors and manages the Direct Rendering Manager (DRM) subsystem on Raspberry Pi Zero
//! (Broadcom `vc4-kms-v3d` driver).
//!
//! In headless operation (without a physical HDMI monitor attached), the DRM connector
//! status is reported as `disconnected`. This causes GStreamer's `kmssink` to fail mode
//! negotiation (caps negotiation failure).
//!
//! This module automatically forces the connector state to `on` via sysfs, enabling
//! stable GPU hardware-accelerated video decoding and zero-CPU framebuffer commits.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

use std::fs;
use std::path::Path;

/// Known DRM KMS connector status paths on Raspberry Pi Broadcom VideoCore IV
const DRM_CONNECTOR_PATHS: &[&str] = &[
    "/sys/class/drm/card0-HDMI-A-1/status",
    "/sys/class/drm/card1-HDMI-A-1/status",
];

/// Ensures the DRM HDMI connector is active.
///
/// If a physical display is disconnected, writing `"on\n"` to the sysfs status file
/// commands the `vc4` driver to force state to `"connected"` with fallback EDID modes,
/// allowing `kmssink` to preroll and render immediately.
pub fn ensure_drm_hdmi_connected() {
    for path_str in DRM_CONNECTOR_PATHS {
        let path = Path::new(path_str);
        if !path.exists() {
            continue;
        }

        match fs::read_to_string(path) {
            Ok(status) => {
                let status_trimmed = status.trim();
                if status_trimmed == "disconnected" {
                    println!(
                        "\x1b[1;33m[drm]\x1b[0m Connector {} is disconnected. Forcing status 'on' (Headless Guard)...",
                        path_str
                    );
                    match fs::write(path, "on\n") {
                        Ok(_) => {
                            println!(
                                "\x1b[1;32m[drm]\x1b[0m DRM HDMI connector successfully forced 'connected'."
                            );
                        }
                        Err(e) => {
                            eprintln!(
                                "\x1b[1;31m[drm]\x1b[0m Warning: failed to write 'on' to {}: {}",
                                path_str, e
                            );
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "\x1b[1;31m[drm]\x1b[0m Failed to read DRM connector {}: {}",
                    path_str, e
                );
            }
        }
    }
}

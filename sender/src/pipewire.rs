//! PipeWire Graph and Linux Display Infrastructure Management
//!
//! Encapsulates:
//! - Kernel DRM virtual HDMI connector management (`/sys/class/drm/card*-HDMI-A-*/status`)
//! - GNOME Mutter DisplayConfig multi-monitor layout setup
//! - PipeWire graph port discovery, linking (`pw-link`), and health monitoring
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::fs;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Ensures the kernel HDMI-A-1 connector is connected with the Pi Zero EDID
pub fn ensure_kernel_hdmi_connected() {
    let status_path = "/sys/class/drm/card1-HDMI-A-1/status";
    let is_connected = fs::read_to_string(status_path)
        .map(|s| s.trim() == "connected")
        .unwrap_or(false);

    if is_connected {
        println!("\x1b[1;32m[+] Kernel HDMI-A-1 is connected.\x1b[0m");
        return;
    }

    println!("\x1b[1;33m[*] Forcing kernel HDMI-A-1 connected with Pi monitor EDID...\x1b[0m");
    let cmd = "sudo tee /sys/kernel/debug/dri/1/HDMI-A-1/edid_override < /home/carlos/ide/ext-monitor/edid/pi-monitor.edid > /dev/null && \
               echo 'on' | sudo tee /sys/kernel/debug/dri/1/HDMI-A-1/force > /dev/null && \
               echo 1 | sudo tee /sys/kernel/debug/dri/1/HDMI-A-1/trigger_hotplug > /dev/null";
    let _ = Command::new("bash").arg("-c").arg(cmd).status();
    thread::sleep(Duration::from_millis(500));
}

/// Applies GNOME extended display layout (side-by-side 1280x720@60)
pub fn ensure_gnome_displays() {
    let check = Command::new("gdbus")
        .args(&[
            "call",
            "--session",
            "--dest",
            "org.gnome.Mutter.DisplayConfig",
            "--object-path",
            "/org/gnome/Mutter/DisplayConfig",
            "--method",
            "org.gnome.Mutter.DisplayConfig.GetCurrentState",
        ])
        .output();

    let (serial, is_configured) = if let Ok(out) = check {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let serial = if let Some(start) = stdout.find("(uint32 ") {
            let rest = &stdout[start + 8..];
            rest.find(',').and_then(|end| rest[..end].trim().parse::<u32>().ok()).unwrap_or(1)
        } else {
            1
        };

        let is_logical = stdout.contains("('HDMI-1'") && stdout.contains("(1920, 0");
        (serial, is_logical)
    } else {
        (1, false)
    };

    if is_configured {
        println!("\x1b[1;32m[+] GNOME Mutter displays already configured with HDMI-1 in extended mode (side-by-side).\x1b[0m");
        return;
    }

    println!("\x1b[1;33m[*] Applying GNOME extended display layout (side-by-side 1600x900, serial={})...\x1b[0m", serial);
    let apply_cmd = format!(
        r#"gdbus call --session --dest org.gnome.Mutter.DisplayConfig --object-path /org/gnome/Mutter/DisplayConfig --method org.gnome.Mutter.DisplayConfig.ApplyMonitorsConfig {} 1 "[(0, 0, 1.0, 0, true, [('eDP-1', '1920x1080@60.003', @a{{sv}} {{}})]), (1920, 0, 1.0, 0, false, [('HDMI-1', '1600x900@59.946', @a{{sv}} {{}})])]" "@a{{sv}} {{}}""#,
        serial
    );
    let _ = Command::new("bash").arg("-c").arg(&apply_cmd).status();
    thread::sleep(Duration::from_millis(500));
}

/// Discovers the PipeWire capture output port for `node_id` and links it to `ext-hdmi-sender`
#[allow(dead_code)]
pub fn link_monitor_port_to_sender(node_id: u32, monitor_name: &str) {
    for _ in 0..10 {
        if let Ok(output) = Command::new("pw-dump").output() {
            if let Ok(dump) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                if let Some(items) = dump.as_array() {
                    let mut out_port = None;

                    for item in items {
                        if item["type"].as_str() == Some("PipeWire:Interface:Port") {
                            let props = &item["info"]["props"];
                            let nid = props["node.id"].as_u64().or_else(|| {
                                props["node.id"].as_str().and_then(|s| s.parse::<u64>().ok())
                            });
                            let dir = props["port.direction"].as_str().unwrap_or("");

                            if nid == Some(node_id as u64) && dir == "out" {
                                out_port = item["id"].as_u64();
                                break;
                            }
                        }
                    }

                    let mut in_port = None;
                    for item in items {
                        if item["type"].as_str() == Some("PipeWire:Interface:Port") {
                            let props = &item["info"]["props"];
                            let dir = props["port.direction"].as_str().unwrap_or("");
                            if dir == "in" {
                                // Skip any audio ports
                                if props["audio.channel"].as_str().is_some() {
                                    continue;
                                }
                                if let Some(dsp) = props["format.dsp"].as_str() {
                                    if dsp.contains("audio") {
                                        continue;
                                    }
                                }

                                if let Some(alias) = props["port.alias"].as_str() {
                                    if alias.contains("ext-video-sender") || alias.contains("ext-hdmi-sender") {
                                        in_port = item["id"].as_u64();
                                        break;
                                    }
                                    if alias.contains("gst-launch") && !alias.contains("_FL") && !alias.contains("_FR") {
                                        in_port = item["id"].as_u64();
                                        break;
                                    }
                                }
                                if let Some(path) = props["object.path"].as_str() {
                                    if path.contains("ext-video-sender") || path.contains("ext-hdmi-sender") {
                                        in_port = item["id"].as_u64();
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    if let Some(p) = out_port {
                        println!("\x1b[1;34m[*] Found {} Output Port: {}\x1b[0m", monitor_name, p);
                        let targets: Vec<String> = if let Some(inp) = in_port {
                            vec![inp.to_string(), "ext-video-sender:input_0".to_string(), "ext-video-sender:input_1".to_string(), "ext-hdmi-sender:input_0".to_string(), "ext-hdmi-sender:input_1".to_string()]
                        } else {
                            vec!["ext-video-sender:input_0".to_string(), "ext-video-sender:input_1".to_string(), "ext-hdmi-sender:input_0".to_string(), "ext-hdmi-sender:input_1".to_string()]
                        };

                        for target in targets {
                            let output = Command::new("pw-link")
                                .arg(p.to_string())
                                .arg(&target)
                                .output();

                            if let Ok(out) = output {
                                let err_str = String::from_utf8_lossy(&out.stderr);
                                if out.status.success() || err_str.contains("File exists") || err_str.contains("existe") {
                                    println!("\x1b[1;32m[+] Successfully linked {} (port {}) -> {}!\x1b[0m", monitor_name, p, target);
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(300));
    }

    eprintln!("\x1b[1;31m[!] Warning: Failed to link {} port automatically after 3s.\x1b[0m", monitor_name);
}

/// Checks whether the PipeWire screencast node is still alive
pub fn is_pipewire_node_alive(node_id: u32) -> bool {
    let output = match Command::new("pw-cli")
        .arg("info")
        .arg(node_id.to_string())
        .output()
    {
        Ok(o) => o,
        Err(_) => return false,
    };
    let s = String::from_utf8_lossy(&output.stdout);
    let err = String::from_utf8_lossy(&output.stderr);
    !s.is_empty() && !s.contains("unknown global") && !err.contains("unknown global")
}

/// Verifies whether `ext-hdmi-sender` is still linked to the PipeWire graph
pub fn is_sender_linked() -> bool {
    if let Ok(output) = Command::new("pw-link").arg("-l").output() {
        let s = String::from_utf8_lossy(&output.stdout);
        return s.contains("ext-hdmi-sender") || s.contains("gst-launch");
    }
    true
}

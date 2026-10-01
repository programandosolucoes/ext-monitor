//! Web Dashboard Hot-Apply UDP Control Protocol
//!
//! Listens on UDP port 5001 for real-time configuration changes emitted by the
//! embedded web dashboard or CLI triggers, returning strongly typed control actions.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use crate::config::{CaptureEngine, ColorProfile, ScaleMode, TransportKind};
use std::net::UdpSocket;

#[derive(Debug, Clone, PartialEq)]
pub enum ControlAction {
    StartStreaming,
    StopStreaming,
    LaunchMiracast,
    SetMode(String),
    SetAudio(bool),
    SetBitrate(u32),
    SetFps(u32),
    SetColorProfile(ColorProfile),
    SetDropOnly(bool),
    SetSkipToFirst(bool),
    SetKeyIntMax(u32),
    SetCapture(CaptureEngine),
    SetMonitor(String),
    SetTransport(TransportKind),
    SetScale(ScaleMode),
    SetCas(bool),
    TriggerHud,
    HideHud,
}

pub struct ControlListener {
    socket: Option<UdpSocket>,
}

impl ControlListener {
    /// Binds non-blocking UDP control listener on port 5001
    pub fn bind(port: u16) -> Self {
        let socket = match UdpSocket::bind(format!("0.0.0.0:{}", port)) {
            Ok(s) => {
                let _ = s.set_nonblocking(true);
                println!(
                    "\x1b[1;32m[+] Control Listener active on UDP port {} (Web Hot-Apply & HUD Trigger)\x1b[0m",
                    port
                );
                Some(s)
            }
            Err(e) => {
                eprintln!("\x1b[1;33m[!] Warning: Failed to bind control UDP socket {}: {}\x1b[0m", port, e);
                None
            }
        };

        Self { socket }
    }

    /// Polls pending control datagrams and returns all actions received
    pub fn poll_actions(&self) -> Vec<ControlAction> {
        let mut actions = Vec::new();
        let socket = match self.socket.as_ref() {
            Some(s) => s,
            None => return actions,
        };

        let mut buf = [0u8; 2048];
        while let Ok((n, _src)) = socket.recv_from(&mut buf) {
            actions.extend(parse_control_payload(&buf[..n]));
        }

        actions
    }
}

/// Parses raw JSON payload into strongly typed ControlAction commands
pub fn parse_control_payload(buf: &[u8]) -> Vec<ControlAction> {
    let mut actions = Vec::new();
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(buf) {
        if let Some(action) = v.get("action").and_then(|x| x.as_str()) {
            match action {
                "start" | "start_streaming" => actions.push(ControlAction::StartStreaming),
                "stop" | "stop_streaming" => actions.push(ControlAction::StopStreaming),
                "launch_miracast" | "miracast" => actions.push(ControlAction::LaunchMiracast),
                "trigger_hud" => actions.push(ControlAction::TriggerHud),
                "hide_hud" | "kill_hud" => actions.push(ControlAction::HideHud),
                _ => {}
            }
        }

        if let Some(mode_str) = v.get("mode").and_then(|x| x.as_str()) {
            actions.push(ControlAction::SetMode(mode_str.to_lowercase()));
        }

        if let Some(audio_val) = v.get("audio").and_then(|x| x.as_bool()) {
            actions.push(ControlAction::SetAudio(audio_val));
        }

        if let Some(bitrate) = v.get("bitrate").and_then(|x| x.as_u64()).map(|x| x as u32) {
            if (150..=15000).contains(&bitrate) {
                actions.push(ControlAction::SetBitrate(bitrate));
            }
        }

        if let Some(fps) = v.get("fps").and_then(|x| x.as_u64()).map(|x| x as u32) {
            if (10..=60).contains(&fps) {
                actions.push(ControlAction::SetFps(fps));
            }
        }

        if let Some(color) = v.get("color").and_then(|x| x.as_str()) {
            let cp = match color {
                "256" => ColorProfile::Economy256,
                "gray" => ColorProfile::Grayscale,
                _ => ColorProfile::TrueColor,
            };
            actions.push(ControlAction::SetColorProfile(cp));
        }

        if let Some(drop_val) = v.get("drop_only").and_then(|x| x.as_bool()) {
            actions.push(ControlAction::SetDropOnly(drop_val));
        }

        if let Some(skip_val) = v.get("skip_to_first").and_then(|x| x.as_bool()) {
            actions.push(ControlAction::SetSkipToFirst(skip_val));
        }

        if let Some(key_int) = v.get("key_int_max").and_then(|x| x.as_u64()).map(|x| x as u32) {
            if (5..=300).contains(&key_int) {
                actions.push(ControlAction::SetKeyIntMax(key_int));
            }
        }

        if let Some(capture_str) = v.get("capture").and_then(|x| x.as_str()) {
            let eng = match capture_str {
                "kms" | "drm" => CaptureEngine::Kms,
                _ => CaptureEngine::Mutter,
            };
            actions.push(ControlAction::SetCapture(eng));
        }

        if let Some(mon_str) = v.get("monitor").and_then(|x| x.as_str()) {
            if !mon_str.trim().is_empty() {
                actions.push(ControlAction::SetMonitor(mon_str.trim().to_string()));
            }
        }

        if let Some(trans_str) = v.get("transport").and_then(|x| x.as_str()) {
            let tk = match trans_str {
                "usb_bulk" | "usb" | "bulk" => TransportKind::UsbBulk,
                "miracast" | "wfd" => {
                    actions.push(ControlAction::LaunchMiracast);
                    TransportKind::Miracast
                }
                _ => TransportKind::Network {
                    ip: "192.168.7.2".to_string(),
                    port: 5000,
                },
            };
            actions.push(ControlAction::SetTransport(tk));
        }

        if let Some(scale_str) = v.get("scale").and_then(|x| x.as_str()) {
            actions.push(ControlAction::SetScale(ScaleMode::from_str(scale_str)));
        }

        if let Some(cas_val) = v.get("cas").and_then(|x| x.as_bool()) {
            actions.push(ControlAction::SetCas(cas_val));
        }
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_control_start_and_stop() {
        let start_json = br#"{"action":"start"}"#;
        let actions = parse_control_payload(start_json);
        assert_eq!(actions, vec![ControlAction::StartStreaming]);

        let stop_json = br#"{"action":"stop"}"#;
        let actions = parse_control_payload(stop_json);
        assert_eq!(actions, vec![ControlAction::StopStreaming]);
    }

    #[test]
    fn test_parse_control_transport_modes() {
        let usb_json = br#"{"transport":"usb_bulk"}"#;
        let actions = parse_control_payload(usb_json);
        assert_eq!(actions, vec![ControlAction::SetTransport(TransportKind::UsbBulk)]);

        let net_json = br#"{"transport":"network"}"#;
        let actions = parse_control_payload(net_json);
        assert_eq!(
            actions,
            vec![ControlAction::SetTransport(TransportKind::Network {
                ip: "192.168.7.2".to_string(),
                port: 5000,
            })]
        );
    }

    #[test]
    fn test_parse_control_fps_and_bitrate_bounds() {
        // Valid bounds
        let valid = br#"{"fps":60,"bitrate":3500}"#;
        let actions = parse_control_payload(valid);
        assert!(actions.contains(&ControlAction::SetFps(60)));
        assert!(actions.contains(&ControlAction::SetBitrate(3500)));

        // Invalid bounds (fps > 60, bitrate < 150)
        let invalid = br#"{"fps":120,"bitrate":50}"#;
        let actions = parse_control_payload(invalid);
        assert!(!actions.iter().any(|a| matches!(a, ControlAction::SetFps(_))));
        assert!(!actions.iter().any(|a| matches!(a, ControlAction::SetBitrate(_))));
    }

    #[test]
    fn test_parse_control_color_and_capture_profile() {
        let json = br#"{"color":"gray","capture":"kms"}"#;
        let actions = parse_control_payload(json);
        assert!(actions.contains(&ControlAction::SetColorProfile(ColorProfile::Grayscale)));
        assert!(actions.contains(&ControlAction::SetCapture(CaptureEngine::Kms)));
    }

    #[test]
    fn test_parse_control_scale_and_cas() {
        let json = br#"{"scale":"off","cas":true}"#;
        let actions = parse_control_payload(json);
        assert!(actions.contains(&ControlAction::SetScale(ScaleMode::Off)));
        assert!(actions.contains(&ControlAction::SetCas(true)));

        let json2 = br#"{"scale":"720p","cas":false}"#;
        let actions2 = parse_control_payload(json2);
        assert!(actions2.contains(&ControlAction::SetScale(ScaleMode::Native720p)));
        assert!(actions2.contains(&ControlAction::SetCas(false)));
    }
}

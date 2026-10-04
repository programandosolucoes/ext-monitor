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
/// Enumerates supported options for Controlaction.
pub enum ControlAction {
    StartStreaming,
    ChromeCastLaunch,
    StopStreaming,
    LaunchMiracast,
    SetMode(String),
    SetAudio(bool),
    SetAudioRate(u32),
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
    SetAutoConnect(bool),
}

/// Represents Controllistener configuration and operational state.
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
                "cast_launch" | "chrome_cast_launch" => actions.push(ControlAction::ChromeCastLaunch),
                "stop" | "stop_streaming" | "standby" | "disable" => actions.push(ControlAction::StopStreaming),
                "launch_miracast" | "miracast" => actions.push(ControlAction::LaunchMiracast),
                "trigger_hud" => actions.push(ControlAction::TriggerHud),
                "hide_hud" | "kill_hud" => actions.push(ControlAction::HideHud),
                _ => {}
            }
        }

        if let Some(mode_str) = v.get("mode").and_then(|x| x.as_str()) {
            let m_lower = mode_str.to_lowercase();
            match m_lower.as_str() {
                "miracast" | "wfd" | "mode2" | "2" | "mode2_miracast" => {
                    if !actions.contains(&ControlAction::LaunchMiracast) {
                        actions.push(ControlAction::LaunchMiracast);
                    }
                    if !actions.iter().any(|a| matches!(a, ControlAction::SetTransport(_))) {
                        actions.push(ControlAction::SetTransport(TransportKind::Miracast));
                    }
                }
                "usb_bulk" | "usb" | "bulk" | "mode3" | "3" | "mode3_usb_bulk" => {
                    if !actions.iter().any(|a| matches!(a, ControlAction::SetTransport(_))) {
                        actions.push(ControlAction::SetTransport(TransportKind::UsbBulk));
                    }
                }
                "network" | "udp" | "mode1" | "1" | "mode1_udp" => {
                    if !actions.iter().any(|a| matches!(a, ControlAction::SetTransport(_))) {
                        actions.push(ControlAction::SetTransport(TransportKind::Network {
                            ip: "192.168.7.2".to_string(),
                            port: 5000,
                        }));
                    }
                }
                _ => {
                    actions.push(ControlAction::SetMode(m_lower));
                }
            }
        }

        if let Some(audio_val) = v.get("audio").and_then(|x| x.as_bool()) {
            actions.push(ControlAction::SetAudio(audio_val));
        }

        if let Some(rate) = v.get("audio_rate").or_else(|| v.get("rate")).and_then(|x| x.as_u64()).map(|x| x as u32) {
            if [44100, 48000, 88200, 96000, 192000].contains(&rate) {
                actions.push(ControlAction::SetAudioRate(rate));
            }
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

        let trans_field = v
            .get("transport")
            .or_else(|| v.get("active_transport"))
            .and_then(|x| x.as_str());
        if let Some(trans_str) = trans_field {
            let tk = match trans_str {
                "usb_bulk" | "usb" | "bulk" | "mode3" | "3" | "mode3_usb_bulk" => {
                    TransportKind::UsbBulk
                }
                "miracast" | "wfd" | "mode2" | "2" | "mode2_miracast" => {
                    if !actions.contains(&ControlAction::LaunchMiracast) {
                        actions.push(ControlAction::LaunchMiracast);
                    }
                    TransportKind::Miracast
                }
                _ => TransportKind::Network {
                    ip: "192.168.7.2".to_string(),
                    port: 5000,
                },
            };
            if let Some(pos) = actions
                .iter()
                .position(|a| matches!(a, ControlAction::SetTransport(_)))
            {
                actions[pos] = ControlAction::SetTransport(tk);
            } else {
                actions.push(ControlAction::SetTransport(tk));
            }
        }

        if let Some(scale_str) = v.get("scale").and_then(|x| x.as_str()) {
            actions.push(ControlAction::SetScale(ScaleMode::from_str(scale_str)));
        }

        if let Some(cas_val) = v.get("cas").and_then(|x| x.as_bool()) {
            actions.push(ControlAction::SetCas(cas_val));
        }

        if let Some(ac) = v.get("auto_connect").and_then(|x| x.as_bool()) {
            actions.push(ControlAction::SetAutoConnect(ac));
        }
    } else {
        if std::env::var("EXT_DEBUG").map(|v| v == "1").unwrap_or(false) {
            eprintln!("\x1b[1;31m[DEBUG] Malformed JSON payload received: {}\x1b[0m", String::from_utf8_lossy(buf));
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

    #[test]
    fn test_parse_control_miracast_actions() {
        let launch_json = br#"{"action":"launch_miracast"}"#;
        let actions = parse_control_payload(launch_json);
        assert_eq!(actions, vec![ControlAction::LaunchMiracast]);

        let trans_miracast = br#"{"transport":"miracast"}"#;
        let actions2 = parse_control_payload(trans_miracast);
        assert!(actions2.contains(&ControlAction::LaunchMiracast));
        assert!(actions2.contains(&ControlAction::SetTransport(TransportKind::Miracast)));

        let mode_miracast = br#"{"mode":"miracast"}"#;
        let actions3 = parse_control_payload(mode_miracast);
        assert!(actions3.contains(&ControlAction::LaunchMiracast));
        assert!(actions3.contains(&ControlAction::SetTransport(TransportKind::Miracast)));

        let active_trans = br#"{"active_transport":"mode2_miracast"}"#;
        let actions4 = parse_control_payload(active_trans);
        assert!(actions4.contains(&ControlAction::LaunchMiracast));
        assert!(actions4.contains(&ControlAction::SetTransport(TransportKind::Miracast)));

        let start_miracast = br#"{"action":"start","transport":"miracast"}"#;
        let actions5 = parse_control_payload(start_miracast);
        assert!(actions5.contains(&ControlAction::StartStreaming));
        assert!(actions5.contains(&ControlAction::LaunchMiracast));
        assert!(actions5.contains(&ControlAction::SetTransport(TransportKind::Miracast)));
    }

    #[test]
    fn test_parse_control_auto_connect() {
        let payload = br#"{"auto_connect":true}"#;
        let actions = parse_control_payload(payload);
        assert_eq!(actions, vec![ControlAction::SetAutoConnect(true)]);

        let payload_off = br#"{"auto_connect":false}"#;
        let actions_off = parse_control_payload(payload_off);
        assert_eq!(actions_off, vec![ControlAction::SetAutoConnect(false)]);
    }

    #[test]
    fn test_parse_control_chrome_cast_launch() {
        let launch_payload = br#"{"action":"chrome_cast_launch"}"#;
        let actions = parse_control_payload(launch_payload);
        assert_eq!(actions, vec![ControlAction::ChromeCastLaunch]);

        let cast_payload = br#"{"action":"cast_launch"}"#;
        let actions2 = parse_control_payload(cast_payload);
        assert_eq!(actions2, vec![ControlAction::ChromeCastLaunch]);
    }

    #[test]
    fn test_parse_control_audio_simultaneous_and_decoupled() {
        // Mode 3 USB Bulk without simultaneous audio (pure video)
        let payload_video_only = br#"{"action":"start","mode":"extend","transport":"usb_bulk","audio":false}"#;
        let actions = parse_control_payload(payload_video_only);
        assert!(actions.contains(&ControlAction::StartStreaming));
        assert!(actions.contains(&ControlAction::SetMode("extend".to_string())));
        assert!(actions.contains(&ControlAction::SetTransport(TransportKind::UsbBulk)));
        assert!(actions.contains(&ControlAction::SetAudio(false)));

        // Mode 1 Network UDP with simultaneous audio enabled
        let payload_with_audio = br#"{"action":"start","mode":"clone","transport":"network","audio":true}"#;
        let actions2 = parse_control_payload(payload_with_audio);
        assert!(actions2.contains(&ControlAction::StartStreaming));
        assert!(actions2.contains(&ControlAction::SetMode("clone".to_string())));
        assert!(actions2.contains(&ControlAction::SetTransport(TransportKind::Network {
            ip: "192.168.7.2".to_string(),
            port: 5000,
        })));
        assert!(actions2.contains(&ControlAction::SetAudio(true)));
    }

    #[test]
    fn test_parse_control_audio_rate_192khz() {
        let payload = br#"{"audio_rate":192000}"#;
        let actions = parse_control_payload(payload);
        assert!(actions.contains(&ControlAction::SetAudioRate(192000)));

        let payload_combo = br#"{"audio":true,"audio_rate":192000}"#;
        let actions2 = parse_control_payload(payload_combo);
        assert!(actions2.contains(&ControlAction::SetAudio(true)));
        assert!(actions2.contains(&ControlAction::SetAudioRate(192000)));
    }
}


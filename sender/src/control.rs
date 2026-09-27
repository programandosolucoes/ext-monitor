//! Web Dashboard Hot-Apply UDP Control Protocol
//!
//! Listens on UDP port 5001 for real-time configuration changes emitted by the
//! embedded web dashboard or CLI triggers, returning strongly typed control actions.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

use crate::config::ColorProfile;
use std::net::UdpSocket;

#[derive(Debug, Clone, PartialEq)]
pub enum ControlAction {
    SetBitrate(u32),
    SetFps(u32),
    SetColorProfile(ColorProfile),
    SetDropOnly(bool),
    SetSkipToFirst(bool),
    SetKeyIntMax(u32),
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
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&buf[..n]) {
                if let Some(action) = v.get("action").and_then(|x| x.as_str()) {
                    match action {
                        "trigger_hud" => actions.push(ControlAction::TriggerHud),
                        "hide_hud" | "kill_hud" => actions.push(ControlAction::HideHud),
                        _ => {}
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
            }
        }

        actions
    }
}

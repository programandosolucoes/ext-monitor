//! Network & Wi-Fi Auto-Discovery Beacon and Responder for Ext-Monitor Receiver
//!
//! Listens on UDP broadcast port 5005 for probe packets ("EXT-MONITOR-DISCOVER")
//! and responds with device information and IP so senders on Wi-Fi or Ethernet
//! can connect seamlessly without hardcoded IPs.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub const DISCOVERY_PORT: u16 = 5005;

/// Spawns the background discovery responder on UDP port 5002
pub fn start_discovery_beacon(running: Arc<AtomicBool>, http_port: u16, stream_port: u16) {
    thread::Builder::new()
        .name("discovery-beacon".to_string())
        .spawn(move || {
            let bind_addr = SocketAddr::from(([0, 0, 0, 0], DISCOVERY_PORT));
            let socket = match UdpSocket::bind(bind_addr) {
                Ok(s) => {
                    let _ = s.set_broadcast(true);
                    let _ = s.set_read_timeout(Some(Duration::from_millis(1000)));
                    println!(
                        "\x1b[1;32m[discovery]\x1b[0m Wi-Fi/LAN Auto-Discovery Responder active on UDP port {}",
                        DISCOVERY_PORT
                    );
                    s
                }
                Err(e) => {
                    eprintln!(
                        "\x1b[1;33m[discovery]\x1b[0m Failed to bind UDP {}: {}. Auto-discovery disabled.",
                        DISCOVERY_PORT, e
                    );
                    return;
                }
            };

            let response_payload = format!(
                "EXT-MONITOR-OFFER http_port={} stream_port={} version=2.3.0\n",
                http_port, stream_port
            );

            let mut buf = [0u8; 1024];
            let mut last_periodic_announce = std::time::Instant::now();

            while running.load(Ordering::Relaxed) {
                // 1. Listen for probes from senders
                if let Ok((len, src)) = socket.recv_from(&mut buf) {
                    let req = String::from_utf8_lossy(&buf[..len]);
                    if req.contains("EXT-MONITOR-DISCOVER") {
                        let _ = socket.send_to(response_payload.as_bytes(), src);
                    }
                }

                // 2. Periodic broadcast announcement (every 10s)
                if last_periodic_announce.elapsed() >= Duration::from_secs(10) {
                    last_periodic_announce = std::time::Instant::now();
                    let target = format!("255.255.255.255:{}", DISCOVERY_PORT);
                    let _ = socket.send_to(response_payload.as_bytes(), target.as_str());
                }
            }
        })
        .expect("Failed to spawn discovery beacon thread");
}

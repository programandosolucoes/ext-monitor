//! Network and Wi-Fi Auto-Discovery for Ext-Monitor Receiver
//!
//! Automatically discovers Raspberry Pi Zero or external monitors on the local
//! Wi-Fi / Ethernet subnet using UDP broadcast probes on port 5005 and SSDP fallback.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

pub const DISCOVERY_PORT: u16 = 5005;
pub const PROBE_PAYLOAD: &[u8] = b"EXT-MONITOR-DISCOVER";

/// Attempts to discover the receiver IP automatically
/// Priority:
/// 1. USB Direct Cable (192.168.7.2) - ultra low latency (<1ms check)
/// 2. Wi-Fi / Ethernet Subnet Broadcast on port 5005
pub fn discover_receiver_ip(timeout: Duration) -> Option<String> {
    // 1. Quick check USB Gadget default IP (192.168.7.2)
    if is_endpoint_alive("192.168.7.2", 8080) {
        return Some("192.168.7.2".to_string());
    }

    println!("\x1b[1;34m[*] USB (192.168.7.2) inativo. Disparando auto-discovery via Wi-Fi/Rede Local...\x1b[0m");

    // 2. Broadcast probe on local subnet
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return None,
    };

    let _ = socket.set_broadcast(true);
    let _ = socket.set_read_timeout(Some(Duration::from_millis(200)));

    let target_broad = format!("255.255.255.255:{}", DISCOVERY_PORT);
    let target_multi = format!("224.0.0.1:{}", DISCOVERY_PORT);
    let broadcast_targets = [
        target_broad.as_str(),
        target_multi.as_str(),
    ];

    let start = Instant::now();
    let mut buf = [0u8; 1024];

    while start.elapsed() < timeout {
        for target in &broadcast_targets {
            let _ = socket.send_to(PROBE_PAYLOAD, target);
        }

        while let Ok((len, src)) = socket.recv_from(&mut buf) {
            let resp = String::from_utf8_lossy(&buf[..len]);
            if resp.contains("EXT-MONITOR-OFFER") {
                let ip = match src {
                    SocketAddr::V4(v4) => v4.ip().to_string(),
                    SocketAddr::V6(v6) => v6.ip().to_string(),
                };
                println!("\x1b[1;32m[+] Appliance Ext-Monitor descoberto via Wi-Fi/Rede em {}!\x1b[0m", ip);
                return Some(ip);
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    None
}

/// Helper to check if HTTP status port responds
fn is_endpoint_alive(ip: &str, port: u16) -> bool {
    use std::net::TcpStream;
    let addr = format!("{}:{}", ip, port);
    TcpStream::connect_timeout(
        &addr.parse().unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], port))),
        Duration::from_millis(150),
    )
    .is_ok()
}

//! Network and Wi-Fi Auto-Discovery for Ext-Monitor Receiver
//!
//! Automatically discovers Raspberry Pi Zero or external monitors on the local
//! Wi-Fi / Ethernet subnet using UDP broadcast probes on port 5005 and SSDP fallback.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::os::unix::io::FromRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
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

/// Spawns a host-side SSDP / DIAL reflector bridge on UDP 1900.
/// This allows Google Chrome running locally on the PC to instantly discover the
/// Pi Zero via DIAL without being blocked by host kernel multicast routing.
pub fn start_host_ssdp_bridge(running: Arc<AtomicBool>, target_ip: String, http_port: u16) {
    thread::Builder::new()
        .name("host-ssdp-bridge".to_string())
        .spawn(move || {
            // 1. If USB gadget interface exists, ensure multicast route is configured
            let _ = std::process::Command::new("ip")
                .args(["route", "replace", "239.255.255.250", "dev", "enx122233445566"])
                .output();

            // 2. Bind UDP 1900 with SO_REUSEADDR and SO_REUSEPORT using libc
            let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
            if fd < 0 {
                eprintln!("\x1b[1;33m[host-ssdp]\x1b[0m Failed to create UDP socket for SSDP bridge.");
                return;
            }

            let opt: libc::c_int = 1;
            unsafe {
                libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_REUSEADDR,
                    &opt as *const _ as *const libc::c_void,
                    std::mem::size_of_val(&opt) as libc::socklen_t,
                );
                libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_REUSEPORT,
                    &opt as *const _ as *const libc::c_void,
                    std::mem::size_of_val(&opt) as libc::socklen_t,
                );

                let mut addr: libc::sockaddr_in = std::mem::zeroed();
                addr.sin_family = libc::AF_INET as libc::sa_family_t;
                addr.sin_port = (1900u16).to_be();
                addr.sin_addr.s_addr = libc::INADDR_ANY;

                if libc::bind(
                    fd,
                    &addr as *const _ as *const libc::sockaddr,
                    std::mem::size_of_val(&addr) as libc::socklen_t,
                ) != 0
                {
                    libc::close(fd);
                    eprintln!("\x1b[1;33m[host-ssdp]\x1b[0m Failed to bind UDP 1900 on host. Continuing without local SSDP bridge.");
                    return;
                }
            }

            let socket = unsafe { UdpSocket::from_raw_fd(fd) };
            let _ = socket.set_broadcast(true);
            let multicast_addr = Ipv4Addr::new(239, 255, 255, 250);
            let _ = socket.join_multicast_v4(&multicast_addr, &Ipv4Addr::UNSPECIFIED);
            let _ = socket.set_read_timeout(Some(Duration::from_millis(500)));

            println!("\x1b[1;32m[host-ssdp]\x1b[0m Local DIAL / Cast SSDP bridge active on 239.255.255.250:1900 -> {}", target_ip);

            let mut buf = [0u8; 2048];
            let mut last_notify = Instant::now();

            while running.load(Ordering::Relaxed) {
                // Periodic SSDP alive announcement to host
                if last_notify.elapsed() >= Duration::from_secs(5) {
                    last_notify = Instant::now();
                    let msg = format!(
                        "NOTIFY * HTTP/1.1\r\n\
                         HOST: 239.255.255.250:1900\r\n\
                         CACHE-CONTROL: max-age=1800\r\n\
                         LOCATION: http://{}:{}/dial/dd.xml\r\n\
                         APPLICATION-URL: http://{}:{}/apps/\r\n\
                         NT: urn:dial-multiscreen-org:service:dial:1\r\n\
                         NTS: ssdp:alive\r\n\
                         SERVER: Linux/6.6 UPnP/1.0 Ext-Monitor/2.3\r\n\
                         USN: uuid:ext-monitor-dial-device::urn:dial-multiscreen-org:service:dial:1\r\n\
                         BOOTID.UPNP.ORG: 1\r\n\
                         CONFIGID.UPNP.ORG: 1\r\n\r\n",
                        target_ip, http_port, target_ip, http_port
                    );
                    let _ = socket.send_to(msg.as_bytes(), "239.255.255.250:1900");
                }

                // Handle M-SEARCH from local Chrome / applications
                if let Ok((len, src)) = socket.recv_from(&mut buf) {
                    let req = String::from_utf8_lossy(&buf[..len]);
                    let trimmed = req.trim_start();
                    if trimmed.starts_with("M-SEARCH") || trimmed.starts_with("m-search") {
                        if req.contains("dial") || req.contains("MediaRenderer") || req.contains("ssdp:all") || req.contains("upnp:rootdevice") {
                            let st = if req.contains("device:dial:1") {
                                "urn:dial-multiscreen-org:device:dial:1"
                            } else if req.contains("dial") {
                                "urn:dial-multiscreen-org:service:dial:1"
                            } else if req.contains("MediaRenderer") {
                                "urn:schemas-upnp-org:device:MediaRenderer:1"
                            } else {
                                "upnp:rootdevice"
                            };

                            let is_dial = st.contains("dial");
                            let loc_path = if is_dial { "/dial/dd.xml" } else { "/upnp/desc.xml" };
                            let usn = if is_dial {
                                format!("uuid:ext-monitor-dial-device::{}", st)
                            } else {
                                format!("uuid:ext-monitor-bcm2835-renderer::{}", st)
                            };
                            let app_url_header = if is_dial {
                                format!("APPLICATION-URL: http://{}:{}/apps/\r\n", target_ip, http_port)
                            } else {
                                String::new()
                            };

                            let response = format!(
                                "HTTP/1.1 200 OK\r\n\
                                 CACHE-CONTROL: max-age=1800\r\n\
                                 DATE: Tue, 29 Sep 2026 18:00:00 GMT\r\n\
                                 EXT:\r\n\
                                 LOCATION: http://{}:{}{}\r\n\
                                 {}SERVER: Linux/6.6 UPnP/1.0 Ext-Monitor/2.3\r\n\
                                 ST: {}\r\n\
                                 USN: {}\r\n\
                                 BOOTID.UPNP.ORG: 1\r\n\
                                 CONFIGID.UPNP.ORG: 1\r\n\
                                 SEARCHPORT.UPNP.ORG: 1900\r\n\r\n",
                                target_ip, http_port, loc_path, app_url_header, st, usn
                            );
                            let _ = socket.send_to(response.as_bytes(), src);
                        }
                    }
                }
            }
        })
        .ok();
}


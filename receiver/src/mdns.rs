//! Pure-Rust mDNS (Multicast DNS / RFC 6762) Responder for Google Cast & Wi-Fi Display
//!
//! Provides zero-dependency, pure-Rust network responder on UDP port 5353 (224.0.0.251)
//! advertising:
//! 1. `_googlecast._tcp.local` (Google Cast / Chrome Media Router)
//! 2. `_display._tcp.local` (Miracast / MS-MICE)
//! 3. `_miracast._tcp.local` (Wi-Fi Display WFD RTSP)
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::os::unix::io::FromRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub const MDNS_PORT: u16 = 5353;
pub const MDNS_MULTICAST_IPV4: [u8; 4] = [224, 0, 0, 251];

/// Encodes a dotted domain name into DNS wire format (length-prefixed labels ending in 0x00).
pub fn encode_dns_name(name: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(name.len() + 2);
    for part in name.split('.') {
        if !part.is_empty() {
            let bytes = part.as_bytes();
            out.push(bytes.len() as u8);
            out.extend_from_slice(bytes);
        }
    }
    out.push(0x00);
    out
}

/// Encodes an array of key=value strings into DNS TXT RDATA (length-prefixed strings).
pub fn encode_dns_txt(txts: &[&str]) -> Vec<u8> {
    let mut out = Vec::new();
    for txt in txts {
        let b = txt.as_bytes();
        let len = b.len().min(255);
        out.push(len as u8);
        out.extend_from_slice(&b[..len]);
    }
    out
}

/// Builds a complete mDNS response packet containing Google Cast, Miracast, and Display records.
pub fn build_full_mdns_response(device_ip: [u8; 4], _http_port: u16, miracast_port: u16) -> Vec<u8> {
    let mut packet = Vec::with_capacity(512);

    // 1. Header (12 bytes)
    // ID: 0, Flags: 0x8400 (Response, Authoritative), QD: 0, AN: 3, NS: 0, AR: 4
    packet.extend_from_slice(&[0x00, 0x00]); // ID
    packet.extend_from_slice(&[0x84, 0x00]); // Flags
    packet.extend_from_slice(&[0x00, 0x00]); // QDCOUNT: 0
    packet.extend_from_slice(&[0x00, 0x03]); // ANCOUNT: 3 PTR records
    packet.extend_from_slice(&[0x00, 0x00]); // NSCOUNT: 0
    packet.extend_from_slice(&[0x00, 0x04]); // ARCOUNT: 4 Additional (SRV x2, TXT, A)

    let cast_service = "_googlecast._tcp.local";
    let cast_instance = "RaspCast._googlecast._tcp.local";
    let display_service = "_display._tcp.local";
    let display_instance = "Raspberry-Pi-Miracast._display._tcp.local";
    let wfd_service = "_miracast._tcp.local";
    let wfd_instance = "Raspberry-Pi-Miracast._miracast._tcp.local";
    let host_name = "pi-zero.local";

    // Answer 1: PTR _googlecast._tcp.local -> cast_instance
    let cast_ptr = encode_dns_name(cast_service);
    let cast_inst_name = encode_dns_name(cast_instance);
    packet.extend_from_slice(&cast_ptr);
    packet.extend_from_slice(&12u16.to_be_bytes()); // TYPE: PTR
    packet.extend_from_slice(&0x0001u16.to_be_bytes()); // CLASS: IN
    packet.extend_from_slice(&120u32.to_be_bytes()); // TTL: 120s
    packet.extend_from_slice(&(cast_inst_name.len() as u16).to_be_bytes());
    packet.extend_from_slice(&cast_inst_name);

    // Answer 2: PTR _display._tcp.local -> display_instance
    let disp_ptr = encode_dns_name(display_service);
    let disp_inst_name = encode_dns_name(display_instance);
    packet.extend_from_slice(&disp_ptr);
    packet.extend_from_slice(&12u16.to_be_bytes()); // TYPE: PTR
    packet.extend_from_slice(&0x0001u16.to_be_bytes()); // CLASS: IN
    packet.extend_from_slice(&120u32.to_be_bytes()); // TTL: 120s
    packet.extend_from_slice(&(disp_inst_name.len() as u16).to_be_bytes());
    packet.extend_from_slice(&disp_inst_name);

    // Answer 3: PTR _miracast._tcp.local -> wfd_instance
    let wfd_ptr = encode_dns_name(wfd_service);
    let wfd_inst_name = encode_dns_name(wfd_instance);
    packet.extend_from_slice(&wfd_ptr);
    packet.extend_from_slice(&12u16.to_be_bytes()); // TYPE: PTR
    packet.extend_from_slice(&0x0001u16.to_be_bytes()); // CLASS: IN
    packet.extend_from_slice(&120u32.to_be_bytes()); // TTL: 120s
    packet.extend_from_slice(&(wfd_inst_name.len() as u16).to_be_bytes());
    packet.extend_from_slice(&wfd_inst_name);

    // Additional 1: SRV for Google Cast (points to 8008 or 8080 on pi-zero.local)
    let host_encoded = encode_dns_name(host_name);
    packet.extend_from_slice(&cast_inst_name);
    packet.extend_from_slice(&33u16.to_be_bytes()); // TYPE: SRV
    packet.extend_from_slice(&0x8001u16.to_be_bytes()); // CLASS: IN (flush)
    packet.extend_from_slice(&120u32.to_be_bytes()); // TTL
    let srv_rdlen = 6 + host_encoded.len() as u16;
    packet.extend_from_slice(&srv_rdlen.to_be_bytes());
    packet.extend_from_slice(&0u16.to_be_bytes()); // Priority
    packet.extend_from_slice(&0u16.to_be_bytes()); // Weight
    packet.extend_from_slice(&8009u16.to_be_bytes()); // Cast V2 TLS Port 8009
    packet.extend_from_slice(&host_encoded);

    // Additional 2: TXT for Google Cast
    let cast_txt = encode_dns_txt(&[
        "id=bcm2835-ext-monitor-01",
        "cd=",
        "rm=",
        "ve=05",
        "md=Chromecast",
        "ic=/setup/icon.png",
        "fn=RaspCast",
        "ca=4101",
        "st=0",
        "bs=",
        "rs=",
        "nf=1",
    ]);
    packet.extend_from_slice(&cast_inst_name);
    packet.extend_from_slice(&16u16.to_be_bytes()); // TYPE: TXT
    packet.extend_from_slice(&0x8001u16.to_be_bytes()); // CLASS: IN (flush)
    packet.extend_from_slice(&120u32.to_be_bytes()); // TTL
    packet.extend_from_slice(&(cast_txt.len() as u16).to_be_bytes());
    packet.extend_from_slice(&cast_txt);

    // Additional 3: SRV for Miracast / WFD
    packet.extend_from_slice(&disp_inst_name);
    packet.extend_from_slice(&33u16.to_be_bytes()); // TYPE: SRV
    packet.extend_from_slice(&0x8001u16.to_be_bytes()); // CLASS: IN (flush)
    packet.extend_from_slice(&120u32.to_be_bytes()); // TTL
    packet.extend_from_slice(&srv_rdlen.to_be_bytes());
    packet.extend_from_slice(&0u16.to_be_bytes());
    packet.extend_from_slice(&0u16.to_be_bytes());
    packet.extend_from_slice(&miracast_port.to_be_bytes());
    packet.extend_from_slice(&host_encoded);

    // Additional 4: A Record for pi-zero.local
    packet.extend_from_slice(&host_encoded);
    packet.extend_from_slice(&1u16.to_be_bytes()); // TYPE: A
    packet.extend_from_slice(&0x8001u16.to_be_bytes()); // CLASS: IN (flush)
    packet.extend_from_slice(&120u32.to_be_bytes()); // TTL
    packet.extend_from_slice(&4u16.to_be_bytes()); // RDLENGTH: 4
    packet.extend_from_slice(&device_ip);

    packet
}

/// Spawns the background mDNS responder daemon on UDP port 5353
pub fn start_mdns_responder(running: Arc<AtomicBool>) {
    thread::Builder::new()
        .name("mdns-responder".to_string())
        .spawn(move || {
            // Ensure multicast route exists on Linux
            let _ = std::process::Command::new("route")
                .args(&["add", "-net", "224.0.0.0", "netmask", "240.0.0.0", "dev", "usb0"])
                .output();

            // Create socket with SO_REUSEADDR and SO_REUSEPORT via libc
            let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
            if fd < 0 {
                eprintln!("\x1b[1;33m[mdns]\x1b[0m Failed to create raw UDP socket for mDNS.");
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

                let loop_opt: libc::c_int = 0;
                libc::setsockopt(
                    fd,
                    libc::IPPROTO_IP,
                    libc::IP_MULTICAST_LOOP,
                    &loop_opt as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                );

                let mut addr: libc::sockaddr_in = std::mem::zeroed();
                addr.sin_family = libc::AF_INET as libc::sa_family_t;
                addr.sin_port = MDNS_PORT.to_be();
                addr.sin_addr.s_addr = libc::INADDR_ANY;

                if libc::bind(
                    fd,
                    &addr as *const _ as *const libc::sockaddr,
                    std::mem::size_of_val(&addr) as libc::socklen_t,
                ) != 0
                {
                    libc::close(fd);
                    eprintln!("\x1b[1;33m[mdns]\x1b[0m Failed to bind UDP 5353. Running without mDNS responder.");
                    return;
                }
            }

            let socket = unsafe { UdpSocket::from_raw_fd(fd) };
            let _ = socket.set_read_timeout(Some(Duration::from_millis(500)));

            let mcast_addr = Ipv4Addr::new(224, 0, 0, 251);
            let _ = socket.join_multicast_v4(&mcast_addr, &Ipv4Addr::new(192, 168, 7, 2));
            let _ = socket.join_multicast_v4(&mcast_addr, &Ipv4Addr::UNSPECIFIED);

            println!("\x1b[1;32m[mdns]\x1b[0m Pure-Rust mDNS Google Cast / Miracast responder active on 224.0.0.251:5353");

            let response_packet = build_full_mdns_response([192, 168, 7, 2], 8080, 7236);

            let announce_targets = [
                SocketAddr::from(([224, 0, 0, 251], MDNS_PORT)),
                SocketAddr::from(([192, 168, 7, 1], MDNS_PORT)),
            ];

            // Initial announcement burst (3 packets)
            for _ in 0..3 {
                for target in &announce_targets {
                    let _ = socket.send_to(&response_packet, target);
                }
                thread::sleep(Duration::from_millis(50));
            }

            let mut buf = [0u8; 2048];
            let mut last_periodic = std::time::Instant::now();

            while running.load(Ordering::Relaxed) {
                // Periodic announcement every 8 seconds
                if last_periodic.elapsed() >= Duration::from_secs(8) {
                    last_periodic = std::time::Instant::now();
                    for target in &announce_targets {
                        let _ = socket.send_to(&response_packet, target);
                    }
                }

                // Answer incoming queries
                if let Ok((len, src)) = socket.recv_from(&mut buf) {
                    if len >= 12 {
                        let flags = u16::from_be_bytes([buf[2], buf[3]]);
                        let is_query = (flags & 0x8000) == 0;
                        if !is_query {
                            continue; // Ignore DNS responses to prevent infinite multicast echo loop
                        }

                        let req_slice = &buf[..len];
                        let req_str = String::from_utf8_lossy(req_slice);
                        if req_str.contains("googlecast")
                            || req_str.contains("display")
                            || req_str.contains("miracast")
                            || req_str.contains("pi-zero")
                        {
                            let _ = socket.send_to(&response_packet, src);
                        }
                    }
                }
            }
        })
        .expect("Failed to spawn mdns responder thread");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_dns_name() {
        let encoded = encode_dns_name("_googlecast._tcp.local");
        assert_eq!(encoded[0], 11); // length of "_googlecast"
        assert_eq!(&encoded[1..12], b"_googlecast");
        assert_eq!(encoded[12], 4); // length of "_tcp"
        assert_eq!(&encoded[13..17], b"_tcp");
        assert_eq!(encoded[17], 5); // length of "local"
        assert_eq!(&encoded[18..23], b"local");
        assert_eq!(encoded[23], 0); // null terminator
    }

    #[test]
    fn test_encode_dns_txt() {
        let txts = ["fn=Ext-Monitor", "ve=05"];
        let encoded = encode_dns_txt(&txts);
        assert_eq!(encoded[0], 14);
        assert_eq!(&encoded[1..15], b"fn=Ext-Monitor");
        assert_eq!(encoded[15], 5);
        assert_eq!(&encoded[16..21], b"ve=05");
    }

    #[test]
    fn test_build_full_mdns_response() {
        let pkt = build_full_mdns_response([192, 168, 7, 2], 8080, 7236);
        assert!(pkt.len() > 100);
        // Flags: 0x8400
        assert_eq!(pkt[2], 0x84);
        assert_eq!(pkt[3], 0x00);
        // ANCOUNT = 3
        assert_eq!(pkt[6], 0x00);
        assert_eq!(pkt[7], 0x03);
        // ARCOUNT = 4
        assert_eq!(pkt[10], 0x00);
        assert_eq!(pkt[11], 0x04);
        // Contains device IP
        assert!(pkt.windows(4).any(|w| w == [192, 168, 7, 2]));
    }
}

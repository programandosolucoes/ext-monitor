//! Pure-Rust mDNS, SSDP, and Direct-USB Network Discovery Engine for Miracast / Wi-Fi Display
//!
//! Provides zero-dependency, pure-Rust network discovery for:
//! - Direct USB connections (e.g. Raspberry Pi Zero W at 192.168.7.2 on port 7236/7250)
//! - Multicast DNS (mDNS RFC 6762 / RFC 6763) for `_display._tcp.local` and `_airplay._tcp.local`
//! - Simple Service Discovery Protocol (SSDP / UPnP) for `urn:schemas-upnp-org:device:MediaRenderer:1`
//!
//! Complies with:
//! - Wi-Fi Display (WFD 1.0) / Miracast
//! - Microsoft Miracast over Infrastructure (MS-MICE)
//! - RFC 6762 (Multicast DNS)
//! - UPnP Device Architecture 1.0 (SSDP)

use std::io::{BufRead, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::time::Duration;

/// Standard direct USB IP address for Raspberry Pi Zero USB Ethernet Gadget
pub const DIRECT_USB_IP: &str = "192.168.7.2";

/// Standard RTSP port for Wi-Fi Display
pub const WFD_DEFAULT_PORT: u16 = 7236;

/// MS-MICE signaling port
pub const WFD_MICE_PORT: u16 = 7250;

/// Multicast address and port for mDNS (RFC 6762)
pub const MDNS_MULTICAST_ADDR: &str = "224.0.0.251:5353";

/// Multicast address and port for SSDP (UPnP)
pub const SSDP_MULTICAST_ADDR: &str = "239.255.255.250:1900";

/// Represents a discovered Miracast / Wi-Fi Display sink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredSink {
    pub name: String,
    pub ip: IpAddr,
    pub port: u16,
    pub protocol: &'static str, // "Direct-USB", "MS-MICE", "WFD/mDNS", "SSDP"
}

impl DiscoveredSink {
    pub fn new(name: impl Into<String>, ip: IpAddr, port: u16, protocol: &'static str) -> Self {
        Self {
            name: name.into(),
            ip,
            port,
            protocol,
        }
    }

    /// Returns protocol kind (compatible with plan specification).
    #[inline]
    pub fn kind(&self) -> &'static str {
        self.protocol
    }
}

impl std::fmt::Display for DiscoveredSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} - {}:{} ({})",
            self.name, self.ip, self.port, self.protocol
        )
    }
}

/// Builds a standard DNS query packet over UDP for the given service name (e.g. `_display._tcp.local`).
pub fn build_mdns_query(service_name: &str) -> Vec<u8> {
    build_mdns_query_with_qu(service_name, false)
}

/// Builds an mDNS query packet, optionally setting the unicast-response (QU) bit.
pub fn build_mdns_query_with_qu(service_name: &str, unicast_response: bool) -> Vec<u8> {
    let mut packet = Vec::with_capacity(64);

    // Header (12 bytes)
    // Transaction ID: 0x0000 (standard for mDNS queries)
    packet.extend_from_slice(&[0x00, 0x00]);
    // Flags: 0x0000 (Standard query, QR=0, Opcode=0, RD=0)
    packet.extend_from_slice(&[0x00, 0x00]);
    // Questions: 1 (QDCOUNT = 0x0001)
    packet.extend_from_slice(&[0x00, 0x01]);
    // Answer RRs: 0
    packet.extend_from_slice(&[0x00, 0x00]);
    // Authority RRs: 0
    packet.extend_from_slice(&[0x00, 0x00]);
    // Additional RRs: 0
    packet.extend_from_slice(&[0x00, 0x00]);

    // Question Section: QNAME (series of length-prefixed labels terminated with 0x00)
    for label in service_name.split('.') {
        if !label.is_empty() {
            let bytes = label.as_bytes();
            packet.push(bytes.len() as u8);
            packet.extend_from_slice(bytes);
        }
    }
    packet.push(0x00); // Root label null terminator

    // QTYPE: PTR (12 = 0x000C)
    packet.extend_from_slice(&12u16.to_be_bytes());

    // QCLASS: IN (1 = 0x0001) or QU (0x8001 if unicast response requested)
    let qclass: u16 = if unicast_response { 0x8001 } else { 0x0001 };
    packet.extend_from_slice(&qclass.to_be_bytes());

    packet
}

/// Helper to parse DNS names with pointer compression (0xC0..) from a packet buffer.
/// Returns `Some((name, next_offset))` where `next_offset` is the offset in `buf`
/// directly after the parsed name field (or pointer) if not jumped, or after the root pointer.
pub fn parse_dns_name(buf: &[u8], mut offset: usize) -> Option<(String, usize)> {
    let mut name = String::new();
    let mut jumped = false;
    let mut return_offset = None;
    let mut hops = 0;

    while offset < buf.len() {
        if hops > 40 {
            return None; // Loop detection
        }
        let len = buf[offset] as usize;
        if len == 0 {
            if !jumped {
                return_offset = Some(offset + 1);
            }
            break;
        } else if (len & 0xC0) == 0xC0 {
            // 14-bit compression pointer
            if offset + 1 >= buf.len() {
                return None;
            }
            let ptr = (((len & 0x3F) << 8) | (buf[offset + 1] as usize)) as usize;
            if !jumped {
                return_offset = Some(offset + 2);
                jumped = true;
            }
            offset = ptr;
            hops += 1;
        } else {
            // Label
            offset += 1;
            if offset + len > buf.len() {
                return None;
            }
            if !name.is_empty() {
                name.push('.');
            }
            if let Ok(label) = std::str::from_utf8(&buf[offset..offset + len]) {
                name.push_str(label);
            }
            offset += len;
            hops += 1;
        }
    }

    return_offset.map(|next_off| (name, next_off))
}

/// Extracts a clean device friendly name from an mDNS service domain name.
/// e.g. "Living Room TV._display._tcp.local" -> "Living Room TV"
fn extract_friendly_name(full_name: &str) -> String {
    if let Some(pos) = full_name.find("._") {
        full_name[..pos].to_string()
    } else if let Some(pos) = full_name.find('.') {
        full_name[..pos].to_string()
    } else {
        full_name.to_string()
    }
}

/// Parses key-value pairs in a DNS TXT record for friendly name (`fn=` or `name=`).
fn parse_txt_friendly_name(rdata: &[u8]) -> Option<String> {
    let mut off = 0;
    while off < rdata.len() {
        let len = rdata[off] as usize;
        off += 1;
        if off + len > rdata.len() {
            break;
        }
        if let Ok(entry) = std::str::from_utf8(&rdata[off..off + len]) {
            if let Some(name) = entry.strip_prefix("fn=") {
                return Some(name.to_string());
            } else if let Some(name) = entry.strip_prefix("name=") {
                return Some(name.to_string());
            } else if let Some(name) = entry.strip_prefix("md=") {
                return Some(name.to_string());
            }
        }
        off += len;
    }
    None
}

/// Parses an mDNS response packet into a `DiscoveredSink` if relevant.
pub fn parse_mdns_response(buf: &[u8], sender_ip: IpAddr) -> Option<DiscoveredSink> {
    if buf.len() < 12 {
        return None;
    }

    let flags = u16::from_be_bytes([buf[2], buf[3]]);
    // Ensure it's a response packet (QR bit set: 0x8000)
    let is_response = (flags & 0x8000) != 0;
    if !is_response {
        return None;
    }

    let qdcount = u16::from_be_bytes([buf[4], buf[5]]) as usize;
    let ancount = u16::from_be_bytes([buf[6], buf[7]]) as usize;
    let nscount = u16::from_be_bytes([buf[8], buf[9]]) as usize;
    let arcount = u16::from_be_bytes([buf[10], buf[11]]) as usize;

    let mut offset = 12;

    // Skip question section
    for _ in 0..qdcount {
        if let Some((_, next_off)) = parse_dns_name(buf, offset) {
            offset = next_off + 4; // Skip QTYPE (2) and QCLASS (2)
        } else {
            return None;
        }
    }

    let mut friendly_name: Option<String> = None;
    let mut srv_port: Option<u16> = None;
    let mut a_ip: Option<IpAddr> = None;
    let total_records = ancount + nscount + arcount;

    for _ in 0..total_records {
        if offset >= buf.len() {
            break;
        }
        let (_rec_name, next_off) = match parse_dns_name(buf, offset) {
            Some(res) => res,
            None => break,
        };
        offset = next_off;
        if offset + 10 > buf.len() {
            break;
        }

        let rtype = u16::from_be_bytes([buf[offset], buf[offset + 1]]);
        let _rclass = u16::from_be_bytes([buf[offset + 2], buf[offset + 3]]);
        let _ttl = u32::from_be_bytes([
            buf[offset + 4],
            buf[offset + 5],
            buf[offset + 6],
            buf[offset + 7],
        ]);
        let rdlength = u16::from_be_bytes([buf[offset + 8], buf[offset + 9]]) as usize;
        offset += 10;

        if offset + rdlength > buf.len() {
            break;
        }
        let rdata_offset = offset;
        let rdata = &buf[offset..offset + rdlength];
        offset += rdlength;

        match rtype {
            12 => {
                // PTR: pointer to service instance name
                if let Some((target, _)) = parse_dns_name(buf, rdata_offset) {
                    if friendly_name.is_none() {
                        friendly_name = Some(extract_friendly_name(&target));
                    }
                }
            }
            33 => {
                // SRV: Priority(2), Weight(2), Port(2), Target(...)
                if rdata.len() >= 6 {
                    let port = u16::from_be_bytes([rdata[4], rdata[5]]);
                    srv_port = Some(port);
                    if friendly_name.is_none() {
                        if let Some((target, _)) = parse_dns_name(buf, rdata_offset + 6) {
                            friendly_name = Some(extract_friendly_name(&target));
                        }
                    }
                }
            }
            1 => {
                // A: 4-byte IPv4 address
                if rdata.len() == 4 {
                    a_ip = Some(IpAddr::V4(Ipv4Addr::new(
                        rdata[0], rdata[1], rdata[2], rdata[3],
                    )));
                }
            }
            16 => {
                // TXT
                if friendly_name.is_none() {
                    if let Some(txt_name) = parse_txt_friendly_name(rdata) {
                        friendly_name = Some(txt_name);
                    }
                }
            }
            _ => {}
        }
    }

    if friendly_name.is_some() || srv_port.is_some() || a_ip.is_some() {
        let ip = a_ip.unwrap_or(sender_ip);
        let port = srv_port.unwrap_or(WFD_DEFAULT_PORT);
        let name = friendly_name.unwrap_or_else(|| format!("Miracast Sink ({})", ip));
        Some(DiscoveredSink {
            name,
            ip,
            port,
            protocol: "WFD/mDNS",
        })
    } else {
        None
    }
}

/// Builds an SSDP M-SEARCH query packet for the given Search Target (ST).
pub fn build_ssdp_msearch(search_target: &str) -> String {
    format!(
        "M-SEARCH * HTTP/1.1\r\n\
        HOST: 239.255.255.250:1900\r\n\
        MAN: \"ssdp:discover\"\r\n\
        MX: 2\r\n\
        ST: {}\r\n\
        \r\n",
        search_target
    )
}

/// Parses an IP address from a URL string (e.g. `http://192.168.1.150:8080/description.xml`).
pub fn parse_ip_from_url(url: &str) -> Option<IpAddr> {
    let without_scheme = if let Some(pos) = url.find("://") {
        &url[pos + 3..]
    } else {
        url
    };

    let host_port = without_scheme.split('/').next().unwrap_or(without_scheme);

    // Bracketed IPv6 address [fe80::1]:8080
    if host_port.starts_with('[') {
        if let Some(end_bracket) = host_port.find(']') {
            let ipv6_str = &host_port[1..end_bracket];
            return ipv6_str.parse::<IpAddr>().ok();
        }
    }

    // IPv4 with optional port: 192.168.1.150:8080
    let host = host_port.split(':').next().unwrap_or(host_port);
    host.parse::<IpAddr>().ok()
}

/// Parses an SSDP HTTP response into a `DiscoveredSink`.
pub fn parse_ssdp_response(response: &str, sender_ip: IpAddr) -> Option<DiscoveredSink> {
    let mut location = None;
    let mut server = None;
    let mut usn = None;
    let mut custom_name = None;
    let mut custom_port = None;

    let mut first_line = true;
    let mut is_ok = false;

    for line in response.split("\r\n") {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if first_line {
            first_line = false;
            if trimmed.contains("200 OK") || trimmed.starts_with("NOTIFY") {
                is_ok = true;
            }
            continue;
        }
        let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
        if parts.len() == 2 {
            let key = parts[0].trim().to_uppercase();
            let value = parts[1].trim();
            match key.as_str() {
                "LOCATION" => location = Some(value.to_string()),
                "SERVER" => server = Some(value.to_string()),
                "USN" => usn = Some(value.to_string()),
                "FRIENDLYNAME" | "FRIENDLY-NAME" | "X-FRIENDLY-NAME" => {
                    custom_name = Some(value.to_string());
                }
                "WFD-PORT" | "RTSP-PORT" => {
                    if let Ok(p) = value.parse::<u16>() {
                        custom_port = Some(p);
                    }
                }
                _ => {}
            }
        }
    }

    if !is_ok && location.is_none() {
        return None;
    }

    let ip = location
        .as_deref()
        .and_then(parse_ip_from_url)
        .unwrap_or(sender_ip);

    let port = custom_port.unwrap_or(WFD_DEFAULT_PORT);

    let name = if let Some(n) = custom_name {
        n
    } else if let Some(srv) = server {
        if srv.contains("MediaRenderer") {
            format!("MediaRenderer ({})", ip)
        } else {
            srv
        }
    } else if let Some(_u) = usn {
        format!("SSDP Device ({})", ip)
    } else {
        format!("UPnP Sink ({})", ip)
    };

    Some(DiscoveredSink {
        name,
        ip,
        port,
        protocol: "SSDP",
    })
}

/// Probes direct USB connection to `192.168.7.2` on port 7236 and 7250.
/// Quick non-blocking connect with timeout (typically 50ms).
pub fn probe_direct_usb(timeout: Duration) -> Vec<DiscoveredSink> {
    let mut sinks = Vec::new();
    let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 7, 2));

    // 1. Probe RTSP port 7236 (Standard Miracast WFD)
    let addr_7236 = SocketAddr::new(ip, WFD_DEFAULT_PORT);
    if TcpStream::connect_timeout(&addr_7236, timeout).is_ok() {
        sinks.push(DiscoveredSink {
            name: "ExtMonitor-Pi0 (192.168.7.2)".to_string(),
            ip,
            port: WFD_DEFAULT_PORT,
            protocol: "Direct-USB",
        });
    }

    // 2. Probe MS-MICE port 7250 (Miracast over Infrastructure)
    let addr_7250 = SocketAddr::new(ip, WFD_MICE_PORT);
    if TcpStream::connect_timeout(&addr_7250, timeout).is_ok() {
        sinks.push(DiscoveredSink {
            name: "ExtMonitor-Pi0 (192.168.7.2)".to_string(),
            ip,
            port: WFD_MICE_PORT,
            protocol: "MS-MICE",
        });
    }

    sinks
}

/// Protocol priority ranking for deduplication. Higher value wins.
fn protocol_priority(proto: &str) -> u8 {
    match proto {
        "Direct-USB" => 4,
        "MS-MICE" => 3,
        "WFD/mDNS" => 2,
        "SSDP" => 1,
        _ => 0,
    }
}

/// Deduplicates sinks by `(ip, port)`, prioritizing Direct-USB > MS-MICE > WFD/mDNS > SSDP.
pub fn deduplicate_sinks(sinks: Vec<DiscoveredSink>) -> Vec<DiscoveredSink> {
    let mut result: Vec<DiscoveredSink> = Vec::new();
    for sink in sinks {
        if let Some(existing) = result
            .iter_mut()
            .find(|s| s.ip == sink.ip && s.port == sink.port)
        {
            if protocol_priority(sink.protocol) > protocol_priority(existing.protocol) {
                *existing = sink;
            }
        } else {
            result.push(sink);
        }
    }
    result
}

/// Runs multicast mDNS and SSDP discovery over UDP within the given timeout.
fn scan_multicast_sinks(timeout: Duration) -> Vec<DiscoveredSink> {
    let mut sinks = Vec::new();
    let start = std::time::Instant::now();

    // Create ephemeral UDP socket for mDNS
    let mdns_socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => {
            s.set_broadcast(true).ok();
            s.set_multicast_ttl_v4(4).ok();
            s.set_nonblocking(true).ok();
            s.join_multicast_v4(
                &Ipv4Addr::new(224, 0, 0, 251),
                &Ipv4Addr::UNSPECIFIED,
            )
            .ok();
            Some(s)
        }
        Err(_) => None,
    };

    // Create ephemeral UDP socket for SSDP
    let ssdp_socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => {
            s.set_broadcast(true).ok();
            s.set_multicast_ttl_v4(4).ok();
            s.set_nonblocking(true).ok();
            s.join_multicast_v4(
                &Ipv4Addr::new(239, 255, 255, 250),
                &Ipv4Addr::UNSPECIFIED,
            )
            .ok();
            Some(s)
        }
        Err(_) => None,
    };

    // Send mDNS queries
    if let Some(ref sock) = mdns_socket {
        if let Ok(mdns_dest) = MDNS_MULTICAST_ADDR.parse::<SocketAddr>() {
            let q1 = build_mdns_query("_display._tcp.local");
            sock.send_to(&q1, mdns_dest).ok();
            let q1_qu = build_mdns_query_with_qu("_display._tcp.local", true);
            sock.send_to(&q1_qu, mdns_dest).ok();

            let q2 = build_mdns_query("_airplay._tcp.local");
            sock.send_to(&q2, mdns_dest).ok();
            let q2_qu = build_mdns_query_with_qu("_airplay._tcp.local", true);
            sock.send_to(&q2_qu, mdns_dest).ok();
        }
    }

    // Send SSDP M-SEARCH
    if let Some(ref sock) = ssdp_socket {
        if let Ok(ssdp_dest) = SSDP_MULTICAST_ADDR.parse::<SocketAddr>() {
            let m1 = build_ssdp_msearch("urn:schemas-upnp-org:device:MediaRenderer:1");
            sock.send_to(m1.as_bytes(), ssdp_dest).ok();
            let m2 = build_ssdp_msearch("ssdp:all");
            sock.send_to(m2.as_bytes(), ssdp_dest).ok();
        }
    }

    let mut mdns_buf = [0u8; 4096];
    let mut ssdp_buf = [0u8; 4096];

    // Poll sockets until timeout expires
    while start.elapsed() < timeout {
        let mut received_any = false;

        if let Some(ref sock) = mdns_socket {
            while let Ok((len, src)) = sock.recv_from(&mut mdns_buf) {
                received_any = true;
                if let Some(sink) = parse_mdns_response(&mdns_buf[..len], src.ip()) {
                    sinks.push(sink);
                }
            }
        }

        if let Some(ref sock) = ssdp_socket {
            while let Ok((len, src)) = sock.recv_from(&mut ssdp_buf) {
                received_any = true;
                if let Ok(msg) = std::str::from_utf8(&ssdp_buf[..len]) {
                    if let Some(sink) = parse_ssdp_response(msg, src.ip()) {
                        sinks.push(sink);
                    }
                }
            }
        }

        if !received_any {
            std::thread::sleep(Duration::from_millis(15));
        }
    }

    sinks
}

/// Scans for Miracast / Wi-Fi Display sinks across Direct-USB, mDNS, and SSDP.
/// Returns deduplicated sinks.
pub fn scan_sinks(timeout: Duration) -> Vec<DiscoveredSink> {
    let mut sinks = Vec::new();

    // 1. Direct USB probe runs concurrently with network multicast discovery
    let usb_thread = std::thread::spawn(|| {
        probe_direct_usb(Duration::from_millis(50))
    });

    // 2. Multicast mDNS & SSDP probes
    let mut net_sinks = scan_multicast_sinks(timeout);
    sinks.append(&mut net_sinks);

    // 3. Collect Direct USB probe results
    if let Ok(mut usb_sinks) = usb_thread.join() {
        sinks.append(&mut usb_sinks);
    }

    // 4. Deduplicate sinks by (ip, port)
    deduplicate_sinks(sinks)
}

/// Prompts the user to interactively choose a discovered sink from stdout/stdin.
pub fn interactive_select_sink(timeout: Duration) -> Option<DiscoveredSink> {
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout();
    interactive_select_sink_with_io(timeout, &mut stdin, &mut stdout)
}

/// Testable interactive selector accepting arbitrary reader and writer streams.
pub fn interactive_select_sink_with_io<R: BufRead, W: Write>(
    timeout: Duration,
    reader: &mut R,
    writer: &mut W,
) -> Option<DiscoveredSink> {
    writeln!(
        writer,
        "\nScanning for Miracast / Wi-Fi Display sinks (timeout {:?})...",
        timeout
    )
    .ok();
    let sinks = scan_sinks(timeout);
    select_sink_from_list(&sinks, reader, writer)
}

/// Selects a sink from a pre-discovered slice of `DiscoveredSink` via user input.
pub fn select_sink_from_list<R: BufRead, W: Write>(
    sinks: &[DiscoveredSink],
    reader: &mut R,
    writer: &mut W,
) -> Option<DiscoveredSink> {
    if sinks.is_empty() {
        writeln!(writer, "No Miracast / Wi-Fi Display sinks discovered.").ok();
        return None;
    }

    writeln!(writer, "\nDiscovered {} sink(s):", sinks.len()).ok();
    for (i, sink) in sinks.iter().enumerate() {
        writeln!(
            writer,
            "  [{}] {} - {}:{} ({})",
            i + 1,
            sink.name,
            sink.ip,
            sink.port,
            sink.protocol
        )
        .ok();
    }
    write!(writer, "\nEnter selection [1-{}] or 'q' to quit: ", sinks.len()).ok();
    writer.flush().ok();

    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return None;
    }

    let input = line.trim();
    if input.eq_ignore_ascii_case("q") || input.eq_ignore_ascii_case("quit") {
        return None;
    }

    if let Ok(choice) = input.parse::<usize>() {
        if choice >= 1 && choice <= sinks.len() {
            return Some(sinks[choice - 1].clone());
        }
    }

    writeln!(writer, "Invalid selection.").ok();
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mdns_query_packet() {
        let packet = build_mdns_query("_display._tcp.local");
        assert!(packet.len() > 12);
        assert_eq!(packet[2], 0x00); // Standard query
        assert_eq!(packet[5], 0x01); // 1 Question
    }

    #[test]
    fn test_mdns_query_packet_structure() {
        let packet = build_mdns_query("_display._tcp.local");
        assert!(packet.len() > 12);

        // Header check
        assert_eq!(packet[0], 0x00); // ID
        assert_eq!(packet[1], 0x00);
        assert_eq!(packet[2], 0x00); // Flags standard query
        assert_eq!(packet[3], 0x00);
        assert_eq!(packet[4], 0x00); // QDCOUNT = 1
        assert_eq!(packet[5], 0x01);
        assert_eq!(packet[6], 0x00); // ANCOUNT = 0
        assert_eq!(packet[7], 0x00);

        // Check QNAME labels
        // Label 1: "_display" (len 8)
        assert_eq!(packet[12], 8);
        assert_eq!(&packet[13..21], b"_display");

        // Label 2: "_tcp" (len 4)
        assert_eq!(packet[21], 4);
        assert_eq!(&packet[22..26], b"_tcp");

        // Label 3: "local" (len 5)
        assert_eq!(packet[26], 5);
        assert_eq!(&packet[27..32], b"local");

        // Null terminator
        assert_eq!(packet[32], 0x00);

        // QTYPE = PTR (12)
        assert_eq!(packet[33], 0x00);
        assert_eq!(packet[34], 12);

        // QCLASS = IN (1)
        assert_eq!(packet[35], 0x00);
        assert_eq!(packet[36], 1);

        // Check unicast variant sets top bit
        let qu_packet = build_mdns_query_with_qu("_display._tcp.local", true);
        assert_eq!(qu_packet[35], 0x80);
        assert_eq!(qu_packet[36], 1);
    }

    #[test]
    fn test_ssdp_msearch_format() {
        let msg = build_ssdp_msearch("urn:schemas-upnp-org:device:MediaRenderer:1");
        assert!(msg.starts_with("M-SEARCH * HTTP/1.1\r\n"));
        assert!(msg.contains("HOST: 239.255.255.250:1900\r\n"));
        assert!(msg.contains("MAN: \"ssdp:discover\"\r\n"));
        assert!(msg.contains("MX: 2\r\n"));
        assert!(msg.contains("ST: urn:schemas-upnp-org:device:MediaRenderer:1\r\n"));
        assert!(msg.ends_with("\r\n\r\n"));
    }

    #[test]
    fn test_ssdp_response_parser() {
        let response = "HTTP/1.1 200 OK\r\n\
            CACHE-CONTROL: max-age=1800\r\n\
            LOCATION: http://192.168.1.150:8080/description.xml\r\n\
            SERVER: Linux/4.14 UPnP/1.0 MediaRenderer/1.0\r\n\
            ST: urn:schemas-upnp-org:device:MediaRenderer:1\r\n\
            USN: uuid:12345678-1234-1234-1234-123456789abc::urn:schemas-upnp-org:device:MediaRenderer:1\r\n\
            \r\n";

        let fallback_ip: IpAddr = "10.0.0.1".parse().unwrap();
        let sink = parse_ssdp_response(response, fallback_ip).expect("Should parse SSDP response");

        assert_eq!(sink.ip, "192.168.1.150".parse::<IpAddr>().unwrap());
        assert_eq!(sink.port, 7236);
        assert_eq!(sink.protocol, "SSDP");
        assert!(sink.name.contains("MediaRenderer"));
    }

    #[test]
    fn test_ssdp_response_parser_fallback_and_invalid() {
        // Response without LOCATION should fallback to sender_ip
        let response = "HTTP/1.1 200 OK\r\n\
            SERVER: CustomTV/2.0\r\n\
            \r\n";
        let sender_ip: IpAddr = "192.168.1.88".parse().unwrap();
        let sink = parse_ssdp_response(response, sender_ip).expect("Should parse with fallback IP");
        assert_eq!(sink.ip, sender_ip);
        assert_eq!(sink.name, "CustomTV/2.0");

        // Non-200 and no LOCATION should return None
        let bad_response = "HTTP/1.1 404 Not Found\r\n\r\n";
        assert!(parse_ssdp_response(bad_response, sender_ip).is_none());
    }

    #[test]
    fn test_sink_deduplication() {
        let ip_pi: IpAddr = "192.168.7.2".parse().unwrap();
        let ip_tv: IpAddr = "192.168.1.100".parse().unwrap();

        let raw_sinks = vec![
            DiscoveredSink {
                name: "Living Room TV (SSDP)".to_string(),
                ip: ip_tv,
                port: 7236,
                protocol: "SSDP",
            },
            DiscoveredSink {
                name: "Living Room TV".to_string(),
                ip: ip_tv,
                port: 7236,
                protocol: "WFD/mDNS",
            },
            DiscoveredSink {
                name: "Pi0-mDNS".to_string(),
                ip: ip_pi,
                port: 7236,
                protocol: "WFD/mDNS",
            },
            DiscoveredSink {
                name: "ExtMonitor-Pi0 (192.168.7.2)".to_string(),
                ip: ip_pi,
                port: 7236,
                protocol: "Direct-USB",
            },
            DiscoveredSink {
                name: "ExtMonitor-Pi0 (192.168.7.2)".to_string(),
                ip: ip_pi,
                port: 7250,
                protocol: "MS-MICE",
            },
        ];

        let deduped = deduplicate_sinks(raw_sinks);

        // Expected 3 distinct (ip, port) pairs:
        // (ip_tv, 7236) -> WFD/mDNS beats SSDP
        // (ip_pi, 7236) -> Direct-USB beats WFD/mDNS
        // (ip_pi, 7250) -> MS-MICE
        assert_eq!(deduped.len(), 3);

        let tv_sink = deduped.iter().find(|s| s.ip == ip_tv && s.port == 7236).unwrap();
        assert_eq!(tv_sink.protocol, "WFD/mDNS");

        let pi_7236 = deduped.iter().find(|s| s.ip == ip_pi && s.port == 7236).unwrap();
        assert_eq!(pi_7236.protocol, "Direct-USB");

        let pi_7250 = deduped.iter().find(|s| s.ip == ip_pi && s.port == 7250).unwrap();
        assert_eq!(pi_7250.protocol, "MS-MICE");
    }

    #[test]
    fn test_parse_ip_from_url_variations() {
        assert_eq!(
            parse_ip_from_url("http://192.168.1.150:8080/description.xml"),
            Some("192.168.1.150".parse().unwrap())
        );
        assert_eq!(
            parse_ip_from_url("http://10.0.0.1/"),
            Some("10.0.0.1".parse().unwrap())
        );
        assert_eq!(
            parse_ip_from_url("https://[fe80::1]:8080/desc"),
            Some("fe80::1".parse().unwrap())
        );
        assert_eq!(parse_ip_from_url("invalid_url"), None);
    }

    #[test]
    fn test_mdns_response_parser() {
        // Construct synthetic mDNS response packet
        let mut pkt = Vec::new();
        // Header
        pkt.extend_from_slice(&[0x00, 0x00]); // ID
        pkt.extend_from_slice(&[0x84, 0x00]); // Flags (QR=1 response, Authoritative)
        pkt.extend_from_slice(&[0x00, 0x00]); // QDCOUNT
        pkt.extend_from_slice(&[0x00, 0x03]); // ANCOUNT = 3
        pkt.extend_from_slice(&[0x00, 0x00]); // NSCOUNT
        pkt.extend_from_slice(&[0x00, 0x00]); // ARCOUNT

        // Answer 1: PTR for _display._tcp.local -> Living Room TV._display._tcp.local
        for part in ["_display", "_tcp", "local"] {
            pkt.push(part.len() as u8);
            pkt.extend_from_slice(part.as_bytes());
        }
        pkt.push(0x00);
        pkt.extend_from_slice(&12u16.to_be_bytes()); // TYPE PTR
        pkt.extend_from_slice(&1u16.to_be_bytes());  // CLASS IN
        pkt.extend_from_slice(&120u32.to_be_bytes()); // TTL

        let mut rdata_ptr = Vec::new();
        for part in ["Living Room TV", "_display", "_tcp", "local"] {
            rdata_ptr.push(part.len() as u8);
            rdata_ptr.extend_from_slice(part.as_bytes());
        }
        rdata_ptr.push(0x00);
        pkt.extend_from_slice(&(rdata_ptr.len() as u16).to_be_bytes());
        pkt.extend_from_slice(&rdata_ptr);

        // Answer 2: SRV for Living Room TV... -> Port 7236
        for part in ["Living Room TV", "_display", "_tcp", "local"] {
            pkt.push(part.len() as u8);
            pkt.extend_from_slice(part.as_bytes());
        }
        pkt.push(0x00);
        pkt.extend_from_slice(&33u16.to_be_bytes()); // TYPE SRV
        pkt.extend_from_slice(&1u16.to_be_bytes());  // CLASS IN
        pkt.extend_from_slice(&120u32.to_be_bytes()); // TTL

        let mut rdata_srv = Vec::new();
        rdata_srv.extend_from_slice(&0u16.to_be_bytes()); // Priority
        rdata_srv.extend_from_slice(&0u16.to_be_bytes()); // Weight
        rdata_srv.extend_from_slice(&7236u16.to_be_bytes()); // Port
        for part in ["pi0", "local"] {
            rdata_srv.push(part.len() as u8);
            rdata_srv.extend_from_slice(part.as_bytes());
        }
        rdata_srv.push(0x00);
        pkt.extend_from_slice(&(rdata_srv.len() as u16).to_be_bytes());
        pkt.extend_from_slice(&rdata_srv);

        // Answer 3: A for pi0.local -> 192.168.1.99
        for part in ["pi0", "local"] {
            pkt.push(part.len() as u8);
            pkt.extend_from_slice(part.as_bytes());
        }
        pkt.push(0x00);
        pkt.extend_from_slice(&1u16.to_be_bytes()); // TYPE A
        pkt.extend_from_slice(&1u16.to_be_bytes()); // CLASS IN
        pkt.extend_from_slice(&120u32.to_be_bytes()); // TTL
        pkt.extend_from_slice(&4u16.to_be_bytes()); // RDLENGTH 4
        pkt.extend_from_slice(&[192, 168, 1, 99]);

        let sender_ip: IpAddr = "192.168.1.1".parse().unwrap();
        let sink = parse_mdns_response(&pkt, sender_ip).expect("Should parse synthetic mDNS response");

        assert_eq!(sink.name, "Living Room TV");
        assert_eq!(sink.ip, "192.168.1.99".parse::<IpAddr>().unwrap());
        assert_eq!(sink.port, 7236);
        assert_eq!(sink.protocol, "WFD/mDNS");
    }

    #[test]
    fn test_select_sink_from_list_interactive() {
        let sinks = vec![
            DiscoveredSink {
                name: "ExtMonitor-Pi0 (192.168.7.2)".to_string(),
                ip: "192.168.7.2".parse().unwrap(),
                port: 7236,
                protocol: "Direct-USB",
            },
            DiscoveredSink {
                name: "Living Room TV".to_string(),
                ip: "192.168.1.100".parse().unwrap(),
                port: 7236,
                protocol: "WFD/mDNS",
            },
        ];

        // Test valid selection "1"
        let mut input = std::io::Cursor::new(b"1\n");
        let mut output = Vec::new();
        let selected = select_sink_from_list(&sinks, &mut input, &mut output);
        assert_eq!(selected, Some(sinks[0].clone()));

        // Test valid selection "2"
        let mut input = std::io::Cursor::new(b"2\n");
        let mut output = Vec::new();
        let selected = select_sink_from_list(&sinks, &mut input, &mut output);
        assert_eq!(selected, Some(sinks[1].clone()));

        // Test quit 'q'
        let mut input = std::io::Cursor::new(b"q\n");
        let mut output = Vec::new();
        let selected = select_sink_from_list(&sinks, &mut input, &mut output);
        assert_eq!(selected, None);

        // Test invalid number
        let mut input = std::io::Cursor::new(b"99\n");
        let mut output = Vec::new();
        let selected = select_sink_from_list(&sinks, &mut input, &mut output);
        assert_eq!(selected, None);

        // Test empty list
        let empty_sinks: Vec<DiscoveredSink> = Vec::new();
        let mut input = std::io::Cursor::new(b"1\n");
        let mut output = Vec::new();
        let selected = select_sink_from_list(&empty_sinks, &mut input, &mut output);
        assert_eq!(selected, None);
    }
}

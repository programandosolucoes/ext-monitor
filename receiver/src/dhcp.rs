//! Embedded Pure-Rust Zero-Gateway DHCP Server for USB OTG (usb0)
//!
//! Exclusively serves the host PC over the USB point-to-point link.
//! Assigns 192.168.7.1 / 255.255.255.0 to the connected computer.
//! Intentionally omits the default gateway (Option 3) to keep the host's
//! primary internet (Wi-Fi/Ethernet) fully active.
//!
//! Uses SO_BINDTODEVICE to strictly isolate DHCP packets to 'usb0'.
//! Eliminates external BusyBox udhcpd, lease files, and external daemons.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

pub const SERVER_IP: [u8; 4] = [192, 168, 7, 2];
pub const HOST_IP: [u8; 4] = [192, 168, 7, 1];
pub const SUBNET_MASK: [u8; 4] = [255, 255, 255, 0];
pub const LEASE_TIME_SECS: u32 = 7200; // 2 hours

const DHCP_MAGIC_COOKIE: [u8; 4] = [99, 130, 83, 99];

const DHCP_DISCOVER: u8 = 1;
const DHCP_OFFER: u8 = 2;
const DHCP_REQUEST: u8 = 3;
const DHCP_ACK: u8 = 5;

#[derive(Clone, Debug)]
pub struct DhcpLease {
    pub mac: String,
    pub ip: String,
    pub timestamp: Instant,
}

static ACTIVE_LEASE: RwLock<Option<DhcpLease>> = RwLock::new(None);

/// Return the most recent DHCP lease assigned to the host PC
pub fn get_active_lease() -> Option<DhcpLease> {
    ACTIVE_LEASE.read().ok()?.clone()
}

/// Start the embedded DHCP server in a background thread
pub fn start_dhcp_server(running: Arc<AtomicBool>) {
    thread::spawn(move || {
        println!("\x1b[1;32m[dhcp-server]\x1b[0m Initializing native pure-Rust DHCP server for usb0...");

        // Retry binding until usb0 interface is available or running is false
        let socket = loop {
            if !running.load(Ordering::SeqCst) {
                return;
            }
            match UdpSocket::bind("0.0.0.0:67") {
                Ok(s) => {
                    let _ = s.set_broadcast(true);
                    let _ = s.set_read_timeout(Some(Duration::from_millis(500)));

                    // Bind to usb0 interface specifically
                    if bind_to_device(&s, "usb0").is_ok() {
                        println!("\x1b[1;32m[dhcp-server]\x1b[0m Successfully bound to usb0 device (Zero-Gateway active)");
                    } else {
                        // In dev environment or before usb0 appears, log and retry
                        println!("\x1b[1;33m[dhcp-server]\x1b[0m usb0 not yet ready, waiting...");
                        thread::sleep(Duration::from_millis(1000));
                        continue;
                    }
                    break s;
                }
                Err(e) => {
                    // Could be port in use (e.g. running udhcpd) or permission error in test
                    eprintln!("\x1b[1;33m[dhcp-server]\x1b[0m Cannot bind port 67 ({}), retrying in 2s...", e);
                    thread::sleep(Duration::from_millis(2000));
                }
            }
        };

        println!(
            "\x1b[1;32m[dhcp-server]\x1b[0m Native DHCP Server online: Server 192.168.7.2 -> Host 192.168.7.1 (Zero-Gateway)"
        );

        let mut buf = [0u8; 1500];
        while running.load(Ordering::SeqCst) {
            match socket.recv_from(&mut buf) {
                Ok((len, _src)) => {
                    handle_dhcp_packet(&socket, &buf, len);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                    // Read timeout, loop around to check running flag
                    continue;
                }
                Err(e) => {
                    eprintln!("\x1b[1;31m[dhcp-server]\x1b[0m Socket recv error: {}", e);
                    thread::sleep(Duration::from_millis(500));
                }
            }
        }

        println!("\x1b[1;33m[dhcp-server]\x1b[0m Native DHCP server stopped.");
    });
}

fn bind_to_device(socket: &UdpSocket, iface_name: &str) -> std::io::Result<()> {
    let mut iface_bytes = iface_name.as_bytes().to_vec();
    iface_bytes.push(0);
    let fd = socket.as_raw_fd();
    let ret = unsafe {
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_BINDTODEVICE,
            iface_bytes.as_ptr() as *const libc::c_void,
            iface_bytes.len() as libc::socklen_t,
        )
    };
    if ret != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn handle_dhcp_packet(socket: &UdpSocket, buf: &[u8], len: usize) {
    if len < 240 {
        return;
    }
    // op == 1 (BOOTREQUEST)
    if buf[0] != 1 {
        return;
    }
    // Check magic cookie at 236..240
    if buf[236..240] != DHCP_MAGIC_COOKIE {
        return;
    }

    let xid = &buf[4..8];
    let hlen = buf[2] as usize;
    if hlen == 0 || hlen > 16 {
        return;
    }
    let client_mac_bytes = &buf[28..28 + hlen.min(6)];
    let mac_str = client_mac_bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>()
        .join(":");

    // Find Option 53 (DHCP Message Type)
    let mut msg_type = None;
    let mut idx = 240;
    while idx < len {
        let opt = buf[idx];
        if opt == 255 {
            break;
        }
        if opt == 0 {
            idx += 1;
            continue;
        }
        if idx + 1 >= len {
            break;
        }
        let opt_len = buf[idx + 1] as usize;
        let opt_start = idx + 2;
        let opt_end = opt_start + opt_len;
        if opt_end > len {
            break;
        }
        if opt == 53 && opt_len == 1 {
            msg_type = Some(buf[opt_start]);
        }
        idx = opt_end;
    }

    let response_type = match msg_type {
        Some(DHCP_DISCOVER) => {
            println!(
                "\x1b[1;32m[dhcp-server]\x1b[0m DHCPDISCOVER from {} on usb0 -> Offering 192.168.7.1",
                mac_str
            );
            DHCP_OFFER
        }
        Some(DHCP_REQUEST) => {
            println!(
                "\x1b[1;32m[dhcp-server]\x1b[0m DHCPREQUEST from {} on usb0 -> Acknowledging 192.168.7.1",
                mac_str
            );
            if let Ok(mut lease) = ACTIVE_LEASE.write() {
                *lease = Some(DhcpLease {
                    mac: mac_str.clone(),
                    ip: "192.168.7.1".to_string(),
                    timestamp: Instant::now(),
                });
            }
            DHCP_ACK
        }
        _ => return,
    };

    // Construct response
    let mut resp = [0u8; 320];
    resp[0] = 2; // BOOTREPLY
    resp[1] = buf[1]; // htype (Ethernet = 1)
    resp[2] = buf[2]; // hlen (6)
    resp[3] = 0; // hops
    resp[4..8].copy_from_slice(xid);
    resp[8..10].copy_from_slice(&[0, 0]); // secs
    resp[10..12].copy_from_slice(&buf[10..12]); // flags (broadcast flag preserved)
    // ciaddr = 0.0.0.0 (offset 12..16)
    // yiaddr = 192.168.7.1 (offset 16..20)
    resp[16..20].copy_from_slice(&HOST_IP);
    // siaddr = 192.168.7.2 (offset 20..24)
    resp[20..24].copy_from_slice(&SERVER_IP);
    // giaddr = 0.0.0.0 (offset 24..28)
    // chaddr = client mac (offset 28..44)
    resp[28..44].copy_from_slice(&buf[28..44]);
    // sname (44..108) = 0
    // file (108..236) = 0
    // Magic cookie (236..240)
    resp[236..240].copy_from_slice(&DHCP_MAGIC_COOKIE);

    // Options start at 240
    let mut opt_idx = 240;

    // Option 53: Message Type (Offer or Ack)
    resp[opt_idx] = 53;
    resp[opt_idx + 1] = 1;
    resp[opt_idx + 2] = response_type;
    opt_idx += 3;

    // Option 54: Server Identifier (192.168.7.2)
    resp[opt_idx] = 54;
    resp[opt_idx + 1] = 4;
    resp[opt_idx + 2..opt_idx + 6].copy_from_slice(&SERVER_IP);
    opt_idx += 6;

    // Option 51: Lease Time (7200 seconds)
    resp[opt_idx] = 51;
    resp[opt_idx + 1] = 4;
    resp[opt_idx + 2..opt_idx + 6].copy_from_slice(&LEASE_TIME_SECS.to_be_bytes());
    opt_idx += 6;

    // Option 1: Subnet Mask (255.255.255.0)
    resp[opt_idx] = 1;
    resp[opt_idx + 1] = 4;
    resp[opt_idx + 2..opt_idx + 6].copy_from_slice(&SUBNET_MASK);
    opt_idx += 6;

    // CRITICAL: Intentionally NO Option 3 (Default Gateway)!
    // This ensures host PC never diverts default routing away from Wi-Fi/LAN!

    // Option 255: End
    resp[opt_idx] = 255;
    opt_idx += 1;

    // Send broadcast to 255.255.255.255:68
    let dest = SocketAddrV4::new(Ipv4Addr::new(255, 255, 255, 255), 68);
    let send_len = opt_idx.max(300); // Pad to at least 300 bytes for BOOTP compliance
    if let Err(e) = socket.send_to(&resp[..send_len], dest) {
        eprintln!("\x1b[1;31m[dhcp-server]\x1b[0m send_to error: {}", e);
    }
}

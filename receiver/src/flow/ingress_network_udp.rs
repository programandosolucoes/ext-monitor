//! Network UDP Ingress Micro-Block (UDP Datagram Transport Receiver)
//!
//! Blueprint Reference: Blueprint 27 (Flow-Oriented Micro-Block Architecture)
//!
//! Single Responsibility:
//! Binds UDP sockets on designated video ingress ports (e.g. port 5000 for primary stream,
//! port 5002 for secondary/fallback), configures high-capacity OS socket receive buffers
//! (512 KB) to prevent packet drops during encoder burst bursts, and receives datagrams
//! cleanly without decoding video payloads.
//!
//! Boundary Isolation Rules:
//! 1. Strictly transport-level: Does not inspect RTP headers, H.264 / HEVC NALUs, or MPEG-TS sync bytes.
//! 2. Autonomous socket reuse: Configures `SO_REUSEADDR` to allow rapid restart or multi-listener modes.
//! 3. Zero dependency on presentation or hardware decoding.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::time::Duration;

/// Primary UDP video stream ingress port (RTP H.264 / HEVC)
pub const DEFAULT_PRIMARY_UDP_PORT: u16 = 5000;

/// Secondary UDP video stream ingress port (fallback or secondary display stream)
pub const DEFAULT_SECONDARY_UDP_PORT: u16 = 5002;

/// Operating System socket receive buffer size (512 KB)
///
/// Blueprint 27 Specification: A 512 KB buffer accommodates several complete 60 FPS
/// video frames during transient CPU scheduler spikes on the single-core ARM1176JZF-S (Pi Zero).
pub const UDP_SOCKET_BUFFER_SIZE: usize = 512 * 1024;

/// Standard Ethernet Maximum Transmission Unit (MTU)
pub const ETHERNET_MTU: usize = 1500;

/// Maximum theoretical IPv4 UDP datagram payload size
pub const MAX_UDP_DATAGRAM_SIZE: usize = 65507;

/// Network UDP Ingress Controller
pub struct NetworkUdpIngress {
    socket: UdpSocket,
    port: u16,
    total_bytes_received: u64,
    total_packets_received: u64,
}

impl NetworkUdpIngress {
    /// Binds a new UDP ingress socket on IPv4 wildcard `0.0.0.0:<port>` with `SO_REUSEADDR`
    /// and a 512 KB receive buffer (`SO_RCVBUF`).
    pub fn bind(port: u16) -> io::Result<Self> {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
        Self::bind_addr(addr)
    }

    /// Binds to a specific `SocketAddr` with reusable address and 512 KB receive buffer.
    pub fn bind_addr(addr: SocketAddr) -> io::Result<Self> {
        let fd = unsafe {
            let fd = libc::socket(
                if addr.is_ipv6() { libc::AF_INET6 } else { libc::AF_INET },
                libc::SOCK_DGRAM,
                0,
            );
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }

            // Enable SO_REUSEADDR
            let opt_reuse: libc::c_int = 1;
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_REUSEADDR,
                &opt_reuse as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            );

            // Configure 512 KB receive buffer (SO_RCVBUF)
            let rcvbuf_size: libc::c_int = UDP_SOCKET_BUFFER_SIZE as libc::c_int;
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVBUF,
                &rcvbuf_size as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            );

            // Bind to specified address
            let bind_res = match addr {
                SocketAddr::V4(v4) => {
                    let mut sin: libc::sockaddr_in = std::mem::zeroed();
                    sin.sin_family = libc::AF_INET as libc::sa_family_t;
                    sin.sin_port = v4.port().to_be();
                    sin.sin_addr.s_addr = u32::from(*v4.ip()).to_be();
                    libc::bind(
                        fd,
                        &sin as *const _ as *const libc::sockaddr,
                        std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
                    )
                }
                SocketAddr::V6(v6) => {
                    let mut sin6: libc::sockaddr_in6 = std::mem::zeroed();
                    sin6.sin6_family = libc::AF_INET6 as libc::sa_family_t;
                    sin6.sin6_port = v6.port().to_be();
                    sin6.sin6_addr.s6_addr = v6.ip().octets();
                    libc::bind(
                        fd,
                        &sin6 as *const _ as *const libc::sockaddr,
                        std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t,
                    )
                }
            };

            if bind_res < 0 {
                let err = io::Error::last_os_error();
                libc::close(fd);
                return Err(err);
            }

            fd
        };

        let socket = unsafe { UdpSocket::from_raw_fd(fd) };
        let bound_port = socket.local_addr()?.port();

        Ok(Self {
            socket,
            port: bound_port,
            total_bytes_received: 0,
            total_packets_received: 0,
        })
    }

    /// Receives a single datagram into `buf`, returning the number of bytes read.
    pub fn recv_datagram(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let (bytes, _src) = self.recv_from(buf)?;
        Ok(bytes)
    }

    /// Receives a datagram into `buf`, returning bytes read and the sender's `SocketAddr`.
    pub fn recv_from(&mut self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let (bytes, src) = self.socket.recv_from(buf)?;
        self.total_bytes_received += bytes as u64;
        self.total_packets_received += 1;
        Ok((bytes, src))
    }

    /// Configures blocking or non-blocking mode on the underlying UDP socket
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.socket.set_nonblocking(nonblocking)
    }

    /// Sets the socket read timeout
    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.socket.set_read_timeout(timeout)
    }

    /// Returns the local socket address
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    /// Returns the port number this ingress is listening on
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Total bytes received across all datagrams
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes_received
    }

    /// Total count of datagrams received
    pub fn total_packets(&self) -> u64 {
        self.total_packets_received
    }

    /// Access to the underlying standard `UdpSocket` reference
    pub fn socket(&self) -> &UdpSocket {
        &self.socket
    }

    /// Returns the actual kernel receive buffer size via `SO_RCVBUF`
    pub fn kernel_rcvbuf_size(&self) -> io::Result<usize> {
        unsafe {
            let mut size: libc::c_int = 0;
            let mut len = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
            let res = libc::getsockopt(
                self.socket.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_RCVBUF,
                &mut size as *mut _ as *mut libc::c_void,
                &mut len,
            );
            if res < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(size as usize)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udp_ingress_bind_and_recv() {
        // Bind to localhost ephemeral port
        let mut ingress = NetworkUdpIngress::bind_addr("127.0.0.1:0".parse().unwrap())
            .expect("must bind ephemeral UDP socket");

        let local_addr = ingress.local_addr().expect("local addr");
        assert_ne!(ingress.port(), 0);
        assert_eq!(local_addr.port(), ingress.port());

        // Create sender client
        let sender = UdpSocket::bind("127.0.0.1:0").expect("sender bind");
        let payload = b"EXT_MONITOR_TEST_DATAGRAM_PAYLOAD_12345";

        sender.send_to(payload, local_addr).expect("send packet");

        let mut buf = [0u8; 1024];
        let (bytes, src_addr) = ingress.recv_from(&mut buf).expect("receive datagram");

        assert_eq!(bytes, payload.len());
        assert_eq!(&buf[..bytes], payload);
        assert_eq!(src_addr, sender.local_addr().unwrap());
        assert_eq!(ingress.total_bytes(), payload.len() as u64);
        assert_eq!(ingress.total_packets(), 1);

        // Send a second packet using recv_datagram
        sender.send_to(b"PACKET_2", local_addr).expect("send packet 2");
        let bytes2 = ingress.recv_datagram(&mut buf).expect("receive datagram 2");
        assert_eq!(bytes2, 8);
        assert_eq!(&buf[..bytes2], b"PACKET_2");
        assert_eq!(ingress.total_bytes(), (payload.len() + 8) as u64);
        assert_eq!(ingress.total_packets(), 2);
    }

    #[test]
    fn test_udp_ingress_rcvbuf_configured() {
        let ingress = NetworkUdpIngress::bind_addr("127.0.0.1:0".parse().unwrap())
            .expect("must bind UDP socket");

        // Linux doubles the requested SO_RCVBUF value for kernel overhead bookkeeping
        // Therefore kernel_rcvbuf_size should be >= UDP_SOCKET_BUFFER_SIZE
        let rcvbuf = ingress.kernel_rcvbuf_size().expect("read SO_RCVBUF");
        assert!(
            rcvbuf >= UDP_SOCKET_BUFFER_SIZE,
            "Kernel rcvbuf {} should be >= configured {}",
            rcvbuf,
            UDP_SOCKET_BUFFER_SIZE
        );
    }

    #[test]
    fn test_udp_port_constants() {
        assert_eq!(DEFAULT_PRIMARY_UDP_PORT, 5000);
        assert_eq!(DEFAULT_SECONDARY_UDP_PORT, 5002);
        assert_eq!(UDP_SOCKET_BUFFER_SIZE, 512 * 1024);
    }
}

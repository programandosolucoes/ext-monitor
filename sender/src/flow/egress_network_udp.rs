//! Network UDP Egress Micro-Block (Host to Pi Zero / Generic Target)
//!
//! Blueprint Reference: Blueprint 27 (Flow-Oriented Micro-Block Architecture)
//!
//! Single Responsibility:
//! Transmits RFC 4571 framed video slices or RTP datagrams over a UDP socket
//! to the receiver on port 5000 / 5002.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::io;
use std::net::{SocketAddr, UdpSocket};

pub const DEFAULT_UDP_BUFFER_SIZE: usize = 524288; // 512 KB

/// UDP Egress Transmitter
pub struct NetworkUdpEgress {
    socket: UdpSocket,
    target_addr: SocketAddr,
    total_bytes_sent: u64,
}

impl NetworkUdpEgress {
    /// Binds local ephemeral UDP socket and sets target destination
    pub fn new(target_addr: SocketAddr) -> io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_nonblocking(false)?;
        Ok(Self {
            socket,
            target_addr,
            total_bytes_sent: 0,
        })
    }

    /// Sends a datagram to the target receiver address
    pub fn send_datagram(&mut self, payload: &[u8]) -> io::Result<usize> {
        let sent = self.socket.send_to(payload, self.target_addr)?;
        self.total_bytes_sent += sent as u64;
        Ok(sent)
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes_sent
    }

    pub fn target(&self) -> SocketAddr {
        self.target_addr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udp_egress_creation_and_transmission() {
        let receiver_socket = UdpSocket::bind("127.0.0.1:0").expect("failed to bind receiver");
        let local_addr = receiver_socket.local_addr().expect("failed to get local addr");

        let mut egress = NetworkUdpEgress::new(local_addr).expect("failed to create egress");
        assert_eq!(egress.target(), local_addr);

        let data = b"TEST_PAYLOAD_H264";
        let sent = egress.send_datagram(data).expect("failed to send");
        assert_eq!(sent, data.len());
        assert_eq!(egress.total_bytes(), data.len() as u64);

        let mut buf = [0u8; 64];
        let (recv_len, _src) = receiver_socket.recv_from(&mut buf).expect("failed to receive");
        assert_eq!(recv_len, data.len());
        assert_eq!(&buf[..recv_len], data);
    }
}

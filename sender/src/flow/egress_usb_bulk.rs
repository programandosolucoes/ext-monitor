//! USB Bulk Egress Micro-Block (Host to Pi Zero)
//!
//! Blueprint Reference: Blueprint 27 (Flow-Oriented Micro-Block Architecture)
//!
//! Single Responsibility:
//! Accepts encoded H.264 / HEVC video frame slices and writes them directly into
//! Endpoint 1/3 Bulk OUT on Raspberry Pi Zero via `rusb`.
//! Handles RFC 4571 framing, ZLP (Zero-Length Packet) generation, and pipe stall recovery.
//!
//! Completely isolated from capture and codec parsing.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use rusb::{Context, DeviceHandle};
use std::time::Duration;

pub const USB_DEFAULT_ENDPOINT_OUT: u8 = 0x03;
pub const USB_WRITE_TIMEOUT: Duration = Duration::from_millis(500);

/// Checks whether a transfer length requires an explicit Zero-Length Packet (ZLP).
/// In USB High-Speed (480 Mbps), transfers that are exact multiples of 512 bytes
/// require a 0-byte packet to signal end-of-transfer to the receiver's hardware FIFO.
#[inline]
pub fn needs_zlp(transfer_len: usize) -> bool {
    transfer_len > 0 && transfer_len % 512 == 0
}

/// Prepends RFC 4571 2-byte big-endian framing length to a payload.
#[inline]
pub fn frame_rfc4571(payload: &[u8]) -> Vec<u8> {
    let len = payload.len() as u16;
    let mut framed = Vec::with_capacity(payload.len() + 2);
    framed.push((len >> 8) as u8);
    framed.push((len & 0xFF) as u8);
    framed.extend_from_slice(payload);
    framed
}

/// USB Bulk Egress Controller
pub struct UsbBulkEgress {
    endpoint_out: u8,
    timeout: Duration,
    total_bytes_written: u64,
}

impl Default for UsbBulkEgress {
    /// Returns default configuration parameters.
    fn default() -> Self {
        Self {
            endpoint_out: USB_DEFAULT_ENDPOINT_OUT,
            timeout: USB_WRITE_TIMEOUT,
            total_bytes_written: 0,
        }
    }
}

impl UsbBulkEgress {
    /// Constructs and initializes a new `new` instance with default or provided parameters.
    pub fn new(endpoint_out: u8, timeout: Duration) -> Self {
        Self {
            endpoint_out,
            timeout,
            total_bytes_written: 0,
        }
    }

    /// Writes raw slice into USB Bulk endpoint, automatically handling ZLP and stall recovery
    pub fn write_slice(
        &mut self,
        handle: &DeviceHandle<Context>,
        slice: &[u8],
    ) -> Result<usize, rusb::Error> {
        let mut total_written = 0;
        let mut offset = 0;

        while offset < slice.len() {
            let chunk_len = std::cmp::min(slice.len() - offset, 65536);
            match handle.write_bulk(self.endpoint_out, &slice[offset..offset + chunk_len], self.timeout) {
                Ok(written) => {
                    offset += written;
                    total_written += written;
                    self.total_bytes_written += written as u64;
                }
                Err(rusb::Error::Pipe) => {
                    // Endpoint halted (stall) - attempt clear halt
                    let _ = handle.clear_halt(self.endpoint_out);
                    return Err(rusb::Error::Pipe);
                }
                Err(e) => return Err(e),
            }
        }

        // Send ZLP if transfer was exact multiple of 512 bytes
        if needs_zlp(total_written) {
            let _ = handle.write_bulk(self.endpoint_out, &[], self.timeout);
        }

        Ok(total_written)
    }

    /// Executes `total_bytes` operational routine.
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes_written
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_zlp_boundary_conditions() {
        assert!(!needs_zlp(0));
        assert!(!needs_zlp(1));
        assert!(!needs_zlp(511));
        assert!(needs_zlp(512));
        assert!(!needs_zlp(513));
        assert!(needs_zlp(1024));
        assert!(needs_zlp(1536));
        assert!(!needs_zlp(1537));
    }

    #[test]
    fn test_rfc4571_framing() {
        let payload = [0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00];
        let framed = frame_rfc4571(&payload);
        assert_eq!(framed.len(), payload.len() + 2);
        assert_eq!(framed[0], 0x00);
        assert_eq!(framed[1], 0x07);
        assert_eq!(&framed[2..], &payload);
    }
}

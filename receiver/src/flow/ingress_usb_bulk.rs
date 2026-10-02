//! USB Bulk Ingress Micro-Block (Pi Zero FunctionFS to Demuxer)
//!
//! Blueprint Reference: Blueprint 27 (Flow-Oriented Micro-Block Architecture)
//!
//! Single Responsibility:
//! Ingests raw video transport bytes from the USB OTG FunctionFS Bulk OUT endpoint
//! (`/dev/usb-ffs/display/ep1`), silently drops Zero-Length Packets (ZLPs) used for hardware
//! FIFO flushes, validates RFC 4571 2-byte framing length headers, and forwards clean byte chunks
//! to downstream demuxers without video-specific decoding.
//!
//! Boundary Isolation Rules:
//! 1. Strictly decoupled: No knowledge of H.264 / HEVC NAL units, syntax, or slice structures.
//! 2. No KMS / DRM awareness: Handles strictly USB transport level buffering and framing.
//! 3. Fast non-blocking or polled operations: Handles FunctionFS endpoint disconnection and errors gracefully.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::fmt;
use std::fs::OpenOptions;
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{IntoRawFd, RawFd};

/// Default FunctionFS endpoint paths for Raspberry Pi Zero / Zero 2 W USB gadget
pub const DEFAULT_USB_FFS_EP1: &str = "/dev/usb-ffs/display/ep1";
pub const DEFAULT_USB_FFS_EP3: &str = "/dev/usb-ffs/display/ep3";

/// Standard USB High-Speed Bulk transfer chunk size
// Performance: Increased from 64KB to 256KB to handle large I-frames
// without fragmentation. H.264 keyframes at 1280x720 can exceed 100KB.
pub const USB_BULK_BUFFER_SIZE: usize = 262144;

/// High-Speed USB max packet size (512 bytes)
pub const USB_HS_PACKET_SIZE: usize = 512;

/// Error conditions produced during USB FunctionFS ingress and stream framing
#[derive(Debug)]
pub enum IngressError {
    /// Standard I/O error from system calls
    Io(io::Error),
    /// Zero-Length Packet (ZLP) was encountered and dropped
    ZeroLengthPacket,
    /// Invalid RFC 4571 framing header or payload length
    InvalidFraming(String),
    /// Target buffer is too small to receive the framed chunk
    BufferTooSmall { required: usize, provided: usize },
    /// Endpoint device path is not accessible or not opened
    EndpointUnavailable(String),
}

impl fmt::Display for IngressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IngressError::Io(err) => write!(f, "USB Ingress I/O error: {}", err),
            IngressError::ZeroLengthPacket => write!(f, "USB Zero-Length Packet (ZLP) dropped"),
            IngressError::InvalidFraming(msg) => write!(f, "RFC 4571 framing error: {}", msg),
            IngressError::BufferTooSmall { required, provided } => {
                write!(
                    f,
                    "Buffer too small: requires {} bytes, but only {} bytes provided",
                    required, provided
                )
            }
            IngressError::EndpointUnavailable(path) => {
                write!(f, "USB FunctionFS endpoint unavailable at '{}'", path)
            }
        }
    }
}

impl std::error::Error for IngressError {}

impl From<io::Error> for IngressError {
    fn from(err: io::Error) -> Self {
        IngressError::Io(err)
    }
}

/// Checks whether a transfer length requires an explicit Zero-Length Packet (ZLP).
///
/// In USB 2.0 High-Speed mode (480 Mbps), endpoints operate with a packet size of 512 bytes.
/// When a host transmits a frame whose byte size is an exact multiple of 512 bytes, the USB
/// hardware FIFO cannot know the transfer ended without an explicit 0-byte transfer (ZLP).
/// On receiver ingress, encountering a 0-byte packet signifies a hardware boundary flush.
#[inline]
pub fn needs_zlp(transfer_len: usize) -> bool {
    transfer_len > 0 && transfer_len % USB_HS_PACKET_SIZE == 0
}

/// Identifies if a read result is a Zero-Length Packet (ZLP).
#[inline]
pub fn is_zlp(bytes_read: usize) -> bool {
    bytes_read == 0
}

/// Encapsulates a payload inside an RFC 4571 2-byte big-endian length prefix.
#[inline]
pub fn frame_rfc4571(payload: &[u8]) -> Vec<u8> {
    let len = payload.len() as u16;
    let mut framed = Vec::with_capacity(payload.len() + 2);
    framed.push((len >> 8) as u8);
    framed.push((len & 0xFF) as u8);
    framed.extend_from_slice(payload);
    framed
}

/// Parses an RFC 4571 frame from a contiguous byte buffer.
///
/// RFC 4571 Section 4 defines:
/// ```text
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// -----------------------------------------------------------------
/// |             LENGTH            |             RTP...             |
/// -----------------------------------------------------------------
/// ```
///
/// Returns:
/// - `Ok(Some((payload_slice, total_consumed_bytes)))` if a complete frame was parsed.
/// - `Ok(None)` if the buffer does not yet contain enough data.
/// - `Err(IngressError)` if framing header is corrupted.
pub fn parse_rfc4571(data: &[u8]) -> Result<Option<(&[u8], usize)>, IngressError> {
    if data.len() < 2 {
        return Ok(None);
    }

    let frame_len = u16::from_be_bytes([data[0], data[1]]) as usize;
    let total_required = 2 + frame_len;

    if data.len() < total_required {
        return Ok(None);
    }

    let payload = &data[2..total_required];
    Ok(Some((payload, total_required)))
}

/// USB Bulk Ingress Controller for USB OTG FunctionFS endpoint.
pub struct UsbBulkIngress {
    endpoint_path: String,
    fd: Option<RawFd>,
    owns_fd: bool,
    total_bytes_received: u64,
    total_zlps_dropped: u64,
}

impl UsbBulkIngress {
    /// Creates a new UsbBulkIngress bound to a specific FunctionFS endpoint path
    pub fn new(endpoint_path: impl Into<String>) -> Self {
        Self {
            endpoint_path: endpoint_path.into(),
            fd: None,
            owns_fd: false,
            total_bytes_received: 0,
            total_zlps_dropped: 0,
        }
    }

    /// Creates an instance using default endpoint 1 (`/dev/usb-ffs/display/ep1`)
    pub fn default_ep1() -> Self {
        Self::new(DEFAULT_USB_FFS_EP1)
    }

    /// Creates an instance wrapping an existing raw file descriptor
    pub fn from_fd(fd: RawFd) -> Self {
        Self {
            endpoint_path: format!("fd://{}", fd),
            fd: Some(fd),
            owns_fd: false,
            total_bytes_received: 0,
            total_zlps_dropped: 0,
        }
    }

    /// Opens the configured FunctionFS endpoint in non-blocking read-only mode
    pub fn open(&mut self) -> Result<RawFd, IngressError> {
        if let Some(fd) = self.fd {
            return Ok(fd);
        }

        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&self.endpoint_path)
            .map_err(|e| IngressError::EndpointUnavailable(format!("{}: {}", self.endpoint_path, e)))?;

        let fd = file.into_raw_fd();
        self.fd = Some(fd);
        self.owns_fd = true;
        Ok(fd)
    }

    /// Reads raw chunk from the USB Bulk endpoint into `buf`.
    ///
    /// If a 0-byte read occurs from the kernel USB FIFO (Zero-Length Packet),
    /// it increments `total_zlps_dropped` and returns `Err(IngressError::ZeroLengthPacket)`.
    ///
    /// Non-blocking EAGAIN / EWOULDBLOCK conditions return `Ok(0)` indicating no data ready.
    pub fn read_frame_chunk(&mut self, buf: &mut [u8]) -> Result<usize, IngressError> {
        let fd = match self.fd {
            Some(fd) => fd,
            None => self.open()?,
        };

        loop {
            let n = unsafe {
                libc::read(
                    fd,
                    buf.as_mut_ptr() as *mut libc::c_void,
                    buf.len(),
                )
            };

            if n < 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::WouldBlock
                    || err.raw_os_error() == Some(libc::EAGAIN)
                    || err.raw_os_error() == Some(libc::EWOULDBLOCK)
                {
                    return Ok(0);
                }
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(IngressError::Io(err));
            }

            if n == 0 {
                // USB Zero-Length Packet: drop and record
                self.total_zlps_dropped += 1;
                return Err(IngressError::ZeroLengthPacket);
            }

            let read_bytes = n as usize;
            self.total_bytes_received += read_bytes as u64;
            return Ok(read_bytes);
        }
    }

    /// Reads chunk from USB, automatically filtering out and dropping ZLP without returning error.
    pub fn read_frame_chunk_dropping_zlp(&mut self, buf: &mut [u8]) -> Result<usize, IngressError> {
        match self.read_frame_chunk(buf) {
            Ok(bytes) => Ok(bytes),
            Err(IngressError::ZeroLengthPacket) => Ok(0),
            Err(other) => Err(other),
        }
    }

    /// Returns the total bytes successfully received over USB Bulk
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes_received
    }

    /// Returns the number of ZLPs dropped
    pub fn total_zlps_dropped(&self) -> u64 {
        self.total_zlps_dropped
    }

    /// Returns the configured endpoint path
    pub fn endpoint_path(&self) -> &str {
        &self.endpoint_path
    }

    /// Returns the raw file descriptor if open
    pub fn raw_fd(&self) -> Option<RawFd> {
        self.fd
    }
}

impl Drop for UsbBulkIngress {
    fn drop(&mut self) {
        if self.owns_fd {
            if let Some(fd) = self.fd.take() {
                unsafe {
                    libc::close(fd);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_zlp() {
        // Zero bytes requires no packet at all
        assert!(!needs_zlp(0));

        // Sub-512 byte packets do not end on boundary
        assert!(!needs_zlp(1));
        assert!(!needs_zlp(256));
        assert!(!needs_zlp(511));

        // Exact multiples of 512 require ZLP flush
        assert!(needs_zlp(512));
        assert!(!needs_zlp(513));
        assert!(needs_zlp(1024));
        assert!(!needs_zlp(1025));
        assert!(needs_zlp(1536));
        assert!(!needs_zlp(1537));
        assert!(needs_zlp(65536)); // 128 * 512
        assert!(!needs_zlp(65535));

        // Test helper is_zlp
        assert!(is_zlp(0));
        assert!(!is_zlp(1));
        assert!(!is_zlp(512));
    }

    #[test]
    fn test_rfc4571_parse() {
        // Incomplete header (< 2 bytes)
        assert_eq!(parse_rfc4571(&[]).unwrap(), None);
        assert_eq!(parse_rfc4571(&[0x00]).unwrap(), None);

        // Frame header for 5-byte payload: [0x00, 0x05]
        let payload = [10, 20, 30, 40, 50];
        let mut framed = frame_rfc4571(&payload);
        assert_eq!(framed.len(), 7);
        assert_eq!(framed[0], 0x00);
        assert_eq!(framed[1], 0x05);

        // Incomplete payload
        assert_eq!(parse_rfc4571(&framed[..4]).unwrap(), None);

        // Complete payload parsed
        let (parsed_payload, consumed) = parse_rfc4571(&framed).unwrap().expect("should parse");
        assert_eq!(parsed_payload, &payload);
        assert_eq!(consumed, 7);

        // Buffer with trailing data (subsequent frame)
        framed.extend_from_slice(&[0x00, 0x02, 0xAA, 0xBB]);
        let (first_payload, first_consumed) = parse_rfc4571(&framed).unwrap().expect("first frame");
        assert_eq!(first_payload, &payload);
        assert_eq!(first_consumed, 7);

        let remainder = &framed[first_consumed..];
        let (second_payload, second_consumed) = parse_rfc4571(remainder).unwrap().expect("second frame");
        assert_eq!(second_payload, &[0xAA, 0xBB]);
        assert_eq!(second_consumed, 4);

        // Zero-length payload
        let zero_framed = frame_rfc4571(&[]);
        let (empty_payload, empty_consumed) = parse_rfc4571(&zero_framed).unwrap().expect("empty frame");
        assert!(empty_payload.is_empty());
        assert_eq!(empty_consumed, 2);
    }

    #[test]
    fn test_usb_bulk_ingress_creation() {
        let ingress = UsbBulkIngress::default_ep1();
        assert_eq!(ingress.endpoint_path(), DEFAULT_USB_FFS_EP1);
        assert_eq!(ingress.total_bytes(), 0);
        assert_eq!(ingress.total_zlps_dropped(), 0);

        let ingress3 = UsbBulkIngress::new(DEFAULT_USB_FFS_EP3);
        assert_eq!(ingress3.endpoint_path(), DEFAULT_USB_FFS_EP3);
    }

    #[test]
    fn test_pipe_read_and_zlp_drop() {
        // Create an OS pipe to simulate FunctionFS endpoint
        let mut pipe_fds = [0 as libc::c_int; 2];
        unsafe {
            let ret = libc::pipe(pipe_fds.as_mut_ptr());
            assert_eq!(ret, 0, "pipe creation must succeed");
            // Set non-blocking on read end
            let flags = libc::fcntl(pipe_fds[0], libc::F_GETFL, 0);
            libc::fcntl(pipe_fds[0], libc::F_SETFL, flags | libc::O_NONBLOCK);
        }

        let read_fd = pipe_fds[0];
        let write_fd = pipe_fds[1];

        let mut ingress = UsbBulkIngress::from_fd(read_fd);

        // Write some payload
        let test_data = [1, 2, 3, 4, 5, 6, 7, 8];
        unsafe {
            libc::write(write_fd, test_data.as_ptr() as *const libc::c_void, test_data.len());
        }

        let mut buf = [0u8; 1024];
        let bytes_read = ingress.read_frame_chunk(&mut buf).expect("read must succeed");
        assert_eq!(bytes_read, test_data.len());
        assert_eq!(&buf[..bytes_read], &test_data);
        assert_eq!(ingress.total_bytes(), test_data.len() as u64);

        // Close write end to simulate EOF / ZLP
        unsafe {
            libc::close(write_fd);
        }

        // Reading now should detect 0-byte EOF/ZLP and increment dropped counter
        let zlp_res = ingress.read_frame_chunk(&mut buf);
        match zlp_res {
            Err(IngressError::ZeroLengthPacket) => {
                assert_eq!(ingress.total_zlps_dropped(), 1);
            }
            other => panic!("Expected ZeroLengthPacket error, got {:?}", other),
        }

        // With read_frame_chunk_dropping_zlp it should return Ok(0)
        let dropped = ingress.read_frame_chunk_dropping_zlp(&mut buf).expect("should drop silently");
        assert_eq!(dropped, 0);
        assert_eq!(ingress.total_zlps_dropped(), 2);

        unsafe {
            libc::close(read_fd);
        }
    }
}

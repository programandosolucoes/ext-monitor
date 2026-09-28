//! Native Linux V4L2 M2M VideoCore IV Hardware Decoder Facade
//!
//! Provides a clean, backwards-compatible public API delegating to the modular
//! `ingress`, `decoder`, `stream`, and `display` subsystems.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

use crate::decoder::V4l2DecoderSession;
use crate::ingress::{MiracastIngress, UdpRtpIngress, UsbBulkIngress};
use std::io;
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

pub struct NativeV4l2Decoder {
    running: Arc<AtomicBool>,
    worker_handle: Option<JoinHandle<()>>,
    active_fd: Option<RawFd>,
}

impl NativeV4l2Decoder {
    /// Probes whether `/dev/video10` (VideoCore IV decoder) is available
    pub fn is_supported() -> bool {
        V4l2DecoderSession::is_supported()
    }

    /// Starts a native background decode loop from a UDP RTP socket
    pub fn start_udp_stream(port: u16) -> io::Result<Self> {
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();

        let worker_handle = thread::spawn(move || {
            UdpRtpIngress::run(port, r);
        });

        Ok(Self {
            running,
            worker_handle: Some(worker_handle),
            active_fd: None,
        })
    }

    /// Starts a native background decode loop from a Miracast MPEG-TS UDP socket
    pub fn start_miracast_stream(port: u16) -> io::Result<Self> {
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();

        let worker_handle = thread::spawn(move || {
            MiracastIngress::run(port, r);
        });

        Ok(Self {
            running,
            worker_handle: Some(worker_handle),
            active_fd: None,
        })
    }

    /// Starts a native background decode loop from a file descriptor (USB Bulk)
    pub fn start_fd_stream(fd: RawFd) -> io::Result<Self> {
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();

        let worker_handle = thread::spawn(move || {
            UsbBulkIngress::run(fd, r);
        });

        Ok(Self {
            running,
            worker_handle: Some(worker_handle),
            active_fd: Some(fd),
        })
    }

    /// Checks whether the background decode thread has finished/exited
    pub fn has_exited(&self) -> bool {
        if let Some(ref handle) = self.worker_handle {
            handle.is_finished()
        } else {
            true
        }
    }

    /// Terminates the native decoding worker
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(fd) = self.active_fd.take() {
            unsafe {
                libc::close(fd);
            }
        }
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
        println!("\x1b[1;33m[native-v4l2]\x1b[0m Hardware Decoder stopped cleanly.");
    }
}

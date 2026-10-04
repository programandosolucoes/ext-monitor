//! UDP RTP Ingress Worker (Mode 1 - Network IP)
//!
//! Listens for RFC 6184 RTP H.264 streams on a UDP socket, depayloads them
//! into complete Access Units via `RtpDepayloader`, submits them to `V4l2DecoderSession`,
//! and blits decoded frames to the HDMI framebuffer.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use crate::decoder::V4l2DecoderSession;
use crate::stream::RtpDepayloader;
use std::io;
use std::net::UdpSocket;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Executes `bind_reusable_udp` operational routine.
fn bind_reusable_udp(port: u16) -> io::Result<UdpSocket> {
    unsafe {
        let fd = libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0);
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let opt: libc::c_int = 1;
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_REUSEADDR,
            &opt as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        );
        let mut addr: libc::sockaddr_in = std::mem::zeroed();
        addr.sin_family = libc::AF_INET as libc::sa_family_t;
        addr.sin_port = port.to_be();
        addr.sin_addr.s_addr = libc::INADDR_ANY;

        for attempt in 0..5 {
            if libc::bind(
                fd,
                &addr as *const _ as *const libc::sockaddr,
                std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
            ) == 0 {
                return Ok(UdpSocket::from_raw_fd(fd));
            }
            if attempt < 4 {
                thread::sleep(Duration::from_millis(100));
            }
        }

        let err = io::Error::last_os_error();
        libc::close(fd);
        Err(err)
    }
}

/// Represents Udprtpingress configuration and operational state.
pub struct UdpRtpIngress;

impl UdpRtpIngress {
    /// Runs the UDP RTP ingress decode and display loop until `running` becomes false
    pub fn run(port: u16, running: Arc<AtomicBool>) {
        let sock = match bind_reusable_udp(port) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("\x1b[1;31m[udp-ingress]\x1b[0m Failed to bind UDP port {}: {}", port, e);
                return;
            }
        };

        // Increase OS receive buffer to 2MB to prevent dropped UDP packets during burst
        unsafe {
            let size: libc::c_int = 2 * 1024 * 1024;
            libc::setsockopt(
                sock.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_RCVBUF,
                &size as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            );
        }

        let _ = sock.set_nonblocking(true);
        let mut buffer = [0u8; 8192];
        let mut pfd = libc::pollfd {
            fd: sock.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };

        let mut decoder = match V4l2DecoderSession::new(1280, 720) {
            Some(s) => s,
            None => {
                eprintln!("\x1b[1;31m[udp-ingress]\x1b[0m Failed to initialize V4L2 M2M decoder session.");
                return;
            }
        };

        let mut depayloader = RtpDepayloader::new();
        let mut completed_frames: Vec<Vec<u8>> = Vec::with_capacity(16);

        println!(
            "\x1b[1;32m[udp-ingress]\x1b[0m Listening for RTP H.264 stream on UDP port {} -> HDMI KMS plane scanout active.",
            port
        );

        while running.load(Ordering::SeqCst) {
            let ret = unsafe { libc::poll(&mut pfd, 1, 15) };
            if ret < 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                break;
            }
            if ret == 0 {
                // Socket idle for 15ms: drain any pending decoded frames from hardware VPU
                decoder.drain_decoded_frames();
                continue;
            }

            if (pfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL)) != 0 {
                break;
            }
            if (pfd.revents & libc::POLLIN) == 0 {
                continue;
            }

            // Drain all available UDP packets from the socket buffer in a tight userspace loop
            while let Ok(n) = sock.recv(&mut buffer) {
                if n > 12 {
                    depayloader.depayload_packet(&buffer[..n], &mut completed_frames);
                }
            }

            // Decode all complete Access Units assembled from the drained burst
            for frame in completed_frames.drain(..) {
                decoder.decode_chunk(&frame);
            }
        }

        if let Ok(cfg) = crate::web::CONFIG.lock() {
            if cfg.mode1 && !crate::flow::ARBITER.is_level0_active() {
                crate::display::SplashEngine::show_ready();
            }
        }
        println!("\x1b[1;32m[udp-ingress]\x1b[0m UDP ingress worker stopped.");
    }
}

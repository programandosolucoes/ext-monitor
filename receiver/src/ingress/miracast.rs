//! Windows Miracast MPEG-TS UDP Ingress Worker (Mode 2 - Wi-Fi Display)
//!
//! Listens for incoming MPEG-TS over UDP stream (port 5002) negotiated via WFD RTSP,
//! extracts elementary H.264 Access Units via pure Rust `TsDemuxer`, submits them
//! to `V4l2DecoderSession`, and blits decoded frames zero-copy to the HDMI display.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use crate::decoder::V4l2DecoderSession;
use crate::display::FramebufferSink;
use crate::stream::{TsDemuxer, parse_sps_dimensions};
use std::io;
use std::net::UdpSocket;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_REUSEPORT,
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

pub struct MiracastIngress;

impl MiracastIngress {
    /// Runs the Miracast MPEG-TS UDP ingress decode and display loop until `running` becomes false
    pub fn run(port: u16, running: Arc<AtomicBool>) {
        let sock = match bind_reusable_udp(port) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "\x1b[1;31m[miracast-ingress]\x1b[0m Failed to bind UDP port {}: {}",
                    port, e
                );
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

        let mut display = match FramebufferSink::open(1280, 720) {
            Ok(d) => d,
            Err(e) => {
                eprintln!(
                    "\x1b[1;31m[miracast-ingress]\x1b[0m Failed to open display: {}",
                    e
                );
                return;
            }
        };

        let mut current_dims = (1280u32, 720u32);
        let mut decoder = match V4l2DecoderSession::new(current_dims.0, current_dims.1) {
            Some(s) => Some(s),
            None => {
                eprintln!(
                    "\x1b[1;31m[miracast-ingress]\x1b[0m Failed to initialize V4L2 M2M decoder session for 1280x720."
                );
                return;
            }
        };

        let mut demuxer = TsDemuxer::new();
        let mut completed_frames: Vec<Vec<u8>> = Vec::with_capacity(16);

        println!(
            "\x1b[1;32m[miracast-ingress]\x1b[0m Listening for MPEG-TS stream on UDP port {} (Native: 1280x720p60 + Dynamic SPS) -> HDMI Display active.",
            port
        );

        let mut last_packet_time = std::time::Instant::now();
        let mut splash_active = true;
        let mut total_packets = 0u64;

        // Show Miracast Splash immediately when ingress worker initializes
        if let Ok(mut lock) = crate::decoder::v4l2_m2m::LATEST_SCREENSHOT_FRAME.lock() {
            *lock = None;
        }
        crate::display::SplashEngine::show_miracast();

        while running.load(Ordering::SeqCst) {
            let ret = unsafe { libc::poll(&mut pfd, 1, 5) };
            if ret < 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                break;
            }
            if ret == 0 {
                demuxer.flush(&mut completed_frames);
                for frame in completed_frames.drain(..) {
                    if let Some((w, h)) = parse_sps_dimensions(&frame) {
                        if (w, h) != current_dims {
                            println!(
                                "\x1b[1;32m[miracast-ingress]\x1b[0m Video format dynamically detected from SPS: {}x{} (reconfiguring V4L2 decoder)...",
                                w, h
                            );
                            current_dims = (w, h);
                            decoder = V4l2DecoderSession::new(w, h);
                        }
                    }

                    if let Some(ref mut dec) = decoder {
                        dec.decode_chunk(&frame, |frame_rgb565| {
                            display.render_frame(frame_rgb565);
                        });
                    }
                }

                if let Some(ref mut dec) = decoder {
                    dec.drain_decoded_frames(|frame_rgb565| {
                        display.render_frame(frame_rgb565);
                    });
                }

                // If stream was active and now idle for > 2 seconds: return to splash screen
                if !splash_active && total_packets > 0 && last_packet_time.elapsed() >= Duration::from_secs(2) {
                    println!("\x1b[1;33m[miracast-ingress]\x1b[0m Miracast stream idle / disconnected -> Returning to Miracast Splash Screen.");
                    if let Ok(mut lock) = crate::decoder::v4l2_m2m::LATEST_SCREENSHOT_FRAME.lock() {
                        *lock = None;
                    }
                    crate::display::SplashEngine::show_miracast();
                    splash_active = true;
                }
                continue;
            }

            if (pfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL)) != 0 {
                break;
            }
            if (pfd.revents & libc::POLLIN) == 0 {
                continue;
            }

            // Drain all available TS packets in tight loop
            while let Ok(n) = sock.recv(&mut buffer) {
                if n >= 188 {
                    last_packet_time = std::time::Instant::now();
                    total_packets += 1;
                    if splash_active {
                        splash_active = false;
                    }
                    if total_packets == 1 || total_packets % 300 == 0 {
                        println!(
                            "\x1b[1;32m[miracast-ingress]\x1b[0m Ingested {} TS packets from UDP 5002 (chunk: {} bytes)",
                            total_packets, n
                        );
                    }
                    demuxer.push_udp_packet(&buffer[..n], &mut completed_frames);
                }
            }

            for frame in completed_frames.drain(..) {
                if let Some((w, h)) = parse_sps_dimensions(&frame) {
                    if (w, h) != current_dims {
                        println!(
                            "\x1b[1;32m[miracast-ingress]\x1b[0m Video format dynamically detected from SPS: {}x{} (reconfiguring V4L2 decoder)...",
                            w, h
                        );
                        current_dims = (w, h);
                        decoder = V4l2DecoderSession::new(w, h);
                    }
                }

                if let Some(ref mut dec) = decoder {
                    dec.decode_chunk(&frame, |frame_rgb565| {
                        display.render_frame(frame_rgb565);
                    });
                }
            }
        }

        if let Ok(mut lock) = crate::decoder::v4l2_m2m::LATEST_SCREENSHOT_FRAME.lock() {
            *lock = None;
        }
        crate::display::SplashEngine::show_miracast();
        println!("\x1b[1;33m[miracast-ingress]\x1b[0m Miracast ingress worker stopped.");
    }
}

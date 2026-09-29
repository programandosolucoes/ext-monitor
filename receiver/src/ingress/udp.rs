//! UDP RTP Ingress Worker (Mode 1 - Network IP)
//!
//! Listens for RFC 6184 RTP H.264 streams on a UDP socket, depayloads them
//! into complete Access Units via `RtpDepayloader`, submits them to `V4l2DecoderSession`,
//! and blits decoded frames to the HDMI framebuffer.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

use crate::decoder::V4l2DecoderSession;
use crate::display::FramebufferSink;
use crate::stream::RtpDepayloader;
use std::io;
use std::net::UdpSocket;
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct UdpRtpIngress;

impl UdpRtpIngress {
    /// Runs the UDP RTP ingress decode and display loop until `running` becomes false
    pub fn run(port: u16, running: Arc<AtomicBool>) {
        let sock = match UdpSocket::bind(format!("0.0.0.0:{}", port)) {
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

        let _ = sock.set_read_timeout(Some(Duration::from_millis(10)));
        let mut buffer = [0u8; 4096];

        let mut display = match FramebufferSink::open(1280, 720) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("\x1b[1;31m[udp-ingress]\x1b[0m Failed to open framebuffer /dev/fb0: {}", e);
                return;
            }
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
            "\x1b[1;32m[udp-ingress]\x1b[0m Listening for RTP H.264 stream on UDP port {} -> HDMI Display active.",
            port
        );

        let mut last_packet_time = std::time::Instant::now();
        let mut splash_active = false;
        let mut total_packets = 0u64;

        while running.load(Ordering::SeqCst) {
            match sock.recv(&mut buffer) {
                Ok(n) if n > 12 => {
                    last_packet_time = std::time::Instant::now();
                    total_packets += 1;
                    if splash_active {
                        splash_active = false;
                    }

                    completed_frames.clear();
                    depayloader.depayload_packet(&buffer[..n], &mut completed_frames);

                    for frame in &completed_frames {
                        decoder.decode_chunk(frame, |frame_rgb565| {
                            display.render_frame(frame_rgb565);
                        });
                    }
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut => {
                    decoder.drain_decoded_frames(|frame_rgb565| {
                        display.render_frame(frame_rgb565);
                    });

                    // If stream was active and truly disconnected (> 15s without packets): clear to black screen
                    if !splash_active && total_packets > 0 && last_packet_time.elapsed() >= Duration::from_secs(15) {
                        println!("\x1b[1;33m[udp-ingress]\x1b[0m UDP stream truly disconnected (>15s) -> Limpando tela do Raspberry (Black screen)...");
                        crate::display::SplashEngine::clear();
                        splash_active = true;
                    }
                    thread::sleep(Duration::from_micros(500));
                }
                Err(e) => {
                    eprintln!("\x1b[1;31m[udp-ingress]\x1b[0m Socket read error: {}", e);
                    break;
                }
            }
        }

        crate::display::SplashEngine::clear();
        println!("\x1b[1;33m[udp-ingress]\x1b[0m UDP ingress worker stopped (Tela limpa).");
    }
}

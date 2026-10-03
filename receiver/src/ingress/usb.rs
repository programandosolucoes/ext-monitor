//! USB Bulk Ingress Worker
//!
//! Ingests H.264 video streams from the USB FunctionFS Bulk OUT endpoint (ep1),
//! supporting both RFC 4571 length-framed RTP packets and raw Annex-B byte streams.
//! Assembles complete Access Units, submits them to `V4l2DecoderSession`, and
//! renders decoded frames directly to the HDMI framebuffer.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use crate::decoder::V4l2DecoderSession;
use crate::display::FramebufferSink;
use crate::stream::{AnnexBAssembler, Rfc4571Assembler, RtpDepayloader};
use std::io;
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[allow(dead_code)]
enum IngressFramingMode {
    AutoDetect,
    Rfc4571,
    AnnexB,
}

pub struct UsbBulkIngress;

impl UsbBulkIngress {
    /// Runs the USB Bulk ingress decode and display loop until `running` becomes false
    pub fn run(read_fd: RawFd, running: Arc<AtomicBool>) {
        let mut display = match FramebufferSink::open(1280, 720) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("\x1b[1;31m[usb-ingress]\x1b[0m Failed to open framebuffer /dev/fb0: {}", e);
                return;
            }
        };

        let mut decoder = match V4l2DecoderSession::new(1280, 720) {
            Some(s) => s,
            None => {
                eprintln!("\x1b[1;31m[usb-ingress]\x1b[0m Failed to initialize V4L2 M2M decoder session.");
                return;
            }
        };

        let framing_mode = IngressFramingMode::AnnexB;
        let mut annexb_assembler = AnnexBAssembler::new();
        let mut rfc_assembler = Rfc4571Assembler::new();
        let mut rtp_depayloader = RtpDepayloader::new();

        let mut completed_frames: Vec<Vec<u8>> = Vec::with_capacity(16);

        println!(
            "\x1b[1;32m[usb-ingress]\x1b[0m Reading H.264 stream from USB Bulk fd {} -> HDMI Display active (Lossless Annex-B).",
            read_fd
        );

        // Set non-blocking mode on the USB Bulk endpoint to prevent blocking indefinitely when switching modes
        unsafe {
            let flags = libc::fcntl(read_fd, libc::F_GETFL, 0);
            if flags >= 0 {
                libc::fcntl(read_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
        }

        let mut total_bytes = 0u64;
        let mut last_log = std::time::Instant::now();


        let mut buffer = [0u8; 65536];
        let mut pfd = libc::pollfd {
            fd: read_fd,
            events: libc::POLLIN,
            revents: 0,
        };

        while running.load(Ordering::SeqCst) {
            // Poll with 20ms timeout so worker thread reacts quickly to shutdown / pause
            let ret = unsafe { libc::poll(&mut pfd, 1, 20) };
            if ret < 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                break;
            }
            if ret == 0 {
                // Idle: flush any assembled AU and ready decoded frames to screen
                annexb_assembler.flush(&mut completed_frames);
                for frame in &completed_frames {
                    decoder.decode_chunk(frame, |frame_rgb565| {
                        display.render_frame(frame_rgb565);
                    });
                }
                completed_frames.clear();
                decoder.drain_decoded_frames(|frame_rgb565| {
                    display.render_frame(frame_rgb565);
                });
                continue;
            }

            if (pfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL)) != 0 {
                break;
            }
            if (pfd.revents & libc::POLLIN) == 0 {
                continue;
            }

            let n = unsafe {
                libc::read(
                    read_fd,
                    buffer.as_mut_ptr() as *mut libc::c_void,
                    buffer.len(),
                )
            };

            if n > 0 {
                let chunk = &buffer[..n as usize];


                match framing_mode {
                    IngressFramingMode::AutoDetect | IngressFramingMode::AnnexB => {
                        annexb_assembler.push(chunk, &mut completed_frames);
                        for frame in &completed_frames {
                            decoder.decode_chunk(frame, |frame_rgb565| {
                                display.render_frame(frame_rgb565);
                            });
                        }
                        completed_frames.clear();
                    }
                    IngressFramingMode::Rfc4571 => {
                        rfc_assembler.push(chunk, &mut rtp_depayloader, &mut completed_frames);
                        for frame in &completed_frames {
                            decoder.decode_chunk(frame, |frame_rgb565| {
                                display.render_frame(frame_rgb565);
                            });
                        }
                        completed_frames.clear();
                    }
                }

                total_bytes += n as u64;
                if last_log.elapsed() >= Duration::from_secs(5) {
                    let mb = (total_bytes as f64) / (1024.0 * 1024.0);
                    println!(
                        "\x1b[1;34m[usb-ingress]\x1b[0m Total received via USB Bulk: {:.2} MB",
                        mb
                    );
                    last_log = std::time::Instant::now();
                }
            } else if n == 0 {
                // USB Zero-Length Packet or bus idle
                if !running.load(Ordering::SeqCst) {
                    println!("\x1b[1;33m[usb-ingress]\x1b[0m ZLP received while stopping. Exiting ingress loop.");
                    break;
                }
                thread::sleep(Duration::from_millis(1));
            } else {
                let err = io::Error::last_os_error();
                if err.raw_os_error() == Some(libc::EBADF)
                    || err.raw_os_error() == Some(libc::ESHUTDOWN)
                    || err.raw_os_error() == Some(libc::ENODEV)
                {
                    println!("\x1b[1;33m[usb-ingress]\x1b[0m Endpoint closed ({}). Exiting ingress loop.", err);
                    break;
                }
                if err.kind() == io::ErrorKind::Interrupted || err.kind() == io::ErrorKind::WouldBlock {
                    thread::sleep(Duration::from_millis(1));
                    continue;
                }
                break;
            }
        }

        annexb_assembler.flush(&mut completed_frames);
        for frame in &completed_frames {
            decoder.decode_chunk(frame, |frame_rgb565| {
                display.render_frame(frame_rgb565);
            });
        }
        completed_frames.clear();
        decoder.drain_decoded_frames(|frame_rgb565| {
            display.render_frame(frame_rgb565);
        });

        if let Ok(cfg) = crate::web::CONFIG.lock() {
            if cfg.mode3 {
                crate::display::SplashEngine::show_ready();
            }
        }
        println!("\x1b[1;33m[usb-ingress]\x1b[0m Ingress worker stopped.");
    }
}

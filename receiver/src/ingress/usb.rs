//! USB Bulk Ingress Worker
//!
//! Ingests H.264 video streams from the USB FunctionFS Bulk OUT endpoint (ep1),
//! supporting both RFC 4571 length-framed RTP packets and raw Annex-B byte streams.
//! Assembles complete Access Units, submits them to `V4l2DecoderSession`, and
//! renders decoded frames directly to the HDMI framebuffer.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

use crate::decoder::V4l2DecoderSession;
use crate::display::FramebufferSink;
use crate::stream::{AnnexBAssembler, Rfc4571Assembler, RtpDepayloader};
use std::fs::File;
use std::io::{self, Read};
use std::os::unix::io::{FromRawFd, RawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

enum IngressFramingMode {
    AutoDetect,
    Rfc4571,
    AnnexB,
}

pub struct UsbBulkIngress;

impl UsbBulkIngress {
    /// Runs the USB Bulk ingress decode and display loop until `running` becomes false
    pub fn run(read_fd: RawFd, running: Arc<AtomicBool>) {
        let mut stream_file = unsafe { File::from_raw_fd(read_fd) };

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

        let mut framing_mode = IngressFramingMode::AutoDetect;
        let mut annexb_assembler = AnnexBAssembler::new();
        let mut rfc_assembler = Rfc4571Assembler::new();
        let mut rtp_depayloader = RtpDepayloader::new();

        let mut completed_frames: Vec<Vec<u8>> = Vec::with_capacity(16);

        println!(
            "\x1b[1;32m[usb-ingress]\x1b[0m Reading H.264 stream from USB Bulk fd {} -> HDMI Display active.",
            read_fd
        );

        let mut total_bytes = 0u64;
        let mut last_log = std::time::Instant::now();

        let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(2);
        let run_read = running.clone();
        let read_handle = thread::spawn(move || {
            let mut buffer = [0u8; 65536];

            while run_read.load(Ordering::SeqCst) {
                match stream_file.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        let chunk = buffer[..n].to_vec();
                        if tx.send(chunk).is_err() {
                            break;
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => {
                        eprintln!("\x1b[1;31m[usb-ingress]\x1b[0m USB Bulk stream read error: {}", e);
                        break;
                    }
                }
            }
        });

        while running.load(Ordering::SeqCst) {
            match rx.recv_timeout(Duration::from_millis(5)) {
                Ok(chunk_vec) => {
                    let chunk = &chunk_vec[..];

                    // Auto-detect framing format on first chunk if needed
                    match framing_mode {
                        IngressFramingMode::AutoDetect => {
                            if chunk.len() >= 4 && (chunk.starts_with(&[0, 0, 0, 1]) || chunk.starts_with(&[0, 0, 1])) {
                                println!("\x1b[1;32m[usb-ingress]\x1b[0m Pure Native H.264 Annex-B Direct Stream active (Lossless, Zero-Artifacts).");
                                framing_mode = IngressFramingMode::AnnexB;
                                annexb_assembler.push(chunk, &mut completed_frames);
                                for frame in &completed_frames {
                                    decoder.decode_chunk(frame, |frame_rgb565| {
                                        display.render_frame(frame_rgb565);
                                    });
                                }
                                completed_frames.clear();
                            } else {
                                println!("\x1b[1;36m[usb-ingress]\x1b[0m Detected RFC 4571 length-framed RTP stream (Ultra-Low Latency).");
                                framing_mode = IngressFramingMode::Rfc4571;
                                rfc_assembler.push(chunk, &mut rtp_depayloader, &mut completed_frames);
                                for frame in &completed_frames {
                                    decoder.decode_chunk(frame, |frame_rgb565| {
                                        display.render_frame(frame_rgb565);
                                    });
                                }
                                completed_frames.clear();
                            }
                        }
                        IngressFramingMode::AnnexB => {
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

                    total_bytes += chunk.len() as u64;
                    if last_log.elapsed() >= Duration::from_secs(5) {
                        let mb = (total_bytes as f64) / (1024.0 * 1024.0);
                        println!(
                            "\x1b[1;34m[usb-ingress]\x1b[0m Total received via USB Bulk: {:.2} MB",
                            mb
                        );
                        last_log = std::time::Instant::now();
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // USB endpoint idle: flush any assembled AU and ready decoded frames to screen immediately
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
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
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
                    break;
                }
            }
        }

        let _ = read_handle.join();
        println!("\x1b[1;33m[usb-ingress]\x1b[0m Ingress worker stopped.");
    }
}

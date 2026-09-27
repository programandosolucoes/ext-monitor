//! Native Broadcom VideoCore IV V4L2 M2M Hardware Decoder & Framebuffer Direct Sink
//!
//! Direct Linux kernel hardware decoding without GStreamer or external multimedia frameworks.
//! Interacts directly with:
//! 1. `/dev/video10`: V4L2 Memory-to-Memory (M2M) hardware H.264 decoder (`bcm2835-codec`).
//! 2. `/dev/fb0`: Hardware HDMI Framebuffer (1280x720 16-bit RGB565 / `vc4drmfb`).
//!
//! Architecture:
//! - Multiplanar Queue 10 (OUTPUT): Receives incoming RFC 6184 H.264 NAL units.
//! - Multiplanar Queue 9 (CAPTURE): Decodes directly to RGB565 (`V4L2_PIX_FMT_RGB565` / `RGBP`).
//! - DMA Zero-Latency Framebuffer Blit: Decoded RGB565 frames are written directly to `/dev/fb0`.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

#![allow(dead_code)]

use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::net::UdpSocket;
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

// --- Linux V4L2 Constants & IOCTLs (ARM 32-bit Architecture) ---
const VIDIOC_QUERYCAP: libc::c_ulong = 0x80685600;
const VIDIOC_S_FMT_204: libc::c_ulong = 0xc0cc5605;
const VIDIOC_REQBUFS: libc::c_ulong = 0xc0145608;
const VIDIOC_QUERYBUF: libc::c_ulong = 0xc0505609;
const VIDIOC_QBUF: libc::c_ulong = 0xc050560f;
const VIDIOC_DQBUF: libc::c_ulong = 0xc0505611;
const VIDIOC_STREAMON: libc::c_ulong = 0x40045612;
const VIDIOC_STREAMOFF: libc::c_ulong = 0x40045613;

const V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE: u32 = 9;  // Decoded raw output frames (RGB565)
const V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE: u32 = 10; // Input compressed stream (H.264)
const V4L2_MEMORY_MMAP: u32 = 1;

// Pixel Formats (FourCC)
const V4L2_PIX_FMT_H264: u32 = 0x34363248;   // 'H264'
const V4L2_PIX_FMT_RGB565: u32 = 0x50424752; // 'RGBP' (16-bit RGB 5-6-5)

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct V4l2Capability {
    driver: [u8; 16],
    card: [u8; 32],
    bus_info: [u8; 32],
    version: u32,
    capabilities: u32,
    device_caps: u32,
    reserved: [u32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct V4l2PlanePixFormat {
    sizeimage: u32,
    bytesperline: u32,
    reserved: [u16; 6],
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct V4l2PixFormatMplane {
    width: u32,
    height: u32,
    pixelformat: u32,
    field: u32,
    colorspace: u32,
    plane_fmt: [V4l2PlanePixFormat; 8],
    num_planes: u8,
    flags: u8,
    union_or_reserved: [u8; 10],
}

#[repr(C)]
struct V4l2Format {
    buf_type: u32,
    fmt: [u8; 200],
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct V4l2RequestBuffers {
    count: u32,
    buf_type: u32,
    memory: u32,
    capabilities: u32,
    flags: u8,
    reserved: [u8; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct V4l2Plane {
    bytesused: u32,
    length: u32,
    mem_offset: u32,
    data_offset: u32,
    reserved: [u32; 11],
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct V4l2Buffer {
    index: u32,
    buf_type: u32,
    bytesused: u32,
    flags: u32,
    field: u32,
    _pad: u32,
    timestamp_sec: i64,
    timestamp_usec: i64,
    timecode: [u32; 4],
    sequence: u32,
    memory: u32,
    planes_ptr: u32, // Pointer to V4l2Plane array on 32-bit ARM (offset 64)
    length: u32,     // Number of planes = 1 (offset 68)
    reserved2: u32,
    reserved: u32,
}

/// Active hardware decoder session for VideoCore IV
pub struct V4l2DecoderSession {
    pub video_fd: RawFd,
    pub fb_fd: RawFd,
    pub fb_ptr: *mut u8,
    pub fb_size: usize,
    pub out_ptrs: Vec<*mut u8>,
    pub out_lens: Vec<usize>,
    pub free_out_indices: Vec<u32>,
    pub cap_ptrs: Vec<*mut u8>,
    pub cap_lens: Vec<usize>,
    pub frames_decoded: u64,
    pub start_time: Instant,
}

impl V4l2DecoderSession {
    pub fn new() -> Option<Self> {
        use std::os::unix::fs::OpenOptionsExt;

        let vfile = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open("/dev/video10")
            .ok()?;
        let fbfile = OpenOptions::new().read(true).write(true).open("/dev/fb0").ok()?;
        let video_fd = vfile.as_raw_fd();
        let fb_fd = fbfile.as_raw_fd();

        std::mem::forget(vfile);
        std::mem::forget(fbfile);

        // Put VT1 into graphics mode to suppress console cursor and text rendering
        if let Ok(tty1) = OpenOptions::new().read(true).write(true).open("/dev/tty1") {
            const KDSETMODE: libc::c_ulong = 0x4B3A;
            const KD_GRAPHICS: libc::c_ulong = 0x01;
            unsafe { libc::ioctl(tty1.as_raw_fd(), KDSETMODE, KD_GRAPHICS); }
        }

        // Force framebuffer unblank
        const FBIOBLANK: libc::c_ulong = 0x4611;
        unsafe { libc::ioctl(fb_fd, FBIOBLANK, 0 as libc::c_int); }

        // Memory map /dev/fb0 directly for zero-copy DMA-like blits
        let fb_size = 1280 * 720 * 2;
        let fb_ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                fb_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fb_fd,
                0,
            ) as *mut u8
        };

        // 1. Set OUTPUT Format (H.264 1280x720)
        let mut out_fmt = V4l2Format {
            buf_type: V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE,
            fmt: [0u8; 200],
        };
        let out_pix = unsafe { &mut *(&mut out_fmt.fmt as *mut _ as *mut V4l2PixFormatMplane) };
        out_pix.width = 1280;
        out_pix.height = 720;
        out_pix.pixelformat = V4L2_PIX_FMT_H264;
        out_pix.num_planes = 1;
        out_pix.plane_fmt[0].sizeimage = 512 * 1024;
        if unsafe { libc::ioctl(video_fd, VIDIOC_S_FMT_204, &mut out_fmt) } != 0 {
            return None;
        }

        // 2. Set CAPTURE Format (RGB565 1280x720)
        let mut cap_fmt = V4l2Format {
            buf_type: V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE,
            fmt: [0u8; 200],
        };
        let cap_pix = unsafe { &mut *(&mut cap_fmt.fmt as *mut _ as *mut V4l2PixFormatMplane) };
        cap_pix.width = 1280;
        cap_pix.height = 720;
        cap_pix.pixelformat = V4L2_PIX_FMT_RGB565;
        cap_pix.num_planes = 1;
        cap_pix.plane_fmt[0].sizeimage = 1280 * 720 * 2;
        if unsafe { libc::ioctl(video_fd, VIDIOC_S_FMT_204, &mut cap_fmt) } != 0 {
            return None;
        }

        // 3. REQBUFS OUTPUT (16 buffers to prevent starvation between headers and slices)
        let mut req_out = V4l2RequestBuffers {
            count: 16,
            buf_type: V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE,
            memory: V4L2_MEMORY_MMAP,
            ..Default::default()
        };
        if unsafe { libc::ioctl(video_fd, VIDIOC_REQBUFS, &mut req_out) } != 0 {
            return None;
        }

        let mut out_ptrs = Vec::new();
        let mut out_lens = Vec::new();
        let mut free_out_indices = Vec::new();
        for i in 0..req_out.count {
            let mut plane: V4l2Plane = Default::default();
            let mut buf = V4l2Buffer {
                index: i,
                buf_type: V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE,
                memory: V4L2_MEMORY_MMAP,
                planes_ptr: &mut plane as *mut _ as u32,
                length: 1,
                ..Default::default()
            };
            if unsafe { libc::ioctl(video_fd, VIDIOC_QUERYBUF, &mut buf) } != 0 {
                return None;
            }
            let ptr = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    plane.length as usize,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED,
                    video_fd,
                    plane.mem_offset as libc::off_t,
                ) as *mut u8
            };
            out_ptrs.push(ptr);
            out_lens.push(plane.length as usize);
            free_out_indices.push(i);
        }

        // 4. REQBUFS CAPTURE (6 buffers for smooth VPU drain & VSYNC pacing)
        let mut req_cap = V4l2RequestBuffers {
            count: 6,
            buf_type: V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE,
            memory: V4L2_MEMORY_MMAP,
            ..Default::default()
        };
        if unsafe { libc::ioctl(video_fd, VIDIOC_REQBUFS, &mut req_cap) } != 0 {
            return None;
        }

        let mut cap_ptrs = Vec::new();
        let mut cap_lens = Vec::new();
        for i in 0..req_cap.count {
            let mut plane: V4l2Plane = Default::default();
            let mut buf = V4l2Buffer {
                index: i,
                buf_type: V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE,
                memory: V4L2_MEMORY_MMAP,
                planes_ptr: &mut plane as *mut _ as u32,
                length: 1,
                ..Default::default()
            };
            if unsafe { libc::ioctl(video_fd, VIDIOC_QUERYBUF, &mut buf) } != 0 {
                return None;
            }
            let ptr = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    plane.length as usize,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED,
                    video_fd,
                    plane.mem_offset as libc::off_t,
                ) as *mut u8
            };
            cap_ptrs.push(ptr);
            cap_lens.push(plane.length as usize);

            // Queue initial capture buffer
            unsafe { libc::ioctl(video_fd, VIDIOC_QBUF, &mut buf) };
        }

        // 5. STREAMON on both queues
        let mut out_type = V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE;
        let mut cap_type = V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE;
        unsafe {
            libc::ioctl(video_fd, VIDIOC_STREAMON, &mut out_type);
            libc::ioctl(video_fd, VIDIOC_STREAMON, &mut cap_type);
        }

        println!("\x1b[1;32m[native-v4l2]\x1b[0m VideoCore IV Hardware VPU Decoder online (1280x720 RGB565 -> /dev/fb0)");

        Some(Self {
            video_fd,
            fb_fd,
            fb_ptr,
            fb_size,
            out_ptrs,
            out_lens,
            free_out_indices,
            cap_ptrs,
            cap_lens,
            frames_decoded: 0,
            start_time: Instant::now(),
        })
    }

    /// Reclaims any OUTPUT buffers that the VideoCore IV hardware has finished processing (Non-blocking)
    fn reclaim_output_buffers(&mut self) {
        loop {
            let mut out_dq_plane = V4l2Plane::default();
            let mut out_dq_buf = V4l2Buffer {
                buf_type: V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE,
                memory: V4L2_MEMORY_MMAP,
                planes_ptr: &mut out_dq_plane as *mut _ as u32,
                length: 1,
                ..Default::default()
            };
            if unsafe { libc::ioctl(self.video_fd, VIDIOC_DQBUF, &mut out_dq_buf) } == 0 {
                let idx = out_dq_buf.index;
                if !self.free_out_indices.contains(&idx) {
                    self.free_out_indices.push(idx);
                }
            } else {
                break;
            }
        }
    }

    /// Drains all available decoded frames from CAPTURE queue and writes directly to /dev/fb0 (Non-blocking)
    pub fn drain_decoded_frames(&mut self) {
        loop {
            let mut cap_plane = V4l2Plane::default();
            let mut cap_buf = V4l2Buffer {
                buf_type: V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE,
                memory: V4L2_MEMORY_MMAP,
                planes_ptr: &mut cap_plane as *mut _ as u32,
                length: 1,
                ..Default::default()
            };
            let ret = unsafe { libc::ioctl(self.video_fd, VIDIOC_DQBUF, &mut cap_buf) };
            if ret == 0 {
                let cap_idx = cap_buf.index as usize;
                if cap_idx < self.cap_ptrs.len() {
                    let frame_ptr = self.cap_ptrs[cap_idx];
                    let frame_size = 1280 * 720 * 2; // 1,843,200 bytes

                    // Direct hardware framebuffer blit to HDMI (/dev/fb0)
                    unsafe {
                        if !self.fb_ptr.is_null() && self.fb_ptr != libc::MAP_FAILED as *mut u8 {
                            std::ptr::copy_nonoverlapping(frame_ptr, self.fb_ptr, frame_size);
                        } else {
                            libc::pwrite(self.fb_fd, frame_ptr as *const libc::c_void, frame_size, 0);
                        }
                    }

                    self.frames_decoded += 1;
                    if self.frames_decoded % 120 == 1 {
                        println!("\x1b[1;32m[native-v4l2]\x1b[0m Hardware VPU decoded & displayed {} frames to HDMI (1280x720@60)", self.frames_decoded);
                    }
                }

                // Re-queue capture buffer immediately
                unsafe { libc::ioctl(self.video_fd, VIDIOC_QBUF, &mut cap_buf) };
            } else {
                break;
            }
        }
    }

    pub fn decode_nal(&mut self, nal: &[u8]) {
        if nal.is_empty() || self.out_ptrs.is_empty() { return; }

        self.reclaim_output_buffers();
        self.drain_decoded_frames();

        // If no output buffer is free, wait and actively drain decoded frames until hardware frees a buffer
        if self.free_out_indices.is_empty() {
            for _ in 0..30 {
                thread::sleep(Duration::from_millis(1));
                self.drain_decoded_frames();
                self.reclaim_output_buffers();
                if !self.free_out_indices.is_empty() {
                    break;
                }
            }
        }

        if let Some(idx) = self.free_out_indices.pop() {
            let uidx = idx as usize;
            let max_len = self.out_lens[uidx];
            let copy_len = nal.len().min(max_len);
            unsafe {
                std::ptr::copy_nonoverlapping(nal.as_ptr(), self.out_ptrs[uidx], copy_len);
            }

            let mut plane = V4l2Plane {
                bytesused: copy_len as u32,
                length: max_len as u32,
                ..Default::default()
            };

            let elapsed = self.start_time.elapsed();
            let mut flags = 0x00002000; // V4L2_BUF_FLAG_TIMESTAMP_MONOTONIC

            // Check if Access Unit contains a keyframe (IDR slice = 5, SPS = 7)
            let mut is_keyframe = false;
            let mut i = 0;
            while i + 4 < nal.len() {
                if nal[i] == 0 && nal[i + 1] == 0 {
                    let (offset, plen) = if nal[i + 2] == 1 {
                        (i, 3)
                    } else if i + 3 < nal.len() && nal[i + 2] == 0 && nal[i + 3] == 1 {
                        (i, 4)
                    } else {
                        (0, 0)
                    };
                    if plen > 0 && offset + plen < nal.len() {
                        let t = nal[offset + plen] & 0x1F;
                        if t == 5 || t == 7 {
                            is_keyframe = true;
                            break;
                        }
                    }
                }
                i += 1;
            }
            if is_keyframe {
                flags |= 0x00000008; // V4L2_BUF_FLAG_KEYFRAME
            }

            let mut buf = V4l2Buffer {
                index: idx,
                buf_type: V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE,
                memory: V4L2_MEMORY_MMAP,
                planes_ptr: &mut plane as *mut _ as u32,
                length: 1,
                timestamp_sec: elapsed.as_secs() as i64,
                timestamp_usec: elapsed.subsec_micros() as i64,
                flags,
                ..Default::default()
            };
            unsafe { libc::ioctl(self.video_fd, VIDIOC_QBUF, &mut buf) };
        } else {
            eprintln!("\x1b[1;31m[native-v4l2]\x1b[0m Warning: OUTPUT queue starved, dropping NAL ({} bytes)", nal.len());
        }

        self.drain_decoded_frames();
    }
}

/// Native V4L2 M2M Hardware Decoder Handle
pub struct NativeV4l2Decoder {
    running: Arc<AtomicBool>,
    worker_handle: Option<JoinHandle<()>>,
}

impl NativeV4l2Decoder {
    /// Probes whether `/dev/video10` (VideoCore IV decoder) is available
    pub fn is_supported() -> bool {
        if let Ok(file) = OpenOptions::new().read(true).write(true).open("/dev/video10") {
            let fd = file.as_raw_fd();
            let mut cap: V4l2Capability = unsafe { std::mem::zeroed() };
            let ret = unsafe { libc::ioctl(fd, VIDIOC_QUERYCAP, &mut cap) };
            if ret == 0 {
                let card = String::from_utf8_lossy(&cap.card);
                return card.contains("bcm2835") || card.contains("codec");
            }
        }
        false
    }

    /// Starts a native background decode loop from a UDP RTP socket
    pub fn start_udp_stream(port: u16) -> io::Result<Self> {
        println!("\x1b[1;32m[native-v4l2]\x1b[0m Initializing VideoCore IV V4L2 M2M H.264 Hardware Decoder on UDP port {}...", port);

        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();

        let worker_handle = thread::spawn(move || {
            Self::udp_decode_worker(port, r);
        });

        Ok(Self {
            running,
            worker_handle: Some(worker_handle),
        })
    }

    /// Starts a native background decode loop from a file descriptor (USB Bulk)
    pub fn start_fd_stream(fd: RawFd) -> io::Result<Self> {
        println!("\x1b[1;32m[native-v4l2]\x1b[0m Initializing VideoCore IV V4L2 M2M H.264 Hardware Decoder on USB Bulk fd {}...", fd);

        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();

        let worker_handle = thread::spawn(move || {
            Self::fd_decode_worker(fd, r);
        });

        Ok(Self {
            running,
            worker_handle: Some(worker_handle),
        })
    }

    /// Terminates the native decoding worker
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
        println!("\x1b[1;33m[native-v4l2]\x1b[0m Hardware Decoder stopped cleanly.");
    }

    fn udp_decode_worker(port: u16, running: Arc<AtomicBool>) {
        let sock = match UdpSocket::bind(format!("0.0.0.0:{}", port)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("\x1b[1;31m[native-v4l2]\x1b[0m Failed to bind UDP port {}: {}", port, e);
                return;
            }
        };

        // Increase OS receive buffer to 2MB to prevent dropped UDP packets during burst
        unsafe {
            use std::os::unix::io::AsRawFd;
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
        let mut buf = [0u8; 4096];
        let mut nal_buffer = Vec::with_capacity(65536);
        let mut parsed_nals = Vec::new();

        let mut session = match V4l2DecoderSession::new() {
            Some(s) => s,
            None => {
                eprintln!("\x1b[1;31m[native-v4l2]\x1b[0m Failed to initialize V4L2 M2M hardware decoder session.");
                return;
            }
        };

        println!("\x1b[1;32m[native-v4l2]\x1b[0m Listening for RTP H.264 stream on UDP port {} -> Displaying to HDMI...", port);

        while running.load(Ordering::SeqCst) {
            match sock.recv(&mut buf) {
                Ok(n) if n > 12 => {
                    // RTP Header is 12 bytes
                    let payload = &buf[12..n];
                    parsed_nals.clear();
                    parse_rtp_h264_payload(payload, &mut nal_buffer, &mut parsed_nals);
                    for nal in &parsed_nals {
                        session.decode_nal(nal);
                    }
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut => {
                    session.drain_decoded_frames();
                    thread::sleep(Duration::from_micros(500));
                }
                Err(e) => {
                    eprintln!("\x1b[1;31m[native-v4l2]\x1b[0m Socket read error: {}", e);
                    break;
                }
            }
        }
    }

    fn fd_decode_worker(read_fd: RawFd, running: Arc<AtomicBool>) {
        // Set read_fd to non-blocking mode to support poll with timeout
        unsafe {
            let flags = libc::fcntl(read_fd, libc::F_GETFL);
            if flags >= 0 {
                libc::fcntl(read_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
        }

        let mut stream_file = unsafe { File::from_raw_fd_unchecked(read_fd) };
        let mut buf = [0u8; 16384];
        let mut stream_buffer = Vec::with_capacity(262144);
        let mut nal_buffer = Vec::with_capacity(65536);
        let mut parsed_nals = Vec::with_capacity(16);
        let mut parser = AnnexBStreamParser::new();
        let mut last_packet_time = Instant::now();
        let mut pending_flush = false;

        let mut session = match V4l2DecoderSession::new() {
            Some(s) => s,
            None => {
                eprintln!("\x1b[1;31m[native-v4l2]\x1b[0m Failed to initialize V4L2 M2M hardware decoder session.");
                return;
            }
        };

        println!("\x1b[1;32m[native-v4l2]\x1b[0m Reading H.264 stream with RFC 4571 Framing & RTP Marker Bit from USB Bulk fd {} -> Displaying to HDMI...", read_fd);

        let mut pfd = libc::pollfd {
            fd: read_fd,
            events: libc::POLLIN,
            revents: 0,
        };

        while running.load(Ordering::SeqCst) {
            pfd.revents = 0;
            let poll_ret = unsafe { libc::poll(&mut pfd, 1, 5) }; // 5ms poll timeout

            if poll_ret > 0 && (pfd.revents & libc::POLLIN) != 0 {
                match stream_file.read(&mut buf) {
                    Ok(0) => {
                        session.drain_decoded_frames();
                        thread::sleep(Duration::from_millis(2));
                    }
                    Ok(n) => {
                        last_packet_time = Instant::now();
                        pending_flush = true;
                        stream_buffer.extend_from_slice(&buf[..n]);

                        // Process RFC 4571 RTP frames: 2-byte big-endian length + RTP packet (version 2 = 0x80)
                        while stream_buffer.len() >= 4 {
                            // Check if stream_buffer matches RFC 4571 RTP packet (V=2 -> 0x80)
                            if (stream_buffer[2] & 0xC0) == 0x80 {
                                let packet_len = u16::from_be_bytes([stream_buffer[0], stream_buffer[1]]) as usize;
                                if packet_len >= 12 && packet_len <= 65535 {
                                    if stream_buffer.len() < 2 + packet_len {
                                        break; // Incomplete packet, wait for next USB chunk
                                    }
                                    let rtp_packet = &stream_buffer[2..2 + packet_len];
                                    let is_marker = (rtp_packet[1] & 0x80) != 0;
                                    let payload = &rtp_packet[12..];

                                    parsed_nals.clear();
                                    parse_rtp_h264_payload(payload, &mut nal_buffer, &mut parsed_nals);
                                    for nal in &parsed_nals {
                                        session.decode_nal(nal);
                                    }

                                    if is_marker {
                                        // End of video frame signaled by RTP Marker bit
                                        session.drain_decoded_frames();
                                        pending_flush = false;
                                    }

                                    stream_buffer.drain(0..2 + packet_len);
                                    continue;
                                }
                            } else if stream_buffer.len() >= 12 && &stream_buffer[0..4] == b"EXMO" {
                                let flags = stream_buffer[4];
                                let payload_len = u32::from_be_bytes([stream_buffer[8], stream_buffer[9], stream_buffer[10], stream_buffer[11]]) as usize;
                                if stream_buffer.len() < 12 + payload_len {
                                    break;
                                }
                                let payload = &stream_buffer[12..12 + payload_len];
                                session.decode_nal(payload);
                                if (flags & 0x01) != 0 {
                                    session.drain_decoded_frames();
                                    pending_flush = false;
                                }
                                stream_buffer.drain(0..12 + payload_len);
                                continue;
                            }

                            // Raw Annex-B fallback: process discrete NALs
                            parsed_nals.clear();
                            parser.push_and_extract(&stream_buffer, &mut parsed_nals);
                            for nal in &parsed_nals {
                                session.decode_nal(nal);
                            }
                            stream_buffer.clear();
                            break;
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        session.drain_decoded_frames();
                        // CRITICAL: FunctionFS data endpoints lack kernel poll handler.
                        // Sleeping 500µs prevents 100% ARM CPU busy-loop saturation.
                        thread::sleep(Duration::from_micros(500));
                    }
                    Err(e) => {
                        eprintln!("\x1b[1;31m[native-v4l2]\x1b[0m USB Bulk stream read error: {}", e);
                        break;
                    }
                }
            } else if poll_ret == 0 {
                session.drain_decoded_frames();
                thread::sleep(Duration::from_micros(500));
            } else {
                session.drain_decoded_frames();
                thread::sleep(Duration::from_micros(500));
            }

            // Flush DPB hardware pipeline if idle: inject AUD (Access Unit Delimiter) to commit any trapped frame
            if pending_flush && last_packet_time.elapsed() >= Duration::from_millis(20) {
                parsed_nals.clear();
                parser.flush_pending(&mut parsed_nals);
                for nal in &parsed_nals {
                    session.decode_nal(nal);
                }
                // H.264 Access Unit Delimiter (AUD NAL type 9) commits any pending slice in VideoCore IV VPU
                static AUD_DELIMITER: [u8; 6] = [0x00, 0x00, 0x00, 0x01, 0x09, 0xF0];
                session.decode_nal(&AUD_DELIMITER);
                session.drain_decoded_frames();
                pending_flush = false;
            }
        }
    }
}

/// Helper to assemble discrete, complete Annex-B Access Units (full video frames) from a continuous byte stream
pub struct AnnexBStreamParser {
    accumulator: Vec<u8>,
    current_frame: Vec<u8>,
    has_slice: bool,
}

impl AnnexBStreamParser {
    pub fn new() -> Self {
        Self {
            accumulator: Vec::with_capacity(256 * 1024),
            current_frame: Vec::with_capacity(256 * 1024),
            has_slice: false,
        }
    }

    pub fn push_and_extract(&mut self, data: &[u8], frames: &mut Vec<Vec<u8>>) {
        if data.is_empty() {
            return;
        }
        self.accumulator.extend_from_slice(data);
        self.extract_frames(frames);
    }

    pub fn flush_pending(&mut self, frames: &mut Vec<Vec<u8>>) {
        // If accumulator contains an in-progress NAL unit without a subsequent start code,
        // move it into current_frame so it is never trapped waiting for next mouse motion
        if let Some((first_offset, _)) = Self::find_start_code(&self.accumulator, 0) {
            let nal = &self.accumulator[first_offset..];
            let start_prefix_len = if nal.starts_with(&[0, 0, 0, 1]) { 4 } else { 3 };
            if nal.len() > start_prefix_len {
                let nal_type = nal[start_prefix_len] & 0x1F;
                if nal_type == 1 || nal_type == 5 {
                    self.has_slice = true;
                }
                if nal.starts_with(&[0, 0, 0, 1]) {
                    self.current_frame.extend_from_slice(nal);
                } else {
                    self.current_frame.extend_from_slice(&[0, 0, 0, 1]);
                    self.current_frame.extend_from_slice(&nal[start_prefix_len..]);
                }
            }
            self.accumulator.clear();
        }

        // Flush completed frame to decoder
        if !self.current_frame.is_empty() {
            frames.push(std::mem::take(&mut self.current_frame));
            self.current_frame.reserve(256 * 1024);
            self.has_slice = false;
        }
    }

    fn extract_frames(&mut self, frames: &mut Vec<Vec<u8>>) {
        // Discard any garbage before the first start code
        if let Some((first_offset, _)) = Self::find_start_code(&self.accumulator, 0) {
            if first_offset > 0 {
                self.accumulator.drain(0..first_offset);
            }
        } else {
            if self.accumulator.len() > 3 {
                let keep_start = self.accumulator.len() - 3;
                self.accumulator.drain(0..keep_start);
            }
            return;
        }

        // Loop extracting complete NAL units from the accumulator
        loop {
            if self.accumulator.len() < 4 {
                break;
            }

            let start_prefix_len = if self.accumulator.starts_with(&[0, 0, 0, 1]) {
                4
            } else if self.accumulator.starts_with(&[0, 0, 1]) {
                3
            } else {
                break;
            };

            // Search for the next start code to know where the current NAL ends
            if let Some((next_offset, _)) = Self::find_start_code(&self.accumulator, start_prefix_len) {
                let nal = &self.accumulator[0..next_offset];
                
                // Inspect NAL type and slice header
                let header_idx = start_prefix_len;
                if header_idx < nal.len() {
                    let nal_type = nal[header_idx] & 0x1F;
                    let is_first_slice = if (nal_type == 1 || nal_type == 5) && nal.len() > header_idx + 1 {
                        (nal[header_idx + 1] & 0x80) != 0
                    } else {
                        false
                    };

                    let is_new_frame = nal_type == 9 // AUD
                        || nal_type == 7 // SPS
                        || (nal_type == 8 && self.has_slice) // PPS following existing frame
                        || is_first_slice;

                    // If this NAL starts a new frame and the current frame already has video slices:
                    if is_new_frame && self.has_slice && !self.current_frame.is_empty() {
                        let completed = std::mem::take(&mut self.current_frame);
                        self.current_frame.reserve(256 * 1024);
                        frames.push(completed);
                        self.has_slice = false;
                    }

                    if nal_type == 1 || nal_type == 5 {
                        self.has_slice = true;
                    }
                }

                // Append this NAL unit (with standard 4-byte 00 00 00 01 start code) to current frame
                if nal.starts_with(&[0, 0, 0, 1]) {
                    self.current_frame.extend_from_slice(nal);
                } else if nal.starts_with(&[0, 0, 1]) {
                    self.current_frame.push(0);
                    self.current_frame.extend_from_slice(nal);
                } else {
                    self.current_frame.extend_from_slice(&[0, 0, 0, 1]);
                    self.current_frame.extend_from_slice(nal);
                }

                self.accumulator.drain(0..next_offset);
            } else {
                // Next start code not found yet. Current NAL is incomplete, wait for more data.
                break;
            }
        }
    }

    fn find_start_code(data: &[u8], start: usize) -> Option<(usize, usize)> {
        if data.len() < start + 3 {
            return None;
        }
        let limit = data.len();
        let mut i = start;
        while i + 2 < limit {
            if data[i] == 0 && data[i + 1] == 0 {
                if data[i + 2] == 1 {
                    return Some((i, 3));
                }
                if i + 3 < limit && data[i + 2] == 0 && data[i + 3] == 1 {
                    return Some((i, 4));
                }
            }
            i += 1;
        }
        None
    }
}

/// Parses RTP H.264 payload into raw NAL units (Single NAL, STAP-A Aggregation, or FU-A Fragmentation)
fn parse_rtp_h264_payload(payload: &[u8], accumulator: &mut Vec<u8>, out_nals: &mut Vec<Vec<u8>>) {
    if payload.is_empty() {
        return;
    }

    let nal_type = payload[0] & 0x1F;

    if nal_type >= 1 && nal_type <= 23 {
        // Single NAL unit packet
        let mut nal = Vec::with_capacity(payload.len() + 4);
        nal.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        nal.extend_from_slice(payload);
        out_nals.push(nal);
    } else if nal_type == 24 {
        // STAP-A Aggregated NAL units (RFC 6184: used by GStreamer rtph264pay for SPS + PPS)
        let mut offset = 1;
        while offset + 2 <= payload.len() {
            let nalu_size = u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;
            offset += 2;
            if offset + nalu_size <= payload.len() {
                let nalu = &payload[offset..offset + nalu_size];
                let mut nal = Vec::with_capacity(nalu_size + 4);
                nal.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                nal.extend_from_slice(nalu);
                out_nals.push(nal);
                offset += nalu_size;
            } else {
                break;
            }
        }
    } else if nal_type == 28 {
        // FU-A Fragmented NAL unit
        if payload.len() < 2 {
            return;
        }
        let fu_header = payload[1];
        let start_bit = (fu_header & 0x80) != 0;
        let end_bit = (fu_header & 0x40) != 0;
        let reconstructed_nal_type = (payload[0] & 0xE0) | (fu_header & 0x1F);

        if start_bit {
            accumulator.clear();
            accumulator.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
            accumulator.push(reconstructed_nal_type);
            accumulator.extend_from_slice(&payload[2..]);
        } else {
            accumulator.extend_from_slice(&payload[2..]);
            if end_bit {
                out_nals.push(accumulator.clone());
                accumulator.clear();
            }
        }
    }
}

trait FromRawFdUnchecked {
    unsafe fn from_raw_fd_unchecked(fd: RawFd) -> File;
}

impl FromRawFdUnchecked for File {
    unsafe fn from_raw_fd_unchecked(fd: RawFd) -> File {
        use std::os::unix::io::FromRawFd;
        File::from_raw_fd(fd)
    }
}

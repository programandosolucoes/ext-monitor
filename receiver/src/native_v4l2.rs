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
use std::time::Duration;

// --- Linux V4L2 Constants & IOCTLs (ARM 32-bit Architecture) ---
const VIDIOC_QUERYCAP: libc::c_ulong = 0x80685600;
const VIDIOC_S_FMT_204: libc::c_ulong = 0xc0cc5605;
const VIDIOC_REQBUFS: libc::c_ulong = 0xc0145608;
const VIDIOC_QUERYBUF: libc::c_ulong = 0xc0445609;
const VIDIOC_QBUF: libc::c_ulong = 0xc044560f;
const VIDIOC_DQBUF: libc::c_ulong = 0xc0445611;
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
struct V4l2Timeval {
    tv_sec: libc::c_long,
    tv_usec: libc::c_long,
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
    timestamp: V4l2Timeval,
    timecode: [u32; 4],
    sequence: u32,
    memory: u32,
    planes_ptr: u32, // Pointer to V4l2Plane array on 32-bit ARM
    length: u32,     // Number of planes = 1
    reserved2: u32,
    reserved: u32,
}

/// Active hardware decoder session for VideoCore IV
pub struct V4l2DecoderSession {
    pub video_fd: RawFd,
    pub fb_fd: RawFd,
    pub out_ptrs: Vec<*mut u8>,
    pub out_lens: Vec<usize>,
    pub free_out_indices: Vec<u32>,
    pub cap_ptrs: Vec<*mut u8>,
    pub cap_lens: Vec<usize>,
}

impl V4l2DecoderSession {
    pub fn new() -> Option<Self> {
        let vfile = OpenOptions::new().read(true).write(true).open("/dev/video10").ok()?;
        let fbfile = OpenOptions::new().read(true).write(true).open("/dev/fb0").ok()?;
        let video_fd = vfile.as_raw_fd();
        let fb_fd = fbfile.as_raw_fd();

        std::mem::forget(vfile);
        std::mem::forget(fbfile);

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

        // 3. REQBUFS OUTPUT (8 buffers to prevent starvation between headers and slices)
        let mut req_out = V4l2RequestBuffers {
            count: 8,
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

        // 4. REQBUFS CAPTURE (4 buffers)
        let mut req_cap = V4l2RequestBuffers {
            count: 4,
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
            out_ptrs,
            out_lens,
            free_out_indices,
            cap_ptrs,
            cap_lens,
        })
    }

    /// Reclaims any OUTPUT buffers that the VideoCore IV hardware has finished processing
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

    /// Drains all available decoded frames from CAPTURE queue and writes directly to /dev/fb0
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
                        libc::pwrite(self.fb_fd, frame_ptr as *const libc::c_void, frame_size, 0);
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

        // If no output buffer is free, wait briefly for hardware VPU to complete
        if self.free_out_indices.is_empty() {
            for _ in 0..5 {
                thread::sleep(Duration::from_millis(1));
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
            let mut buf = V4l2Buffer {
                index: idx,
                buf_type: V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE,
                memory: V4L2_MEMORY_MMAP,
                planes_ptr: &mut plane as *mut _ as u32,
                length: 1,
                ..Default::default()
            };
            unsafe { libc::ioctl(self.video_fd, VIDIOC_QBUF, &mut buf) };
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

        let _ = sock.set_read_timeout(Some(Duration::from_millis(200)));
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
                    thread::sleep(Duration::from_millis(2));
                }
                Err(e) => {
                    eprintln!("\x1b[1;31m[native-v4l2]\x1b[0m Socket read error: {}", e);
                    break;
                }
            }
        }
    }

    fn fd_decode_worker(read_fd: RawFd, running: Arc<AtomicBool>) {
        let mut stream_file = unsafe { File::from_raw_fd_unchecked(read_fd) };
        let mut buf = [0u8; 16384];

        let mut session = match V4l2DecoderSession::new() {
            Some(s) => s,
            None => {
                eprintln!("\x1b[1;31m[native-v4l2]\x1b[0m Failed to initialize V4L2 M2M hardware decoder session.");
                return;
            }
        };

        println!("\x1b[1;32m[native-v4l2]\x1b[0m Reading raw H.264 stream from USB Bulk fd {} -> Displaying to HDMI...", read_fd);

        while running.load(Ordering::SeqCst) {
            match stream_file.read(&mut buf) {
                Ok(0) => {
                    thread::sleep(Duration::from_millis(10));
                }
                Ok(n) => {
                    session.decode_nal(&buf[..n]);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    session.drain_decoded_frames();
                    thread::sleep(Duration::from_millis(2));
                }
                Err(e) => {
                    eprintln!("\x1b[1;31m[native-v4l2]\x1b[0m USB Bulk stream read error: {}", e);
                    break;
                }
            }
        }
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

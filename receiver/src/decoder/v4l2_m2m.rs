//! VideoCore IV V4L2 M2M Hardware Decoder Session
//!
//! Controls `/dev/video10` (bcm2835-codec) to decode complete H.264 Access Units (AUs)
//! directly into raw RGB565 frames with hardware acceleration on Raspberry Pi.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

use crate::decoder::v4l2_types::*;
use std::fs::{File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, RawFd};
use std::time::{Duration, Instant};

pub struct V4l2DecoderSession {
    _video_file: File,
    video_fd: RawFd,
    out_ptrs: Vec<*mut u8>,
    out_lens: Vec<usize>,
    free_out_indices: Vec<u32>,
    cap_ptrs: Vec<*mut u8>,
    cap_lens: Vec<usize>,
    frames_decoded: u64,
    start_time: Instant,
    negotiated_cap_fmt: u32,
    rgb565_buf: Vec<u8>,
    width: usize,
    height: usize,
}

impl V4l2DecoderSession {
    /// Probes whether `/dev/video10` is supported on this platform
    pub fn is_supported() -> bool {
        if let Ok(file) = OpenOptions::new().read(true).write(true).open("/dev/video10") {
            let fd = file.as_raw_fd();
            let mut cap: V4l2Capability = unsafe { std::mem::zeroed() };
            if unsafe { libc::ioctl(fd, VIDIOC_QUERYCAP, &mut cap) } == 0 {
                let card = String::from_utf8_lossy(&cap.card);
                return card.contains("bcm2835") || card.contains("codec");
            }
        }
        false
    }

    /// Initializes a new hardware decoder session for 1280x720 H.264 -> RGB565
    pub fn new(width: u32, height: u32) -> Option<Self> {
        let video_file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open("/dev/video10")
            .ok()?;
        let video_fd = video_file.as_raw_fd();
        unsafe {
            let flags = libc::fcntl(video_fd, libc::F_GETFL, 0);
            libc::fcntl(video_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }

        // 0. Enumerate and log supported formats on /dev/video10
        println!("\x1b[1;34m[v4l2-m2m]\x1b[0m Probing formats on /dev/video10:");
        for idx in 0..16 {
            let mut desc = V4l2FmtDesc {
                index: idx,
                buf_type: V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE,
                ..Default::default()
            };
            if unsafe { libc::ioctl(video_fd, VIDIOC_ENUM_FMT, &mut desc) } == 0 {
                let fourcc_bytes = desc.pixelformat.to_le_bytes();
                let fourcc = String::from_utf8_lossy(&fourcc_bytes);
                let name = String::from_utf8_lossy(&desc.description);
                println!("  [OUTPUT {}] '{}' - {}", idx, fourcc, name.trim_matches('\0'));
            } else {
                break;
            }
        }
        for idx in 0..16 {
            let mut desc = V4l2FmtDesc {
                index: idx,
                buf_type: V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE,
                ..Default::default()
            };
            if unsafe { libc::ioctl(video_fd, VIDIOC_ENUM_FMT, &mut desc) } == 0 {
                let fourcc_bytes = desc.pixelformat.to_le_bytes();
                let fourcc = String::from_utf8_lossy(&fourcc_bytes);
                let name = String::from_utf8_lossy(&desc.description);
                println!("  [CAPTURE {}] '{}' - {}", idx, fourcc, name.trim_matches('\0'));
            } else {
                break;
            }
        }

        // 1. Set OUTPUT Format (H.264)
        let mut out_fmt = V4l2Format {
            buf_type: V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE,
            fmt: [0u8; 200],
        };
        let out_pix = unsafe { &mut *(&mut out_fmt.fmt as *mut _ as *mut V4l2PixFormatMplane) };
        out_pix.width = width;
        out_pix.height = height;
        out_pix.pixelformat = V4L2_PIX_FMT_H264;
        out_pix.num_planes = 1;
        out_pix.plane_fmt[0].sizeimage = 512 * 1024;
        if unsafe { libc::ioctl(video_fd, VIDIOC_S_FMT, &mut out_fmt) } != 0 {
            let err = std::io::Error::last_os_error();
            eprintln!("\x1b[1;31m[v4l2-m2m]\x1b[0m Failed to set OUTPUT format on /dev/video10: {}", err);
            return None;
        }

        // 2. Negotiate CAPTURE Format: VideoCore IV VPU natively decodes H.264 into YUV420 (YU12) or NV12.
        // It cannot decode H.264 directly into RGB565; color_convert transforms YUV420 to RGB565 in SIMD (< 3% CPU).
        let candidate_fmts = [
            (V4L2_PIX_FMT_YUV420, "YUV420", width * height * 3 / 2),
            (V4L2_PIX_FMT_NV12, "NV12", width * height * 3 / 2),
            (V4L2_PIX_FMT_YUV420M, "YUV420M", width * height * 3 / 2),
            (V4L2_PIX_FMT_NV12M, "NV12M", width * height * 3 / 2),
            (V4L2_PIX_FMT_RGB565, "RGB565", width * height * 2),
        ];

        let mut negotiated_fmt = 0u32;
        for &(fmt_code, fmt_name, expected_size) in &candidate_fmts {
            let mut cap_fmt = V4l2Format {
                buf_type: V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE,
                fmt: [0u8; 200],
            };
            let cap_pix = unsafe { &mut *(&mut cap_fmt.fmt as *mut _ as *mut V4l2PixFormatMplane) };
            cap_pix.width = width;
            cap_pix.height = height;
            cap_pix.pixelformat = fmt_code;
            cap_pix.num_planes = 1;
            cap_pix.plane_fmt[0].sizeimage = expected_size;
            if unsafe { libc::ioctl(video_fd, VIDIOC_S_FMT, &mut cap_fmt) } == 0 {
                negotiated_fmt = fmt_code;
                println!(
                    "\x1b[1;32m[v4l2-m2m]\x1b[0m Negotiated CAPTURE format: {} (FourCC: '{}', sizeimage: {})",
                    fmt_name,
                    String::from_utf8_lossy(&fmt_code.to_le_bytes()),
                    cap_pix.plane_fmt[0].sizeimage
                );
                break;
            }
        }

        if negotiated_fmt == 0 {
            eprintln!("\x1b[1;31m[v4l2-m2m]\x1b[0m Could not negotiate any supported CAPTURE format on /dev/video10");
            return None;
        }

        // 3. REQBUFS & MMAP OUTPUT (20 buffers for smooth pipeline depth)
        let mut req_out = V4l2RequestBuffers {
            count: 20,
            buf_type: V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE,
            memory: V4L2_MEMORY_MMAP,
            ..Default::default()
        };
        if unsafe { libc::ioctl(video_fd, VIDIOC_REQBUFS, &mut req_out) } != 0 {
            return None;
        }

        let mut out_ptrs = Vec::with_capacity(req_out.count as usize);
        let mut out_lens = Vec::with_capacity(req_out.count as usize);
        let mut free_out_indices = Vec::with_capacity(req_out.count as usize);
        for i in 0..req_out.count {
            let mut plane = V4l2Plane::default();
            let mut buf = V4l2Buffer {
                index: i,
                buf_type: V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE,
                memory: V4L2_MEMORY_MMAP,
                planes_ptr: &mut plane as *mut _ as u32,
                length: 1,
                ..Default::default()
            };
            if unsafe { libc::ioctl(video_fd, VIDIOC_QUERYBUF, &mut buf) } != 0 {
                let err = std::io::Error::last_os_error();
                eprintln!("\x1b[1;31m[v4l2-m2m]\x1b[0m OUTPUT VIDIOC_QUERYBUF[{}] failed: {}", i, err);
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
            if ptr.is_null() || ptr == libc::MAP_FAILED as *mut u8 {
                return None;
            }
            out_ptrs.push(ptr);
            out_lens.push(plane.length as usize);
            free_out_indices.push(i);
        }

        // 4. REQBUFS & MMAP CAPTURE (20 buffers)
        let mut req_cap = V4l2RequestBuffers {
            count: 20,
            buf_type: V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE,
            memory: V4L2_MEMORY_MMAP,
            ..Default::default()
        };
        if unsafe { libc::ioctl(video_fd, VIDIOC_REQBUFS, &mut req_cap) } != 0 {
            return None;
        }

        let mut cap_ptrs = Vec::with_capacity(req_cap.count as usize);
        let mut cap_lens = Vec::with_capacity(req_cap.count as usize);
        for i in 0..req_cap.count {
            let mut plane = V4l2Plane::default();
            let mut buf = V4l2Buffer {
                index: i,
                buf_type: V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE,
                memory: V4L2_MEMORY_MMAP,
                planes_ptr: &mut plane as *mut _ as u32,
                length: 1,
                ..Default::default()
            };
            if unsafe { libc::ioctl(video_fd, VIDIOC_QUERYBUF, &mut buf) } != 0 {
                let err = std::io::Error::last_os_error();
                eprintln!("\x1b[1;31m[v4l2-m2m]\x1b[0m CAPTURE VIDIOC_QUERYBUF[{}] failed: {}", i, err);
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
            if ptr.is_null() || ptr == libc::MAP_FAILED as *mut u8 {
                return None;
            }
            cap_ptrs.push(ptr);
            cap_lens.push(plane.length as usize);

            // Pre-queue capture buffer
            unsafe {
                libc::ioctl(video_fd, VIDIOC_QBUF, &mut buf);
            }
        }

        // 5. STREAMON on both queues
        let mut out_type = V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE;
        let mut cap_type = V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE;
        unsafe {
            libc::ioctl(video_fd, VIDIOC_STREAMON, &mut out_type);
            libc::ioctl(video_fd, VIDIOC_STREAMON, &mut cap_type);
        }

        println!("\x1b[1;32m[v4l2-m2m]\x1b[0m VideoCore IV M2M Hardware Decoder pipeline running.");

        Some(Self {
            _video_file: video_file,
            video_fd,
            out_ptrs,
            out_lens,
            free_out_indices,
            cap_ptrs,
            cap_lens,
            frames_decoded: 0,
            start_time: Instant::now(),
            negotiated_cap_fmt: negotiated_fmt,
            rgb565_buf: vec![0u8; (width * height * 2) as usize],
            width: width as usize,
            height: height as usize,
        })
    }

    /// Feeds an encoded chunk/AU and drains decoded frames, ensuring the CAPTURE queue never starves the OUTPUT queue
    pub fn decode_chunk<F: FnMut(&[u8])>(&mut self, chunk: &[u8], mut on_frame: F) {
        if chunk.is_empty() || self.out_ptrs.is_empty() {
            return;
        }

        // 1. Drain ready decoded frames to free up VPU capture slots
        self.drain_decoded_frames(&mut on_frame);
        self.reclaim_output_buffers();

        // 2. Acquire a free output buffer, actively draining capture frames while waiting
        let idx = match self.free_out_indices.pop() {
            Some(i) => i,
            None => {
                let mut acquired = None;
                for _ in 0..100 {
                    self.drain_decoded_frames(&mut on_frame);
                    self.reclaim_output_buffers();
                    if let Some(i) = self.free_out_indices.pop() {
                        acquired = Some(i);
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_micros(500));
                }
                match acquired {
                    Some(i) => i,
                    None => {
                        eprintln!("\x1b[1;31m[v4l2-m2m]\x1b[0m Hardware VPU buffer wait timeout ({} bytes)", chunk.len());
                        return;
                    }
                }
            }
        };

        // 3. Submit chunk to hardware
        self.submit_to_buffer(idx, chunk);

        // 4. Wait briefly (up to 4ms) for hardware VPU to finish decoding and display immediately!
        // This eliminates the 1-frame latency gap and completely removes mouse trails!
        self.wait_and_drain(Duration::from_millis(4), &mut on_frame);
    }

    #[allow(dead_code)]
    pub fn queue_encoded_access_unit(&mut self, au: &[u8]) -> bool {
        if au.is_empty() || self.out_ptrs.is_empty() {
            return false;
        }

        self.reclaim_output_buffers();

        let idx = match self.free_out_indices.pop() {
            Some(i) => i,
            None => {
                // All output buffers are queued. Wait briefly for hardware to release one.
                for _ in 0..50 {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    self.reclaim_output_buffers();
                    if let Some(i) = self.free_out_indices.pop() {
                        return self.submit_to_buffer(i, au);
                    }
                }
                eprintln!("\x1b[1;31m[v4l2-m2m]\x1b[0m Warning: OUTPUT queue starved, dropping frame ({} bytes)", au.len());
                return false;
            }
        };

        self.submit_to_buffer(idx, au)
    }

    fn submit_to_buffer(&mut self, idx: u32, au: &[u8]) -> bool {
        let uidx = idx as usize;
        let max_len = self.out_lens[uidx];
        let copy_len = au.len().min(max_len);
        unsafe {
            std::ptr::copy_nonoverlapping(au.as_ptr(), self.out_ptrs[uidx], copy_len);
        }

        let mut plane = V4l2Plane {
            bytesused: copy_len as u32,
            length: max_len as u32,
            ..Default::default()
        };

        let elapsed = self.start_time.elapsed();
        let mut flags = V4L2_BUF_FLAG_TIMESTAMP_MONOTONIC;

        // Detect if Access Unit contains an IDR slice (type 5) or SPS (type 7)
        if Self::contains_keyframe(au) {
            flags |= V4L2_BUF_FLAG_KEYFRAME;
        }

        let mut buf = V4l2Buffer {
            index: idx,
            buf_type: V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE,
            memory: V4L2_MEMORY_MMAP,
            planes_ptr: &mut plane as *mut _ as u32,
            length: 1,
            timestamp: V4l2Timeval {
                tv_sec: elapsed.as_secs() as i32,
                tv_usec: elapsed.subsec_micros() as i32,
            },
            flags,
            ..Default::default()
        };

        let ret = unsafe { libc::ioctl(self.video_fd, VIDIOC_QBUF, &mut buf) };
        ret == 0
    }

    /// Drains all available decoded frames from the CAPTURE queue and hands them to the callback
    pub fn drain_decoded_frames<F: FnMut(&[u8])>(&mut self, mut on_frame: F) {
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
            if ret != 0 {
                break;
            }

            let cap_idx = cap_buf.index as usize;
            if cap_idx < self.cap_ptrs.len() {
                let plane_len = self.cap_lens[cap_idx];
                let slice = unsafe { std::slice::from_raw_parts(self.cap_ptrs[cap_idx], plane_len) };

                if self.negotiated_cap_fmt == V4L2_PIX_FMT_RGB565 {
                    on_frame(slice);
                } else if self.negotiated_cap_fmt == V4L2_PIX_FMT_YUV420 || self.negotiated_cap_fmt == V4L2_PIX_FMT_YUV420M {
                    crate::decoder::color_convert::yuv420_to_rgb565(slice, &mut self.rgb565_buf, self.width, self.height);
                    on_frame(&self.rgb565_buf);
                } else if self.negotiated_cap_fmt == V4L2_PIX_FMT_NV12 || self.negotiated_cap_fmt == V4L2_PIX_FMT_NV12M {
                    crate::decoder::color_convert::nv12_to_rgb565(slice, &mut self.rgb565_buf, self.width, self.height);
                    on_frame(&self.rgb565_buf);
                } else {
                    on_frame(slice);
                }

                self.frames_decoded += 1;
                if self.frames_decoded % 120 == 1 {
                    println!(
                        "\x1b[1;32m[v4l2-m2m]\x1b[0m Hardware VPU decoded & displayed {} frames (1280x720@60)",
                        self.frames_decoded
                    );
                }
            }

            // Re-queue capture buffer immediately
            let mut requeue_plane = V4l2Plane::default();
            let mut requeue_buf = V4l2Buffer {
                index: cap_buf.index,
                buf_type: V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE,
                memory: V4L2_MEMORY_MMAP,
                planes_ptr: &mut requeue_plane as *mut _ as u32,
                length: 1,
                ..Default::default()
            };
            let q_ret = unsafe { libc::ioctl(self.video_fd, VIDIOC_QBUF, &mut requeue_buf) };
            if q_ret != 0 {
                let err = std::io::Error::last_os_error();
                eprintln!("\x1b[1;31m[v4l2-m2m]\x1b[0m CAPTURE VIDIOC_QBUF[{}] failed: {}", cap_buf.index, err);
            }
        }
    }

    /// Waits up to `timeout` for the hardware VPU to finish decoding and drains all ready frames.
    /// Uses libc::poll on video_fd (which natively supports POLLIN on CAPTURE queue)
    pub fn wait_and_drain<F: FnMut(&[u8])>(&mut self, timeout: Duration, mut on_frame: F) {
        self.drain_decoded_frames(&mut on_frame);

        let ms = timeout.as_millis().min(20) as i32;
        if ms > 0 {
            let mut pfd = libc::pollfd {
                fd: self.video_fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let ret = unsafe { libc::poll(&mut pfd, 1, ms) };
            if ret > 0 && (pfd.revents & libc::POLLIN) != 0 {
                self.drain_decoded_frames(&mut on_frame);
            }
        }
    }

    /// Reclaims completed OUTPUT buffers from the decoder (Non-blocking)
    fn reclaim_output_buffers(&mut self) {
        loop {
            let mut out_plane = V4l2Plane::default();
            let mut out_buf = V4l2Buffer {
                buf_type: V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE,
                memory: V4L2_MEMORY_MMAP,
                planes_ptr: &mut out_plane as *mut _ as u32,
                length: 1,
                ..Default::default()
            };
            if unsafe { libc::ioctl(self.video_fd, VIDIOC_DQBUF, &mut out_buf) } == 0 {
                let idx = out_buf.index;
                if !self.free_out_indices.contains(&idx) {
                    self.free_out_indices.push(idx);
                }
            } else {
                break;
            }
        }
    }

    fn contains_keyframe(data: &[u8]) -> bool {
        let mut i = 0;
        while i + 4 < data.len() {
            if data[i] == 0 && data[i + 1] == 0 {
                let (offset, plen) = if data[i + 2] == 1 {
                    (i, 3)
                } else if i + 3 < data.len() && data[i + 2] == 0 && data[i + 3] == 1 {
                    (i, 4)
                } else {
                    (0, 0)
                };
                if plen > 0 && offset + plen < data.len() {
                    let nal_type = data[offset + plen] & 0x1F;
                    if nal_type == 5 || nal_type == 7 {
                        return true;
                    }
                }
            }
            i += 1;
        }
        false
    }
}

impl Drop for V4l2DecoderSession {
    fn drop(&mut self) {
        let mut out_type = V4L2_BUF_TYPE_VIDEO_OUTPUT_MPLANE;
        let mut cap_type = V4L2_BUF_TYPE_VIDEO_CAPTURE_MPLANE;
        unsafe {
            libc::ioctl(self.video_fd, VIDIOC_STREAMOFF, &mut out_type);
            libc::ioctl(self.video_fd, VIDIOC_STREAMOFF, &mut cap_type);
        }

        for (ptr, len) in self.out_ptrs.iter().zip(self.out_lens.iter()) {
            unsafe {
                libc::munmap(*ptr as *mut libc::c_void, *len);
            }
        }
        for (ptr, len) in self.cap_ptrs.iter().zip(self.cap_lens.iter()) {
            unsafe {
                libc::munmap(*ptr as *mut libc::c_void, *len);
            }
        }
        println!("\x1b[1;33m[v4l2-m2m]\x1b[0m Hardware Decoder session closed cleanly.");
    }
}

unsafe impl Send for V4l2DecoderSession {}

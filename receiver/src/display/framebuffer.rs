//! Linux Framebuffer Direct Display Sink (/dev/fb0)
//!
//! Provides zero-copy memory-mapped display blitting to the Raspberry Pi HDMI output.
//! Configures console VT graphics mode to suppress console cursor and terminal text.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::io::AsRawFd;

pub struct FramebufferSink {
    _file: File,
    fb_ptr: *mut u8,
    fb_size: usize,
    #[allow(dead_code)]
    width: u32,
    #[allow(dead_code)]
    height: u32,
    graphics_mode: bool,
}

impl FramebufferSink {
    /// Opens `/dev/fb0`, unblanks it, and mmaps the framebuffer.
    /// VT1 text console is preserved until the first video frame arrives.
    pub fn open(width: u32, height: u32) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/fb0")?;
        let fb_fd = file.as_raw_fd();

        // 1. Force framebuffer unblank
        const FBIOBLANK: libc::c_ulong = 0x4611;
        unsafe {
            libc::ioctl(fb_fd, FBIOBLANK, 0 as libc::c_int);
        }

        // 2. Memory map /dev/fb0 for direct DMA-like memory blits (RGB565 = 2 bytes per pixel)
        let fb_size = (width * height * 2) as usize;
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

        if fb_ptr.is_null() || fb_ptr == libc::MAP_FAILED as *mut u8 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            _file: file,
            fb_ptr,
            fb_size,
            width,
            height,
            graphics_mode: false,
        })
    }

    /// Transitions console VT1 to graphics mode upon rendering the first live frame
    fn enable_graphics_mode(&mut self) {
        if !self.graphics_mode {
            if let Ok(tty1) = OpenOptions::new().read(true).write(true).open("/dev/tty1") {
                const KDSETMODE: libc::c_ulong = 0x4B3A;
                const KD_GRAPHICS: libc::c_ulong = 0x01;
                unsafe {
                    libc::ioctl(tty1.as_raw_fd(), KDSETMODE, KD_GRAPHICS);
                }
            }
            self.graphics_mode = true;
        }
    }

    /// Blits a full decoded raw frame (RGB565) directly into the HDMI framebuffer
    #[inline(always)]
    pub fn render_frame(&mut self, data: &[u8]) {
        self.enable_graphics_mode();
        let copy_len = data.len().min(self.fb_size);
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), self.fb_ptr, copy_len);
        }
    }

    #[allow(dead_code)]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl Drop for FramebufferSink {
    fn drop(&mut self) {
        if !self.fb_ptr.is_null() && self.fb_ptr != libc::MAP_FAILED as *mut u8 {
            unsafe {
                libc::munmap(self.fb_ptr as *mut libc::c_void, self.fb_size);
            }
        }
        // Restore VT1 text mode if graphics mode was activated
        if self.graphics_mode {
            if let Ok(tty1) = OpenOptions::new().read(true).write(true).open("/dev/tty1") {
                const KDSETMODE: libc::c_ulong = 0x4B3A;
                const KD_TEXT: libc::c_ulong = 0x00;
                unsafe {
                    libc::ioctl(tty1.as_raw_fd(), KDSETMODE, KD_TEXT);
                }
            }
        }
    }
}

// Safety: The raw pointer to mmap'd framebuffer is safely accessed within the struct
unsafe impl Send for FramebufferSink {}
unsafe impl Sync for FramebufferSink {}

//! Pure Rust HDMI Splash Screen Engine for Raspberry Pi Zero
//!
//! Provides zero-delay hardware splash rendering directly into `/dev/fb0` (1280x720 RGB565).
//! Embeds compressed splash assets directly within the binary for zero external runtime dependencies.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::fs::OpenOptions;
use std::io;
use std::os::unix::io::AsRawFd;

const SPLASH_LOADING_GZ: &[u8] =
    include_bytes!("../../../build-appliance/overlay/etc/splash_loading.raw.gz");
const SPLASH_READY_GZ: &[u8] =
    include_bytes!("../../../build-appliance/overlay/etc/splash_ready.raw.gz");

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const FB_SIZE: usize = (WIDTH * HEIGHT * 2) as usize; // 1,843,200 bytes

pub struct SplashEngine;

impl SplashEngine {
    /// Renders the Loading Splash ("Aguarde carregando..." in 4 languages) to /dev/fb0
    pub fn show_loading() {
        if let Ok(raw_bytes) = decompress_gzip(SPLASH_LOADING_GZ) {
            blit_to_framebuffer(&raw_bytes);
            println!("\x1b[1;36m[splash]\x1b[0m Loading splash screen displayed.");
        }
    }

    /// Renders the Ready Splash (3 modes guide in 4 languages) to /dev/fb0
    pub fn show_ready() {
        if let Ok(raw_bytes) = decompress_gzip(SPLASH_READY_GZ) {
            blit_to_framebuffer(&raw_bytes);
            println!("\x1b[1;32m[splash]\x1b[0m Ready splash screen displayed (4 Modes • 4 Languages • IoT Media).");
        }
    }

    /// Clears the display to black
    #[allow(dead_code)]
    pub fn clear() {
        let black = vec![0u8; FB_SIZE];
        blit_to_framebuffer(&black);
        println!("\x1b[1;33m[splash]\x1b[0m Display cleared.");
    }
}

/// Decompresses raw DEFLATE stream embedded within gzip container using pure Rust miniz_oxide
fn decompress_gzip(gz: &[u8]) -> io::Result<Vec<u8>> {
    if gz.len() < 18 || gz[0] != 0x1F || gz[1] != 0x8B {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid GZIP header"));
    }

    // Skip 10-byte GZIP header, omit 8-byte trailer (CRC32 + ISIZE)
    let deflate_payload = &gz[10..gz.len() - 8];
    miniz_oxide::inflate::decompress_to_vec(deflate_payload)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Decompression failed: {:?}", e)))
}

/// Directly copies RGB565 buffer into memory-mapped /dev/fb0 and forces scanout re-attachment
fn blit_to_framebuffer(buffer: &[u8]) {
    if buffer.len() < FB_SIZE {
        return;
    }

    let file = match OpenOptions::new().read(true).write(true).open("/dev/fb0") {
        Ok(f) => f,
        Err(_) => return,
    };

    let fd = file.as_raw_fd();

    const FBIOBLANK: libc::c_ulong = 0x4611;
    const FBIOPAN_DISPLAY: libc::c_ulong = 0x4606;
    const FBIOGET_VSCREENINFO: libc::c_ulong = 0x4600;
    const FBIOPUT_VSCREENINFO: libc::c_ulong = 0x4601;

    #[repr(C)]
    #[derive(Default)]
    struct FbVarScreeninfo {
        xres: u32,
        yres: u32,
        xres_virtual: u32,
        yres_virtual: u32,
        xoffset: u32,
        yoffset: u32,
        bits_per_pixel: u32,
        grayscale: u32,
        red: [u32; 4],
        green: [u32; 4],
        blue: [u32; 4],
        transp: [u32; 4],
        nonstd: u32,
        activate: u32,
        height: u32,
        width: u32,
        accel_flags: u32,
        pixclock: u32,
        left_margin: u32,
        right_margin: u32,
        upper_margin: u32,
        lower_margin: u32,
        hsync_len: u32,
        vsync_len: u32,
        sync: u32,
        vmode: u32,
        rotate: u32,
        colorspace: u32,
        reserved: [u32; 4],
    }

    let mut vinfo = FbVarScreeninfo::default();
    unsafe {
        let _ = libc::ioctl(fd, FBIOBLANK, 0 as libc::c_int);
        if libc::ioctl(fd, FBIOGET_VSCREENINFO, &mut vinfo) == 0 {
            vinfo.activate = 0; // FB_ACTIVATE_NOW
            let _ = libc::ioctl(fd, FBIOPUT_VSCREENINFO, &mut vinfo);
            let _ = libc::ioctl(fd, FBIOPAN_DISPLAY, &mut vinfo);
        }
    }

    // Memory map framebuffer
    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            FB_SIZE,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd,
            0,
        ) as *mut u8
    };

    if ptr.is_null() || ptr == libc::MAP_FAILED as *mut u8 {
        return;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(buffer.as_ptr(), ptr, FB_SIZE);
        libc::munmap(ptr as *mut libc::c_void, FB_SIZE);
        let _ = libc::ioctl(fd, FBIOPAN_DISPLAY, &mut vinfo);
    }
}

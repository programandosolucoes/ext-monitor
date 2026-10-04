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
const SPLASH_MIRACAST_GZ: &[u8] =
    include_bytes!("../../../build-appliance/overlay/etc/splash_miracast.raw.gz");

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

    /// Renders the Dedicated Miracast Connection Guide Splash (Win+K • 4 Languages) to /dev/fb0
    pub fn show_miracast() {
        if let Ok(raw_bytes) = decompress_gzip(SPLASH_MIRACAST_GZ) {
            blit_to_framebuffer(&raw_bytes);
            println!("\x1b[1;35m[splash]\x1b[0m Miracast connection guide displayed (Win+K • 4 Languages • WFD RTSP 7236).");
        } else {
            Self::show_ready();
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
/// Complies with RFC 1952 header parsing (FEXTRA, FNAME, FCOMMENT, FHCRC)
fn decompress_gzip(gz: &[u8]) -> io::Result<Vec<u8>> {
    if gz.len() < 18 || gz[0] != 0x1F || gz[1] != 0x8B {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid GZIP header"));
    }

    let flg = gz[3];
    let mut offset = 10;

    // FEXTRA: skip 2-byte length + extra field
    if flg & 0x04 != 0 {
        if offset + 2 > gz.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Truncated FEXTRA"));
        }
        let xlen = u16::from_le_bytes([gz[offset], gz[offset + 1]]) as usize;
        offset += 2 + xlen;
    }

    // FNAME: zero-terminated string
    if flg & 0x08 != 0 {
        while offset < gz.len() && gz[offset] != 0 {
            offset += 1;
        }
        offset += 1; // skip null byte
    }

    // FCOMMENT: zero-terminated string
    if flg & 0x10 != 0 {
        while offset < gz.len() && gz[offset] != 0 {
            offset += 1;
        }
        offset += 1; // skip null byte
    }

    // FHCRC: 2-byte header CRC
    if flg & 0x02 != 0 {
        offset += 2;
    }

    if offset + 8 > gz.len() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Corrupted GZIP stream"));
    }

    let deflate_payload = &gz[offset..gz.len() - 8];
    miniz_oxide::inflate::decompress_to_vec(deflate_payload)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Decompression failed: {:?}", e)))
}

/// Directly copies RGB565 buffer into memory-mapped /dev/fb0 and forces scanout re-attachment
fn blit_to_framebuffer(buffer: &[u8]) {
    if buffer.len() < FB_SIZE {
        return;
    }

    let mut file = match OpenOptions::new().read(true).write(true).open("/dev/fb0") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("\x1b[1;31m[splash]\x1b[0m Failed to open /dev/fb0: {}", e);
            return;
        }
    };

    let fd = file.as_raw_fd();

    const FBIOBLANK: libc::c_ulong = 0x4611;
    const FBIOPAN_DISPLAY: libc::c_ulong = 0x4606;
    const FBIOGET_VSCREENINFO: libc::c_ulong = 0x4600;

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
        let _ = libc::ioctl(fd, FBIOBLANK as _, 0 as libc::c_int);
        let _ = libc::ioctl(fd, FBIOGET_VSCREENINFO as _, &mut vinfo);
    }

    if let Ok(tty1) = OpenOptions::new().read(true).write(true).open("/dev/tty1") {
        const KDSETMODE: libc::c_ulong = 0x4B3A;
        const KD_GRAPHICS: libc::c_ulong = 0x01;
        unsafe {
            libc::ioctl(tty1.as_raw_fd(), KDSETMODE as _, KD_GRAPHICS);
        }
    }

    use std::io::{Seek, SeekFrom, Write};
    let _ = file.seek(SeekFrom::Start(0));
    if let Err(e) = file.write_all(&buffer[..FB_SIZE]) {
        eprintln!("\x1b[1;31m[splash]\x1b[0m write_all to /dev/fb0 failed: {}", e);
    }
    let _ = file.flush();
    unsafe {
        let _ = libc::ioctl(fd, FBIOPAN_DISPLAY as _, &mut vinfo);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_splashes_decompress_to_fb_size() {
        let loading = decompress_gzip(SPLASH_LOADING_GZ).expect("Failed to decompress splash_loading");
        assert_eq!(loading.len(), FB_SIZE, "splash_loading must match 1280x720x2");

        let ready = decompress_gzip(SPLASH_READY_GZ).expect("Failed to decompress splash_ready");
        assert_eq!(ready.len(), FB_SIZE, "splash_ready must match 1280x720x2");

        let miracast = decompress_gzip(SPLASH_MIRACAST_GZ).expect("Failed to decompress splash_miracast");
        assert_eq!(miracast.len(), FB_SIZE, "splash_miracast must match 1280x720x2");
    }
}

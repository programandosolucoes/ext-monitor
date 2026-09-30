//! Native Pure-Rust Splash Screen Generator for ExtMonitor
//!
//! Generates 1280x720 RGB565 raw and gzip-compressed images for the Pi Zero
//! framebuffer (/dev/fb0) during boot and idle states without requiring Python or Pillow.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::fs;
use std::path::Path;

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;

struct ImageBuffer {
    width: usize,
    height: usize,
    data: Vec<[u8; 3]>, // RGB888
}

impl ImageBuffer {
    fn new(width: usize, height: usize, initial_color: [u8; 3]) -> Self {
        Self {
            width,
            height,
            data: vec![initial_color; width * height],
        }
    }

    fn set_pixel(&mut self, x: usize, y: usize, color: [u8; 3]) {
        if x < self.width && y < self.height {
            self.data[y * self.width + x] = color;
        }
    }

    #[allow(dead_code)]
    fn blend_pixel(&mut self, x: usize, y: usize, color: [u8; 3], alpha: f32) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            let bg = self.data[idx];
            let inv = 1.0 - alpha;
            let r = (bg[0] as f32 * inv + color[0] as f32 * alpha) as u8;
            let g = (bg[1] as f32 * inv + color[1] as f32 * alpha) as u8;
            let b = (bg[2] as f32 * inv + color[2] as f32 * alpha) as u8;
            self.data[idx] = [r, g, b];
        }
    }

    fn draw_gradient_vertical(&mut self, top_color: [u8; 3], bottom_color: [u8; 3]) {
        for y in 0..self.height {
            let ratio = y as f32 / self.height as f32;
            let inv = 1.0 - ratio;
            let r = (top_color[0] as f32 * inv + bottom_color[0] as f32 * ratio) as u8;
            let g = (top_color[1] as f32 * inv + bottom_color[1] as f32 * ratio) as u8;
            let b = (top_color[2] as f32 * inv + bottom_color[2] as f32 * ratio) as u8;
            for x in 0..self.width {
                self.set_pixel(x, y, [r, g, b]);
            }
        }
    }

    fn draw_grid(&mut self, step: usize, color: [u8; 3]) {
        for x in (0..self.width).step_by(step) {
            for y in 0..self.height {
                self.set_pixel(x, y, color);
            }
        }
        for y in (0..self.height).step_by(step) {
            for x in 0..self.width {
                self.set_pixel(x, y, color);
            }
        }
    }

    fn fill_rect(&mut self, x0: usize, y0: usize, w: usize, h: usize, color: [u8; 3]) {
        for y in y0..(y0 + h).min(self.height) {
            for x in x0..(x0 + w).min(self.width) {
                self.set_pixel(x, y, color);
            }
        }
    }

    fn draw_rect_outline(&mut self, x0: usize, y0: usize, w: usize, h: usize, stroke: usize, color: [u8; 3]) {
        self.fill_rect(x0, y0, w, stroke, color);
        self.fill_rect(x0, y0 + h.saturating_sub(stroke), w, stroke, color);
        self.fill_rect(x0, y0, stroke, h, color);
        self.fill_rect(x0 + w.saturating_sub(stroke), y0, stroke, h, color);
    }

    fn draw_circle(&mut self, cx: isize, cy: isize, radius: isize, stroke: isize, color: [u8; 3]) {
        let r_inner = (radius - stroke).max(0);
        let r_sq_outer = radius * radius;
        let r_sq_inner = r_inner * r_inner;

        let y_min = (cy - radius).max(0) as usize;
        let y_max = (cy + radius).min(self.height as isize - 1) as usize;
        let x_min = (cx - radius).max(0) as usize;
        let x_max = (cx + radius).min(self.width as isize - 1) as usize;

        for y in y_min..=y_max {
            for x in x_min..=x_max {
                let dx = x as isize - cx;
                let dy = y as isize - cy;
                let d_sq = dx * dx + dy * dy;
                if d_sq <= r_sq_outer && d_sq >= r_sq_inner {
                    self.set_pixel(x, y, color);
                }
            }
        }
    }

    fn fill_circle(&mut self, cx: isize, cy: isize, radius: isize, color: [u8; 3]) {
        let r_sq = radius * radius;
        let y_min = (cy - radius).max(0) as usize;
        let y_max = (cy + radius).min(self.height as isize - 1) as usize;
        let x_min = (cx - radius).max(0) as usize;
        let x_max = (cx + radius).min(self.width as isize - 1) as usize;

        for y in y_min..=y_max {
            for x in x_min..=x_max {
                let dx = x as isize - cx;
                let dy = y as isize - cy;
                if dx * dx + dy * dy <= r_sq {
                    self.set_pixel(x, y, color);
                }
            }
        }
    }

    /// Converts RGB888 to RGB565 Little-Endian (framebuffer format for BCM2835 /dev/fb0)
    fn to_rgb565(&self) -> Vec<u8> {
        let mut raw = Vec::with_capacity(self.width * self.height * 2);
        for pixel in &self.data {
            let r5 = ((pixel[0] as u16) >> 3) & 0x1F;
            let g6 = ((pixel[1] as u16) >> 2) & 0x3F;
            let b5 = ((pixel[2] as u16) >> 3) & 0x1F;
            let val = (r5 << 11) | (g6 << 5) | b5;
            raw.push((val & 0xFF) as u8);
            raw.push(((val >> 8) & 0xFF) as u8);
        }
        raw
    }
}

/// Standard CRC-32 table computation (polynomial 0xEDB88320)
fn compute_crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Compresses raw data into RFC 1952 Gzip container
fn gzip_compress(data: &[u8]) -> Vec<u8> {
    // 10-byte Gzip header
    let mut gz = vec![0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff];
    // Deflate compressed stream (level 9 maximum compression)
    let deflated = miniz_oxide::deflate::compress_to_vec(data, 9);
    gz.extend_from_slice(&deflated);
    // CRC-32 (4 bytes LE)
    let crc = compute_crc32(data);
    gz.extend_from_slice(&crc.to_le_bytes());
    // Uncompressed input size modulo 2^32 (4 bytes LE)
    let isize = data.len() as u32;
    gz.extend_from_slice(&isize.to_le_bytes());
    gz
}

/// Generates the early boot "Loading..." splash screen
pub fn generate_loading_splash(output_dir: &Path) -> Result<(), String> {
    println!("\x1b[1;34m[*] Gerando tela de splash de inicialização (splash_loading.raw.gz) em Rust...\x1b[0m");
    let mut img = ImageBuffer::new(WIDTH, HEIGHT, [10, 14, 23]);

    // Deep tech gradient background
    img.draw_gradient_vertical([10, 14, 23], [16, 22, 36]);

    // Tech ambient grid
    img.draw_grid(80, [20, 28, 45]);

    // Top Brand Accent line
    img.fill_rect(WIDTH / 2 - 250, 40, 500, 3, [88, 166, 255]);

    // Center circular logo frame
    let cx = WIDTH as isize / 2;
    let cy = 260;
    img.draw_circle(cx, cy, 65, 3, [31, 111, 235]);
    img.fill_circle(cx, cy, 50, [22, 27, 34]);
    img.draw_circle(cx, cy, 50, 2, [88, 166, 255]);

    // Inner glowing core
    img.fill_circle(cx, cy, 22, [56, 189, 248]);
    img.fill_circle(cx, cy, 14, [240, 246, 252]);

    // Status card container
    let card_x = WIDTH / 2 - 320;
    let card_y = 350;
    let card_w = 640;
    let card_h = 130;
    img.fill_rect(card_x, card_y, card_w, card_h, [18, 24, 38]);
    img.draw_rect_outline(card_x, card_y, card_w, card_h, 2, [31, 111, 235]);

    // Progress bar frame
    let bar_x = WIDTH / 2 - 260;
    let bar_y = 425;
    let bar_w = 520;
    let bar_h = 12;
    img.fill_rect(bar_x, bar_y, bar_w, bar_h, [33, 38, 45]);
    // Progress fill 75%
    img.fill_rect(bar_x, bar_y, 390, bar_h, [56, 189, 248]);

    // Bottom decorative status cards
    for i in 0..4 {
        let x = 120 + i * 265;
        let y = 540;
        let w = 245;
        let h = 90;
        img.fill_rect(x, y, w, h, [14, 18, 28]);
        img.draw_rect_outline(x, y, w, h, 1, [33, 45, 68]);
        img.fill_rect(x + 10, y + 10, 10, 10, [46, 160, 67]); // green status dot
    }

    // Convert to RGB565 and Gzip
    let raw = img.to_rgb565();
    let gz = gzip_compress(&raw);

    fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
    let raw_path = output_dir.join("splash_loading.raw");
    let gz_path = output_dir.join("splash_loading.raw.gz");

    fs::write(&raw_path, &raw).map_err(|e| e.to_string())?;
    fs::write(&gz_path, &gz).map_err(|e| e.to_string())?;

    println!("\x1b[1;32m[+] splash_loading.raw.gz gerada com sucesso! ({} bytes compactados)\x1b[0m", gz.len());
    Ok(())
}

/// Generates the idle "Ready for Connection" splash screen
pub fn generate_ready_splash(output_dir: &Path) -> Result<(), String> {
    println!("\x1b[1;34m[*] Gerando tela de splash de prontidão (splash_ready.raw.gz) em Rust...\x1b[0m");
    let mut img = ImageBuffer::new(WIDTH, HEIGHT, [8, 12, 20]);

    img.draw_gradient_vertical([8, 12, 20], [13, 17, 28]);
    img.draw_grid(80, [18, 24, 40]);

    // Top status indicator: READY GREEN
    img.fill_rect(WIDTH / 2 - 280, 35, 560, 4, [46, 160, 67]);

    // Center circular ready badge
    let cx = WIDTH as isize / 2;
    let cy = 180;
    img.draw_circle(cx, cy, 60, 3, [46, 160, 67]);
    img.fill_circle(cx, cy, 48, [18, 30, 24]);
    img.draw_circle(cx, cy, 48, 2, [63, 185, 80]);
    img.fill_circle(cx, cy, 20, [46, 160, 67]);

    // Connection command bar
    let cmd_x = WIDTH / 2 - 380;
    let cmd_y = 280;
    let cmd_w = 760;
    let cmd_h = 60;
    img.fill_rect(cmd_x, cmd_y, cmd_w, cmd_h, [13, 17, 26]);
    img.draw_rect_outline(cmd_x, cmd_y, cmd_w, cmd_h, 2, [56, 189, 248]);

    // 4 Modes Cards
    let mode_names_colors = [
        [56, 189, 248], // Mode 1: Blue (UDP)
        [163, 113, 247], // Mode 2: Purple (USB Raw)
        [240, 136, 62],  // Mode 3: Orange (USB Bulk Direct)
        [63, 185, 80],   // Mode 4: Green (TCP Loopback)
    ];

    for i in 0..4 {
        let x = 80 + i * 285;
        let y = 370;
        let w = 265;
        let h = 180;
        img.fill_rect(x, y, w, h, [16, 21, 32]);
        img.draw_rect_outline(x, y, w, h, 2, mode_names_colors[i]);
        // Top accent line in card
        img.fill_rect(x, y, w, 4, mode_names_colors[i]);
    }

    // Bottom Web Dashboard info bar
    let bar_x = 80;
    let bar_y = 580;
    let bar_w = WIDTH - 160;
    let bar_h = 80;
    img.fill_rect(bar_x, bar_y, bar_w, bar_h, [18, 24, 38]);
    img.draw_rect_outline(bar_x, bar_y, bar_w, bar_h, 1, [48, 54, 61]);

    let raw = img.to_rgb565();
    let gz = gzip_compress(&raw);

    fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
    let raw_path = output_dir.join("splash_ready.raw");
    let gz_path = output_dir.join("splash_ready.raw.gz");

    fs::write(&raw_path, &raw).map_err(|e| e.to_string())?;
    fs::write(&gz_path, &gz).map_err(|e| e.to_string())?;

    println!("\x1b[1;32m[+] splash_ready.raw.gz gerada com sucesso! ({} bytes compactados)\x1b[0m", gz.len());
    Ok(())
}

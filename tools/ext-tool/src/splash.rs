//! Native Pure-Rust Splash Screen Generator for ExtMonitor
//!
//! Generates 1280x720 RGB565 raw and gzip-compressed images for the Pi Zero
//! framebuffer (/dev/fb0) during boot and idle states without requiring Python or Pillow.
//!
//! 100% Pure Rust • TrueType / OpenType font rasterization via fontdue • Gzip compression
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::fs;
use std::path::Path;
use std::process::Command;

pub const WIDTH: usize = 1280;
pub const HEIGHT: usize = 720;

pub struct FontSet {
    pub bold: fontdue::Font,
    pub regular: fontdue::Font,
    #[allow(dead_code)]
    pub cjk: Option<fontdue::Font>,
}

impl FontSet {
    pub fn load() -> Self {
        let bold_paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        ];
        let mut bold_bytes = None;
        for p in bold_paths {
            if let Ok(b) = fs::read(p) {
                bold_bytes = Some(b);
                break;
            }
        }
        let bold_data = bold_bytes.expect("Bold TrueType font (DejaVuSans-Bold / LiberationSans-Bold) required");
        let bold = fontdue::Font::from_bytes(bold_data, fontdue::FontSettings::default()).expect("Failed to parse bold font");

        let regular_paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ];
        let mut regular_bytes = None;
        for p in regular_paths {
            if let Ok(b) = fs::read(p) {
                regular_bytes = Some(b);
                break;
            }
        }
        let reg_data = regular_bytes.expect("Regular TrueType font required");
        let regular = fontdue::Font::from_bytes(reg_data, fontdue::FontSettings::default()).expect("Failed to parse regular font");

        let cjk_paths = [
            "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
            "/usr/share/fonts/truetype/arphic/ukai.ttc",
            "/usr/share/fonts/truetype/arphic/uming.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        ];
        let mut cjk = None;
        for p in cjk_paths {
            if let Ok(b) = fs::read(p) {
                if let Ok(f) = fontdue::Font::from_bytes(b, fontdue::FontSettings::default()) {
                    cjk = Some(f);
                    break;
                }
            }
        }

        Self { bold, regular, cjk }
    }
}

/// Selects the appropriate font for a given character, falling back to CJK when missing
#[inline]
pub fn select_font<'a>(
    primary: &'a fontdue::Font,
    cjk: Option<&'a fontdue::Font>,
    ch: char,
) -> &'a fontdue::Font {
    if let Some(cjk_font) = cjk {
        let is_cjk = ('\u{2E80}'..='\u{9FFF}').contains(&ch)
            || ('\u{F900}'..='\u{FAFF}').contains(&ch)
            || ('\u{FF00}'..='\u{FFEF}').contains(&ch)
            || ('\u{3000}'..='\u{303F}').contains(&ch)
            || ('\u{20000}'..='\u{2FA1F}').contains(&ch);
        if is_cjk || primary.lookup_glyph_index(ch) == 0 {
            if cjk_font.lookup_glyph_index(ch) != 0 {
                return cjk_font;
            }
        }
    }
    primary
}

pub struct ImageBuffer {
    pub width: usize,
    pub height: usize,
    pub data: Vec<[u8; 3]>, // RGB888
}

impl ImageBuffer {
    pub fn new(width: usize, height: usize, initial_color: [u8; 3]) -> Self {
        Self {
            width,
            height,
            data: vec![initial_color; width * height],
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: [u8; 3]) {
        if x < self.width && y < self.height {
            self.data[y * self.width + x] = color;
        }
    }

    pub fn blend_pixel(&mut self, x: usize, y: usize, color: [u8; 3], alpha: f32) {
        if x < self.width && y < self.height && alpha > 0.001 {
            let idx = y * self.width + x;
            let bg = self.data[idx];
            let a = alpha.clamp(0.0, 1.0);
            let inv = 1.0 - a;
            let r = (bg[0] as f32 * inv + color[0] as f32 * a).round() as u8;
            let g = (bg[1] as f32 * inv + color[1] as f32 * a).round() as u8;
            let b = (bg[2] as f32 * inv + color[2] as f32 * a).round() as u8;
            self.data[idx] = [r, g, b];
        }
    }

    pub fn draw_gradient_vertical(&mut self, top_color: [u8; 3], bottom_color: [u8; 3]) {
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

    pub fn draw_grid(&mut self, step: usize, color: [u8; 3]) {
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

    pub fn draw_line(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, color: [u8; 3], width: usize) {
        if y0 == y1 {
            let y_start = y0.saturating_sub(width / 2);
            let y_end = (y_start + width).min(self.height);
            let x_start = x0.min(x1);
            let x_end = x0.max(x1).min(self.width);
            for y in y_start..y_end {
                for x in x_start..=x_end {
                    self.set_pixel(x, y, color);
                }
            }
        } else if x0 == x1 {
            let x_start = x0.saturating_sub(width / 2);
            let x_end = (x_start + width).min(self.width);
            let y_start = y0.min(y1);
            let y_end = y0.max(y1).min(self.height);
            for y in y_start..=y_end {
                for x in x_start..x_end {
                    self.set_pixel(x, y, color);
                }
            }
        }
    }

    pub fn fill_rect(&mut self, x0: usize, y0: usize, w: usize, h: usize, color: [u8; 3]) {
        for y in y0..(y0 + h).min(self.height) {
            for x in x0..(x0 + w).min(self.width) {
                self.set_pixel(x, y, color);
            }
        }
    }

    pub fn draw_rect_outline(&mut self, x0: usize, y0: usize, w: usize, h: usize, stroke: usize, color: [u8; 3]) {
        self.fill_rect(x0, y0, w, stroke, color);
        self.fill_rect(x0, y0 + h.saturating_sub(stroke), w, stroke, color);
        self.fill_rect(x0, y0, stroke, h, color);
        self.fill_rect(x0 + w.saturating_sub(stroke), y0, stroke, h, color);
    }

    pub fn fill_rounded_rect(&mut self, x0: usize, y0: usize, w: usize, h: usize, radius: usize, color: [u8; 3]) {
        let r = radius.min(w / 2).min(h / 2);
        self.fill_rect(x0 + r, y0, w - 2 * r, h, color);
        self.fill_rect(x0, y0 + r, r, h - 2 * r, color);
        self.fill_rect(x0 + w - r, y0 + r, r, h - 2 * r, color);
        self.fill_circle((x0 + r) as isize, (y0 + r) as isize, r as isize, color);
        self.fill_circle((x0 + w - r - 1) as isize, (y0 + r) as isize, r as isize, color);
        self.fill_circle((x0 + r) as isize, (y0 + h - r - 1) as isize, r as isize, color);
        self.fill_circle((x0 + w - r - 1) as isize, (y0 + h - r - 1) as isize, r as isize, color);
    }

    #[allow(dead_code)]
    pub fn draw_rounded_rect_outline(&mut self, x0: usize, y0: usize, w: usize, h: usize, radius: usize, stroke: usize, color: [u8; 3]) {
        self.draw_rect_outline(x0 + radius, y0, w.saturating_sub(2 * radius), stroke, stroke, color);
        self.draw_rect_outline(x0 + radius, y0 + h.saturating_sub(stroke), w.saturating_sub(2 * radius), stroke, stroke, color);
        self.draw_rect_outline(x0, y0 + radius, stroke, h.saturating_sub(2 * radius), stroke, color);
        self.draw_rect_outline(x0 + w.saturating_sub(stroke), y0 + radius, stroke, h.saturating_sub(2 * radius), stroke, color);
    }

    pub fn draw_circle(&mut self, cx: isize, cy: isize, radius: isize, stroke: isize, color: [u8; 3]) {
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

    pub fn fill_circle(&mut self, cx: isize, cy: isize, radius: isize, color: [u8; 3]) {
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

    pub fn measure_text_width(
        &self,
        font: &fontdue::Font,
        cjk: Option<&fontdue::Font>,
        text: &str,
        size: f32,
    ) -> usize {
        let mut width = 0.0f32;
        for ch in text.chars() {
            let active = select_font(font, cjk, ch);
            let (metrics, _) = active.rasterize(ch, size);
            width += metrics.advance_width;
        }
        width.round() as usize
    }

    pub fn draw_text(
        &mut self,
        font: &fontdue::Font,
        cjk: Option<&fontdue::Font>,
        text: &str,
        size: f32,
        mut x: usize,
        y: usize,
        color: [u8; 3],
    ) {
        let baseline_y = y as isize + (size * 0.8) as isize;
        for ch in text.chars() {
            let active = select_font(font, cjk, ch);
            let (metrics, bitmap) = active.rasterize(ch, size);
            let glyph_x = x as isize + metrics.xmin as isize;
            let glyph_y = baseline_y - metrics.ymin as isize - metrics.height as isize;

            for r in 0..metrics.height {
                for c in 0..metrics.width {
                    let alpha = bitmap[r * metrics.width + c];
                    if alpha > 0 {
                        let px = glyph_x + c as isize;
                        let py = glyph_y + r as isize;
                        if px >= 0 && py >= 0 {
                            self.blend_pixel(px as usize, py as usize, color, alpha as f32 / 255.0);
                        }
                    }
                }
            }
            x = (x as isize + metrics.advance_width.round() as isize).max(0) as usize;
        }
    }

    pub fn draw_text_centered(
        &mut self,
        font: &fontdue::Font,
        cjk: Option<&fontdue::Font>,
        text: &str,
        size: f32,
        center_x: usize,
        y: usize,
        color: [u8; 3],
    ) {
        let w = self.measure_text_width(font, cjk, text, size);
        let x = center_x.saturating_sub(w / 2);
        self.draw_text(font, cjk, text, size, x, y, color);
    }

    /// Converts RGB888 to RGB565 Little-Endian (framebuffer format for BCM2835 /dev/fb0)
    pub fn to_rgb565(&self) -> Vec<u8> {
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

/// Compresses raw data into RFC 1952 Gzip container in pure Rust
pub fn gzip_compress(data: &[u8]) -> Vec<u8> {
    let mut gz = vec![0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff];
    let deflated = miniz_oxide::deflate::compress_to_vec(data, 9);
    gz.extend_from_slice(&deflated);
    let crc = compute_crc32(data);
    gz.extend_from_slice(&crc.to_le_bytes());
    let isize = data.len() as u32;
    gz.extend_from_slice(&isize.to_le_bytes());
    gz
}

pub fn get_version() -> String {
    if let Ok(v) = std::env::var("EXT_MONITOR_VERSION") {
        if !v.trim().is_empty() {
            return v.trim().to_string();
        }
    }
    if let Ok(out) = Command::new("git").args(["describe", "--tags", "--always"]).output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
        }
    }
    "v2.9.0-final".to_string()
}

pub fn save_splash(img: &ImageBuffer, output_dir: &Path, name: &str) -> Result<(), String> {
    fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
    let raw = img.to_rgb565();
    let gz = gzip_compress(&raw);

    let raw_path = output_dir.join(format!("{}.raw", name));
    let gz_path = output_dir.join(format!("{}.raw.gz", name));

    fs::write(&raw_path, &raw).map_err(|e| e.to_string())?;
    fs::write(&gz_path, &gz).map_err(|e| e.to_string())?;

    println!(
        "\x1b[1;32m[+] Pure-Rust Splash {}.raw.gz gerada com sucesso! ({} bytes gz)\x1b[0m",
        name,
        gz.len()
    );
    Ok(())
}

/// Generates the early boot "Loading..." splash screen in 100% Pure Rust
pub fn generate_loading_splash(output_dir: &Path, fonts: &FontSet, version: &str) -> Result<(), String> {
    let cjk = fonts.cjk.as_ref();
    let mut img = ImageBuffer::new(WIDTH, HEIGHT, [10, 14, 23]);
    img.draw_gradient_vertical([10, 14, 23], [16, 22, 36]);
    img.draw_grid(80, [20, 28, 45]);

    // Top Brand Header
    img.draw_text_centered(&fonts.bold, cjk, "EXT-MONITOR PI ZERO", 26.0, WIDTH / 2, 50, [88, 166, 255]);
    let sub = format!("Appliance Minimal 33MB • VideoCore IV KMS • Áudio HDMI (Opus 48k) • Bluetooth A2DP • {}", version);
    img.draw_text_centered(&fonts.regular, cjk, &sub, 13.0, WIDTH / 2, 90, [139, 148, 158]);

    // Center Gear Badge
    let cx = (WIDTH / 2) as isize;
    let cy = 240;
    img.draw_circle(cx, cy, 55, 3, [31, 111, 235]);
    img.fill_circle(cx, cy, 45, [22, 27, 34]);
    img.draw_circle(cx, cy, 45, 2, [88, 166, 255]);
    img.draw_text_centered(&fonts.bold, cjk, "EXT", 18.0, WIDTH / 2, 230, [56, 189, 248]);

    // Central Title
    img.draw_text_centered(&fonts.bold, cjk, "INICIANDO SISTEMA • INITIALIZING APPLIANCE", 20.0, WIDTH / 2, 325, [240, 246, 252]);

    // Progress bar frame
    let bx = WIDTH / 2 - 260;
    let by = 365;
    let bw = 520;
    let bh = 10;
    img.fill_rounded_rect(bx, by, bw, bh, 4, [33, 38, 45]);
    img.fill_rounded_rect(bx, by, 410, bh, 4, [56, 189, 248]);

    // 4 Languages Loading Messages Box
    let card_x = WIDTH / 2 - 420;
    let card_y = 400;
    let card_w = 840;
    let card_h = 225;
    img.fill_rounded_rect(card_x, card_y, card_w, card_h, 10, [22, 27, 34]);
    img.draw_rect_outline(card_x, card_y, card_w, card_h, 1, [48, 54, 61]);

    let messages = [
        ("[PT]", "Carregando GPU VideoCore IV, áudio HDMI, Bluetooth A2DP, UPnP/Cast e Visualizador 30 FPS...", [56, 189, 248]),
        ("[EN]", "Initializing VideoCore IV GPU, HDMI audio, Bluetooth A2DP, UPnP/Cast & 30 FPS Visualizer...", [88, 166, 255]),
        ("[IT]", "Caricamento GPU VideoCore IV, audio HDMI, Bluetooth A2DP, UPnP/Cast e Visualizzatore 30 FPS...", [163, 113, 247]),
        ("[ZH]", "正在載入 VideoCore IV GPU、HDMI 音訊、藍牙 A2DP、UPnP/Cast 與 30 FPS 視覺化...", [63, 185, 80]),
    ];

    for (idx, (tag, text, col)) in messages.iter().enumerate() {
        let row_y = card_y + 16 + (idx * 50);
        img.draw_text(&fonts.bold, cjk, tag, 14.0, card_x + 25, row_y, *col);
        img.draw_text(&fonts.regular, cjk, text, 13.0, card_x + 85, row_y, [201, 209, 217]);
    }

    let foot = format!("Iniciando USB Gadget, Wi-Fi Display, Bluetooth A2DP, UPnP/DLNA e Painel Web 8080 • {}", version);
    img.draw_text_centered(&fonts.regular, cjk, &foot, 12.0, WIDTH / 2, 665, [110, 118, 129]);

    save_splash(&img, output_dir, "splash_loading")
}

/// Generates the idle "Ready for Connection" splash screen with 4 quadrants and badges
pub fn generate_ready_splash(output_dir: &Path, fonts: &FontSet, version: &str) -> Result<(), String> {
    let cjk = fonts.cjk.as_ref();
    let mut img = ImageBuffer::new(WIDTH, HEIGHT, [8, 12, 20]);
    img.draw_gradient_vertical([8, 12, 20], [14, 18, 30]);

    // Top Banner Header
    img.draw_text(&fonts.bold, cjk, "EXT-MONITOR PI ZERO", 20.0, 40, 22, [56, 189, 248]);
    img.draw_text(&fonts.bold, cjk, "•  ÁUDIO DIGITAL HDMI & IOT MEDIA RENDERER", 12.0, 310, 26, [163, 113, 247]);

    // Badge top right
    let badge_rect_x = WIDTH - 520;
    img.fill_rounded_rect(badge_rect_x, 18, 480, 28, 5, [18, 48, 28]);
    img.draw_rect_outline(badge_rect_x, 18, 480, 28, 1, [46, 160, 67]);
    img.draw_text_centered(&fonts.bold, cjk, "● 60 FPS • ÁUDIO OPUS • BLUETOOTH A2DP • IOT CAST • VISUALIZADOR", 10.0, badge_rect_x + 240, 25, [86, 211, 100]);

    img.draw_line(40, 56, WIDTH - 40, 56, [33, 38, 45], 1);

    // 4 Language Quadrants
    struct Quad<'a> {
        flag: &'a str,
        lang: &'a str,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        border: [u8; 3],
        modes: [(&'a str, &'a str); 4],
    }

    let quads = [
        Quad {
            flag: "[PT]",
            lang: "PORTUGUÊS (BRASIL)",
            x: 40,
            y: 66,
            w: 580,
            h: 268,
            border: [31, 111, 235],
            modes: [
                ("Modo 1 (Rede IP Wayland):", "Execute ./scripts/start.sh (H.264 60 FPS + Áudio Opus 48k)"),
                ("Modo 2 (Windows Miracast):", "No Windows tecle Win+K e projete tela e áudio nativamente"),
                ("Modo 3 (USB Bulk Direto):", "Conecte cabo USB e use --transport=usb (Latência < 1ms)"),
                ("Modo 4 (Bluetooth & IoT Cast):", "Pareie celular (A2DP), UPnP/DLNA & YouTube Cast com espectro"),
            ],
        },
        Quad {
            flag: "[EN]",
            lang: "ENGLISH",
            x: 660,
            y: 66,
            w: 580,
            h: 268,
            border: [56, 189, 248],
            modes: [
                ("Mode 1 (Wayland Network):", "Run ./scripts/start.sh (H.264 60 FPS + HDMI Audio Opus 48k)"),
                ("Mode 2 (Windows Miracast):", "Press Win+K on Windows to project screen & native audio"),
                ("Mode 3 (Direct USB Bulk):", "Connect USB cable with --transport=usb (< 1ms ultra latency)"),
                ("Mode 4 (Bluetooth & IoT Cast):", "Pair phone via A2DP, stream via UPnP or YouTube with visualizer"),
            ],
        },
        Quad {
            flag: "[IT]",
            lang: "ITALIANO",
            x: 40,
            y: 346,
            w: 580,
            h: 268,
            border: [163, 113, 247],
            modes: [
                ("Modo 1 (Rete IP Wayland):", "Esegui ./scripts/start.sh (H.264 60 FPS + Audio HDMI Opus 48k)"),
                ("Modo 2 (Windows Miracast):", "Su Windows premi Win+K per schermo e audio nativi"),
                ("Modo 3 (USB Diretto Bulk):", "Collega cavo USB con --transport=usb (Latenza < 1ms)"),
                ("Modo 4 (Bluetooth e IoT Cast):", "Associa smartphone A2DP, streaming UPnP/YouTube con spettro"),
            ],
        },
        Quad {
            flag: "[ZH]",
            lang: "繁體中文 (TRADITIONAL CHINESE)",
            x: 660,
            y: 346,
            w: 580,
            h: 268,
            border: [63, 185, 80],
            modes: [
                ("模式 1 (Wayland 網路 IP):", "執行 ./scripts/start.sh (H.264 60 FPS + HDMI 音訊 48k)"),
                ("模式 2 (Windows Miracast):", "在 Windows 10/11 按 Win+K 投影螢幕與音訊"),
                ("模式 3 (USB 直連 Bulk):", "連接 USB 傳輸線並使用 --transport=usb (延遲 < 1ms)"),
                ("模式 4 (藍牙與 IoT 投影):", "配對手機藍牙 A2DP、UPnP 與 YouTube 即時頻譜"),
            ],
        },
    ];

    for q in &quads {
        img.fill_rounded_rect(q.x, q.y, q.w, q.h, 8, [18, 22, 32]);
        img.draw_rect_outline(q.x, q.y, q.w, q.h, 1, [36, 44, 62]);
        img.draw_line(q.x + 12, q.y, q.x + q.w - 12, q.y, q.border, 3);

        let title = format!("{}  {}", q.flag, q.lang);
        img.draw_text(&fonts.bold, cjk, &title, 13.0, q.x + 18, q.y + 12, [240, 246, 252]);

        let start_y = q.y + 36;
        for (m_idx, (m_label, m_desc)) in q.modes.iter().enumerate() {
            let item_y = start_y + (m_idx * 54);
            img.fill_rounded_rect(q.x + 12, item_y, q.w - 24, 48, 5, [24, 30, 44]);
            img.draw_text(&fonts.bold, cjk, m_label, 11.5, q.x + 20, item_y + 6, q.border);
            img.draw_text(&fonts.regular, cjk, m_desc, 10.5, q.x + 20, item_y + 24, [180, 190, 205]);
        }
    }

    // Service Badges & Footer
    img.draw_line(40, 624, WIDTH - 40, 624, [33, 38, 45], 1);

    let badges = [
        ("● Web UI: 192.168.7.2:8080", [31, 111, 235]),
        ("● YouTube Cast (DIAL)", [220, 38, 38]),
        ("● UPnP / DLNA AV", [163, 113, 247]),
        ("● Bluetooth A2DP Sink", [56, 189, 248]),
        ("● Espectro HDMI 30 FPS", [46, 160, 67]),
        ("● USB Bulk <1ms", [234, 179, 8]),
    ];

    let mut bx = 40;
    for (b_text, b_col) in &badges {
        let text_w = img.measure_text_width(&fonts.bold, cjk, b_text, 10.5);
        let bw = text_w + 20;
        img.fill_rounded_rect(bx, 634, bw, 24, 5, [20, 25, 38]);
        img.draw_rect_outline(bx, 634, bw, 24, 1, *b_col);
        img.draw_text_centered(&fonts.bold, cjk, b_text, 10.5, bx + bw / 2, 640, [230, 237, 243]);
        bx += bw + 10;
    }

    let foot = format!(
        "Raspberry Pi Zero W  •  Alsa HDMI (bcm2835)  •  VideoCore IV KMS Framebuffer  •  Zero-Copy DMA  •  Freeze Final {}",
        version
    );
    img.draw_text_centered(&fonts.regular, cjk, &foot, 11.5, WIDTH / 2, 680, [120, 130, 142]);

    save_splash(&img, output_dir, "splash_ready")
}

/// Generates the dedicated Miracast WFD splash screen in 100% Pure Rust
pub fn generate_miracast_splash(output_dir: &Path, fonts: &FontSet, version: &str) -> Result<(), String> {
    let cjk = fonts.cjk.as_ref();
    let mut img = ImageBuffer::new(WIDTH, HEIGHT, [8, 14, 26]);
    img.draw_gradient_vertical([8, 14, 26], [15, 22, 38]);
    img.draw_grid(80, [18, 26, 44]);

    // Header
    img.draw_text(&fonts.bold, cjk, "EXT-MONITOR PI ZERO", 20.0, 40, 22, [56, 189, 248]);
    img.draw_text(&fonts.bold, cjk, "•  MODO 2: WINDOWS MIRACAST (WI-FI DISPLAY)", 12.0, 310, 26, [0, 164, 239]);

    let badge_rect_x = WIDTH - 520;
    img.fill_rounded_rect(badge_rect_x, 18, 480, 28, 5, [18, 48, 28]);
    img.draw_rect_outline(badge_rect_x, 18, 480, 28, 1, [46, 160, 67]);
    img.draw_text_centered(&fonts.bold, cjk, "● RTSP 7236 • MS-MICE 7250 • 60 FPS • ÁUDIO ESTÉREO", 10.0, badge_rect_x + 240, 25, [86, 211, 100]);

    img.draw_line(40, 56, WIDTH - 40, 56, [33, 44, 65], 1);

    // Hero Banner
    let hero_x = 40;
    let hero_y = 66;
    let hero_w = WIDTH - 80;
    let hero_h = 104;
    img.fill_rounded_rect(hero_x, hero_y, hero_w, hero_h, 8, [18, 25, 42]);
    img.draw_rect_outline(hero_x, hero_y, hero_w, hero_h, 2, [0, 164, 239]);

    // Win key
    img.fill_rounded_rect(65, 86, 105, 64, 7, [10, 35, 70]);
    img.draw_rect_outline(65, 86, 105, 64, 2, [0, 164, 239]);
    img.draw_text_centered(&fonts.bold, cjk, "Win", 18.0, 65 + 52, 106, [240, 246, 252]);

    img.draw_text_centered(&fonts.bold, cjk, "+", 22.0, 192, 104, [56, 189, 248]);

    // K key
    img.fill_rounded_rect(215, 86, 70, 64, 7, [10, 35, 70]);
    img.draw_rect_outline(215, 86, 70, 64, 2, [0, 164, 239]);
    img.draw_text_centered(&fonts.bold, cjk, "K", 20.0, 215 + 35, 106, [240, 246, 252]);

    // Hero instructions
    img.draw_text(&fonts.bold, cjk, "Pressione as teclas Win + K no Windows 10 ou 11 para conectar", 16.0, 315, 84, [255, 255, 255]);
    img.draw_text(&fonts.regular, cjk, "Abra o menu de Transmissão (Cast) e selecione Ext-Monitor na lista de Telas Sem Fio.", 12.0, 315, 110, [160, 185, 220]);
    img.draw_text(&fonts.bold, cjk, "● Status do Receptor: Aguardando requisição RTSP na porta TCP 7236 (WFD Ativo)...", 11.5, 315, 134, [86, 211, 100]);

    // 4 Instructions Quadrants
    struct InstQuad<'a> {
        flag: &'a str,
        lang: &'a str,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        border: [u8; 3],
        steps: [(&'a str, &'a str); 3],
    }

    let inst_quads = [
        InstQuad {
            flag: "[PT]",
            lang: "PORTUGUÊS (BRASIL)",
            x: 40,
            y: 184,
            w: 580,
            h: 212,
            border: [31, 111, 235],
            steps: [
                ("Passo 1:", "No seu teclado Windows, pressione Win + K (ou clique em Transmitir)"),
                ("Passo 2:", "Selecione Ext-Monitor na lista de monitores sem fio disponíveis"),
                ("Passo 3:", "Escolha Estender Área de Trabalho ou Duplicar com áudio digital HDMI"),
            ],
        },
        InstQuad {
            flag: "[EN]",
            lang: "ENGLISH",
            x: 660,
            y: 184,
            w: 580,
            h: 212,
            border: [56, 189, 248],
            steps: [
                ("Step 1:", "On your Windows PC, press Win + K (or click Cast in Action Center)"),
                ("Step 2:", "Select Ext-Monitor from the list of available Wireless Displays"),
                ("Step 3:", "Choose Extend desktop or Duplicate screen with low-latency audio"),
            ],
        },
        InstQuad {
            flag: "[IT]",
            lang: "ITALIANO",
            x: 40,
            y: 408,
            w: 580,
            h: 212,
            border: [163, 113, 247],
            steps: [
                ("Passo 1:", "Sulla tastiera di Windows premi Win + K (oppure clicca Trasmetti)"),
                ("Passo 2:", "Seleziona Ext-Monitor dall elenco dei dispositivi wireless"),
                ("Passo 3:", "Scegli Estendi desktop o Duplica schermo con audio digitale HDMI"),
            ],
        },
        InstQuad {
            flag: "[ZH]",
            lang: "繁體中文 (TRADITIONAL CHINESE)",
            x: 660,
            y: 408,
            w: 580,
            h: 212,
            border: [63, 185, 80],
            steps: [
                ("步驟 1:", "在 Windows 10/11 電腦上按下快速鍵 Win + K 進行投放"),
                ("步驟 2:", "在搜尋到的無線顯示器清單中點擊 Ext-Monitor"),
                ("步驟 3:", "選擇延伸桌面或同步複製螢幕，享受超低延遲音訊"),
            ],
        },
    ];

    for q in &inst_quads {
        img.fill_rounded_rect(q.x, q.y, q.w, q.h, 8, [18, 23, 36]);
        img.draw_rect_outline(q.x, q.y, q.w, q.h, 1, [32, 42, 60]);
        img.draw_line(q.x + 12, q.y, q.x + q.w - 12, q.y, q.border, 3);

        let title = format!("{}  {}", q.flag, q.lang);
        img.draw_text(&fonts.bold, cjk, &title, 13.0, q.x + 16, q.y + 12, [240, 246, 252]);

        let start_y = q.y + 36;
        for (s_idx, (s_num, s_desc)) in q.steps.iter().enumerate() {
            let sy = start_y + (s_idx * 54);
            img.fill_rounded_rect(q.x + 12, sy, q.w - 24, 48, 5, [24, 30, 46]);
            img.draw_text(&fonts.bold, cjk, s_num, 11.5, q.x + 20, sy + 6, q.border);
            img.draw_text(&fonts.regular, cjk, s_desc, 10.5, q.x + 20, sy + 24, [180, 195, 215]);
        }
    }

    img.draw_line(40, 628, WIDTH - 40, 628, [33, 44, 65], 1);

    let badges = [
        ("● WFD RTSP: 7236", [0, 164, 239]),
        ("● MS-MICE: 7250", [163, 113, 247]),
        ("● IP: 192.168.7.2", [56, 189, 248]),
        ("● Áudio: LPCM 48k", [46, 160, 67]),
        ("● Decodificador: VideoCore IV M2M", [234, 179, 8]),
        ("● Status: PRONTO PARA CONECTAR", [86, 211, 100]),
    ];

    let mut bx = 40;
    for (b_text, b_col) in &badges {
        let text_w = img.measure_text_width(&fonts.bold, cjk, b_text, 10.5);
        let bw = text_w + 20;
        img.fill_rounded_rect(bx, 638, bw, 24, 5, [20, 25, 38]);
        img.draw_rect_outline(bx, 638, bw, 24, 1, *b_col);
        img.draw_text_centered(&fonts.bold, cjk, b_text, 10.5, bx + bw / 2, 644, [230, 237, 243]);
        bx += bw + 10;
    }

    let foot = format!(
        "Raspberry Pi Zero W  •  Wi-Fi Display (Miracast WFD)  •  VideoCore IV KMS Framebuffer  •  Freeze Final {}",
        version
    );
    img.draw_text_centered(&fonts.regular, cjk, &foot, 11.5, WIDTH / 2, 686, [120, 130, 142]);

    save_splash(&img, output_dir, "splash_miracast")
}

/// Generates all splash screens (loading, ready, miracast) in 100% Pure Rust into all target locations
pub fn generate_all_splashes(project_root: &Path) -> Result<(), String> {
    let version = get_version();
    println!("\x1b[1;36m[*] Ext-Monitor 100% Pure-Rust Splash Generator — Versão: {}\x1b[0m", version);

    let fonts = FontSet::load();

    let targets = [
        project_root.join("build-appliance/overlay/etc"),
        project_root.join("build-appliance/initramfs/etc"),
        project_root.join("scripts/splash"),
    ];

    for target in &targets {
        println!("\x1b[1;34m[*] Gerando telas de splash em: {:?}\x1b[0m", target);
        generate_loading_splash(target, &fonts, &version)?;
        generate_ready_splash(target, &fonts, &version)?;
        generate_miracast_splash(target, &fonts, &version)?;
    }

    println!("\x1b[1;32m[+] Todas as telas de splash geradas com sucesso em Pure Rust com versão {}!\x1b[0m", version);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fontset_cjk_loading() {
        let fonts = FontSet::load();
        assert!(fonts.cjk.is_some(), "CJK font must be loaded");
        let cjk = fonts.cjk.as_ref().unwrap();
        let idx = cjk.lookup_glyph_index('繁');
        assert!(idx != 0, "Glyph index for 繁 must be non-zero");
        let (metrics, bitmap) = cjk.rasterize('繁', 14.0);
        assert!(metrics.width > 0, "繁 should have positive width");
        assert!(!bitmap.is_empty(), "繁 should have non-empty bitmap");
    }

    #[test]
    fn test_traditional_chinese_rendering() {
        let fonts = FontSet::load();
        let cjk = fonts.cjk.as_ref();
        let mut img = ImageBuffer::new(400, 100, [0, 0, 0]);
        let zh_sample = "繁體中文 模式 1 執行 投影 螢幕";
        let w = img.measure_text_width(&fonts.bold, cjk, zh_sample, 14.0);
        assert!(w > 100, "Measured width should be positive and substantial: {}", w);
        img.draw_text(&fonts.bold, cjk, zh_sample, 14.0, 10, 10, [255, 255, 255]);
        let non_zero_pixels = img.data.iter().any(|&p| p != [0, 0, 0]);
        assert!(non_zero_pixels, "Pixels must be drawn for Traditional Chinese text");
    }
}

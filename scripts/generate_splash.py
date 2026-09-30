#!/usr/bin/env python3
"""
ExtMonitor Splash Screen Generator (1280x720 RGB565)
Generates high-definition splash screens for:
1. Loading / Boot ("Aguarde carregando...") in 4 languages
2. Ready / Idle ("Pronto para Conexão - 4 Modos de Operação • IoT Media • Bluetooth A2DP") in 4 languages

Author: Carlos Alberto <carlosalberto4ti@gmail.com>
"""

import os
import gzip
from PIL import Image, ImageDraw, ImageFont

WIDTH = 1280
HEIGHT = 720

def get_font(size, bold=True, is_cjk=False):
    if is_cjk:
        cjk_paths = [
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc" if bold else "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        ]
        for p in cjk_paths:
            if os.path.exists(p):
                try:
                    return ImageFont.truetype(p, size)
                except Exception:
                    pass
    paths = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf" if bold else "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf" if bold else "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ]
    for p in paths:
        if os.path.exists(p):
            try:
                return ImageFont.truetype(p, size)
            except Exception:
                pass
    return ImageFont.load_default()

def draw_gradient_background(draw, width, height, top_color, bottom_color):
    for y in range(height):
        ratio = y / float(height)
        r = int(top_color[0] * (1 - ratio) + bottom_color[0] * ratio)
        g = int(top_color[1] * (1 - ratio) + bottom_color[1] * ratio)
        b = int(top_color[2] * (1 - ratio) + bottom_color[2] * ratio)
        draw.line([(0, y), (width, y)], fill=(r, g, b))

def make_rgb565(img):
    img = img.convert("RGB")
    raw = bytearray()
    for y in range(img.height):
        for x in range(img.width):
            r, g, b = img.getpixel((x, y))
            r5 = (r >> 3) & 0x1F
            g6 = (g >> 2) & 0x3F
            b5 = (b >> 3) & 0x1F
            val = (r5 << 11) | (g6 << 5) | b5
            raw.append(val & 0xFF)
            raw.append((val >> 8) & 0xFF)
    return bytes(raw)

def generate_loading_splash(output_dir):
    img = Image.new("RGB", (WIDTH, HEIGHT))
    draw = ImageDraw.Draw(img)

    # Deep high-tech dark background
    draw_gradient_background(draw, WIDTH, HEIGHT, (10, 14, 23), (16, 22, 36))

    # Ambient subtle grid lines
    for x in range(0, WIDTH, 80):
        draw.line([(x, 0), (x, HEIGHT)], fill=(20, 28, 45), width=1)
    for y in range(0, HEIGHT, 80):
        draw.line([(0, y), (WIDTH, y)], fill=(20, 28, 45), width=1)

    # Top Brand Header
    font_brand = get_font(28, bold=True)
    draw.text((WIDTH // 2, 70), "EXT-MONITOR PI ZERO", fill=(88, 166, 255), anchor="mm", font=font_brand)

    font_sub = get_font(14, bold=False)
    draw.text((WIDTH // 2, 105), "Appliance Minimal 33MB • VideoCore IV KMS • Áudio HDMI (Opus 48k) • Bluetooth A2DP • IoT Media Renderer", fill=(139, 148, 158), anchor="mm", font=font_sub)

    # Center Glowing Frame
    cx, cy = WIDTH // 2, 260
    r_outer = 65
    draw.ellipse([(cx - r_outer, cy - r_outer), (cx + r_outer, cy + r_outer)], outline=(31, 111, 235), width=3)
    draw.ellipse([(cx - 50, cy - 50), (cx + 50, cy + 50)], fill=(22, 27, 34), outline=(88, 166, 255), width=2)
    font_icon = get_font(36, bold=True)
    draw.text((cx, cy), "⚙", fill=(56, 189, 248), anchor="mm", font=font_icon)

    # Central Title
    font_title = get_font(23, bold=True)
    draw.text((WIDTH // 2, 355), "INICIANDO SISTEMA • INITIALIZING APPLIANCE", fill=(240, 246, 252), anchor="mm", font=font_title)

    # Loading bar
    bx, by, bw, bh = WIDTH // 2 - 260, 388, 520, 10
    draw.rounded_rectangle([(bx, by), (bx + bw, by + bh)], radius=5, fill=(33, 38, 45))
    draw.rounded_rectangle([(bx, by), (bx + 410, by + bh)], radius=5, fill=(56, 189, 248))

    # 4 Languages Loading Messages Box
    card_x, card_y, card_w, card_h = WIDTH // 2 - 420, 420, 840, 215
    draw.rounded_rectangle([(card_x, card_y), (card_x + card_w, card_y + card_h)], radius=12, fill=(22, 27, 34), outline=(48, 54, 61), width=1)

    font_lang = get_font(14, bold=False)
    font_lang_cjk = get_font(14, bold=False, is_cjk=True)
    font_tag = get_font(14, bold=True)

    messages = [
        ("[PT]", "Carregando GPU VideoCore IV, áudio HDMI, Bluetooth A2DP, UPnP/Cast e Visualizador 30 FPS...", (56, 189, 248), False),
        ("[EN]", "Initializing VideoCore IV GPU, HDMI audio, Bluetooth A2DP, UPnP/Cast & 30 FPS Visualizer...", (88, 166, 255), False),
        ("[IT]", "Caricamento GPU VideoCore IV, audio HDMI, Bluetooth A2DP, UPnP/Cast e Visualizzatore 30 FPS...", (163, 113, 247), False),
        ("[ZH]", "正在加载 VideoCore IV GPU、HDMI 音频、蓝牙 A2DP、UPnP/Cast 投屏与 30 FPS 动态频谱...", (63, 185, 80), True),
    ]

    for idx, (tag, text, col, is_cjk) in enumerate(messages):
        row_y = card_y + 18 + (idx * 48)
        draw.text((card_x + 25, row_y), tag, fill=col, font=font_tag)
        f_to_use = font_lang_cjk if is_cjk else font_lang
        draw.text((card_x + 90, row_y), text, fill=(201, 209, 217), font=f_to_use)

    # Footer
    font_foot = get_font(13, bold=False)
    draw.text((WIDTH // 2, 675), "Iniciando USB Gadget, Wi-Fi Display, Bluetooth A2DP Sink, SSDP Discovery, UPnP/DLNA e Painel Web 8080...", fill=(110, 118, 129), anchor="mm", font=font_foot)

    # Save PNG and Raw RGB565 GZ
    os.makedirs(output_dir, exist_ok=True)
    png_path = os.path.join(output_dir, "splash_loading.png")
    raw_gz_path = os.path.join(output_dir, "splash_loading.raw.gz")

    img.save(png_path)
    raw_bytes = make_rgb565(img)
    with gzip.open(raw_gz_path, "wb") as f:
        f.write(raw_bytes)

    print(f"Generated {png_path} and {raw_gz_path} ({len(raw_bytes)} bytes raw -> {os.path.getsize(raw_gz_path)} bytes gz)")

def generate_ready_splash(output_dir):
    img = Image.new("RGB", (WIDTH, HEIGHT))
    draw = ImageDraw.Draw(img)

    # Elegant deep gradient
    draw_gradient_background(draw, WIDTH, HEIGHT, (8, 12, 20), (14, 18, 30))

    # Top Banner Header
    font_brand = get_font(23, bold=True)
    draw.text((40, 36), "EXT-MONITOR PI ZERO", fill=(56, 189, 248), anchor="lm", font=font_brand)

    font_aud_tag = get_font(12, bold=True)
    draw.text((330, 36), "•  ÁUDIO DIGITAL HDMI & IOT MEDIA RENDERER", fill=(163, 113, 247), anchor="lm", font=font_aud_tag)

    font_badge = get_font(11, bold=True)
    badge_text = "● 60 FPS • ÁUDIO OPUS • BLUETOOTH A2DP • IOT CAST • VISUALIZADOR"
    draw.rounded_rectangle([(WIDTH - 530, 20), (WIDTH - 40, 52)], radius=6, fill=(18, 48, 28), outline=(46, 160, 67), width=1)
    draw.text((WIDTH - 285, 36), badge_text, fill=(86, 211, 100), anchor="mm", font=font_badge)

    draw.line([(40, 62), (WIDTH - 40, 62)], fill=(33, 38, 45), width=1)

    # 4 Language Quadrants Layout (2 columns x 2 rows)
    # Col 1: X = 40 to 620, Col 2: X = 660 to 1240
    # Row 1: Y = 70 to 338, Row 2: Y = 348 to 616
    quads = [
        {
            "lang": "PORTUGUÊS (BRASIL)",
            "flag": "[PT]",
            "x": 40, "y": 70, "w": 580, "h": 268,
            "border": (31, 111, 235),
            "modes": [
                ("Modo 1 (Rede IP Wayland):", "Execute ./scripts/start.sh (H.264 60 FPS + Áudio Opus 48k)"),
                ("Modo 2 (Windows Miracast):", "No Windows tecle Win+K e projete tela e áudio nativamente"),
                ("Modo 3 (USB Bulk Direto):", "Conecte cabo USB e use --transport=usb (Latência < 1ms)"),
                ("Modo 4 (Bluetooth & IoT Cast):", "Pareie celular (A2DP), UPnP/DLNA & YouTube Cast com espectro"),
            ]
        },
        {
            "lang": "ENGLISH",
            "flag": "[EN]",
            "x": 660, "y": 70, "w": 580, "h": 268,
            "border": (56, 189, 248),
            "modes": [
                ("Mode 1 (Wayland Network):", "Run ./scripts/start.sh (H.264 60 FPS + HDMI Audio Opus 48k)"),
                ("Mode 2 (Windows Miracast):", "Press Win+K on Windows to project screen & native audio"),
                ("Mode 3 (Direct USB Bulk):", "Connect USB cable with --transport=usb (< 1ms ultra latency)"),
                ("Mode 4 (Bluetooth & IoT Cast):", "Pair phone via A2DP, stream via UPnP or YouTube with visualizer"),
            ]
        },
        {
            "lang": "ITALIANO",
            "flag": "[IT]",
            "x": 40, "y": 348, "w": 580, "h": 268,
            "border": (163, 113, 247),
            "is_cjk": False,
            "modes": [
                ("Modo 1 (Rete IP Wayland):", "Esegui ./scripts/start.sh (H.264 60 FPS + Audio HDMI Opus 48k)"),
                ("Modo 2 (Windows Miracast):", "Su Windows premi Win+K per schermo e audio nativi"),
                ("Modo 3 (USB Diretto Bulk):", "Collega cavo USB con --transport=usb (Latenza < 1ms)"),
                ("Modo 4 (Bluetooth e IoT Cast):", "Associa smartphone A2DP, streaming UPnP/YouTube con spettro"),
            ]
        },
        {
            "lang": "中文 (CHINESE)",
            "flag": "[ZH]",
            "x": 660, "y": 348, "w": 580, "h": 268,
            "border": (63, 185, 80),
            "is_cjk": True,
            "modes": [
                ("模式 1 (Wayland 网络 IP):", "运行 ./scripts/start.sh (H.264 60 FPS + HDMI 音频 Opus 48k)"),
                ("模式 2 (Windows Miracast):", "Windows 10/11 按 Win+K 投影屏幕与立体声"),
                ("模式 3 (USB 硬件直连):", "连接 USB 线并使用 --transport=usb (超低延迟 < 1ms)"),
                ("模式 4 (蓝牙与 IoT 投屏):", "手机蓝牙 A2DP 连接、UPnP 与 YouTube 投屏伴随动态频谱"),
            ]
        },
    ]

    font_qlang = get_font(14, bold=True)
    font_qlang_cjk = get_font(14, bold=True, is_cjk=True)
    font_mlabel = get_font(12, bold=True)
    font_mlabel_cjk = get_font(12, bold=True, is_cjk=True)
    font_mdesc = get_font(11, bold=False)
    font_mdesc_cjk = get_font(11, bold=False, is_cjk=True)

    for q in quads:
        x, y, w, h = q["x"], q["y"], q["w"], q["h"]
        is_cjk = q.get("is_cjk", False)
        # Background card
        draw.rounded_rectangle([(x, y), (x + w, y + h)], radius=10, fill=(18, 22, 32), outline=(36, 44, 62), width=1)
        # Top Accent border inside card
        draw.line([(x + 12, y), (x + w - 12, y)], fill=q["border"], width=3)

        # Header of card
        draw.text((x + 18, y + 16), f"{q['flag']}  {q['lang']}", fill=(240, 246, 252), font=font_qlang_cjk if is_cjk else font_qlang)

        # Modes list (4 modes, 52px each)
        start_y = y + 40
        for m_idx, (m_label, m_desc) in enumerate(q["modes"]):
            item_y = start_y + (m_idx * 54)
            draw.rounded_rectangle([(x + 12, item_y), (x + w - 12, item_y + 48)], radius=6, fill=(24, 30, 44))
            draw.text((x + 20, item_y + 8), m_label, fill=q["border"], font=font_mlabel_cjk if is_cjk else font_mlabel)
            draw.text((x + 20, item_y + 26), m_desc, fill=(180, 190, 205), font=font_mdesc_cjk if is_cjk else font_mdesc)

    # Service Pills & Footer Bar
    draw.line([(40, 626), (WIDTH - 40, 626)], fill=(33, 38, 45), width=1)

    # Interactive services badges
    badges = [
        ("● Web UI: 192.168.7.2:8080", (31, 111, 235)),
        ("● YouTube Cast (DIAL)", (220, 38, 38)),
        ("● UPnP / DLNA AV", (163, 113, 247)),
        ("● Bluetooth A2DP Sink", (56, 189, 248)),
        ("● Espectro HDMI 30 FPS", (46, 160, 67)),
        ("● USB Bulk <1ms", (234, 179, 8)),
    ]
    font_badge_srv = get_font(11, bold=True)
    bx_cursor = 40
    for b_text, b_color in badges:
        bw_calc = int(len(b_text) * 7.2) + 20
        draw.rounded_rectangle([(bx_cursor, 636), (bx_cursor + bw_calc, 660)], radius=5, fill=(20, 25, 38), outline=b_color, width=1)
        draw.text((bx_cursor + bw_calc // 2, 648), b_text, fill=(230, 237, 243), anchor="mm", font=font_badge_srv)
        bx_cursor += bw_calc + 10

    font_foot = get_font(12, bold=False)
    foot_text = "Raspberry Pi Zero W  •  Alsa HDMI (bcm2835)  •  VideoCore IV KMS Framebuffer  •  Zero-Copy DMA  •  Freeze Final v2.3.0"
    draw.text((WIDTH // 2, 686), foot_text, fill=(120, 130, 142), anchor="mm", font=font_foot)

    # Save PNG and Raw RGB565 GZ
    os.makedirs(output_dir, exist_ok=True)
    png_path = os.path.join(output_dir, "splash_ready.png")
    raw_gz_path = os.path.join(output_dir, "splash_ready.raw.gz")

    img.save(png_path)
    raw_bytes = make_rgb565(img)
    with gzip.open(raw_gz_path, "wb") as f:
        f.write(raw_bytes)

    print(f"Generated {png_path} and {raw_gz_path} ({len(raw_bytes)} bytes raw -> {os.path.getsize(raw_gz_path)} bytes gz)")

if __name__ == "__main__":
    out_dir = "/home/carlos/ide/ext-monitor/build-appliance/overlay/etc"
    generate_loading_splash(out_dir)
    generate_ready_splash(out_dir)
    # Also save copies for artifacts/preview
    generate_loading_splash("/home/carlos/ide/ext-monitor/scripts/splash")
    generate_ready_splash("/home/carlos/ide/ext-monitor/scripts/splash")

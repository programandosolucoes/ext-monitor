#!/usr/bin/env python3
"""
ExtMonitor Splash Screen Generator (1280x720 RGB565)
Generates high-definition splash screens for:
1. Loading / Boot ("Aguarde carregando...") in 4 languages
2. Ready / Idle ("Pronto para Conexão - 3 Modos de Operação") in 4 languages

Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>
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
    draw.text((WIDTH // 2, 80), "EXT-MONITOR PI ZERO", fill=(88, 166, 255), anchor="mm", font=font_brand)

    font_sub = get_font(16, bold=False)
    draw.text((WIDTH // 2, 115), "Appliance Minimal 33MB • VideoCore IV VPU • Áudio Digital HDMI (ALSA Opus)", fill=(139, 148, 158), anchor="mm", font=font_sub)

    # Center Glowing Frame
    cx, cy = WIDTH // 2, 280
    r_outer = 65
    draw.ellipse([(cx - r_outer, cy - r_outer), (cx + r_outer, cy + r_outer)], outline=(31, 111, 235), width=3)
    draw.ellipse([(cx - 50, cy - 50), (cx + 50, cy + 50)], fill=(22, 27, 34), outline=(88, 166, 255), width=2)
    font_icon = get_font(36, bold=True)
    draw.text((cx, cy), "⚙", fill=(56, 189, 248), anchor="mm", font=font_icon)

    # Central Title
    font_title = get_font(24, bold=True)
    draw.text((WIDTH // 2, 380), "INICIANDO SISTEMA • INITIALIZING SYSTEM", fill=(240, 246, 252), anchor="mm", font=font_title)

    # Loading bar
    bx, by, bw, bh = WIDTH // 2 - 250, 415, 500, 10
    draw.rounded_rectangle([(bx, by), (bx + bw, by + bh)], radius=5, fill=(33, 38, 45))
    draw.rounded_rectangle([(bx, by), (bx + 340, by + bh)], radius=5, fill=(56, 189, 248))

    # 4 Languages Loading Messages Box
    card_x, card_y, card_w, card_h = WIDTH // 2 - 380, 460, 760, 190
    draw.rounded_rectangle([(card_x, card_y), (card_x + card_w, card_y + card_h)], radius=12, fill=(22, 27, 34), outline=(48, 54, 61), width=1)

    font_lang = get_font(15, bold=False)
    font_lang_cjk = get_font(15, bold=False, is_cjk=True)
    font_tag = get_font(14, bold=True)

    messages = [
        ("[PT]", "Carregando drivers GPU VideoCore IV, áudio HDMI e rede...", (56, 189, 248), False),
        ("[EN]", "Initializing VideoCore IV GPU, HDMI audio & network...", (88, 166, 255), False),
        ("[IT]", "Caricamento driver GPU VideoCore IV, audio HDMI e rete...", (163, 113, 247), False),
        ("[ZH]", "正在加载 VideoCore IV GPU、HDMI 音频与网络驱动...", (63, 185, 80), True),
    ]

    for idx, (tag, text, col, is_cjk) in enumerate(messages):
        row_y = card_y + 25 + (idx * 38)
        draw.text((card_x + 30, row_y), tag, fill=col, font=font_tag)
        f_to_use = font_lang_cjk if is_cjk else font_lang
        draw.text((card_x + 105, row_y), text, fill=(201, 209, 217), font=f_to_use)

    # Footer
    font_foot = get_font(14, bold=False)
    draw.text((WIDTH // 2, 680), "Iniciando USB Gadget, Wi-Fi Display, Áudio Opus 48kHz e Painel Web...", fill=(110, 118, 129), anchor="mm", font=font_foot)

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
    font_brand = get_font(26, bold=True)
    draw.text((40, 38), "EXT-MONITOR PI ZERO", fill=(56, 189, 248), anchor="lm", font=font_brand)

    font_badge = get_font(13, bold=True)
    badge_text = "● VÍDEO & ÁUDIO HDMI PRONTOS / READY"
    draw.rounded_rectangle([(WIDTH - 380, 24), (WIDTH - 40, 52)], radius=6, fill=(18, 48, 28), outline=(46, 160, 67), width=1)
    draw.text((WIDTH - 210, 38), badge_text, fill=(86, 211, 100), anchor="mm", font=font_badge)

    font_aud_tag = get_font(13, bold=True)
    draw.text((360, 38), "•  ÁUDIO DIGITAL HDMI ATIVO (ALSA Opus 48kHz)", fill=(163, 113, 247), anchor="lm", font=font_aud_tag)

    draw.line([(40, 68), (WIDTH - 40, 68)], fill=(33, 38, 45), width=1)

    # 4 Language Quadrants Layout (2 columns x 2 rows)
    # Col 1: X = 40 to 620, Col 2: X = 660 to 1240
    # Row 1: Y = 82 to 345, Row 2: Y = 360 to 625
    quads = [
        # (title, flag, [ (mode_label, mode_desc) ], x, y, width, height, border_color)
        {
            "lang": "PORTUGUÊS (BRASIL)",
            "flag": "[PT]",
            "x": 40, "y": 80, "w": 580, "h": 270,
            "border": (31, 111, 235),
            "modes": [
                ("Modo 1 (Rede IP):", "Execute ./scripts/start.sh (Vídeo H.264 + Áudio HDMI Opus 48k)"),
                ("Modo 2 (Miracast):", "No Windows 10/11 tecle Win+K e projete tela e áudio nativos"),
                ("Modo 3 (USB Direto):", "Conecte cabo USB e use --transport=usb (Latência < 1ms)"),
            ]
        },
        {
            "lang": "ENGLISH",
            "flag": "[EN]",
            "x": 660, "y": 80, "w": 580, "h": 270,
            "border": (56, 189, 248),
            "modes": [
                ("Mode 1 (Network IP):", "Run ./scripts/start.sh (H.264 Video + HDMI Audio Opus 48k)"),
                ("Mode 2 (Miracast):", "On Windows 10/11 press Win+K to project screen & native sound"),
                ("Mode 3 (Direct USB):", "Connect USB cable and pass --transport=usb (< 1ms latency)"),
            ]
        },
        {
            "lang": "ITALIANO",
            "flag": "[IT]",
            "x": 40, "y": 365, "w": 580, "h": 270,
            "border": (163, 113, 247),
            "is_cjk": False,
            "modes": [
                ("Modo 1 (Rete IP):", "Esegui ./scripts/start.sh (Video H.264 + Audio HDMI Opus 48k)"),
                ("Modo 2 (Miracast):", "Su Windows 10/11 premi Win+K per schermo e audio nativi"),
                ("Modo 3 (USB Diretto):", "Collega cavo USB e usa --transport=usb (Latenza < 1ms)"),
            ]
        },
        {
            "lang": "中文 (CHINESE)",
            "flag": "[ZH]",
            "x": 660, "y": 365, "w": 580, "h": 270,
            "border": (63, 185, 80),
            "is_cjk": True,
            "modes": [
                ("模式 1 (网络 IP):", "运行 ./scripts/start.sh (H.264 视频 + HDMI 音频 Opus 48k)"),
                ("模式 2 (Miracast):", "Windows 10/11 按 Win+K 投影屏幕与声音"),
                ("模式 3 (USB 直连):", "连接 USB 线并使用 --transport=usb (超低延迟 < 1ms)"),
            ]
        },
    ]

    font_qlang = get_font(16, bold=True)
    font_qlang_cjk = get_font(16, bold=True, is_cjk=True)
    font_mlabel = get_font(14, bold=True)
    font_mlabel_cjk = get_font(14, bold=True, is_cjk=True)
    font_mdesc = get_font(13, bold=False)
    font_mdesc_cjk = get_font(13, bold=False, is_cjk=True)

    for q in quads:
        x, y, w, h = q["x"], q["y"], q["w"], q["h"]
        is_cjk = q.get("is_cjk", False)
        # Background card
        draw.rounded_rectangle([(x, y), (x + w, y + h)], radius=10, fill=(18, 22, 32), outline=(36, 44, 62), width=1)
        # Top Accent border inside card
        draw.line([(x + 12, y), (x + w - 12, y)], fill=q["border"], width=3)

        # Header of card
        draw.text((x + 20, y + 22), f"{q['flag']}  {q['lang']}", fill=(240, 246, 252), font=font_qlang_cjk if is_cjk else font_qlang)

        # Modes list
        start_y = y + 55
        for m_idx, (m_label, m_desc) in enumerate(q["modes"]):
            item_y = start_y + (m_idx * 65)
            # Item background pill
            draw.rounded_rectangle([(x + 15, item_y), (x + w - 15, item_y + 55)], radius=6, fill=(24, 30, 44))
            draw.text((x + 28, item_y + 12), m_label, fill=q["border"], font=font_mlabel_cjk if is_cjk else font_mlabel)
            draw.text((x + 28, item_y + 32), m_desc, fill=(180, 190, 205), font=font_mdesc_cjk if is_cjk else font_mdesc)

    # Footer Information Bar
    draw.line([(40, 650), (WIDTH - 40, 650)], fill=(33, 38, 45), width=1)
    font_foot = get_font(14, bold=False)
    foot_text = "IP: 192.168.7.2  •  Dashboard: http://192.168.7.2:8080  •  Áudio HDMI: Opus 48kHz (UDP 5004)  •  VideoCore IV KMS"
    draw.text((WIDTH // 2, 680), foot_text, fill=(110, 118, 129), anchor="mm", font=font_foot)

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

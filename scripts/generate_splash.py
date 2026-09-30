#!/usr/bin/env python3
"""
ExtMonitor Pure-Rust/Python Splash Screen Generator (1280x720 RGB565)
Generates high-definition splash screens for:
1. Loading / Boot ("Aguarde carregando...") in 4 languages
2. Ready / Idle ("Pronto para Conexão - 4 Modos de Operação • IoT Media • Bluetooth A2DP") in 4 languages
3. Miracast / Wi-Fi Display ("Modo 2: Windows Miracast Win+K") in 4 languages

Author: Carlos Alberto <carlosalberto4ti@gmail.com>
"""

import os
import gzip
from PIL import Image, ImageDraw, ImageFont

WIDTH = 1280
HEIGHT = 720

def get_font(size, bold=True, is_cjk=False):
    if is_cjk:
        for p in [
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc" if bold else "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        ]:
            if os.path.exists(p):
                try:
                    return ImageFont.truetype(p, size)
                except Exception:
                    pass
    for p in [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf" if bold else "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf" if bold else "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ]:
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

def generate_miracast_splash(output_dir):
    img = Image.new("RGB", (WIDTH, HEIGHT))
    draw = ImageDraw.Draw(img)

    # Deep gradient background
    draw_gradient_background(draw, WIDTH, HEIGHT, (8, 14, 26), (15, 22, 38))

    # Subtle grid
    for x in range(0, WIDTH, 80):
        draw.line([(x, 0), (x, HEIGHT)], fill=(18, 26, 44), width=1)
    for y in range(0, HEIGHT, 80):
        draw.line([(0, y), (WIDTH, y)], fill=(18, 26, 44), width=1)

    # Header
    font_brand = get_font(23, bold=True)
    draw.text((40, 36), "EXT-MONITOR PI ZERO", fill=(56, 189, 248), anchor="lm", font=font_brand)

    font_tag = get_font(13, bold=True)
    draw.text((330, 36), "•  MODO 2: WINDOWS MIRACAST (WI-FI DISPLAY)", fill=(0, 164, 239), anchor="lm", font=font_tag)

    badge_text = "● RTSP 7236 • MS-MICE 7250 • 60 FPS • ÁUDIO ESTÉREO"
    font_badge = get_font(11, bold=True)
    draw.rounded_rectangle([(WIDTH - 490, 20), (WIDTH - 40, 52)], radius=6, fill=(18, 48, 28), outline=(46, 160, 67), width=1)
    draw.text((WIDTH - 265, 36), badge_text, fill=(86, 211, 100), anchor="mm", font=font_badge)

    draw.line([(40, 60), (WIDTH - 40, 60)], fill=(33, 44, 65), width=1)

    # Hero Banner
    draw.rounded_rectangle([(40, 68), (WIDTH - 40, 172)], radius=10, fill=(18, 25, 42), outline=(0, 164, 239), width=2)

    # Keys visual: [ ⊞ Win ] + [ K ]
    draw.rounded_rectangle([(65, 88), (170, 152)], radius=8, fill=(10, 35, 70), outline=(0, 164, 239), width=2)
    font_key = get_font(20, bold=True)
    draw.text((117, 120), "⊞ Win", fill=(240, 246, 252), anchor="mm", font=font_key)

    font_plus = get_font(24, bold=True)
    draw.text((192, 118), "+", fill=(56, 189, 248), anchor="mm", font=font_plus)

    draw.rounded_rectangle([(215, 88), (285, 152)], radius=8, fill=(10, 35, 70), outline=(0, 164, 239), width=2)
    draw.text((250, 120), "K", fill=(240, 246, 252), anchor="mm", font=font_key)

    # Instructions next to keys
    font_hero_title = get_font(18, bold=True)
    draw.text((315, 96), "Pressione as teclas Win + K no Windows 10 ou 11 para conectar", fill=(255, 255, 255), font=font_hero_title)

    font_hero_sub = get_font(13, bold=False)
    draw.text((315, 122), "Abra o menu de Transmissão (Cast) e selecione Ext-Monitor na lista de Telas Sem Fio.", fill=(160, 185, 220), font=font_hero_sub)

    font_status_live = get_font(12, bold=True)
    draw.text((315, 144), "● Status do Receptor: Aguardando requisição RTSP na porta TCP 7236 (WFD Ativo)...", fill=(86, 211, 100), font=font_status_live)

    # 4 Multilingual Quadrants (2 x 2)
    quads = [
        {
            "lang": "PORTUGUÊS (BRASIL)", "flag": "[PT]",
            "x": 40, "y": 184, "w": 580, "h": 212,
            "border": (31, 111, 235), "is_cjk": False,
            "steps": [
                ("Passo 1:", "No seu teclado Windows, pressione Win + K (ou clique em Transmitir na barra de tarefas)"),
                ("Passo 2:", "Selecione Ext-Monitor na lista de monitores e telas sem fio disponíveis"),
                ("Passo 3:", "Escolha Estender Área de Trabalho ou Duplicar com áudio digital HDMI"),
            ]
        },
        {
            "lang": "ENGLISH", "flag": "[EN]",
            "x": 660, "y": 184, "w": 580, "h": 212,
            "border": (56, 189, 248), "is_cjk": False,
            "steps": [
                ("Step 1:", "On your Windows PC, press Win + K (or click Cast in Action Center / Quick Settings)"),
                ("Step 2:", "Select Ext-Monitor from the list of available Wireless Displays"),
                ("Step 3:", "Choose Extend desktop or Duplicate screen with low-latency HDMI audio"),
            ]
        },
        {
            "lang": "ITALIANO", "flag": "[IT]",
            "x": 40, "y": 408, "w": 580, "h": 212,
            "border": (163, 113, 247), "is_cjk": False,
            "steps": [
                ("Passo 1:", "Sulla tastiera di Windows premi Win + K (oppure clicca Trasmetti nelle notifiche)"),
                ("Passo 2:", "Seleziona Ext-Monitor dall elenco dei dispositivi di proiezione wireless"),
                ("Passo 3:", "Scegli Estendi desktop o Duplica schermo con audio digitale HDMI integrato"),
            ]
        },
        {
            "lang": "中文 (CHINESE)", "flag": "[ZH]",
            "x": 660, "y": 408, "w": 580, "h": 212,
            "border": (63, 185, 80), "is_cjk": True,
            "steps": [
                ("步骤 1:", "在 Windows 10/11 电脑上按下快捷键 Win + K（或点击操作中心中的投屏）"),
                ("步骤 2:", "在搜索到的可用无线显示器列表中点击选择 Ext-Monitor 进行连接"),
                ("步骤 3:", "选择扩展桌面或复制主屏幕，享受低延迟硬件加速与数字音频"),
            ]
        },
    ]

    font_qlang = get_font(13, bold=True)
    font_qlang_cjk = get_font(13, bold=True, is_cjk=True)
    font_snum = get_font(11, bold=True)
    font_snum_cjk = get_font(11, bold=True, is_cjk=True)
    font_sdesc = get_font(11, bold=False)
    font_sdesc_cjk = get_font(11, bold=False, is_cjk=True)

    for q in quads:
        x, y, w, h = q["x"], q["y"], q["w"], q["h"]
        is_cjk = q["is_cjk"]
        draw.rounded_rectangle([(x, y), (x + w, y + h)], radius=8, fill=(18, 23, 36), outline=(32, 42, 60), width=1)
        draw.line([(x + 12, y), (x + w - 12, y)], fill=q["border"], width=3)
        draw.text((x + 16, y + 14), f"{q['flag']}  {q['lang']}", fill=(240, 246, 252), font=font_qlang_cjk if is_cjk else font_qlang)

        start_y = y + 38
        for s_idx, (s_num, s_desc) in enumerate(q["steps"]):
            sy = start_y + (s_idx * 54)
            draw.rounded_rectangle([(x + 12, sy), (x + w - 12, sy + 48)], radius=5, fill=(24, 30, 46))
            draw.text((x + 20, sy + 8), s_num, fill=q["border"], font=font_snum_cjk if is_cjk else font_snum)
            draw.text((x + 20, sy + 25), s_desc, fill=(180, 195, 215), font=font_sdesc_cjk if is_cjk else font_sdesc)

    # Footer bar
    draw.line([(40, 630), (WIDTH - 40, 630)], fill=(33, 44, 65), width=1)

    badges = [
        ("● WFD RTSP: 7236", (0, 164, 239)),
        ("● MS-MICE: 7250", (163, 113, 247)),
        ("● IP: 192.168.7.2", (56, 189, 248)),
        ("● Áudio: LPCM 48k", (46, 160, 67)),
        ("● Decodificador: VideoCore IV M2M", (234, 179, 8)),
        ("● Status: PRONTO PARA CONECTAR", (86, 211, 100)),
    ]
    font_badge_srv = get_font(11, bold=True)
    bx_cursor = 40
    for b_text, b_color in badges:
        bw_calc = int(len(b_text) * 7.2) + 20
        draw.rounded_rectangle([(bx_cursor, 640), (bx_cursor + bw_calc, 664)], radius=5, fill=(20, 25, 38), outline=b_color, width=1)
        draw.text((bx_cursor + bw_calc // 2, 652), b_text, fill=(230, 237, 243), anchor="mm", font=font_badge_srv)
        bx_cursor += bw_calc + 10

    font_foot = get_font(12, bold=False)
    foot_text = "Raspberry Pi Zero W  •  Wi-Fi Display (Miracast WFD)  •  VideoCore IV KMS Framebuffer  •  Freeze Final v2.3.0"
    draw.text((WIDTH // 2, 690), foot_text, fill=(120, 130, 142), anchor="mm", font=font_foot)

    os.makedirs(output_dir, exist_ok=True)
    out_png = os.path.join(output_dir, "splash_miracast.png")
    out_raw_gz = os.path.join(output_dir, "splash_miracast.raw.gz")
    img.save(out_png)
    raw_bytes = make_rgb565(img)
    with gzip.open(out_raw_gz, "wb") as f:
        f.write(raw_bytes)

    print(f"Generated {out_png} and {out_raw_gz} ({len(raw_bytes)} bytes raw -> {os.path.getsize(out_raw_gz)} bytes gz)")

if __name__ == "__main__":
    for d in ["/home/carlos/ide/ext-monitor/build-appliance/overlay/etc",
              "/home/carlos/ide/ext-monitor/build-appliance/initramfs/etc",
              "/home/carlos/ide/ext-monitor/scripts/splash"]:
        generate_miracast_splash(d)

#!/usr/bin/env python3
"""
Test script to generate a single frame of the dual monitor extended desktop.
"""

from PIL import Image, ImageDraw, ImageFont
import os

def render_frame(progress=0.0):
    canvas_w, canvas_h = 1200, 620
    img = Image.new("RGBA", (canvas_w, canvas_h), (12, 16, 24, 255))
    draw = ImageDraw.Draw(img)

    # Fonts
    font_title = ImageFont.truetype("/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf", 15)
    font_label = ImageFont.truetype("/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf", 12)
    font_mono = ImageFont.truetype("/usr/share/fonts/truetype/liberation/LiberationMono-Bold.ttf", 11)
    font_sm = ImageFont.truetype("/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf", 11)
    font_xs = ImageFont.truetype("/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf", 10)

    # 1. Top Bar
    draw.rectangle([(0, 0), (canvas_w, 40)], fill=(8, 12, 18, 255))
    draw.line([(0, 40), (canvas_w, 40)], fill=(0, 229, 255, 100), width=1)
    draw.text((20, 11), "⚡ ext-monitor: Demonstração de Área de Trabalho Estendida (Linux Wayland ➔ Raspberry Pi Zero)", fill=(255, 255, 255), font=font_title)

    # Badges
    badges = [
        ("60 FPS", (0, 255, 102)),
        ("18ms Latência", (0, 229, 255)),
        ("VideoCore IV DMA", (179, 136, 255)),
        ("Zero-Gateway DHCP", (255, 179, 0)),
    ]
    bx = canvas_w - 20
    for text, color in reversed(badges):
        bbox = font_mono.getbbox(text)
        bw = (bbox[2] - bbox[0]) + 12
        bx -= bw + 8
        draw.rounded_rectangle([(bx, 8), (bx + bw, 32)], radius=4, fill=(18, 26, 40, 255), outline=color, width=1)
        draw.text((bx + 6, 12), text, fill=color, font=font_mono)

    # Monitor 1 Geometry (Left - Laptop)
    m1_x, m1_y, m1_w, m1_h = 30, 55, 550, 475
    # Monitor 2 Geometry (Right - Pi Zero)
    m2_x, m2_y, m2_w, m2_h = 620, 55, 550, 475

    # Draw Monitor 1 Bezel
    draw.rounded_rectangle([(m1_x, m1_y), (m1_x + m1_w, m1_y + m1_h)], radius=10, fill=(28, 33, 43, 255), outline=(60, 70, 85, 255), width=2)
    # Screen 1 inner
    s1_x, s1_y, s1_w, s1_h = m1_x + 12, m1_y + 30, m1_w - 24, m1_h - 42
    draw.rectangle([(s1_x, s1_y), (s1_x + s1_w, s1_y + s1_h)], fill=(16, 22, 34, 255))
    draw.text((m1_x + 16, m1_y + 8), "💻 Monitor 1: Notebook Host (GNOME Wayland eDP-1 • 1920x1080)", fill=(180, 200, 220), font=font_label)

    # Draw Monitor 2 Bezel
    draw.rounded_rectangle([(m2_x, m2_y), (m2_x + m2_w, m2_y + m2_h)], radius=10, fill=(28, 33, 43, 255), outline=(0, 229, 255, 120), width=2)
    # Screen 2 inner
    s2_x, s2_y, s2_w, s2_h = m2_x + 12, m2_y + 30, m2_w - 24, m2_h - 42
    draw.rectangle([(s2_x, s2_y), (s2_x + s2_w, s2_y + s2_h)], fill=(16, 22, 34, 255))
    draw.text((m2_x + 16, m2_y + 8), "🖥️ Monitor 2: Raspberry Pi Zero (HDMI-1 • 60 FPS VideoCore IV KMS)", fill=(0, 229, 255), font=font_label)

    # Wallpaper gradient / grid on both screens
    for sy in range(0, s1_h, 30):
        draw.line([(s1_x, s1_y + sy), (s1_x + s1_w, s1_y + sy)], fill=(22, 30, 46, 255), width=1)
        draw.line([(s2_x, s2_y + sy), (s2_x + s2_w, s2_y + sy)], fill=(22, 30, 46, 255), width=1)

    # Screen 1 background window: Code editor
    draw.rounded_rectangle([(s1_x + 20, s1_y + 30), (s1_x + 360, s1_y + 320)], radius=6, fill=(20, 25, 35, 240), outline=(40, 50, 70), width=1)
    draw.rectangle([(s1_x + 20, s1_y + 30), (s1_x + 360, s1_y + 54)], fill=(30, 36, 50, 255))
    draw.text((s1_x + 30, s1_y + 36), "main.rs - ext-sender", fill=(150, 170, 200), font=font_xs)
    code_lines = [
        "use ext_sender::encoder::VaapiEncoder;",
        "let mut encoder = VaapiEncoder::new('/dev/dri/renderD128')?;",
        "println!('[+] Hardware VA-API Encoder Online');",
        "loop {",
        "    let frame = screencast.next_dma_buf()?;",
        "    encoder.encode_nal(&frame, &udp_socket)?;",
        "}"
    ]
    for idx, cl in enumerate(code_lines):
        draw.text((s1_x + 30, s1_y + 65 + idx * 16), cl, fill=(120, 180, 240) if "VA-API" in cl else (160, 175, 195), font=font_mono)

    # Screen 2 overlay: Translucent Diagnostic HUD in top-right
    hud_w, hud_h = 240, 95
    hud_x = s2_x + s2_w - hud_w - 12
    hud_y = s2_y + 12
    draw.rounded_rectangle([(hud_x, hud_y), (hud_x + hud_w, hud_y + hud_h)], radius=6, fill=(8, 14, 24, 230), outline=(0, 229, 255, 180), width=1)
    draw.text((hud_x + 10, hud_y + 8), "⚡ VideoCore IV V4L2 M2M", fill=(0, 229, 255), font=font_label)
    draw.text((hud_x + 10, hud_y + 28), "• FPS Real:     60.0 FPS [Fluido]", fill=(0, 255, 102), font=font_mono)
    draw.text((hud_x + 10, hud_y + 44), "• Latência Link: 18.2 ms [Sub-20ms]", fill=(0, 229, 255), font=font_mono)
    draw.text((hud_x + 10, hud_y + 60), "• CPU no Pi:    0.4% [Hardware Puro]", fill=(255, 179, 0), font=font_mono)
    draw.text((hud_x + 10, hud_y + 76), "• Link USB:     192.168.7.2 (OTG)", fill=(179, 136, 255), font=font_mono)

    # 4. The Moving File Manager Window!
    # Virtual X from 40 to (s1_w + s2_w - win_w)
    win_w, win_h = 310, 210
    total_travel = (s1_w + s2_w) - win_w - 60
    virt_x = 40 + progress * total_travel
    win_y = s1_y + 80

    # Render File Manager window onto a transparent scratch surface
    win_img = Image.new("RGBA", (win_w, win_h), (0, 0, 0, 0))
    w_draw = ImageDraw.Draw(win_img)
    # Window base
    w_draw.rounded_rectangle([(0, 0), (win_w, win_h)], radius=8, fill=(24, 30, 42, 250), outline=(0, 229, 255, 200) if progress > 0.8 else (70, 85, 110, 255), width=2)
    # Titlebar
    w_draw.rounded_rectangle([(0, 0), (win_w, 28)], radius=8, fill=(35, 44, 60, 255))
    w_draw.rectangle([(0, 20), (win_w, 28)], fill=(35, 44, 60, 255))
    # Window buttons
    w_draw.ellipse([(8, 8), (18, 18)], fill=(255, 95, 87))
    w_draw.ellipse([(24, 8), (34, 18)], fill=(255, 189, 46))
    w_draw.ellipse([(40, 8), (50, 18)], fill=(39, 201, 63))
    w_draw.text((60, 6), "📁 Arquivos - /home/carlos/ext-monitor", fill=(220, 235, 255), font=font_label)

    # Sidebar
    w_draw.rectangle([(0, 28), (95, win_h - 22)], fill=(20, 25, 36, 255))
    w_draw.line([(95, 28), (95, win_h - 22)], fill=(45, 55, 75, 255), width=1)
    sidebar_items = ["Início", "Documentos", "ext-monitor", "Downloads", "Vídeos"]
    for idx, item in enumerate(sidebar_items):
        iy = 36 + idx * 22
        if item == "ext-monitor":
            w_draw.rounded_rectangle([(4, iy - 2), (91, iy + 16)], radius=4, fill=(0, 140, 255, 180))
            w_draw.text((12, iy), "📂 " + item, fill=(255, 255, 255), font=font_xs)
        else:
            w_draw.text((12, iy), "📁 " + item, fill=(140, 160, 185), font=font_xs)

    # Files Grid
    files = [
        ("📁 src", "Pasta"),
        ("📁 release", "Pasta"),
        ("📄 README.md", "14 KB"),
        ("💾 ext-monitor-pi0.img", "32 MB"),
        ("🦀 Cargo.toml", "243 B"),
        ("⚙️ config.txt", "1.2 KB")
    ]
    for idx, (fname, fsize) in enumerate(files):
        col = idx % 2
        row = idx // 2
        fx = 108 + col * 100
        fy = 36 + row * 45
        is_target = "ext-monitor-pi0.img" in fname and progress > 0.85
        if is_target:
            w_draw.rounded_rectangle([(fx - 4, fy - 2), (fx + 94, fy + 38)], radius=4, fill=(0, 229, 255, 50), outline=(0, 229, 255), width=1)
        w_draw.text((fx, fy), fname, fill=(0, 229, 255) if is_target else (220, 230, 245), font=font_xs)
        w_draw.text((fx + 16, fy + 16), fsize, fill=(120, 140, 165), font=font_xs)

    # Window status footer
    w_draw.rectangle([(0, win_h - 22), (win_w, win_h)], fill=(28, 35, 48, 255))
    footer_text = "✓ Funcionando no Monitor 2 (Pi Zero)" if progress > 0.85 else "6 itens • 32.4 MB livres"
    w_draw.text((12, win_h - 18), footer_text, fill=(0, 255, 102) if progress > 0.85 else (140, 160, 185), font=font_xs)

    # 5. Composite Window onto Screen 1 and Screen 2 with Exact Boundary Splitting!
    # virt_x is relative to s1_x (0 at s1_x)
    # Split point is s1_w
    if virt_x + win_w <= s1_w:
        # Fully inside Screen 1
        img.paste(win_img, (int(s1_x + virt_x), int(win_y)), win_img)
    elif virt_x < s1_w and virt_x + win_w > s1_w:
        # STRADDLING BOTH SCREENS!
        # Left portion on Screen 1: width = s1_w - virt_x
        split_w = int(s1_w - virt_x)
        left_crop = win_img.crop((0, 0, split_w, win_h))
        img.paste(left_crop, (int(s1_x + virt_x), int(win_y)), left_crop)

        # Right portion on Screen 2: width = win_w - split_w
        right_crop = win_img.crop((split_w, 0, win_w, win_h))
        img.paste(right_crop, (int(s2_x), int(win_y)), right_crop)
    else:
        # Fully inside Screen 2!
        s2_virt_x = virt_x - s1_w
        img.paste(win_img, (int(s2_x + s2_virt_x), int(win_y)), win_img)

    # 6. Mouse Cursor
    cursor_x = int(s1_x + virt_x + 140) if virt_x + win_w <= s1_w else (
        int(s1_x + virt_x + 140) if (virt_x + 140) < s1_w else int(s2_x + (virt_x + 140 - s1_w))
    )
    cursor_y = int(win_y + 12) if progress < 0.85 else int(win_y + 90) # moves to file at end
    # Draw arrow cursor
    draw.polygon([(cursor_x, cursor_y), (cursor_x + 12, cursor_y + 12), (cursor_x + 5, cursor_y + 12), (cursor_x + 9, cursor_y + 19), (cursor_x + 6, cursor_y + 20), (cursor_x + 2, cursor_y + 13), (cursor_x, cursor_y + 15)], fill=(255, 255, 255), outline=(0, 0, 0))

    # 7. Bottom Hardware Connection Diagram
    draw.rectangle([(0, canvas_h - 55), (canvas_w, canvas_h)], fill=(8, 12, 18, 255))
    draw.line([(0, canvas_h - 55), (canvas_w, canvas_h - 55)], fill=(0, 229, 255, 60), width=1)
    diag_text = "🔗 Fluxo de Hardware: [ Notebook Host ]  ──[ Cabo USB OTG 480 Mbps (Zero-Gateway DHCP) ]──▶  [ Raspberry Pi Zero v1.3 (BCM2835) ]  ──[ Cabo mini-HDMI ]──▶  [ Monitor Secundário 60 FPS ]"
    draw.text((30, canvas_h - 38), diag_text, fill=(160, 190, 220), font=font_label)

    return img

if __name__ == "__main__":
    test_img = render_frame(0.45) # 45% = right in the middle of splitting!
    test_img.save("docs/assets/test_frame.png")
    print("Test frame saved to docs/assets/test_frame.png")

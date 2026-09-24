#!/usr/bin/env python3
import os
from PIL import Image, ImageDraw, ImageFont, ImageFilter

def create_splash():
    WIDTH = 1600
    HEIGHT = 900
    
    bg_path = "/home/carlos/.gemini/antigravity-ide/brain/75679085-ed8d-4905-9fe8-69518ca69c49/splash_bg_1790278876510.jpg"
    if os.path.exists(bg_path):
        bg = Image.open(bg_path).convert("RGBA")
        bg = bg.resize((WIDTH, HEIGHT), Image.Resampling.LANCZOS)
    else:
        bg = Image.new("RGBA", (WIDTH, HEIGHT), (10, 14, 23, 255))
    
    # Dark frosted glass overlay
    overlay = Image.new("RGBA", (WIDTH, HEIGHT), (8, 12, 20, 200))
    img = Image.alpha_composite(bg, overlay)
    draw = ImageDraw.Draw(img)
    
    # Fonts
    font_title = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 36)
    font_sub = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 17)
    font_badge = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 13)
    font_card_title = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 20)
    font_body = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 15)
    font_bold = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 15)
    font_code = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf", 15)
    font_code_sm = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf", 12)
    font_footer = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf", 13)

    # Top Badge
    badge_text = "• RASPBERRY PI ZERO W  |  HARDWARE GPU SECOND MONITOR ENGINE •"
    bbox = font_badge.getbbox(badge_text)
    bw, bh = bbox[2] - bbox[0], bbox[3] - bbox[1]
    bx, by = (WIDTH - bw) // 2, 45
    pad_x, pad_y = 18, 6
    draw.rounded_rectangle([bx - pad_x, by - pad_y, bx + bw + pad_x, by + bh + pad_y], radius=8, fill=(0, 229, 255, 30), outline=(0, 229, 255, 180), width=1)
    draw.text((bx, by), badge_text, font=font_badge, fill=(0, 229, 255, 255))

    # Main Title
    title_text = "Monitor Estendido por Aceleração de Hardware GPU"
    t_bbox = font_title.getbbox(title_text)
    tw = t_bbox[2] - t_bbox[0]
    draw.text(((WIDTH - tw) // 2, 85), title_text, font=font_title, fill=(255, 255, 255, 255))

    # Subtitle
    sub_text = "Sessão Ubuntu Wayland  •  AMD Radeon 610M VA-API  •  VideoCore IV KMS Direct Scanout"
    s_bbox = font_sub.getbbox(sub_text)
    sw = s_bbox[2] - s_bbox[0]
    draw.text(((WIDTH - sw) // 2, 135), sub_text, font=font_sub, fill=(160, 180, 210, 240))

    # 3 Cards Layout
    card_w = 460
    card_h = 580
    gap = 40
    start_x = (WIDTH - (3 * card_w + 2 * gap)) // 2
    card_y = 190

    cards_data = [
        {
            "step": "PASSO 1",
            "title": "Conexão & Rede USB",
            "accent": (0, 229, 255),
            "lines": [
                ("1. Conecte o cabo Micro-USB:", font_bold, (255, 255, 255)),
                ("   Use a porta USB interna de dados", font_body, (180, 200, 220)),
                ("   (não a porta marcada PWR-IN).", font_body, (180, 200, 220)),
                ("", font_body, (0, 0, 0)),
                ("2. No seu computador, configure o IP:", font_bold, (255, 255, 255)),
                ("   IP do seu PC: 192.168.7.1", font_code, (0, 255, 180)),
                ("   Máscara:      255.255.255.0", font_code, (180, 200, 220)),
                ("", font_body, (0, 0, 0)),
                ("3. O Raspberry Pi responderá em:", font_bold, (255, 255, 255)),
                ("   IP do Pi:     192.168.7.2", font_code, (0, 229, 255)),
                ("   Porta UDP:    5000 (Vídeo)", font_code, (180, 200, 220)),
                ("", font_body, (0, 0, 0)),
                ("✓ Suporta também Wi-Fi ou Ethernet!", font_sub, (0, 255, 140)),
            ]
        },
        {
            "step": "PASSO 2",
            "title": "Painel de Controle Web",
            "accent": (0, 255, 120),
            "lines": [
                ("1. Abra no navegador do seu PC:", font_bold, (255, 255, 255)),
                ("   http://192.168.7.2:8080", font_code, (0, 255, 180)),
                ("", font_body, (0, 0, 0)),
                ("2. Recursos disponíveis a quente:", font_bold, (255, 255, 255)),
                ("   • Seletor de FPS (12 / 30 / 60 FPS)", font_body, (180, 200, 220)),
                ("   • Perfil 256 Cores (Coarse QP)", font_body, (180, 200, 220)),
                ("   • Modo Grayscale / Monocromático", font_body, (180, 200, 220)),
                ("   • Auto-scaling de Bitrate por FPS", font_body, (180, 200, 220)),
                ("   • Telemetria ao vivo VPU/CPU/RAM", font_body, (180, 200, 220)),
                ("   • Botão Reativar HUD (60 seg)", font_body, (180, 200, 220)),
                ("", font_body, (0, 0, 0)),
                ("✓ Altere parâmetros sem reiniciar!", font_sub, (0, 255, 140)),
            ]
        },
        {
            "step": "PASSO 3",
            "title": "Conectar sua Sessão",
            "accent": (180, 100, 255),
            "lines": [
                ("1. No PC, execute no terminal (Auto):", font_bold, (255, 255, 255)),
                ("   curl -sSL http://192.168.7.2:8080/connect.sh | bash", font_code_sm, (0, 255, 180)),
                ("", font_body, (0, 0, 0)),
                ("2. Ou baixe o pacote pronto no navegador:", font_bold, (255, 255, 255)),
                ("   Acesse http://192.168.7.2:8080 e baixe", font_body, (180, 200, 220)),
                ("   ext-monitor-client.tar.gz (1.2 MB)", font_code_sm, (0, 229, 255)),
                ("", font_body, (0, 0, 0)),
                ("3. Se já tiver o código clonado no PC:", font_bold, (255, 255, 255)),
                ("   ./scripts/start.sh extend auto 12 hud 256", font_code_sm, (180, 100, 255)),
                ("", font_body, (0, 0, 0)),
                ("4. O monitor assumirá a sessão:", font_bold, (255, 255, 255)),
                ("   Exibe boas-vindas e telemetria,", font_body, (180, 200, 220)),
                ("   e oculta o HUD após 1 minuto.", font_body, (180, 200, 220)),
            ]
        }
    ]

    for i, c in enumerate(cards_data):
        cx = start_x + i * (card_w + gap)
        # Card Background
        card_bg = Image.new("RGBA", (card_w, card_h), (14, 20, 32, 220))
        img.paste(card_bg, (cx, card_y), card_bg)
        
        # Border
        accent = c["accent"]
        draw.rounded_rectangle([cx, card_y, cx + card_w, card_y + card_h], radius=14, outline=(accent[0], accent[1], accent[2], 160), width=2)
        
        # Top Step Pill
        draw.rounded_rectangle([cx + 20, card_y + 18, cx + 110, card_y + 44], radius=6, fill=(accent[0], accent[1], accent[2], 40), outline=(accent[0], accent[1], accent[2], 200), width=1)
        draw.text((cx + 32, card_y + 22), c["step"], font=font_badge, fill=accent)
        
        # Card Title
        draw.text((cx + 20, card_y + 55), c["title"], font=font_card_title, fill=(255, 255, 255))
        draw.line([cx + 20, card_y + 90, cx + card_w - 20, card_y + 90], fill=(60, 80, 110, 120), width=1)
        
        # Content lines
        curr_y = card_y + 105
        for text, font, color in c["lines"]:
            if text == "":
                curr_y += 10
                continue
            draw.text((cx + 24, curr_y), text, font=font, fill=color)
            curr_y += 24

    # Bottom Status Bar
    bar_y = 810
    draw.rounded_rectangle([start_x, bar_y, WIDTH - start_x, bar_y + 55], radius=10, fill=(12, 16, 26, 230), outline=(0, 229, 255, 100), width=1)
    status_text = "PAINEL: 1600x900@59.95Hz (Nativo 1:1)  |  VPU BCM2835: 500 MHz  |  STATUS: Aguardando Stream UDP:5000..."
    draw.text((start_x + 30, bar_y + 18), status_text, font=font_footer, fill=(0, 255, 160))
    
    web_text = "Painel Web: http://192.168.7.2:8080"
    w_bbox = font_footer.getbbox(web_text)
    ww = w_bbox[2] - w_bbox[0]
    draw.text((WIDTH - start_x - ww - 30, bar_y + 18), web_text, font=font_footer, fill=(0, 229, 255))

    output_path = "/home/carlos/ide/ext-monitor/splash.png"
    img.convert("RGB").save(output_path, "PNG", quality=95)
    print(f"[+] Splash screen gerada com sucesso: {output_path} ({WIDTH}x{HEIGHT})")

if __name__ == "__main__":
    create_splash()

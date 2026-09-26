#!/bin/bash
# ==============================================================================
# ext-monitor: Linux Host System Installer (Client & Server/Daemon)
# ==============================================================================
# Installs:
# 1. ext-sender into /usr/local/bin
# 2. udev rule for instant auto-configuration of USB network interface (100 txqueuelen)
# 3. Desktop application entry (Ext-Monitor Second Screen)
# 4. Optional systemd service for zero-interaction plug-and-play extension
#
# License: MIT
# Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>
# ==============================================================================
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

echo -e "\x1b[1;36m========================================================================\x1b[0m"
echo -e "\x1b[1;36m  EXT-MONITOR: Instalador do Driver de Extensão no Host PC (Linux)      \x1b[0m"
echo -e "\x1b[1;36m========================================================================\x1b[0m"

# 1. Verifica dependências essenciais
echo -e "\x1b[1;34m[*] Verificando dependências do sistema...\x1b[0m"
MISSING=""
command -v gst-launch-1.0 >/dev/null 2>&1 || MISSING="$MISSING gstreamer1.0-tools"
command -v pipewire >/dev/null 2>&1 || MISSING="$MISSING pipewire"
command -v pw-link >/dev/null 2>&1 || MISSING="$MISSING pipewire-bin"

if [ -n "$MISSING" ]; then
    echo -e "\x1b[1;33m[!] Pacotes recomendados ausentes:$MISSING\x1b[0m"
    if command -v apt-get >/dev/null 2>&1; then
        echo -e "    Instalando via apt: sudo apt-get update && sudo apt-get install -y $MISSING"
        sudo apt-get update && sudo apt-get install -y $MISSING
    elif command -v dnf >/dev/null 2>&1; then
        sudo dnf install -y $MISSING
    elif command -v pacman >/dev/null 2>&1; then
        sudo pacman -S --noconfirm $MISSING
    fi
fi

# 2. Compila ou localiza binário ext-sender
if [ -f "$REPO_DIR/target/release/ext-sender" ]; then
    SENDER_BIN="$REPO_DIR/target/release/ext-sender"
elif [ -f "./ext-sender" ]; then
    SENDER_BIN="./ext-sender"
elif command -v cargo >/dev/null 2>&1 && [ -d "$REPO_DIR/sender" ]; then
    echo -e "\x1b[1;34m[*] Compilando ext-sender com cargo --release...\x1b[0m"
    (cd "$REPO_DIR/sender" && cargo build --release)
    SENDER_BIN="$REPO_DIR/target/release/ext-sender"
else
    echo -e "\x1b[1;31m[!] Erro: Binário ext-sender não encontrado.\x1b[0m"
    exit 1
fi

echo -e "\x1b[1;34m[*] Instalando ext-sender em /usr/local/bin/ext-sender...\x1b[0m"
sudo cp "$SENDER_BIN" /usr/local/bin/ext-sender
sudo chmod +x /usr/local/bin/ext-sender

# 3. Instala scripts auxiliares
if [ -f "$SCRIPT_DIR/start.sh" ]; then
    sudo cp "$SCRIPT_DIR/start.sh" /usr/local/bin/ext-monitor-start
    sudo chmod +x /usr/local/bin/ext-monitor-start
fi
if [ -f "$SCRIPT_DIR/connect.sh" ]; then
    sudo cp "$SCRIPT_DIR/connect.sh" /usr/local/bin/ext-monitor-connect
    sudo chmod +x /usr/local/bin/ext-monitor-connect
fi

# 4. Instala regras udev
echo -e "\x1b[1;34m[*] Instalando regras udev para plug-and-play imediato...\x1b[0m"
if [ -f "$SCRIPT_DIR/99-ext-monitor.rules" ]; then
    sudo cp "$SCRIPT_DIR/99-ext-monitor.rules" /etc/udev/rules.d/99-ext-monitor.rules
    sudo udevadm control --reload-rules 2>/dev/null || true
    sudo udevadm trigger 2>/dev/null || true
fi

# 5. Cria lançador de desktop (Desktop Entry)
DESKTOP_DIR="/usr/share/applications"
if [ -d "$DESKTOP_DIR" ]; then
    echo -e "\x1b[1;34m[*] Criando lançador no menu de aplicativos (Desktop Entry)...\x1b[0m"
    sudo tee "$DESKTOP_DIR/ext-monitor.desktop" >/dev/null << 'EOF'
[Desktop Entry]
Name=Ext-Monitor (Segunda Tela Pi Zero)
Comment=Estende sua área de trabalho para a TV/monitor via Raspberry Pi Zero
Exec=/usr/local/bin/ext-monitor-start extend auto 30 false economy --bitrate=800
Icon=video-display
Terminal=true
Type=Application
Categories=Utility;Graphics;Settings;
Keywords=display;monitor;screen;hdmi;usb;raspberry;
EOF
fi

echo -e "\n\x1b[1;32m========================================================================\x1b[0m"
echo -e "\x1b[1;32m  INSTALAÇÃO CONCLUÍDA COM SUCESSO!                                     \x1b[0m"
echo -e "\x1b[1;32m========================================================================\x1b[0m"
echo -e "Você já pode iniciar a segunda tela executando:"
echo -e "  \x1b[1;37mext-monitor-start\x1b[0m  (ou pelo menu de aplicativos: 'Ext-Monitor')"
echo -e "\nOu conectar de forma rápida a qualquer momento via:"
echo -e "  \x1b[1;37mcurl -sSL http://192.168.7.2:8080/connect.sh | bash\x1b[0m\n"

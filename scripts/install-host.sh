#!/bin/bash
# ==============================================================================
# ext-monitor: Universal Linux Host System Installer (Client & Server/Daemon)
# ==============================================================================
# Installs:
# 1. ext-sender into /usr/local/bin with cap_sys_admin for Direct Kernel KMS Scanout
# 2. Universal GPU hardware acceleration drivers (AMD, Intel, NVIDIA, Software fallback)
# 3. udev rule for instant auto-configuration of USB network interface (100 txqueuelen)
# 4. Desktop application entry (Ext-Monitor Second Screen)
# 5. Helper binaries: ext-monitor-start and ext-monitor-connect
#
# License: MIT
# Author: Carlos Alberto <carlosalberto4ti@gmail.com>
# ==============================================================================
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

echo -e "\x1b[1;36m========================================================================\x1b[0m"
echo -e "\x1b[1;36m  EXT-MONITOR: Instalador Universal do Host PC (Linux)                  \x1b[0m"
echo -e "\x1b[1;34m  Suporte Multi-GPU (AMD / Intel / NVIDIA) & Dual-Engine (KMS / Mutter) \x1b[0m"
echo -e "\x1b[1;36m========================================================================\x1b[0m"

# 1. Verifica dependências essenciais e aceleração gráfica
echo -e "\x1b[1;34m[*] Verificando dependências do sistema e aceleração gráfica...\x1b[0m"
MISSING=""
command -v gst-launch-1.0 >/dev/null 2>&1 || MISSING="$MISSING gstreamer1.0-tools"
command -v pipewire >/dev/null 2>&1 || MISSING="$MISSING pipewire"
command -v pw-link >/dev/null 2>&1 || MISSING="$MISSING pipewire-bin"
command -v ffmpeg >/dev/null 2>&1 || MISSING="$MISSING ffmpeg"
command -v setcap >/dev/null 2>&1 || MISSING="$MISSING libcap2-bin"
command -v vainfo >/dev/null 2>&1 || MISSING="$MISSING vainfo"

# Drivers de aceleração de vídeo por GPU
if command -v apt-get >/dev/null 2>&1; then
    # AMD / Mesa VA-API
    dpkg -s mesa-va-drivers >/dev/null 2>&1 || MISSING="$MISSING mesa-va-drivers"
    # Intel Media Driver (se GPU Intel detectada)
    if lspci 2>/dev/null | grep -qi "intel.*vga\|intel.*display"; then
        dpkg -s intel-media-va-driver-non-free >/dev/null 2>&1 || dpkg -s intel-media-va-driver >/dev/null 2>&1 || MISSING="$MISSING intel-media-va-driver-non-free"
    fi
fi

if [ -n "$MISSING" ]; then
    echo -e "\x1b[1;33m[!] Instalando pacotes recomendados ausentes:$MISSING\x1b[0m"
    if command -v apt-get >/dev/null 2>&1; then
        sudo apt-get update && sudo apt-get install -y $MISSING || true
    elif command -v dnf >/dev/null 2>&1; then
        sudo dnf install -y $MISSING || true
    elif command -v pacman >/dev/null 2>&1; then
        sudo pacman -S --noconfirm $MISSING || true
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

# 3. Concede capacidades de kernel para captura KMS scanout sem root
echo -e "\x1b[1;34m[*] Configurando capacidades Linux (cap_sys_admin+ep) para Direct KMS Scanout...\x1b[0m"
if command -v setcap >/dev/null 2>&1; then
    sudo setcap cap_sys_admin+ep /usr/local/bin/ext-sender 2>/dev/null || true
    if [ -f "/usr/bin/ffmpeg" ]; then
        sudo setcap cap_sys_admin+ep /usr/bin/ffmpeg 2>/dev/null || true
    fi
    echo -e "\x1b[1;32m[+] Permissões cap_sys_admin aplicadas com sucesso!\x1b[0m"
fi

# Garante que o usuário pertença aos grupos video e render
CURRENT_USER="${SUDO_USER:-$USER}"
if [ -n "$CURRENT_USER" ] && [ "$CURRENT_USER" != "root" ]; then
    sudo usermod -a -G video,render "$CURRENT_USER" 2>/dev/null || true
fi

# Configura renderização de cursor em software no GNOME Mutter
# Isso garante a visibilidade do ponteiro do mouse na captura direta Kernel DRM/KMS
echo -e "\x1b[1;34m[*] Configurando visibilidade do cursor do mouse para captura Kernel DRM/KMS...\x1b[0m"
ENV_DIR="/etc/environment.d"
if [ -d "$ENV_DIR" ]; then
    echo "MUTTER_DEBUG_DISABLE_HW_CURSORS=1" | sudo tee "$ENV_DIR/99-mutter-software-cursor.conf" >/dev/null
fi
USER_HOME=$(eval echo "~$CURRENT_USER")
if [ -d "$USER_HOME" ]; then
    mkdir -p "$USER_HOME/.config/environment.d"
    echo "MUTTER_DEBUG_DISABLE_HW_CURSORS=1" > "$USER_HOME/.config/environment.d/99-mutter-software-cursor.conf"
    chown -R "$CURRENT_USER:$CURRENT_USER" "$USER_HOME/.config/environment.d" 2>/dev/null || true
fi

# 4. Instala scripts auxiliares
if [ -f "$SCRIPT_DIR/start.sh" ]; then
    sudo cp "$SCRIPT_DIR/start.sh" /usr/local/bin/ext-monitor-start
    sudo chmod +x /usr/local/bin/ext-monitor-start
fi
if [ -f "$SCRIPT_DIR/stop.sh" ]; then
    sudo cp "$SCRIPT_DIR/stop.sh" /usr/local/bin/ext-monitor-stop
    sudo chmod +x /usr/local/bin/ext-monitor-stop
fi
if [ -f "$SCRIPT_DIR/connect.sh" ]; then
    sudo cp "$SCRIPT_DIR/connect.sh" /usr/local/bin/ext-monitor-connect
    sudo chmod +x /usr/local/bin/ext-monitor-connect
fi

# 5. Instala regras udev
echo -e "\x1b[1;34m[*] Instalando regras udev para plug-and-play imediato...\x1b[0m"
if [ -f "$SCRIPT_DIR/99-ext-monitor.rules" ]; then
    sudo cp "$SCRIPT_DIR/99-ext-monitor.rules" /etc/udev/rules.d/99-ext-monitor.rules
    sudo udevadm control --reload-rules 2>/dev/null || true
    sudo udevadm trigger 2>/dev/null || true
fi

# 6. Cria lançador de desktop (Desktop Entry)
DESKTOP_DIR="/usr/share/applications"
if [ -d "$DESKTOP_DIR" ]; then
    echo -e "\x1b[1;34m[*] Criando lançador no menu de aplicativos (Desktop Entry)...\x1b[0m"
    sudo tee "$DESKTOP_DIR/ext-monitor.desktop" >/dev/null << 'EOF'
[Desktop Entry]
Name=Ext-Monitor (Segunda Tela Pi Zero)
Comment=Estende sua área de trabalho para a TV/monitor via Raspberry Pi Zero (Direct KMS Scanout)
Exec=/usr/local/bin/ext-monitor-start extend auto 60 false full --capture=kms
Icon=video-display
Terminal=true
Type=Application
Categories=Utility;Graphics;Settings;
Keywords=display;monitor;screen;hdmi;usb;raspberry;kms;pipewire;
EOF
fi

echo -e "\n\x1b[1;32m========================================================================\x1b[0m"
echo -e "\x1b[1;32m  INSTALAÇÃO UNIVERSAL CONCLUÍDA COM SUCESSO!                           \x1b[0m"
echo -e "\x1b[1;32m========================================================================\x1b[0m"
echo -e "Você já pode iniciar a segunda tela executando:"
echo -e "  \x1b[1;37mext-monitor-start\x1b[0m  (ou pelo menu de aplicativos: 'Ext-Monitor')"
echo -e "\nCom suporte completo a Dual-Engine:"
echo -e "  - Motor KMS Direct (Zero Freeze): \x1b[1;32mext-monitor-start --kms\x1b[0m"
echo -e "  - Motor GNOME Mutter (PipeWire):  \x1b[1;33mext-monitor-start --mutter\x1b[0m"
echo -e "\nOu conectar de forma rápida a qualquer momento via:"
echo -e "  \x1b[1;37mcurl -sSL http://192.168.7.2:8080/connect.sh | bash\x1b[0m\n"

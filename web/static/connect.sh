#!/bin/bash
# ==============================================================================
# Pi Zero GPU Display Engine • Cliente de Conexão Rápida para PC (Linux)
# Executado via: curl -sSL http://192.168.7.2:8080/connect.sh | bash
# ==============================================================================
set -e

PI_IP="192.168.7.2"
PI_PORT="5000"
CLIENT_DIR="$HOME/.local/share/ext-monitor"
BIN_PATH="$CLIENT_DIR/ext-sender"
DOWNLOAD_URL="http://${PI_IP}:8080/download/ext-sender"

echo -e "\x1b[1;36m===============================================================\x1b[0m"
echo -e "\x1b[1;36m  Pi Zero GPU Display Engine • Conexão Instantânea de Monitor   \x1b[0m"
echo -e "\x1b[1;36m===============================================================\x1b[0m"

# 1. Verificar dependências de streaming (GStreamer & PipeWire)
MISSING_PKGS=""
if ! command -v gst-launch-1.0 >/dev/null 2>&1; then
    MISSING_PKGS="$MISSING_PKGS gstreamer1.0-tools"
fi
if ! command -v pw-link >/dev/null 2>&1; then
    MISSING_PKGS="$MISSING_PKGS pipewire-bin"
fi

if [ -n "$MISSING_PKGS" ]; then
    echo -e "\x1b[1;33m[*] Instalando dependências necessárias ($MISSING_PKGS)...\x1b[0m"
    if command -v apt-get >/dev/null 2>&1; then
        sudo apt-get update -qq && sudo apt-get install -y -qq $MISSING_PKGS gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-vaapi
    elif command -v dnf >/dev/null 2>&1; then
        sudo dnf install -y gstreamer1-plugins-base gstreamer1-plugins-good gstreamer1-plugins-bad-free pipewire-utils
    elif command -v pacman >/dev/null 2>&1; then
        sudo pacman -S --noconfirm gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad pipewire
    else
        echo -e "\x1b[1;31m[!] Por favor, instale o GStreamer 1.0 e PipeWire no seu sistema.\x1b[0m"
        exit 1
    fi
fi

# 2. Criar diretório do cliente e baixar o binário nativo pré-compilado se necessário
mkdir -p "$CLIENT_DIR"
if [ ! -f "$BIN_PATH" ] || [ "$1" == "--update" ] || [ "$1" == "-u" ]; then
    echo -e "\x1b[1;34m[*] Baixando cliente nativo ext-sender de $DOWNLOAD_URL...\x1b[0m"
    curl -sSL "$DOWNLOAD_URL" -o "$BIN_PATH.tmp"
    chmod +x "$BIN_PATH.tmp"
    mv "$BIN_PATH.tmp" "$BIN_PATH"
    echo -e "\x1b[1;32m[+] Cliente ext-sender pronto em $BIN_PATH\x1b[0m"
fi

# 3. Otimizar tamanho da fila USB (Buffer Bloat) se interface for detectada
for iface in $(ip -o link show | awk -F': ' '{print $2}' | grep -E '^enx|^usb'); do
    sudo ip link set "$iface" txqueuelen 100 2>/dev/null || true
done

# 4. Parâmetros dinâmicos
MODE="${1:-extend}"
FPS="${2:-12}"
COLOR="${3:-256}"
HUD="${4:-hud}"

echo -e "\x1b[1;32m[*] Conectando monitor secundário ao Raspberry Pi ($PI_IP:$PI_PORT)...\x1b[0m"
echo -e "\x1b[1;33m[*] Parâmetros: Modo=$MODE, FPS=$FPS, Cores=$COLOR, HUD=$HUD\x1b[0m"
echo -e "\x1b[1;35m[*] Dica: Altere configurações a quente em http://$PI_IP:8080\x1b[0m"

# 5. Executar o cliente de streaming
exec "$BIN_PATH" "$PI_IP" "$PI_PORT" 0 "$MODE" auto "$FPS" "$HUD" "$COLOR"

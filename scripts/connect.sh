#!/bin/bash
# ==============================================================================
# ext-monitor: Universal One-Line Host Connect & Launcher
# ==============================================================================
# Usage:
#   curl -sSL http://192.168.7.2:8080/connect.sh | bash
#   ou: ./scripts/connect.sh [extend|clone] [fps] [bitrate]
# ==============================================================================
set -e

PI_IP="${TARGET_IP:-192.168.7.2}"
MODE="${1:-extend}"
FPS="${2:-30}"
BITRATE="${3:-800}"

echo -e "\x1b[1;36m========================================================================\x1b[0m"
echo -e "\x1b[1;36m  EXT-MONITOR: Conector Automático de Segunda Tela (Pi Zero W / 2 W)    \x1b[0m"
echo -e "\x1b[1;36m========================================================================\x1b[0m"

# 1. Verifica conectividade com o Pi Zero
if ! ping -c 1 -W 1 "$PI_IP" >/dev/null 2>&1; then
    echo -e "\x1b[1;33m[*] Aguardando interface de rede USB com o Pi Zero ($PI_IP)...\x1b[0m"
    for i in $(seq 1 10); do
        if ping -c 1 -W 1 "$PI_IP" >/dev/null 2>&1; then
            break
        fi
        sleep 1
    done
fi

if ! ping -c 1 -W 1 "$PI_IP" >/dev/null 2>&1; then
    echo -e "\x1b[1;31m[!] Erro: Não foi possível conectar ao Pi Zero em $PI_IP.\x1b[0m"
    echo -e "    Verifique se o cabo USB está conectado na porta de dados (USB central do Pi Zero)."
    exit 1
fi

echo -e "\x1b[1;32m[+] Pi Zero detectado e respondendo em $PI_IP!\x1b[0m"

# 2. Se já estiver no diretório do repositório, usa o executável local
if [ -f "./scripts/start.sh" ] && [ -f "./target/release/ext-sender" ]; then
    exec ./scripts/start.sh "$MODE" auto "$FPS" false economy --bitrate="$BITRATE"
fi

# 3. Caso contrário, baixa as ferramentas portáteis do próprio Pi Zero
WORK_DIR="$HOME/.local/share/ext-monitor"
mkdir -p "$WORK_DIR"
cd "$WORK_DIR"

if [ ! -f "$WORK_DIR/ext-sender" ] || [ "$1" = "--update" ]; then
    echo -e "\x1b[1;34m[*] Baixando ferramentas do cliente diretamente do Pi Zero...\x1b[0m"
    curl -sSL "http://$PI_IP:8080/download/client.tar.gz" -o client.tar.gz 2>/dev/null || \
    curl -sSL "http://$PI_IP:8080/download/ext-sender" -o ext-sender 2>/dev/null
    
    if [ -f client.tar.gz ]; then
        tar -xzf client.tar.gz
        rm -f client.tar.gz
    fi
    chmod +x ext-sender start.sh 2>/dev/null || true
fi

if [ -f "$WORK_DIR/start.sh" ]; then
    exec "$WORK_DIR/start.sh" "$MODE" auto "$FPS" false economy --bitrate="$BITRATE"
elif [ -f "$WORK_DIR/ext-sender" ]; then
    exec "$WORK_DIR/ext-sender" "$PI_IP" 5000 "$BITRATE" "$MODE" auto "$FPS" false 256
else
    echo -e "\x1b[1;31m[!] Falha ao obter o executável ext-sender do Pi Zero.\x1b[0m"
    exit 1
fi

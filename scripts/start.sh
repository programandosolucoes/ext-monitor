#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

# Auto-inclusão da automação udev / watcher na primeira execução (com ou sem sudo)
"$SCRIPT_DIR/setup-autoconnect.sh" 2>/dev/null || true

MODE="${1:-extend}"
ENCODER="${2:-auto}"
FPS="${3:-30}"
HUD="${4:-false}"
TARGET_IP="${TARGET_IP:-192.168.7.2}"
TARGET_PORT="${TARGET_PORT:-5000}"

BITRATE="${BITRATE:-0}"

# Check for flags, custom IP arguments, and color profiles
COLOR_FLAG=""
for arg in "$@"; do
    if [ "$arg" = "hud" ] || [ "$arg" = "--hud" ]; then
        HUD="hud"
    elif [ "$arg" = "256" ] || [ "$arg" = "--256" ] || [ "$arg" = "--colors=256" ] || [ "$arg" = "economy" ] || [ "$arg" = "--economy" ]; then
        COLOR_FLAG="256"
        if [ "$BITRATE" = "0" ]; then
            BITRATE="800"
        fi
    elif [ "$arg" = "gray" ] || [ "$arg" = "--gray" ] || [ "$arg" = "bw" ] || [ "$arg" = "--bw" ]; then
        COLOR_FLAG="gray"
    elif [[ "$arg" =~ ^--bitrate=([0-9]+)$ ]]; then
        BITRATE="${BASH_REMATCH[1]}"
    elif [[ "$arg" =~ ^-b=([0-9]+)$ ]]; then
        BITRATE="${BASH_REMATCH[1]}"
    elif [[ "$arg" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        TARGET_IP="$arg"
    elif [[ "$arg" =~ ^--ip=([0-9]+\.[0-9]+\.[0-9]+\.[0-9]+)$ ]]; then
        TARGET_IP="${BASH_REMATCH[1]}"
    elif [[ "$arg" =~ ^--port=([0-9]+)$ ]]; then
        TARGET_PORT="${BASH_REMATCH[1]}"
    fi
done

# Otimização do barramento USB e filas no Host (Elimina Buffer Bloat)
if ip link show enx122233445566 >/dev/null 2>&1; then
    sudo ip link set enx122233445566 txqueuelen 100 2>/dev/null || true
fi

echo -e "\x1b[1;32m[*] Iniciando ext-sender (Modo: $MODE, Encoder: $ENCODER, FPS: $FPS, Bitrate: ${BITRATE:-auto} kbps, HUD: $HUD, Cor: ${COLOR_FLAG:-padrão}, Destino: $TARGET_IP:$TARGET_PORT)...\x1b[0m"

# Mata instâncias anteriores se existirem (inclusive pipelines gst-launch órfãos)
pkill -f "ext-sender" 2>/dev/null || true
killall -9 gst-launch-1.0 2>/dev/null || true
sleep 0.5

# Executa o binário do sender com bitrate
"$REPO_DIR/target/release/ext-sender" "$TARGET_IP" "$TARGET_PORT" "$BITRATE" "$MODE" "$ENCODER" "$FPS" "$HUD" $COLOR_FLAG

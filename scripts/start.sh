#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

MODE="${1:-extend}"
ENCODER="${2:-auto}"
FPS="${3:-30}"
HUD="${4:-false}"
TARGET_IP="${TARGET_IP:-192.168.7.2}"
TARGET_PORT="${TARGET_PORT:-5000}"

# Check for flags and custom IP arguments
for arg in "$@"; do
    if [ "$arg" = "hud" ] || [ "$arg" = "--hud" ]; then
        HUD="hud"
    elif [[ "$arg" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        TARGET_IP="$arg"
    elif [[ "$arg" =~ ^--ip=([0-9]+\.[0-9]+\.[0-9]+\.[0-9]+)$ ]]; then
        TARGET_IP="${BASH_REMATCH[1]}"
    elif [[ "$arg" =~ ^--port=([0-9]+)$ ]]; then
        TARGET_PORT="${BASH_REMATCH[1]}"
    fi
done

echo -e "\x1b[1;32m[*] Iniciando ext-sender (Modo: $MODE, Encoder: $ENCODER, FPS: $FPS, HUD: $HUD, Destino: $TARGET_IP:$TARGET_PORT)...\x1b[0m"

# Mata instâncias anteriores se existirem
pkill -f "ext-sender" 2>/dev/null || true
sleep 0.5

# Executa o binário do sender com o IP configurado (USB Gadget, Placa Ethernet ou Wi-Fi)
"$REPO_DIR/target/release/ext-sender" "$TARGET_IP" "$TARGET_PORT" 8000 "$MODE" "$ENCODER" "$FPS" "$HUD"

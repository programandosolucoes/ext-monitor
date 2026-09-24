#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

MODE="${1:-extend}"
ENCODER="${2:-auto}"
FPS="${3:-27}"
HUD="${4:-false}"

# Check if any argument is 'hud' or '--hud'
for arg in "$@"; do
    if [ "$arg" = "hud" ] || [ "$arg" = "--hud" ]; then
        HUD="hud"
    fi
done

echo -e "\x1b[1;32m[*] Iniciando ext-sender (Modo: $MODE, Encoder: $ENCODER, FPS: $FPS, HUD: $HUD)...\x1b[0m"

# Mata instâncias anteriores se existirem
pkill -f "ext-sender" 2>/dev/null || true
sleep 0.5

# Executa o binário do sender
"$REPO_DIR/target/release/ext-sender" 192.168.7.2 5000 8000 "$MODE" "$ENCODER" "$FPS" "$HUD"

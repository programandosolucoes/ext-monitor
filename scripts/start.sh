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
COLOR_FLAG="full"
EXTRA_FLAGS=()
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
    elif [ "$arg" = "full" ] || [ "$arg" = "--full" ] || [ "$arg" = "24bit" ] || [ "$arg" = "--24bit" ] || [ "$arg" = "truecolor" ] || [ "$arg" = "--truecolor" ]; then
        COLOR_FLAG="full"
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
    elif [ "$arg" = "--drop-only" ] || [ "$arg" = "--no-drop-only" ] || [[ "$arg" =~ ^--drop-only= ]]; then
        EXTRA_FLAGS+=("$arg")
    elif [ "$arg" = "--economy" ] || [ "$arg" = "economy" ]; then
        EXTRA_FLAGS+=("--drop-only")
    elif [ "$arg" = "--continuous" ] || [ "$arg" = "--cfr" ] || [ "$arg" = "continuous" ]; then
        EXTRA_FLAGS+=("--no-drop-only")
    elif [ "$arg" = "--skip-to-first" ] || [ "$arg" = "--no-skip-to-first" ] || [[ "$arg" =~ ^--skip-to-first= ]]; then
        EXTRA_FLAGS+=("$arg")
    elif [[ "$arg" =~ ^--key-int-max=([0-9]+)$ ]] || [[ "$arg" =~ ^--idr=([0-9]+)$ ]] || [[ "$arg" =~ ^--key-int=([0-9]+)$ ]]; then
        EXTRA_FLAGS+=("$arg")
    elif [ "$arg" = "--transport=usb" ] || [ "$arg" = "--usb" ] || [ "$arg" = "--usb-bulk" ] || [[ "$arg" =~ ^--transport= ]]; then
        EXTRA_FLAGS+=("$arg")
    elif [ "$arg" = "--kms" ] || [ "$arg" = "kms" ] || [ "$arg" = "--drm" ]; then
        EXTRA_FLAGS+=("--capture=kms")
    elif [ "$arg" = "--mutter" ] || [ "$arg" = "mutter" ] || [ "$arg" = "--gnome" ]; then
        EXTRA_FLAGS+=("--capture=mutter")
    elif [[ "$arg" =~ ^--capture= ]]; then
        EXTRA_FLAGS+=("$arg")
    elif [[ "$arg" =~ ^--monitor= ]] || [[ "$arg" =~ ^--connector= ]]; then
        EXTRA_FLAGS+=("$arg")
    elif [ "$arg" = "--no-audio" ] || [ "$arg" = "--audio=off" ] || [ "$arg" = "--audio=false" ]; then
        EXTRA_FLAGS+=("--no-audio")
    elif [ "$arg" = "--audio" ] || [ "$arg" = "--audio=on" ] || [ "$arg" = "--audio=true" ]; then
        EXTRA_FLAGS+=("--audio")
    elif [[ "$arg" =~ ^--audio-port= ]]; then
        EXTRA_FLAGS+=("$arg")
    fi
done

# Otimização do barramento USB e filas no Host (Elimina Buffer Bloat)
if ip link show enx122233445566 >/dev/null 2>&1; then
    sudo ip link set enx122233445566 txqueuelen 100 2>/dev/null || true
fi

echo -e "\x1b[1;32m[*] Iniciando ext-sender (Modo: $MODE, Encoder: $ENCODER, FPS: $FPS, Bitrate: ${BITRATE:-auto} kbps, HUD: $HUD, Cor: ${COLOR_FLAG:-padrão}, Destino: $TARGET_IP:$TARGET_PORT, Extras: ${EXTRA_FLAGS[*]})...\x1b[0m"

# Mata instâncias anteriores se existirem (inclusive pipelines gst-launch órfãos)
pkill -f "ext-sender" 2>/dev/null || true
killall -9 gst-launch-1.0 2>/dev/null || true
sleep 0.5

# Localização inteligente do script de Damage Pacer
PACER_SCRIPT=""
if [ -f "$SCRIPT_DIR/wayland-damage-pacer.py" ]; then
    PACER_SCRIPT="$SCRIPT_DIR/wayland-damage-pacer.py"
elif [ -f "/usr/local/bin/wayland-damage-pacer.py" ]; then
    PACER_SCRIPT="/usr/local/bin/wayland-damage-pacer.py"
fi

PACER_PID=""
if [ -n "$WAYLAND_DISPLAY" ] && [ -n "$PACER_SCRIPT" ]; then
    if ! pgrep -f "wayland-damage-pacer.py" >/dev/null 2>&1; then
        echo -e "\x1b[1;34m[*] Ativando Wayland Damage Pacer (elimina quiescência do Mutter a 60 FPS)...\x1b[0m"
        python3 "$PACER_SCRIPT" >/dev/null 2>&1 &
        PACER_PID=$!
    fi
fi

cleanup() {
    if [ -n "$PACER_PID" ]; then
        kill "$PACER_PID" 2>/dev/null || true
    fi
    pkill -f "wayland-damage-pacer.py" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# Localização inteligente do binário do ext-sender
SENDER_BIN=""
if [ -f "$REPO_DIR/target/release/ext-sender" ]; then
    SENDER_BIN="$REPO_DIR/target/release/ext-sender"
elif [ -f "/usr/local/bin/ext-sender" ]; then
    SENDER_BIN="/usr/local/bin/ext-sender"
elif command -v ext-sender >/dev/null 2>&1; then
    SENDER_BIN="$(command -v ext-sender)"
else
    echo -e "\x1b[1;31m[!] Erro: Binário ext-sender não encontrado.\x1b[0m"
    exit 1
fi

# Executa o binário do sender com bitrate e flags extras
"$SENDER_BIN" "$TARGET_IP" "$TARGET_PORT" "$BITRATE" "$MODE" "$ENCODER" "$FPS" "$HUD" $COLOR_FLAG "${EXTRA_FLAGS[@]}"


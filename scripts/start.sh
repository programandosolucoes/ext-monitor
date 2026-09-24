#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

MODE="${1:-extend}"
ENCODER="${2:-auto}"

echo -e "\x1b[1;32m[*] Iniciando ext-sender (Modo: $MODE, Encoder: $ENCODER)...\x1b[0m"

# Mata instâncias anteriores se existirem
pkill -f "ext-sender" 2>/dev/null || true
sleep 0.5

# Executa o binário do sender
"$REPO_DIR/target/release/ext-sender" 192.168.7.2 5000 8000 "$MODE" "$ENCODER"

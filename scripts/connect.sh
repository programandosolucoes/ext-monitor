#!/bin/bash
# ==============================================================================
# ext-monitor: Universal One-Line Host Connect & Launcher
# Usage:
#   curl -sSL http://192.168.7.2:8080/connect.sh | bash
# ==============================================================================
set -e

PI_IP="${TARGET_IP:-192.168.7.2}"

echo -e "\x1b[1;36m========================================================================\x1b[0m"
echo -e "\x1b[1;36m  EXT-MONITOR: Conector Automático de Segunda Tela (Pi Zero)            \x1b[0m"
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
    echo -e "    Certifique-se de que o cabo USB está conectado na porta de dados (USB central)."
    exit 1
fi

echo -e "\x1b[1;32m[+] Pi Zero detectado em $PI_IP!\x1b[0m"

# 2. Se ext-sender já existir no PATH do sistema, executa direto
if command -v ext-sender >/dev/null 2>&1; then
    exec ext-sender "$@"
fi

if [ -f "./target/release/ext-sender" ]; then
    exec ./target/release/ext-sender "$@"
fi

# 3. Baixa ext-sender direto do Pi Zero para ~/.local/bin
WORK_DIR="$HOME/.local/bin"
mkdir -p "$WORK_DIR"
echo -e "\x1b[1;34m[*] Baixando binário nativo ext-sender diretamente do Pi Zero...\x1b[0m"
curl -sSL "http://$PI_IP:8080/download/ext-sender" -o "$WORK_DIR/ext-sender"
chmod +x "$WORK_DIR/ext-sender"

exec "$WORK_DIR/ext-sender" "$@"

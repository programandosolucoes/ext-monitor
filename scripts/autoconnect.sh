#!/bin/bash
# ==============================================================================
# ext-monitor-autoconnect.sh
# Script disparado automaticamente via udev / systemd quando o Pi Zero é conectado
# Fica tentando adicionar e ativar a segunda tela assim que o Pi responder.
# ==============================================================================

REPO_DIR="/home/carlos/ide/ext-monitor"
TARGET_IP="192.168.7.2"
MAX_ATTEMPTS=45 # Tenta por até ~90 segundos (cobre todo o ciclo de boot do Pi)

echo "[*] [UDEV Autoconnect] Dispositivo USB Pi Zero detectado!"
echo "[*] [UDEV Autoconnect] Aguardando Raspberry Pi Zero inicializar e responder na rede..."

# Se o ext-sender já estiver rodando e respondendo, verifica se o link está vivo
if pgrep -f "ext-sender" >/dev/null; then
    if ping -c 1 -W 1 "$TARGET_IP" >/dev/null 2>&1; then
        echo "[+] [UDEV Autoconnect] ext-sender já está ativo e Pi Zero online. Nada a fazer."
        exit 0
    else
        echo "[!] [UDEV Autoconnect] ext-sender órfão detectado sem resposta de rede. Reiniciando..."
        pkill -f "ext-sender" 2>/dev/null
        sleep 1
    fi
fi

# Loop de tentativas de conexão (ficar tentando adicionar a segunda tela)
ONLINE=0
for i in $(seq 1 $MAX_ATTEMPTS); do
    if ping -c 1 -W 1 "$TARGET_IP" >/dev/null 2>&1; then
        echo "[+] [UDEV Autoconnect] Pi Zero respondeu ao ping na tentativa $i!"
        ONLINE=1
        break
    fi
    sleep 2
done

if [ $ONLINE -ne 1 ]; then
    echo "[-] [UDEV Autoconnect] Timeout aguardando o Pi Zero responder em $TARGET_IP após $((MAX_ATTEMPTS * 2))s."
    notify-send -u low -i video-display "Pi Zero Monitor" "Pi Zero conectado via USB, mas não respondeu na rede após 90s." 2>/dev/null || true
    exit 1
fi

# Aguarda 2 segundos adicionais para estabilização de serviços (ext-receiver e systemd)
sleep 2

# Notificação visual na área de trabalho do GNOME
notify-send -u normal -i video-display "Pi Zero Monitor" "Raspberry Pi Zero online! Ativando segunda tela..." 2>/dev/null || true

# Executa o inicializador com as configurações recomendadas
echo "[*] [UDEV Autoconnect] Disparando ext-sender na sessão do usuário..."
cd "$REPO_DIR" || exit 1
exec ./scripts/start.sh extend auto 12 hud 256

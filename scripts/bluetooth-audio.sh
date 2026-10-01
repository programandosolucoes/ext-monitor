#!/usr/bin/env bash
# ==============================================================================
# Gerenciador de Áudio Bluetooth A2DP Sink - Ext-Monitor
# Transforma o Raspberry Pi Zero W em receptor de áudio Bluetooth para TV HDMI
# Autor: Carlos Alberto <carlosalberto4ti@gmail.com>
# ==============================================================================

set -euo pipefail

PI_IP="192.168.7.2"
ALSA_DEVICE="hw:0,0"

show_help() {
    echo -e "\x1b[1;36m===============================================================\x1b[0m"
    echo -e "\x1b[1;36m  EXT-MONITOR - BLUETOOTH A2DP SINK (ÁUDIO SEM FIO PARA TV)    \x1b[0m"
    echo -e "\x1b[1;36m===============================================================\x1b[0m"
    echo -e "Permite que smartphones, tablets e notebooks transmitam áudio"
    echo -e "direto para os alto-falantes da TV HDMI via Bluetooth 4.1 (A2DP)."
    echo ""
    echo -e "\x1b[1;33mComandos:\x1b[0m"
    echo -e "  $0 pair       -> Torna o Pi Zero visível e pareável por 60 segundos"
    echo -e "  $0 status     -> Exibe estado do controlador Bluetooth e conexões"
    echo -e "  $0 test-tone  -> Toca um tom de teste estéreo na saída HDMI da TV"
    echo -e "  $0 route-alsa -> Conecta bluez-alsa diretamente ao hardware HDMI ($ALSA_DEVICE)"
    echo -e "\x1b[1;36m===============================================================\x1b[0m"
}

pair_mode() {
    echo -e "\x1b[1;34m[*] Ativando modo de pareamento Bluetooth A2DP no Raspberry Pi...\x1b[0m"
    if curl -s -X POST "http://${PI_IP}:8080/api/bluetooth/discoverable" >/dev/null 2>&1; then
        echo -e "\x1b[1;32m[✓] Sinal enviado com sucesso via Painel Web (192.168.7.2:8080)!\x1b[0m"
    else
        echo -e "\x1b[1;33m[*] Executando comando localmente via bluetoothctl...\x1b[0m"
        bluetoothctl power on >/dev/null 2>&1 || true
        bluetoothctl discoverable on >/dev/null 2>&1 || true
        bluetoothctl pairable on >/dev/null 2>&1 || true
    fi
    echo -e "\x1b[1;32m[✓] O dispositivo 'ext-monitor' agora está visível no Bluetooth!\x1b[0m"
    echo -e "    Abra as configurações de Bluetooth do seu smartphone/PC e selecione 'ext-monitor'."
}

status_mode() {
    echo -e "\x1b[1;36m[*] Status do Bluetooth:\x1b[0m"
    if command -v bluetoothctl >/dev/null 2>&1; then
        bluetoothctl show 2>/dev/null | grep -E "Name|Powered|Discoverable|Pairable" || echo "Controlador Bluetooth não detectado localmente."
    else
        echo "bluetoothctl não instalado neste sistema host."
    fi
}

test_tone() {
    echo -e "\x1b[1;34m[*] Testando saída HDMI ALSA com som senoidal (440Hz / 48kHz)...\x1b[0m"
    if command -v speaker-test >/dev/null 2>&1; then
        speaker-test -D default -c 2 -t sine -f 440 -l 1 || true
    else
        echo "speaker-test não disponível."
    fi
}

route_alsa() {
    echo -e "\x1b[1;34m[*] Roteando bluez-alsa para HDMI ($ALSA_DEVICE)...\x1b[0m"
    if command -v bluealsa-aplay >/dev/null 2>&1; then
        bluealsa-aplay -D "$ALSA_DEVICE" 00:00:00:00:00:00 &
        echo -e "\x1b[1;32m[✓] bluealsa-aplay ativo em background!\x1b[0m"
    else
        echo "bluealsa-aplay não presente no host local (este serviço opera nativo no Pi Zero)."
    fi
}

CMD="${1:-pair}"

case "$CMD" in
    pair|on|discoverable)
        pair_mode
        ;;
    status|info)
        status_mode
        ;;
    test-tone|test)
        test_tone
        ;;
    route-alsa|alsa)
        route_alsa
        ;;
    *)
        show_help
        exit 1
        ;;
esac

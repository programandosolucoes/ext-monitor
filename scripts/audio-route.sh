#!/usr/bin/env bash
# ==============================================================================
# Gerenciador de Roteamento de Áudio Híbrido - Ext-Monitor
# Permite alternar saída de som entre Headset Local e HDMI da TV (Raspberry Pi)
# Autor: Carlos Alberto <carlosalberto4ti@gmail.com>
# ==============================================================================

set -euo pipefail

VIRTUAL_SINK_NAME="Raspberry_Pi_HDMI_Audio"

ensure_virtual_sink() {
    if ! pactl list sinks short 2>/dev/null | grep -q "$VIRTUAL_SINK_NAME"; then
        echo -e "\x1b[1;34m[*] Criando dispositivo de áudio virtual '$VIRTUAL_SINK_NAME'...\x1b[0m"
        pactl load-module module-null-sink \
            sink_name="$VIRTUAL_SINK_NAME" \
            sink_properties=device.description=Raspberry_Pi_HDMI_Audio \
            >/dev/null 2>&1 || true
    fi
}

get_default_sink() {
    pactl get-default-sink 2>/dev/null || wpctl status 2>/dev/null | grep -A 5 "Sinks:" | grep '\*' | awk '{print $3}' || echo "desconhecido"
}

get_local_sink() {
    # Procura headset USB (Yealink UH34 ou similar), depois caixas internas
    local usb_sink
    usb_sink=$(pactl list sinks short 2>/dev/null | grep -i "usb" | grep -v "$VIRTUAL_SINK_NAME" | head -n 1 | awk '{print $2}' || true)
    if [ -n "$usb_sink" ]; then
        echo "$usb_sink"
        return
    fi
    local speaker_sink
    speaker_sink=$(pactl list sinks short 2>/dev/null | grep -i "speaker" | head -n 1 | awk '{print $2}' || true)
    if [ -n "$speaker_sink" ]; then
        echo "$speaker_sink"
        return
    fi
    pactl list sinks short 2>/dev/null | grep -v "$VIRTUAL_SINK_NAME" | head -n 1 | awk '{print $2}' || true
}

show_status() {
    ensure_virtual_sink
    local cur_sink
    cur_sink=$(get_default_sink)
    echo -e "\x1b[1;36m====================================================\x1b[0m"
    echo -e "\x1b[1;36m  STATUS DO ÁUDIO - EXT-MONITOR (MODO HÍBRIDO)      \x1b[0m"
    echo -e "\x1b[1;36m====================================================\x1b[0m"
    echo -e "  Saída Ativa Padrão: \x1b[1;32m$cur_sink\x1b[0m"
    if [[ "$cur_sink" == *"$VIRTUAL_SINK_NAME"* ]]; then
        echo -e "  Modo Atual:         \x1b[1;33m[ TV HDMI ] Todo o áudio do PC vai para a TV\x1b[0m"
    else
        echo -e "  Modo Atual:         \x1b[1;32m[ HÍBRIDO ] Vídeo na TV e Áudio no Fone/PC\x1b[0m"
    fi
    echo ""
    echo -e "\x1b[1;34mDispositivos de Saída Disponíveis:\x1b[0m"
    pactl list sinks short 2>/dev/null | while read -r idx name driver fmt state; do
        if [[ "$name" == "$cur_sink" ]]; then
            echo -e "  -> [\x1b[1;32mATIVO\x1b[0m] #$idx: $name ($driver)"
        else
            echo -e "     [     ] #$idx: $name ($driver)"
        fi
    done
    echo ""
    echo -e "\x1b[1;33mComandos de Troca Rápida:\x1b[0m"
    echo -e "  ./scripts/audio-route.sh local   -> Volta áudio para o fone/caixas locais"
    echo -e "  ./scripts/audio-route.sh pi      -> Envia todo o som do PC para a TV HDMI"
    echo -e "  ./scripts/audio-route.sh status  -> Mostra este painel"
    echo -e "\x1b[1;36m====================================================\x1b[0m"
}

set_sink_pi() {
    ensure_virtual_sink
    pactl set-default-sink "$VIRTUAL_SINK_NAME"
    echo -e "\x1b[1;32m[✓] Saída padrão de áudio redirecionada para a TV HDMI ($VIRTUAL_SINK_NAME)!\x1b[0m"
    echo -e "    O áudio dos navegadores e players será transmitido via Opus/UDP ao Raspberry Pi."
}

set_sink_local() {
    local target
    target=$(get_local_sink)
    if [ -z "$target" ]; then
        echo -e "\x1b[1;31m[!] Erro: Nenhum dispositivo local encontrado.\x1b[0m"
        exit 1
    fi
    pactl set-default-sink "$target"
    echo -e "\x1b[1;32m[✓] Saída padrão de áudio restaurada para o dispositivo local ($target)!\x1b[0m"
    echo -e "    Modo híbrido ativo: você pode trabalhar na TV enquanto escuta YouTube no fone."
}

ACTION="${1:-status}"

case "$ACTION" in
    setup)
        ensure_virtual_sink
        echo -e "\x1b[1;32m[✓] Dispositivo virtual de áudio configurado.\x1b[0m"
        ;;
    pi|tv|hdmi)
        set_sink_pi
        ;;
    local|headset|pc|fone)
        set_sink_local
        ;;
    status|info)
        show_status
        ;;
    *)
        echo "Uso: $0 [status|local|pi|setup]"
        exit 1
        ;;
esac

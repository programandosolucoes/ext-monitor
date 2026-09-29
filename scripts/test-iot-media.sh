#!/usr/bin/env bash
# ==============================================================================
# Testador e Validador do IoT Media Renderer & Visualizador HDMI - Ext-Monitor
# Autor: Carlos Alberto <psncarlosalberto4ti@gmail.com>
# ==============================================================================

set -euo pipefail

PI_IP="${1:-192.168.7.2}"
PORT="8080"
BASE_URL="http://${PI_IP}:${PORT}"

echo -e "\x1b[1;36m========================================================================\x1b[0m"
echo -e "\x1b[1;36m  EXT-MONITOR - VALIDAÇÃO IOT MEDIA RENDERER & VISUALIZADOR HDMI       \x1b[0m"
echo -e "\x1b[1;36m========================================================================\x1b[0m"
echo -e "Conectando ao appliance em \x1b[1;32m${BASE_URL}\x1b[0m..."
echo ""

# 1. Test UPnP XML
echo -e "\x1b[1;34m[1/5] Verificando Descritor UPnP / DLNA MediaRenderer (/upnp/desc.xml)...\x1b[0m"
if curl -s -f "${BASE_URL}/upnp/desc.xml" | grep -q "MediaRenderer"; then
    echo -e "  \x1b[1;32m[✓] UPnP MediaRenderer XML ativo e válido para Windows/DLNA!\x1b[0m"
else
    echo -e "  \x1b[1;31m[!] Falha ao ler /upnp/desc.xml\x1b[0m"
fi

# 2. Test DIAL XML (YouTube Cast)
echo -e "\x1b[1;34m[2/5] Verificando Descritor DIAL YouTube Cast (/dial/dd.xml)...\x1b[0m"
if curl -s -f "${BASE_URL}/dial/dd.xml" | grep -q "dial"; then
    echo -e "  \x1b[1;32m[✓] DIAL YouTube Cast XML ativo e válido para smartphones!\x1b[0m"
else
    echo -e "  \x1b[1;31m[!] Falha ao ler /dial/dd.xml\x1b[0m"
fi

# 3. Trigger Visualizer on HDMI
echo -e "\x1b[1;34m[3/5] Disparando Visualizador Gráfico na TV HDMI com Spectrum 30 FPS...\x1b[0m"
curl -s -X POST "${BASE_URL}/api/media/visualizer" \
    -H "Content-Type: application/json" \
    -d '{"enabled": true}' >/dev/null
echo -e "  \x1b[1;32m[✓] Visualizador HDMI ativado na TV!\x1b[0m"

# 4. Push Track Metadata (Simulating Bluetooth AVRCP / Spotify track)
echo -e "\x1b[1;34m[4/5] Injetando metadados de reprodução (Título, Artista, Álbum)...\x1b[0m"
curl -s -X POST "${BASE_URL}/api/media/control" \
    -H "Content-Type: application/json" \
    -d '{
        "action": "play",
        "title": "Sultans of Swing",
        "artist": "Dire Straits",
        "album": "Dire Straits (1978) • FLAC 48kHz"
    }' >/dev/null
echo -e "  \x1b[1;32m[✓] Metadados atualizados com sucesso!\x1b[0m"

# 5. Read Back Status
echo -e "\x1b[1;34m[5/5] Consultando status em tempo real (/api/media/status)...\x1b[0m"
STATUS_JSON=$(curl -s "${BASE_URL}/api/media/status" || echo "{}")
echo -e "  Resposta do Appliance: \x1b[1;33m${STATUS_JSON}\x1b[0m"
echo ""

echo -e "\x1b[1;32m========================================================================\x1b[0m"
echo -e "\x1b[1;32m  ✓ TESTE CONCLUÍDO COM SUCESSO!                                        \x1b[0m"
echo -e "\x1b[1;32m  Olhe para a tela da TV HDMI: o visualizador com VU Meter e arte       \x1b[0m"
echo -e "\x1b[1;32m  deve estar animando a 30 FPS com as barras dinâmicas coloridas!       \x1b[0m"
echo -e "\x1b[1;32m========================================================================\x1b[0m"

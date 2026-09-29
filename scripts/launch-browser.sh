#!/bin/bash
# launch-browser.sh — Abre o navegador no monitor secundário sem suspensão de vídeo
#
# Problema resolvido:
#   Chrome/Chromium e Firefox suspendem a decodificação de vídeo (YouTube, etc.)
#   quando o cursor do mouse sai da janela ("Window Occlusion Tracking").
#   Isso faz o stream congelar no monitor Pi Zero mesmo com o pipeline ativo.
#
# Solução:
#   - Chrome: flag --disable-backgrounding-occluded-windows desativa a detecção
#     de oclusão, mantendo a renderização do <video> ativa a 60 FPS contínuos.
#   - Firefox: prefer media.suspend-bkgnd-video.enabled=false via perfil temporário.
#
# Uso:
#   ./launch-browser.sh [chrome|firefox|brave|edge] [url]
#   ./launch-browser.sh chrome https://youtube.com
#   ./launch-browser.sh firefox https://youtube.com
#   ./launch-browser.sh              # detecta automaticamente o navegador disponível

set -e

BROWSER="${1:-auto}"
URL="${2:-https://www.youtube.com}"

# Offset X do monitor secundário (ajuste se necessário)
# Para 1920px de largura do monitor principal:
SECONDARY_X="${EXT_MONITOR_X:-1920}"
SECONDARY_Y="${EXT_MONITOR_Y:-0}"

# Flags comuns do Chrome que desativam a suspensão de vídeo por oclusão
CHROME_FLAGS=(
    "--disable-backgrounding-occluded-windows"   # Desativa occlusion tracking
    "--disable-renderer-backgrounding"            # Evita throttle de renderers de fundo
    "--disable-background-timer-throttling"       # Mantém timers JS em rate normal
    "--disable-background-media-suspend"          # Nunca suspende mídia em abas de fundo
    "--disable-hang-monitor"                      # Sem kill em tabs lentas
    "--window-position=${SECONDARY_X},${SECONDARY_Y}"  # Abre direto no monitor secundário
    "--new-window"
)

launch_chrome_variant() {
    local bin="$1"
    shift
    echo -e "\x1b[1;32m[ext-monitor] Iniciando $bin com anti-occlusion flags...\x1b[0m"
    echo -e "\x1b[1;34m[ext-monitor] YouTube e vídeos serão transmitidos mesmo sem o mouse na janela.\x1b[0m"
    exec "$bin" "${CHROME_FLAGS[@]}" "$@" "$URL"
}

launch_firefox() {
    echo -e "\x1b[1;32m[ext-monitor] Iniciando Firefox com media.suspend-bkgnd-video desativado...\x1b[0m"
    echo -e "\x1b[1;34m[ext-monitor] YouTube e vídeos serão transmitidos mesmo sem o mouse na janela.\x1b[0m"

    # Cria perfil temporário com a preferência de vídeo de fundo desativada
    TMPPROFILE=$(mktemp -d /tmp/firefox-ext-monitor-XXXXXX)
    mkdir -p "$TMPPROFILE"
    cat > "$TMPPROFILE/user.js" <<'EOF'
// Desativa suspensão de vídeo quando a aba perde o foco (occlusion tracking)
user_pref("media.suspend-bkgnd-video.enabled", false);
user_pref("media.suspend-bkgnd-video.delay-ms", 0);
// Mantém Web Audio ativo em abas de fundo
user_pref("dom.audiochannel.mutedByDefault", false);
EOF
    exec firefox --profile "$TMPPROFILE" --new-window "$URL"
}

detect_and_launch() {
    if command -v google-chrome &>/dev/null; then
        launch_chrome_variant "google-chrome"
    elif command -v google-chrome-stable &>/dev/null; then
        launch_chrome_variant "google-chrome-stable"
    elif command -v chromium &>/dev/null; then
        launch_chrome_variant "chromium"
    elif command -v chromium-browser &>/dev/null; then
        launch_chrome_variant "chromium-browser"
    elif command -v brave-browser &>/dev/null; then
        launch_chrome_variant "brave-browser"
    elif command -v microsoft-edge &>/dev/null; then
        launch_chrome_variant "microsoft-edge"
    elif command -v firefox &>/dev/null; then
        launch_firefox
    else
        echo -e "\x1b[1;31m[ext-monitor] ERRO: Nenhum navegador suportado encontrado!\x1b[0m"
        echo "   Instale Google Chrome, Chromium, Brave ou Firefox."
        exit 1
    fi
}

case "$BROWSER" in
    chrome|google-chrome|google-chrome-stable)
        launch_chrome_variant "google-chrome"
        ;;
    chromium|chromium-browser)
        launch_chrome_variant "chromium"
        ;;
    brave|brave-browser)
        launch_chrome_variant "brave-browser"
        ;;
    edge|microsoft-edge)
        launch_chrome_variant "microsoft-edge"
        ;;
    firefox)
        launch_firefox
        ;;
    auto|*)
        detect_and_launch
        ;;
esac

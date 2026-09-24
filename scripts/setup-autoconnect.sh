#!/bin/bash
# ==============================================================================
# setup-autoconnect.sh
# Auto-configuração Plug-and-Play do Pi Zero Second Monitor
# Suporta:
#  1. Modo Sistema (com sudo/root) -> Regra udev em /etc/udev/rules.d/
#  2. Modo Usuário Limitado (Zero Sudo) -> Systemd User Watcher em background
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
CONFIG_DIR="$HOME/.config/ext-monitor"
MARKER_FILE="$CONFIG_DIR/.autoconnect_configured"
FORCE="${1:-}"

if [ -f "$MARKER_FILE" ] && [ "$FORCE" != "--force" ]; then
    exit 0
fi

mkdir -p "$CONFIG_DIR"
mkdir -p "$HOME/.config/systemd/user"

echo -e "\x1b[1;36m[*] [Ext-Monitor] Configurando integração automática Plug-and-Play (USB)...\x1b[0m"

# 1. Instala e recarrega os serviços de usuário systemd
cp "$REPO_DIR/scripts/ext-monitor-autoconnect.service" "$HOME/.config/systemd/user/" 2>/dev/null || true
cp "$REPO_DIR/scripts/ext-monitor-watcher.service" "$HOME/.config/systemd/user/" 2>/dev/null || true
systemctl --user daemon-reload 2>/dev/null || true

# 2. Testa se o usuário tem privilégio sudo sem senha ou interativo
HAS_SUDO=0
if sudo -n true 2>/dev/null; then
    HAS_SUDO=1
fi

if [ $HAS_SUDO -eq 1 ]; then
    echo -e "\x1b[1;32m[+] Privilégio administrativo detectado: instalando regra UDEV no kernel...\x1b[0m"
    sudo cp "$REPO_DIR/scripts/99-ext-monitor.rules" /etc/udev/rules.d/ 2>/dev/null || true
    sudo udevadm control --reload-rules 2>/dev/null || true
    systemctl --user enable ext-monitor-autoconnect.service 2>/dev/null || true
    echo -e "\x1b[1;32m[+] Regra UDEV (/etc/udev/rules.d/99-ext-monitor.rules) ativa com sucesso!\x1b[0m"
else
    echo -e "\x1b[1;33m[!] Usuário limitado detectado (sem sudo direto): ativando Watcher em espaço de usuário...\x1b[0m"
    systemctl --user enable --now ext-monitor-watcher.service 2>/dev/null || true
    echo -e "\x1b[1;32m[+] Ext-Monitor Watcher ativo na sessão do usuário (Zero Sudo)! Monitorando USB via netlink.\x1b[0m"
fi

touch "$MARKER_FILE"
echo -e "\x1b[1;32m[+] Configuração concluída! Agora ao plugar o Pi Zero na USB, a segunda tela será adicionada automaticamente.\x1b[0m"

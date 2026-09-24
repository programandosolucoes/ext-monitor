#!/bin/bash
# ==============================================================================
# usb-watcher.sh: Monitor de eventos USB em Espaço de Usuário (Zero Sudo)
# Escuta o netlink socket do kernel via 'udevadm monitor' sem precisar de root.
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
AUTOCONNECT="$SCRIPT_DIR/autoconnect.sh"

echo "[*] [USB Watcher] Monitor de hardware USB iniciado para o usuário $(whoami)..."

# Executa udevadm monitor filtrando eventos de adição no subsistema USB
udevadm monitor --udev --subsystem-match=usb --property 2>/dev/null | while read -r line; do
    case "$line" in
        *PRODUCT=1d6b/104/*|*ID_MODEL_ID=0104*|*ID_VENDOR_ID=1d6b*)
            echo "[+] [USB Watcher] Evento USB do Pi Zero detectado!"
            # Dispara o script de conexão inteligente em segundo plano
            "$AUTOCONNECT" &
            sleep 6 # Debounce para evitar múltiplos disparos durante a enumeração de interfaces USB
            ;;
    esac
done

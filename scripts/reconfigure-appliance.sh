#!/bin/bash
# ==============================================================================
# Appliance USB Reconfiguration Tool for Raspberry Pi Zero
#
# Configures and switches modes on the Raspberry Pi Zero strictly via USB Serial
# ZERO IP / ZERO NETWORK REQUIRED
#
# Usage:
#   ./scripts/reconfigure-appliance.sh status
#   ./scripts/reconfigure-appliance.sh mode <network|usb-bulk>
#   ./scripts/reconfigure-appliance.sh restart
#   ./scripts/reconfigure-appliance.sh reboot
#
# License: MIT
# Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>
# ==============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERIAL_TOOL="$SCRIPT_DIR/serial-console.sh"

ACTION="${1:-status}"

case "$ACTION" in
    status)
        echo -e "\x1b[1;34m[*] Querying Pi Zero appliance status via USB Serial (/dev/ttyACM0)...\x1b[0m"
        "$SERIAL_TOOL" "echo '=== SYSTEM STATUS ==='; uptime; echo '=== PROCESSES ==='; ps | grep ext-receiver; echo '=== MEMORY ==='; free -m; echo '=== IP ADDRESSES ==='; ifconfig lo; ifconfig usb0 2>/dev/null || true; echo '=== HDMI STATUS ==='; cat /sys/class/drm/card0-HDMI-A-1/status 2>/dev/null || true; cat /sys/class/graphics/fb0/blank 2>/dev/null || true"
        ;;
    mode)
        TARGET_MODE="${2:-network}"
        echo -e "\x1b[1;34m[*] Switching appliance to Mode: $TARGET_MODE via USB Serial...\x1b[0m"
        if [ "$TARGET_MODE" = "usb-bulk" ]; then
            "$SERIAL_TOOL" "pkill -f ext-receiver 2>/dev/null; /usr/local/bin/ext-receiver --mode=usb-bulk &"
            echo -e "\x1b[1;32m[+] Pi Zero switched to Mode 2 (Direct USB Bulk)!\x1b[0m"
        else
            "$SERIAL_TOOL" "pkill -f ext-receiver 2>/dev/null; /usr/local/bin/ext-receiver --mode=network &"
            echo -e "\x1b[1;32m[+] Pi Zero switched to Mode 1 (Network + Miracast Hybrid)!\x1b[0m"
        fi
        ;;
    restart)
        echo -e "\x1b[1;33m[*] Restarting ext-receiver service on Pi Zero via USB Serial...\x1b[0m"
        "$SERIAL_TOOL" "pkill -f ext-receiver 2>/dev/null; sleep 1; /usr/local/bin/ext-receiver &"
        echo -e "\x1b[1;32m[+] ext-receiver restarted successfully!\x1b[0m"
        ;;
    reboot)
        echo -e "\x1b[1;31m[*] Sending reboot command to Pi Zero via USB Serial...\x1b[0m"
        "$SERIAL_TOOL" "reboot"
        echo -e "\x1b[1;32m[+] Reboot signal sent. Pi Zero will reboot into RAM in < 2 seconds.\x1b[0m"
        ;;
    unblank)
        echo -e "\x1b[1;33m[*] Forcing HDMI unblank on Pi Zero via USB Serial...\x1b[0m"
        "$SERIAL_TOOL" "echo 0 > /sys/class/graphics/fb0/blank && echo '[+] Framebuffer fb0 unblanked (active)!'"
        ;;
    disable-net)
        echo -e "\x1b[1;33m[*] Disabling USB network on Pi Zero via USB Serial (Pure Offline Mode)...\x1b[0m"
        "$SERIAL_TOOL" "ifconfig usb0 down && echo '[+] usb0 network disabled. Appliance running strictly via USB!'"
        ;;
    enable-net)
        echo -e "\x1b[1;32m[*] Enabling USB network on Pi Zero via USB Serial...\x1b[0m"
        "$SERIAL_TOOL" "ifconfig usb0 192.168.7.2 netmask 255.255.255.0 up && echo '[+] usb0 online at 192.168.7.2'"
        ;;
    test-pattern)
        echo -e "\x1b[1;34m[*] Sending test pattern to HDMI display via USB Serial...\x1b[0m"
        "$SERIAL_TOOL" "echo 0 > /sys/class/graphics/fb0/blank; cat /dev/urandom | head -c 1843200 > /dev/fb0; echo '[+] Test pattern rendered to HDMI!'"
        ;;
    shell)
        echo -e "\x1b[1;32m[*] Spawning interactive USB Serial Shell on Pi Zero...\x1b[0m"
        exec "$SERIAL_TOOL"
        ;;
    menu|"")
        echo -e "\x1b[1;32m========================================================================\x1b[0m"
        echo -e "\x1b[1;32m  Raspberry Pi Zero Appliance Manager (100% USB Serial - Zero IP)       \x1b[0m"
        echo -e "\x1b[1;34m  Device: /dev/ttyACM0 | Baud: 115200 | Zero Network Required           \x1b[0m"
        echo -e "\x1b[1;32m========================================================================\x1b[0m"
        echo -e "  1) Query Full Appliance Status (Uptime, Temp, RAM, Processes)"
        echo -e "  2) Unblank / Wake HDMI Screen (echo 0 > fb0/blank)"
        echo -e "  3) Show HDMI Test Pattern (Verify TV display)"
        echo -e "  4) Switch to Mode 1 (Network + Miracast Hybrid)"
        echo -e "  5) Switch to Mode 2 (Pure USB Bulk - Zero IP)"
        echo -e "  6) Disable USB Network (Test pure serial-only offline)"
        echo -e "  7) Enable USB Network (192.168.7.2)"
        echo -e "  8) Open Interactive Terminal Shell (/bin/sh on Pi Zero)"
        echo -e "  9) Restart ext-receiver service"
        echo -e " 10) Reboot Pi Zero Appliance"
        echo -e "  0) Exit"
        echo -e "\x1b[1;32m========================================================================\x1b[0m"
        read -p "Select option [0-10]: " choice
        case "$choice" in
            1) "$0" status ;;
            2) "$0" unblank ;;
            3) "$0" test-pattern ;;
            4) "$0" mode network ;;
            5) "$0" mode usb-bulk ;;
            6) "$0" disable-net ;;
            7) "$0" enable-net ;;
            8) "$0" shell ;;
            9) "$0" restart ;;
            10) "$0" reboot ;;
            *) exit 0 ;;
        esac
        ;;
    *)
        echo "Usage: $0 {status|unblank|test-pattern|mode <network|usb-bulk>|disable-net|enable-net|shell|restart|reboot}"
        exit 1
        ;;
esac

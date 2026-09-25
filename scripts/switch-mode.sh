#!/bin/bash
# switch-mode.sh - Display Pipeline Mode Switcher for Raspberry Pi Zero
#
# Manages video stream input priorities while PRESERVING:
# 1. USB Serial Console (/dev/ttyGS0 -> /dev/ttyACM0 on Host) - Always Alive
# 2. USB Network & Web Dashboard (http://192.168.7.2:8080)    - Always Alive
#
# Operational Modes:
# - Mode 1: Network + Miracast Hybrid (UDP port 5000 + RTSP port 7236 for Windows Win+K)
# - Mode 2: USB Bulk Direct (Direct High-Speed USB 480 Mbps video streaming)
#
# Usage:
#   switch-mode.sh network     # Switch video input to Network / Miracast
#   switch-mode.sh usb-bulk    # Switch video input to USB Bulk Direct
#   switch-mode.sh status      # Display status of all 3 subsystems
#
# License: MIT
# Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

set -e

MODE="${1:-status}"

case "$MODE" in
    network|mode1|1)
        echo -e "\x1b[1;34m[*] Activating MODE 1: Network + Miracast Hybrid...\x1b[0m"
        sudo systemctl restart ext-receiver
        echo -e "\x1b[1;32m[+] Mode 1 active.\x1b[0m"
        echo -e "    - Linux Wayland UDP: port 5000"
        echo -e "    - Windows Miracast:  RTSP port 7236 (Win + K)"
        echo -e "    - Web Dashboard:     http://192.168.7.2:8080"
        echo -e "    - Serial Console:    /dev/ttyGS0 (/dev/ttyACM0 on Host)"
        ;;

    usb-bulk|mode2|2)
        echo -e "\x1b[1;34m[*] Activating MODE 2: USB Bulk Direct...\x1b[0m"
        # Ensure FunctionFS display endpoint is mounted
        /usr/local/bin/setup-usb-bulk.sh 2>/dev/null || true

        # Restart ext-receiver in USB Bulk mode
        sudo pkill -f ext-receiver 2>/dev/null || true
        sudo ext-receiver --mode=usb-bulk >/dev/null 2>&1 &
        echo -e "\x1b[1;32m[+] Mode 2 active.\x1b[0m"
        echo -e "    - Video Input:    USB Bulk Endpoint 1 (Direct 480 Mbps)"
        echo -e "    - Web Dashboard:  http://192.168.7.2:8080 (Preserved)"
        echo -e "    - Serial Console: /dev/ttyGS0 (Preserved)"
        ;;

    status)
        echo -e "\x1b[1;32m========================================================================\x1b[0m"
        echo -e "\x1b[1;32m  Raspberry Pi Zero Display Receiver - Subsystem Status                 \x1b[0m"
        echo -e "\x1b[1;32m========================================================================\x1b[0m"

        # 1. Serial Console
        if [ -c /dev/ttyGS0 ] || systemctl is-active serial-getty@ttyGS0.service >/dev/null 2>&1; then
            echo -e "1. Serial Console:   \x1b[1;32mONLINE (/dev/ttyGS0 -> /dev/ttyACM0 on Host)\x1b[0m"
        else
            echo -e "1. Serial Console:   \x1b[1;33mOFFLINE\x1b[0m"
        fi

        # 2. USB Network & Web Dashboard
        if ip a show usb0 2>/dev/null | grep -q "inet 192.168.7.2"; then
            echo -e "2. USB Network:      \x1b[1;32mONLINE (192.168.7.2/24)\x1b[0m"
            echo -e "   Web Dashboard:    \x1b[1;34mhttp://192.168.7.2:8080\x1b[0m"
        else
            echo -e "2. USB Network:      \x1b[1;31mOFFLINE\x1b[0m"
        fi

        # 3. Video Pipeline
        if pgrep -f "ext-receiver.*usb-bulk" >/dev/null; then
            echo -e "3. Video Pipeline:   \x1b[1;32mMODE 2 (Direct USB Bulk via FunctionFS)\x1b[0m"
        elif pgrep -f "ext-receiver" >/dev/null; then
            echo -e "3. Video Pipeline:   \x1b[1;32mMODE 1 (Network UDP 5000 + Miracast RTSP 7236)\x1b[0m"
        else
            echo -e "3. Video Pipeline:   \x1b[1;31mSTOPPED\x1b[0m"
        fi
        echo -e "\x1b[1;32m========================================================================\x1b[0m"
        ;;

    *)
        echo "Usage: $0 {network|usb-bulk|status}"
        exit 1
        ;;
esac

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

PI_IP="${PI_IP:-192.168.7.2}"
MODE="${1:-status}"

case "$MODE" in
    miracast|windows|win|wfd|network|mode1|1)
        echo -e "\x1b[1;34m[*] Ativando Modo Miracast / Rede no Pi Zero ($PI_IP)...\x1b[0m"
        curl -s --max-time 3 -X POST -H "Content-Type: application/json" -d '{"mode1":true,"mode2":true,"mode3":false}' "http://$PI_IP:8080/api/modes" >/dev/null 2>&1 || true
        echo -e "\x1b[1;32m[+] Modo Miracast (Windows Win + K / WFD) ativo!\x1b[0m"
        echo -e "    - Windows Miracast:  Porta RTSP 7236 (Win + K)"
        echo -e "    - Linux Wayland UDP: Porta 5000"
        echo -e "    - Painel Web:        http://$PI_IP:8080"
        ;;

    usb-bulk|bulk|usb|mode3|mode2|3|2)
        echo -e "\x1b[1;34m[*] Ativando Modo USB Bulk Direto no Pi Zero ($PI_IP)...\x1b[0m"
        curl -s --max-time 3 -X POST -H "Content-Type: application/json" -d '{"mode1":false,"mode2":false,"mode3":true}' "http://$PI_IP:8080/api/modes" >/dev/null 2>&1 || true
        echo -e "\x1b[1;32m[+] Modo USB Bulk Direto ativo!\x1b[0m"
        echo -e "    - Canal de Vídeo:    USB FunctionFS Endpoint (480 Mbps)"
        echo -e "    - Painel Web:        http://$PI_IP:8080"
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

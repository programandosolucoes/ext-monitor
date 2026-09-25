#!/bin/bash
# switch-mode.sh - Dual-Mode Switcher for Raspberry Pi Zero Display Receiver
#
# Switches between:
# - Mode 1: Network + Miracast Hybrid (g_multi USB Ethernet + RTSP port 7236 + Web UI port 8080)
# - Mode 2: USB Bulk Direct (FunctionFS f_fs gadget, zero TCP/IP stack overhead)
#
# Usage:
#   switch-mode.sh network     # Switch to Mode 1 (Default)
#   switch-mode.sh usb-bulk    # Switch to Mode 2 (USB Bulk Direct)
#   switch-mode.sh status      # Display active mode
#
# License: MIT
# Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

set -e

MODE="${1:-status}"

case "$MODE" in
    network|mode1|1)
        echo -e "\x1b[1;34m[*] Switching to MODE 1: Network + Miracast Hybrid...\x1b[0m"
        sudo systemctl stop ext-receiver 2>/dev/null || true

        # Clean up FunctionFS if mounted
        if mountpoint -q /dev/usb-ffs/display 2>/dev/null; then
            echo -e "\x1b[1;33m[*] Unmounting FunctionFS gadget...\x1b[0m"
            sudo umount /dev/usb-ffs/display 2>/dev/null || true
        fi

        # Reload g_multi kernel module if needed
        if ! lsmod | grep -q g_multi; then
            echo -e "\x1b[1;34m[*] Loading g_multi kernel module...\x1b[0m"
            sudo modprobe g_multi 2>/dev/null || true
        fi

        # Restart ext-receiver in default network mode
        echo -e "\x1b[1;32m[+] Starting ext-receiver in Mode 1 (Network + Miracast + Web UI)...\x1b[0m"
        sudo systemctl restart ext-receiver
        echo -e "\x1b[1;32m[+] Mode 1 active. Web Dashboard: http://192.168.7.2:8080\x1b[0m"
        ;;

    usb-bulk|mode2|2)
        echo -e "\x1b[1;34m[*] Switching to MODE 2: USB Bulk Direct via FunctionFS...\x1b[0m"
        sudo systemctl stop ext-receiver 2>/dev/null || true

        # Run FunctionFS setup
        /usr/local/bin/setup-usb-bulk.sh 2>/dev/null || /home/pi/setup-usb-bulk.sh 2>/dev/null || /home/carlos/ide/ext-monitor/scripts/setup-usb-bulk.sh

        # Run ext-receiver in USB Bulk mode
        echo -e "\x1b[1;32m[+] Launching ext-receiver in Mode 2 (USB Bulk Direct)...\x1b[0m"
        sudo ext-receiver --mode=usb-bulk &
        echo -e "\x1b[1;32m[+] Mode 2 active. Run 'ext-sender --transport=usb' on Linux host.\x1b[0m"
        ;;

    status)
        echo -e "\x1b[1;32m========================================================================\x1b[0m"
        echo -e "\x1b[1;32m  Raspberry Pi Zero Display Receiver - Operational Status               \x1b[0m"
        echo -e "\x1b[1;32m========================================================================\x1b[0m"
        if pgrep -f "ext-receiver.*usb-bulk" >/dev/null; then
            echo -e "Active Mode: \x1b[1;32mMODE 2 (USB Bulk Direct via FunctionFS)\x1b[0m"
        elif pgrep -f "ext-receiver" >/dev/null; then
            echo -e "Active Mode: \x1b[1;32mMODE 1 (Network + Miracast Hybrid on UDP 5000 & TCP 7236)\x1b[0m"
            echo -e "Web Panel:   \x1b[1;34mhttp://192.168.7.2:8080\x1b[0m"
        else
            echo -e "Active Mode: \x1b[1;31mINACTIVE (ext-receiver stopped)\x1b[0m"
        fi
        ;;

    *)
        echo "Usage: $0 {network|usb-bulk|status}"
        exit 1
        ;;
esac

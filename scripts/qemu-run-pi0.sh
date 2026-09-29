#!/bin/bash
# ==============================================================================
# QEMU Raspberry Pi Zero Emulator Runner
# Emulates a Raspberry Pi Zero (ARM1176JZF-S) with VideoCore IV & USB Hub
#
# Allows running and testing ext-monitor appliance images completely on PC
# without needing physical hardware or a micro-SD card.
#
# License: MIT
# Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>
# ==============================================================================

set -e

SCRIPT_DIR="$(dirname "$(readlink -f "$0")")"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
IMAGES_DIR="${PROJECT_ROOT}/build-buildroot/buildroot-2024.02.1/output/images"

SD_IMAGE="${1:-${IMAGES_DIR}/sdcard.img}"
KERNEL_IMAGE="${IMAGES_DIR}/zImage"
DTB_IMAGE="${IMAGES_DIR}/bcm2708-rpi-zero.dtb"

echo -e "\x1b[1;32m========================================================================\x1b[0m"
echo -e "\x1b[1;32m  QEMU ARM: Raspberry Pi Zero (raspi0) Extended Display Emulator       \x1b[0m"
echo -e "\x1b[1;34m  Emulating: ARMv6 1.0 GHz | RAM: 512MB | VideoCore IV | USB Gadget     \x1b[0m"
echo -e "\x1b[1;32m========================================================================\x1b[0m\n"

if [ ! -f "$SD_IMAGE" ]; then
    echo -e "\x1b[1;33m[!] Note: SD image not found at: $SD_IMAGE\x1b[0m"
    echo -e "    Run \x1b[1;32m${SCRIPT_DIR}/buildroot/build-minimal-image.sh\x1b[0m first to build it."
    echo -e "    Alternatively, specify path: \x1b[1;37m$0 <path/to/sdcard.img>\x1b[0m\n"
    exit 1
fi

echo -e "\x1b[1;34m[*] Forwarding Ports to Host:\x1b[0m"
echo -e "    - HTTP Web Dashboard: \x1b[1;32mhttp://localhost:8080\x1b[0m"
echo -e "    - Miracast RTSP WFD:  \x1b[1;32mtcp://localhost:7236\x1b[0m"
echo -e "    - RTP Video Stream:   \x1b[1;32mudp://localhost:5000\x1b[0m"
echo -e "\x1b[1;33m[*] Starting QEMU (Press Ctrl + A then X to exit)...\x1b[0m\n"

QEMU_CMD="qemu-system-arm -M raspi0"

if [ -f "$KERNEL_IMAGE" ] && [ -f "$DTB_IMAGE" ]; then
    $QEMU_CMD \
        -kernel "$KERNEL_IMAGE" \
        -dtb "$DTB_IMAGE" \
        -sd "$SD_IMAGE" \
        -append "console=ttyAMA0,115200 root=/dev/ram0 rdinit=/init quiet vt.global_cursor_default=0" \
        -netdev user,id=net0,hostfwd=tcp::8080-:8080,hostfwd=tcp::7236-:7236,hostfwd=udp::5000-:5000 \
        -device usb-net,netdev=net0 \
        -serial stdio
else
    # Direct SD card boot mode
    $QEMU_CMD \
        -sd "$SD_IMAGE" \
        -netdev user,id=net0,hostfwd=tcp::8080-:8080,hostfwd=tcp::7236-:7236,hostfwd=udp::5000-:5000 \
        -device usb-net,netdev=net0 \
        -serial stdio
fi

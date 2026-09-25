#!/bin/bash
# setup-usb-bulk.sh - Configures Linux USB FunctionFS Gadget (Mode 2) on Raspberry Pi Zero
#
# Enables direct USB Bulk endpoint streaming bypassing TCP/IP and UDP.
#
# License: MIT
# Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

set -e

UDC_NAME=$(ls /sys/class/udc | head -n1)
GADGET_DIR="/sys/kernel/config/usb_gadget/g_display"
FFS_DIR="/dev/usb-ffs/display"

echo -e "\x1b[1;34m[*] Configuring USB FunctionFS Display Gadget (UDC: $UDC_NAME)...\x1b[0m"

# 1. Ensure kernel modules are loaded
modprobe libcomposite 2>/dev/null || true
modprobe usb_f_fs 2>/dev/null || true

# 2. Unbind existing gadget if active
if [ -d "$GADGET_DIR" ]; then
    echo "" > "$GADGET_DIR/UDC" 2>/dev/null || true
    umount "$FFS_DIR" 2>/dev/null || true
    rm -f "$GADGET_DIR/configs/c.1/ffs.display" 2>/dev/null || true
    rmdir "$GADGET_DIR/functions/ffs.display" 2>/dev/null || true
    rmdir "$GADGET_DIR/configs/c.1/strings/0x409" 2>/dev/null || true
    rmdir "$GADGET_DIR/configs/c.1" 2>/dev/null || true
    rmdir "$GADGET_DIR/strings/0x409" 2>/dev/null || true
    rmdir "$GADGET_DIR" 2>/dev/null || true
fi

# 3. Create USB Gadget descriptors
mkdir -p "$GADGET_DIR"
cd "$GADGET_DIR"

echo 0x1d6b > idVendor       # Linux Foundation
echo 0x0104 > idProduct      # Multifunction / Display Gadget
echo 0x0200 > bcdUSB         # USB 2.0 High-Speed (480 Mbps)
echo 0xef > bDeviceClass
echo 0x02 > bDeviceSubClass
echo 0x01 > bDeviceProtocol

mkdir -p strings/0x409
echo "fedcba9876543210" > strings/0x409/serialnumber
echo "Raspberry Pi" > strings/0x409/manufacturer
echo "Pi Zero VideoCore IV Display" > strings/0x409/product

mkdir -p configs/c.1/strings/0x409
echo "USB Bulk Direct Display" > configs/c.1/strings/0x409/configuration
echo 500 > configs/c.1/MaxPower

mkdir -p functions/ffs.display
ln -s functions/ffs.display configs/c.1/

# 4. Mount FunctionFS user-space directory
mkdir -p "$FFS_DIR"
if ! mountpoint -q "$FFS_DIR"; then
    mount -t functionfs display "$FFS_DIR"
fi

echo -e "\x1b[1;32m[+] FunctionFS mounted at $FFS_DIR. Ready for ext-receiver to open ep0.\x1b[0m"

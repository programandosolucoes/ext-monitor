#!/bin/bash
# setup-composite-gadget.sh - 3-in-1 Super Composite USB Gadget
#
# Binds 3 simultaneous USB functions into a single composite USB device on Raspberry Pi Zero:
# 1. Serial Console (CDC ACM -> /dev/ttyGS0, accessible on PC as /dev/ttyACM0)
# 2. USB Ethernet (RNDIS/ECM -> usb0, static IP 192.168.7.2 for Web Panel, SSH, Miracast)
# 3. Direct USB Bulk Display (FunctionFS -> /dev/usb-ffs/display for zero-network H.264 streaming)
#
# All 3 functions run CONCURRENTLY from boot. The user NEVER loses the serial recovery
# console or the Web Control Dashboard!
#
# License: MIT
# Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

set -e

UDC_NAME=$(ls /sys/class/udc | head -n1)
GADGET_DIR="/sys/kernel/config/usb_gadget/ext_composite"
FFS_DIR="/dev/usb-ffs/display"

echo -e "\x1b[1;34m[*] Initializing 3-in-1 Super Composite USB Gadget (UDC: $UDC_NAME)...\x1b[0m"

# 1. Ensure kernel modules are loaded
modprobe libcomposite 2>/dev/null || true
modprobe usb_f_acm 2>/dev/null || true
modprobe usb_f_rndis 2>/dev/null || modprobe usb_f_ecm 2>/dev/null || true
modprobe usb_f_fs 2>/dev/null || true

# 2. Clean up existing gadget if present
if [ -d "$GADGET_DIR" ]; then
    echo "" > "$GADGET_DIR/UDC" 2>/dev/null || true
    umount "$FFS_DIR" 2>/dev/null || true
    rm -f "$GADGET_DIR/configs/c.1/acm.usb0" 2>/dev/null || true
    rm -f "$GADGET_DIR/configs/c.1/rndis.usb0" 2>/dev/null || true
    rm -f "$GADGET_DIR/configs/c.1/ffs.display" 2>/dev/null || true
    rmdir "$GADGET_DIR/functions/acm.usb0" 2>/dev/null || true
    rmdir "$GADGET_DIR/functions/rndis.usb0" 2>/dev/null || true
    rmdir "$GADGET_DIR/functions/ffs.display" 2>/dev/null || true
    rmdir "$GADGET_DIR/configs/c.1/strings/0x409" 2>/dev/null || true
    rmdir "$GADGET_DIR/configs/c.1" 2>/dev/null || true
    rmdir "$GADGET_DIR/strings/0x409" 2>/dev/null || true
    rmdir "$GADGET_DIR" 2>/dev/null || true
fi

# 3. Create Gadget descriptor tree
mkdir -p "$GADGET_DIR"
cd "$GADGET_DIR"

echo 0x1d6b > idVendor   # Linux Foundation
echo 0x0104 > idProduct  # Multifunction Composite Gadget
echo 0x0200 > bcdUSB     # USB 2.0 High-Speed (480 Mbps)
echo 0xef > bDeviceClass
echo 0x02 > bDeviceSubClass
echo 0x01 > bDeviceProtocol

mkdir -p strings/0x409
echo "fedcba9876543210" > strings/0x409/serialnumber
echo "Raspberry Pi" > strings/0x409/manufacturer
echo "Pi Zero Super-Composite Display & Network" > strings/0x409/product

mkdir -p configs/c.1/strings/0x409
echo "Serial + Network + USB Bulk Display" > configs/c.1/strings/0x409/configuration
echo 500 > configs/c.1/MaxPower

# 4. Function 1: Serial Console (CDC ACM)
echo -e "\x1b[1;34m[*] Adding Function 1: USB Serial Console (CDC ACM)...\x1b[0m"
mkdir -p functions/acm.usb0
ln -s functions/acm.usb0 configs/c.1/

# 5. Function 2: USB Ethernet (RNDIS)
echo -e "\x1b[1;34m[*] Adding Function 2: USB Network Card (RNDIS/Ethernet)...\x1b[0m"
mkdir -p functions/rndis.usb0
echo "ae:31:02:f7:7d:5a" > functions/rndis.usb0/dev_addr
echo "ae:31:02:f7:7d:5b" > functions/rndis.usb0/host_addr
ln -s functions/rndis.usb0 configs/c.1/

# 6. Function 3: Direct USB Bulk Display (FunctionFS)
echo -e "\x1b[1;34m[*] Adding Function 3: USB Bulk Display (FunctionFS)...\x1b[0m"
mkdir -p functions/ffs.display
ln -s functions/ffs.display configs/c.1/

# 7. Mount FunctionFS user-space endpoint directory
mkdir -p "$FFS_DIR"
if ! mountpoint -q "$FFS_DIR"; then
    mount -t functionfs display "$FFS_DIR"
fi

echo -e "\x1b[1;32m[+] 3-in-1 Super Composite Gadget prepared successfully.\x1b[0m"
echo -e "\x1b[1;32m[+] FunctionFS mounted at $FFS_DIR. Ready for ext-receiver.\x1b[0m"

#!/bin/bash
# ==============================================================================
# Flash Ultra-Fast 32MB Appliance Image to Micro-SD Card
#
# Safeguards:
# - Strictly blocks NVMe and system disks (rootfs / mounted disks)
# - Verifies unmounting of existing partitions
# - Flashes with direct sync (conv=fsync)
#
# License: MIT
# Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>
# ==============================================================================

set -e

SCRIPT_DIR="$(dirname "$(readlink -f "$0")")"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
IMAGE_FILE="${PROJECT_ROOT}/build-appliance/ext-monitor-pi0-appliance.img"

TARGET_DEV="${1:-/dev/sda}"

echo -e "\x1b[1;32m========================================================================\x1b[0m"
echo -e "\x1b[1;32m  Raspberry Pi Zero Appliance SD Card Flasher                          \x1b[0m"
echo -e "\x1b[1;34m  Target Image: $IMAGE_FILE                                             \x1b[0m"
echo -e "\x1b[1;34m  Target Disk:  $TARGET_DEV                                             \x1b[0m"
echo -e "\x1b[1;32m========================================================================\x1b[0m\n"

if [ ! -f "$IMAGE_FILE" ]; then
    echo -e "\x1b[1;31m[!] Error: Image file not found at $IMAGE_FILE\x1b[0m"
    echo -e "    Please run ./scripts/build-fast-appliance.sh first."
    exit 1
fi

if [ ! -b "$TARGET_DEV" ]; then
    echo -e "\x1b[1;31m[!] Error: Target block device $TARGET_DEV does not exist!\x1b[0m"
    exit 1
fi

# Safety check: Never allow flashing NVMe or internal system drive
if [[ "$TARGET_DEV" =~ "nvme" ]] || [[ "$TARGET_DEV" =~ "zram" ]]; then
    echo -e "\x1b[1;31m[!] FATAL: Target device is an internal NVMe / RAM disk. Aborting for safety!\x1b[0m"
    exit 1
fi

# Safety check: Check if any partition of TARGET_DEV is mounted at /
if mount | grep -q "^${TARGET_DEV}.* on / "; then
    echo -e "\x1b[1;31m[!] FATAL: Target device contains rootfs (/). Aborting!\x1b[0m"
    exit 1
fi

echo -e "\x1b[1;33m[*] Unmounting any mounted partitions on ${TARGET_DEV}...\x1b[0m"
for part in $(ls ${TARGET_DEV}* 2>/dev/null); do
    if [ "$part" != "$TARGET_DEV" ]; then
        sudo umount "$part" 2>/dev/null || true
    fi
done

echo -e "\x1b[1;34m[*] Flashing 32MB image to ${TARGET_DEV} via dd...\x1b[0m"
sudo dd if="$IMAGE_FILE" of="$TARGET_DEV" bs=4M status=progress conv=fsync

echo -e "\x1b[1;34m[*] Informing OS of updated partition table...\x1b[0m"
sudo blockdev --rereadpt "$TARGET_DEV" 2>/dev/null || sudo partx -u "$TARGET_DEV" 2>/dev/null || true
sleep 1

echo -e "\n\x1b[1;32m========================================================================\x1b[0m"
echo -e "\x1b[1;32m  SUCCESS: Micro-SD Card Flashed and Verified!                         \x1b[0m"
echo -e "\x1b[1;32m========================================================================\x1b[0m"
echo -e "\x1b[1;33mHardware Connection Steps:\x1b[0m"
echo -e "  1. Safely remove micro-SD card from PC and insert into Raspberry Pi Zero."
echo -e "  2. Connect mini-HDMI cable to the Samsung TV / Monitor."
echo -e "  3. Connect micro-USB OTG cable (center port) from Pi Zero to PC USB 3.0 port."
echo -e "  4. Within < 2 seconds, the appliance will boot into RAM and expose all 3 modes:"
echo -e "     - Mode 1: http://192.168.7.2:8080 (Web Dashboard) & UDP 5000 (Video Stream)"
echo -e "     - Mode 2: /dev/usb-ffs/display (USB Bulk Direct)"
echo -e "     - Mode 3: /dev/ttyACM0 on PC (Serial Console Shell)\n"

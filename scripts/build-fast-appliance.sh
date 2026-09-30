#!/bin/bash
# ==============================================================================
# Ultra-Fast 32MB Appliance Image Builder for Raspberry Pi Zero
#
# Assembles an ultra-lean 32MB bootable SD card image:
# 1. Raspberry Pi official firmware & VideoCore IV kernel (~11MB)
# 2. Pure Rust ext-receiver (ARMv6 stripped ~642KB)
# 3. Minimal glibc runtime + busybox (<2MB initramfs)
# 4. FAT32 single partition (100% RAM initramfs boot in <2s, 0% SD corruption risk)
#
# License: MIT
# Author: Carlos Alberto <carlosalberto4ti@gmail.com>
# ==============================================================================

set -e

SCRIPT_DIR="$(dirname "$(readlink -f "$0")")"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="${PROJECT_ROOT}/build-appliance"
OUTPUT_IMG="${BUILD_DIR}/ext-monitor-pi0-appliance.img"

echo -e "\x1b[1;32m========================================================================\x1b[0m"
echo -e "\x1b[1;32m  Raspberry Pi Zero Ultra-Fast Appliance Image Builder                 \x1b[0m"
echo -e "\x1b[1;34m  Target: 32MB SD card image | Boots in < 2s | 100% RAM Initramfs      \x1b[0m"
echo -e "\x1b[1;32m========================================================================\x1b[0m\n"

# 1. Compile host ext-sender and appliance ext-receiver
echo -e "\x1b[1;34m[*] Step 1: Compiling host ext-sender and pure-Rust ext-receiver for ARMv6...\x1b[0m"
(
    cd "${PROJECT_ROOT}/sender"
    cargo build --release
)
(
    cd "${PROJECT_ROOT}/receiver"
    cargo build --release --target arm-unknown-linux-gnueabihf
)

RECEIVER_BIN="${PROJECT_ROOT}/target/arm-unknown-linux-gnueabihf/release/ext-receiver"
if command -v arm-linux-gnueabihf-strip >/dev/null 2>&1; then
    arm-linux-gnueabihf-strip "$RECEIVER_BIN"
fi
cp "$RECEIVER_BIN" "${BUILD_DIR}/initramfs/usr/local/bin/ext-receiver"
chmod +x "${BUILD_DIR}/initramfs/usr/local/bin/ext-receiver"

# 2. Package client tools archive for one-click downloads
echo -e "\x1b[1;34m[*] Step 2: Packaging client tools archive for web dashboard downloads...\x1b[0m"
mkdir -p "${BUILD_DIR}/initramfs/var/www/download"
cargo build --release -p ext-sender
TMP_CLIENT=$(mktemp -d)
if [ -f "${PROJECT_ROOT}/target/release/ext-sender" ]; then
    cp "${PROJECT_ROOT}/target/release/ext-sender" "${TMP_CLIENT}/ext-sender"
    strip "${TMP_CLIENT}/ext-sender" 2>/dev/null || true
    cp "${PROJECT_ROOT}/scripts/start.sh" "${TMP_CLIENT}/start.sh"
    cp "${PROJECT_ROOT}/scripts/stop.sh" "${TMP_CLIENT}/stop.sh"
    cp "${PROJECT_ROOT}/scripts/connect.sh" "${TMP_CLIENT}/connect.sh"
    cp "${PROJECT_ROOT}/scripts/install-host.sh" "${TMP_CLIENT}/install-host.sh"
    cp "${PROJECT_ROOT}/scripts/wayland-damage-pacer.py" "${TMP_CLIENT}/wayland-damage-pacer.py"
    cp "${PROJECT_ROOT}/scripts/99-ext-monitor.rules" "${TMP_CLIENT}/99-ext-monitor.rules"
    cp "${PROJECT_ROOT}/scripts/show-welcome-window.py" "${TMP_CLIENT}/show-welcome-window.py"
    cat << 'EOF' > "${TMP_CLIENT}/README.txt"
========================================================================
EXT-MONITOR: Pacote de Ferramentas Portáteis do Host PC
========================================================================
Instruções Rápidas:
1. Conecte o cabo USB do Pi Zero (porta USB de dados) no PC.
2. Para instalar o driver de sistema de forma permanente:
     sudo ./install-host.sh
3. Ou para iniciar imediatamente sem instalar nada no sistema:
     ./start.sh extend auto 30 false full --bitrate=3000
========================================================================
EOF
    chmod +x "${TMP_CLIENT}"/*.sh "${TMP_CLIENT}/ext-sender" "${TMP_CLIENT}/show-welcome-window.py" 2>/dev/null || true
    tar -czf "${BUILD_DIR}/initramfs/var/www/download/client.tar.gz" -C "${TMP_CLIENT}" .
    cp "${TMP_CLIENT}/ext-sender" "${BUILD_DIR}/initramfs/var/www/download/ext-sender"
    cp "${PROJECT_ROOT}/scripts/connect.sh" "${BUILD_DIR}/initramfs/var/www/download/connect.sh" 2>/dev/null || true
    cp "${PROJECT_ROOT}/scripts/99-ext-monitor.rules" "${BUILD_DIR}/initramfs/var/www/download/99-ext-monitor.rules" 2>/dev/null || true
fi
rm -rf "${TMP_CLIENT}"

# 3. Package initramfs.cpio.gz
echo -e "\x1b[1;34m[*] Step 3: Packaging minimal initramfs.cpio.gz...\x1b[0m"
(
    cd "${BUILD_DIR}/initramfs"
    sudo mkdir -p dev
    sudo mknod -m 600 dev/console c 5 1 2>/dev/null || true
    sudo mknod -m 666 dev/null c 1 3 2>/dev/null || true
    sudo mknod -m 666 dev/zero c 1 5 2>/dev/null || true
    sudo mknod -m 666 dev/tty1 c 4 1 2>/dev/null || true
    sudo find . -print0 | sudo cpio --null -ov --format=newc -R 0:0 2>/dev/null | gzip -9 > "${BUILD_DIR}/boot/initramfs.cpio.gz"
)

# 4. Create 32MB Disk Image matching BCM2835 Boot ROM Sector 1 geometry
echo -e "\x1b[1;34m[*] Step 4: Generating 32MB bootable appliance image (Sector 1, FAT16, 2KB clusters)...\x1b[0m"
dd if=/dev/zero of="$OUTPUT_IMG" bs=512 count=65537 status=none
echo "label: dos
label-id: 0x00000000
unit: sectors

1 : start=1, size=65536, type=c, bootable" | sfdisk "$OUTPUT_IMG" >/dev/null 2>&1

LOOP_DEV=$(sudo losetup -fP --show "$OUTPUT_IMG")
sudo mkfs.fat -F 16 -s 4 -R 4 -n "EXTMONITOR" "${LOOP_DEV}p1" >/dev/null

MOUNT_DIR=$(mktemp -d)
sudo mount "${LOOP_DEV}p1" "$MOUNT_DIR"
sudo cp -r "${BUILD_DIR}/boot/"* "$MOUNT_DIR/"
sudo umount "$MOUNT_DIR"
rm -rf "$MOUNT_DIR"
sudo losetup -d "$LOOP_DEV"

# 5. Copy raw image and compress release artifacts
echo -e "\x1b[1;34m[*] Step 5: Updating release artifacts (33MB raw image & compressed GZ)...\x1b[0m"
mkdir -p "${PROJECT_ROOT}/release"
cp -f "$OUTPUT_IMG" "${PROJECT_ROOT}/release/ext-monitor-pi0-appliance.img"
gzip -c9 "$OUTPUT_IMG" > "${PROJECT_ROOT}/release/ext-monitor-pi0-appliance.img.gz"
(
    cd "${PROJECT_ROOT}/release"
    sha256sum ext-monitor-pi0-appliance.img ext-monitor-pi0-appliance.img.gz > SHA256SUMS
)

echo -e "\n\x1b[1;32m========================================================================\x1b[0m"
echo -e "\x1b[1;32m  SUCCESS: Universal Appliance Image Ready (Pi Zero 1 & Zero 2 W)!      \x1b[0m"
echo -e "\x1b[1;34m  Image Location: $OUTPUT_IMG                                          \x1b[0m"
echo -e "\x1b[1;34m  Size: $(du -h "$OUTPUT_IMG" | cut -f1)                               \x1b[0m"
echo -e "\x1b[1;34m  Release GZ: ${PROJECT_ROOT}/release/ext-monitor-pi0-appliance.img.gz  \x1b[0m"
echo -e "\x1b[1;32m========================================================================\x1b[0m"
echo -e "\x1b[1;33mTo flash directly to a micro-SD card:\x1b[0m"
echo -e "  sudo dd if=$OUTPUT_IMG of=/dev/sdX bs=4M status=progress conv=fsync\n"

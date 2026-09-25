#!/bin/bash
# ==============================================================================
# Buildroot Minimal Appliance Builder for Raspberry Pi Zero Extended Monitor
# Produces an ultra-lean ~32MB bootable SD card image (sdcard.img)
#
# Architecture:
# - Boots in ~2 seconds directly into RAM (initramfs)
# - Read-only rootfs (100% corruption-proof on sudden power disconnect)
# - Single 32MB FAT32 partition
# - Contains VideoCore IV VPU driver, composite USB gadget, and pure Rust receiver
#
# License: MIT
# Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>
# ==============================================================================

set -e

SCRIPT_DIR="$(dirname "$(readlink -f "$0")")"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILDROOT_VERSION="2024.02.1"
BUILD_DIR="${PROJECT_ROOT}/build-buildroot"
DL_DIR="${PROJECT_ROOT}/downloads"

echo -e "\x1b[1;32m========================================================================\x1b[0m"
echo -e "\x1b[1;32m  Raspberry Pi Zero Minimal Appliance Image Builder (Buildroot)        \x1b[0m"
echo -e "\x1b[1;34m  Target: ~32MB SD card image | Boots in ~2s | Zero-Corruption Initramfs\x1b[0m"
echo -e "\x1b[1;32m========================================================================\x1b[0m\n"

# 1. Compile pure-Rust ext-receiver for ARMv6 Pi Zero
echo -e "\x1b[1;34m[*] Step 1: Compiling and stripping pure-Rust ext-receiver for ARMv6...\x1b[0m"
(
    cd "${PROJECT_ROOT}/receiver"
    cargo build --release --target arm-unknown-linux-gnueabihf
)

RECEIVER_BIN="${PROJECT_ROOT}/target/arm-unknown-linux-gnueabihf/release/ext-receiver"
if [ ! -f "$RECEIVER_BIN" ]; then
    echo -e "\x1b[1;31m[!] Error: Receiver binary not found at $RECEIVER_BIN\x1b[0m"
    exit 1
fi

# Strip symbols to minimize size
if command -v arm-linux-gnueabihf-strip >/dev/null 2>&1; then
    arm-linux-gnueabihf-strip "$RECEIVER_BIN"
fi

# 2. Stage binary into rootfs overlay
OVERLAY_BIN_DIR="${SCRIPT_DIR}/rootfs-overlay/usr/local/bin"
mkdir -p "$OVERLAY_BIN_DIR"
cp "$RECEIVER_BIN" "${OVERLAY_BIN_DIR}/ext-receiver"
chmod +x "${OVERLAY_BIN_DIR}/ext-receiver"
echo -e "\x1b[1;32m[+] Staged ext-receiver ($(du -h "${OVERLAY_BIN_DIR}/ext-receiver" | cut -f1)) into rootfs overlay.\x1b[0m"

# 3. Check for Buildroot source
mkdir -p "$DL_DIR" "$BUILD_DIR"
BUILDROOT_TAR="buildroot-${BUILDROOT_VERSION}.tar.xz"
BUILDROOT_SRC="${BUILD_DIR}/buildroot-${BUILDROOT_VERSION}"

if [ ! -d "$BUILDROOT_SRC" ]; then
    if [ ! -f "${DL_DIR}/${BUILDROOT_TAR}" ]; then
        echo -e "\x1b[1;34m[*] Step 2: Downloading Buildroot ${BUILDROOT_VERSION}...\x1b[0m"
        wget -c -O "${DL_DIR}/${BUILDROOT_TAR}" "https://buildroot.org/downloads/${BUILDROOT_TAR}"
    fi
    echo -e "\x1b[1;34m[*] Extracting Buildroot source...\x1b[0m"
    tar -xf "${DL_DIR}/${BUILDROOT_TAR}" -C "$BUILD_DIR"
fi

# 4. Configure Buildroot with ext-monitor defconfig
echo -e "\x1b[1;34m[*] Step 3: Configuring Buildroot for Raspberry Pi Zero...\x1b[0m"
cd "$BUILDROOT_SRC"
make BR2_EXTERNAL="${SCRIPT_DIR}" defconfig BR2_DEFCONFIG="${SCRIPT_DIR}/ext_monitor_defconfig"

# 5. Build minimal appliance image
echo -e "\x1b[1;34m[*] Step 4: Compiling Linux kernel & assembling 32MB sdcard.img...\x1b[0m"
echo -e "\x1b[1;33m[!] Note: First build compiles the ARM cross-toolchain and kernel (takes ~15-25m).\x1b[0m"
make -j"$(nproc)"

OUTPUT_IMG="${BUILDROOT_SRC}/output/images/sdcard.img"
if [ -f "$OUTPUT_IMG" ]; then
    echo -e "\n\x1b[1;32m========================================================================\x1b[0m"
    echo -e "\x1b[1;32m  SUCCESS: 32MB Minimal SD Card Image Generated!                        \x1b[0m"
    echo -e "\x1b[1;34m  Image Location: $OUTPUT_IMG                                           \x1b[0m"
    echo -e "\x1b[1;34m  Size: $(du -h "$OUTPUT_IMG" | cut -f1)                                \x1b[0m"
    echo -e "\x1b[1;32m========================================================================\x1b[0m"
    echo -e "\x1b[1;33mTo flash to an SD card:\x1b[0m"
    echo -e "  sudo dd if=$OUTPUT_IMG of=/dev/sdX bs=4M status=progress conv=fsync"
    echo -e "\x1b[1;33mTo test in QEMU:\x1b[0m"
    echo -e "  ${PROJECT_ROOT}/scripts/qemu-run-pi0.sh\n"
fi

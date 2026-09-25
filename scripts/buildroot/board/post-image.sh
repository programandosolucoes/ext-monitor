#!/bin/sh

set -e

BOARD_DIR="$(dirname "$0")"
BINARIES_DIR="${1:-output/images}"

cp "${BOARD_DIR}/config.txt" "${BINARIES_DIR}/rpi-firmware/config.txt"
cp "${BOARD_DIR}/cmdline.txt" "${BINARIES_DIR}/rpi-firmware/cmdline.txt"

# Generate sdcard.img using genimage
GENIMAGE_CFG="${BOARD_DIR}/genimage.cfg"
GENIMAGE_TMP="${BINARIES_DIR}/genimage.tmp"

rm -rf "${GENIMAGE_TMP}"

genimage \
    --rootpath "${TARGET_DIR}" \
    --tmppath "${GENIMAGE_TMP}" \
    --inputpath "${BINARIES_DIR}" \
    --outputpath "${BINARIES_DIR}" \
    --config "${GENIMAGE_CFG}"

echo "========================================================================"
echo "  Minimal Appliance Image successfully generated:                       "
echo "  ${BINARIES_DIR}/sdcard.img (32 MB)                                    "
echo "========================================================================"
exit 0

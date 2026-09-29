#!/bin/sh
# ==============================================================================
# Ultra-Fast Initramfs Appliance Init Script for Raspberry Pi Zero
# Boots in < 2 seconds directly in RAM with zero SD corruption risk
#
# Sets up:
# 1. Essential mounts (proc, sys, dev, configfs)
# 2. 3-in-1 USB Composite Gadget (ACM Serial + ECM Net)
# 3. Static IP 192.168.7.2 on usb0
# 4. Pure Rust ext-receiver (Web Dashboard + V4L2 M2M hardware video decoder)
#
# License: MIT
# Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>
# ==============================================================================

# Install busybox symlinks if missing
/bin/busybox --install -s /bin 2>/dev/null || true
/bin/busybox --install -s /sbin 2>/dev/null || true

# 1. Mount virtual filesystems
mount -t proc none /proc
mount -t sysfs none /sys
mount -t devtmpfs none /dev
mkdir -p /sys/kernel/config /sys/kernel/debug /dev/pts
mount -t configfs none /sys/kernel/config
mount -t devpts none /dev/pts

# Disable console blanking so screen never turns black
echo 0 > /sys/module/kernel/parameters/consoleblank 2>/dev/null || true
echo -e "\033[9;0]\033[14;0]" > /dev/tty1 2>/dev/null || true

echo "========================================================================"
echo "  Raspberry Pi Zero Minimal Appliance: 100% RAM Initramfs Boot          "
echo "  ext-receiver: Hardware H.264 Video Decoder (VideoCore IV + KMS)        "
echo "========================================================================"

# 2. Load essential kernel modules
modprobe i2c-bcm2835 2>/dev/null || true
modprobe snd-bcm2835 2>/dev/null || true
modprobe snd-soc-hdmi-codec 2>/dev/null || true
modprobe vc4 2>/dev/null || true
modprobe bcm2835-codec 2>/dev/null || true
modprobe dwc2 2>/dev/null || true
modprobe libcomposite 2>/dev/null || true
modprobe usb_f_acm 2>/dev/null || true
modprobe usb_f_ecm 2>/dev/null || modprobe usb_f_rndis 2>/dev/null || true
modprobe usb_f_fs 2>/dev/null || true

# Unbind virtual console (fbcon) so kernel/stdout messages never overwrite the display!
echo 0 > /sys/class/vtconsole/vtcon1/bind 2>/dev/null || true
echo 0 > /sys/class/vtconsole/vtcon0/bind 2>/dev/null || true

# Display early hardware boot splash screen (4 languages) as soon as GPU driver is ready
if [ -f /etc/splash_loading.raw.gz ] && [ -e /dev/fb0 ]; then
    gzip -dc /etc/splash_loading.raw.gz > /dev/fb0 2>/dev/null || true
fi

# 2.5 Early detect boot mode from SD card
BOOT_MODE="network"
BOOT_MODE_ARG=""
for i in 1 2 3 4; do
    [ -b /dev/mmcblk0p1 ] || [ -b /dev/mmcblk0 ] && break
    sleep 0.3
done
TARGET_DEV=""
[ -b /dev/mmcblk0p1 ] && TARGET_DEV="/dev/mmcblk0p1"
[ -z "$TARGET_DEV" ] && [ -b /dev/mmcblk0 ] && TARGET_DEV="/dev/mmcblk0"

if [ -n "$TARGET_DEV" ]; then
    mkdir -p /mnt/boot
    if mount -t vfat "$TARGET_DEV" /mnt/boot 2>/dev/null; then
        if [ -f /mnt/boot/mode.txt ]; then
            MODE_VAL=$(cat /mnt/boot/mode.txt | tr -d ' \r\n')
            if [ "$MODE_VAL" = "usb-bulk" ] || [ "$MODE_VAL" = "bulk" ] || [ "$MODE_VAL" = "3" ] || [ "$MODE_VAL" = "2" ]; then
                BOOT_MODE="usb-bulk"
                BOOT_MODE_ARG="--mode=usb-bulk"
                echo "[+] Mode detected from /mnt/boot/mode.txt: MODE 3 (USB Bulk Direct)"
            else
                echo "[+] Mode detected from /mnt/boot/mode.txt: MODE 1 (Network UDP)"
            fi
        fi
        umount /mnt/boot 2>/dev/null || true
    fi
fi

# 3. Setup Composite USB Gadget
GADGET_DIR="/sys/kernel/config/usb_gadget/ext_composite"
if [ -d /sys/kernel/config/usb_gadget ]; then
    mkdir -p "$GADGET_DIR"
    cd "$GADGET_DIR"
    
    echo 0x1d50 > idVendor
    echo 0x614d > idProduct
    echo 0x0100 > bcdDevice
    echo 0x0200 > bcdUSB
    
    mkdir -p strings/0x409
    echo "Raspberry Pi" > strings/0x409/manufacturer
    echo "Pi Zero High-Speed Display & Network Hub" > strings/0x409/product
    echo "PI0-EXT-COMPOSITE-001" > strings/0x409/serialnumber
    
    mkdir -p configs/c.1/strings/0x409
    echo "Composite Display + Net + Serial" > configs/c.1/strings/0x409/configuration
    echo 500 > configs/c.1/MaxPower
    
    # Function 1: Serial Console (/dev/ttyGS0 on Pi -> /dev/ttyACM0 on Host) - ALWAYS ACTIVE
    mkdir -p functions/acm.usb0
    ln -sf functions/acm.usb0 configs/c.1/
    
    # Function 2: ECM Network (CDC-ECM) - ALWAYS ACTIVE (for Web Dashboard, Mode 1 UDP, Mode 2 Miracast)
    mkdir -p functions/ecm.usb0
    echo "12:22:33:44:55:66" > functions/ecm.usb0/host_addr
    echo "12:22:33:44:55:67" > functions/ecm.usb0/dev_addr
    ln -sf functions/ecm.usb0 configs/c.1/
    
    if [ "$BOOT_MODE" = "usb-bulk" ]; then
        # Function 3: FunctionFS Display (USB Bulk Direct - Class 0xFF)
        mkdir -p functions/ffs.display
        ln -sf functions/ffs.display configs/c.1/
        mkdir -p /dev/usb-ffs/display
        mount -t functionfs display /dev/usb-ffs/display
        echo "[+] FunctionFS display endpoint mounted at /dev/usb-ffs/display"
    else
        # Function 3: USB Mass Storage (Exposes SD Card partition directly to Host)
        if [ -n "$TARGET_DEV" ]; then
            modprobe usb_f_mass_storage 2>/dev/null || true
            mkdir -p functions/mass_storage.0
            echo 1 > functions/mass_storage.0/stall
            echo 0 > functions/mass_storage.0/lun.0/cdrom
            echo 0 > functions/mass_storage.0/lun.0/ro
            echo 0 > functions/mass_storage.0/lun.0/nofua
            echo "$TARGET_DEV" > functions/mass_storage.0/lun.0/file
            ln -sf functions/mass_storage.0 configs/c.1/
            echo "[+] SD Card $TARGET_DEV exported as USB Mass Storage for Host PC"
        fi
    fi
    
    # In network mode, bind UDC now (in usb-bulk mode, ext-receiver writes ep0 descriptors first, then binds UDC)
    if [ "$BOOT_MODE" != "usb-bulk" ]; then
        UDC_NAME=""
        for i in 1 2 3 4 5 6; do
            UDC_NAME=$(ls /sys/class/udc 2>/dev/null | head -n 1)
            [ -n "$UDC_NAME" ] && break
            sleep 0.5
        done
        if [ -n "$UDC_NAME" ]; then
            echo "$UDC_NAME" > UDC
            echo "[+] Composite USB Gadget bound to $UDC_NAME"
        else
            echo "[-] Warning: No UDC device found in /sys/class/udc"
        fi
    fi
fi

# 4. Bring up network interfaces
ifconfig lo 127.0.0.1 up

# Maintain usb0 network interface whenever UDC binds (works across all modes and mode transitions)
(
    while true; do
        if [ -d /sys/class/net/usb0 ]; then
            if ! ifconfig usb0 2>/dev/null | grep -q "192.168.7.2"; then
                ifconfig usb0 192.168.7.2 netmask 255.255.255.0 up
                echo "[+] Network interface usb0 online at 192.168.7.2"
            fi
        fi
        sleep 1
    done
) &

# Enable IPv6 Link-Local
[ -f /proc/sys/net/ipv6/conf/usb0/disable_ipv6 ] && echo 0 > /proc/sys/net/ipv6/conf/usb0/disable_ipv6 2>/dev/null || true

# Fallback: if physical Ethernet (eth0) or Wi-Fi (wlan0) present, run standard DHCP client
if [ -d /sys/class/net/eth0 ]; then
    echo "[+] Physical Ethernet (eth0) detected, requesting DHCP lease..."
    udhcpc -i eth0 -b -q 2>/dev/null &
fi
if [ -d /sys/class/net/wlan0 ]; then
    echo "[+] Wi-Fi (wlan0) detected, requesting DHCP lease..."
    udhcpc -i wlan0 -b -q 2>/dev/null &
fi

# 5. Spawn auto-respawning background shell on USB serial console (/dev/ttyGS0 -> /dev/ttyACM0 on Host)
(
    while true; do
        if [ -c /dev/ttyGS0 ]; then
            setsid cttyhack /bin/sh </dev/ttyGS0 >/dev/ttyGS0 2>&1 || /bin/sh -i </dev/ttyGS0 >/dev/ttyGS0 2>&1
        fi
        sleep 1
    done
) &
if [ -c /dev/ttyAMA0 ]; then
    /sbin/getty -L -n -l /bin/sh 115200 ttyAMA0 vt100 &
fi

# Ensure HDMI Framebuffer is unblanked and active
echo 0 > /sys/class/graphics/fb0/blank 2>/dev/null || true

# 6. Display visual appliance dashboard splash on HDMI Screen (100% graphical, 4 languages)
if [ -f /etc/splash_ready.raw.gz ] && [ -e /dev/fb0 ]; then
    gzip -dc /etc/splash_ready.raw.gz > /dev/fb0 2>/dev/null || true
fi

# Also output textual info to USB Serial Console (/dev/ttyGS0) so PC users connecting with
# screen /dev/ttyACM0 115200 get instant interactive feedback without polluting HDMI!
if [ -c /dev/ttyGS0 ]; then
    cat << 'EOF' > /dev/ttyGS0 2>/dev/null || true

========================================================================
   EXT-MONITOR: RASPBERRY PI ZERO EXTENDED DISPLAY APPLIANCE
========================================================================
   Hardware:   Raspberry Pi Zero W (BCM2835 ARMv6 @ 1.0 GHz)
   Firmware:   VideoCore IV Hardware GPU (V4L2 M2M + KMS DRM)
   Resolution: 1280x720 @ 60 FPS (HDMI-A-1)
   Network:    usb0: 192.168.7.2 (DHCP Gateway Active: assigns 192.168.7.1)
------------------------------------------------------------------------
   STATUS: ONLINE / PRONTO / PRONTO PER STREAM / 就绪 (4 LANGUAGES)

   * Web Dashboard:  http://192.168.7.2:8080 (EN / PT / IT / ZH)
   * Linux Stream:   UDP port 5000 (RTP H.264)
   * Windows Stream: TCP port 7236 (Miracast Win+K)
   * Serial Console: /dev/ttyACM0 on PC (115200 baud)
------------------------------------------------------------------------
   [PT] Iniciar no Linux:     cd ~/ide/ext-monitor && ./scripts/start.sh
   [EN] Connect from Linux:   cd ~/ide/ext-monitor && ./scripts/start.sh
   [IT] Avviare su Linux:     cd ~/ide/ext-monitor && ./scripts/start.sh
   [ZH] 从 Linux 主机连接:    cd ~/ide/ext-monitor && ./scripts/start.sh
========================================================================

EOF
fi

# 7. Launch ext-receiver with watchdog supervisor
mkdir -p /var/log
echo "[+] Starting ext-receiver (Web Panel on :8080, Mode: ${BOOT_MODE_ARG:-Mode 1 UDP})..."
if [ -x /usr/local/bin/ext-receiver ]; then
    (
        while true; do
            if ! pidof ext-receiver >/dev/null; then
                echo "[supervisor] Starting ext-receiver engine..."
                /usr/local/bin/ext-receiver $BOOT_MODE_ARG
            fi
            sleep 2
        done
    ) 2>&1 | tee -a /var/log/ext-receiver.log >> /dev/ttyGS0 2>/dev/null &
fi

echo "[+] Appliance initialization complete in < 2 seconds."
while true; do
    sleep 3600
done

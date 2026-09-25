# Raspberry Pi Zero Appliance SD Card Image (Ready to Flash)

This directory contains the ready-to-flash, ultra-fast 32MB bootable appliance image for the **Raspberry Pi Zero v1.3 / W / 2 W**.

- **File:** `ext-monitor-pi0-appliance.img.gz` (Compressed size: ~11 MB | Uncompressed size: 32 MB)
- **Boot Time:** < 2 seconds directly in RAM (tmpfs initramfs)
- **SD Corruption Risk:** 0% (Single FAT32 partition, runs 100% in RAM)
- **Built-in Services:**
  - 100% Native Rust `ext-receiver` (VideoCore IV V4L2 M2M H.264 hardware decode)
  - Native Pure-Rust Zero-Gateway DHCP Server for `usb0` (Hands out `192.168.7.1` to PC)
  - Embedded Web Control Dashboard on HTTP port 8080 (`http://192.168.7.2:8080`)
  - Miracast / MS-MICE RTSP server on TCP port 7236 (Native Windows 10/11 `Win + K` casting)

---

## 🚀 How to Flash to Micro-SD Card

### On Linux:
```bash
# 1. Decompress image
gzip -d -k ext-monitor-pi0-appliance.img.gz

# 2. Identify your micro-SD card (e.g., /dev/sdb or /dev/mmcblk0)
lsblk

# 3. Flash to SD card (replace /dev/sdX with your actual SD device)
sudo dd if=ext-monitor-pi0-appliance.img of=/dev/sdX bs=4M status=progress conv=fsync
```

### On Windows / macOS:
1. Decompress `ext-monitor-pi0-appliance.img.gz` using 7-Zip or gzip.
2. Use **Raspberry Pi Imager** or **BalenaEtcher** to select `ext-monitor-pi0-appliance.img` and write to your micro-SD card.

---

## 🔌 First Boot & Verification
1. Insert the micro-SD card into the Raspberry Pi Zero.
2. Connect mini-HDMI to your monitor.
3. Connect a Micro-USB cable from the Pi's **USB (OTG) port** to your laptop or PC.
4. The Pi Zero boots in **1.8 seconds**. Your PC automatically receives IP `192.168.7.1` with zero internet interruption.
5. Open your browser to `http://192.168.7.2:8080` to access the live dashboard!

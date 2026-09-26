# Raspberry Pi Zero / Zero 2 W Appliance SD Card Image (Ready to Flash)

This directory contains the universal, ready-to-flash, ultra-fast bootable appliance image for the **Raspberry Pi Zero v1.2 / v1.3 / W / WH** and **Raspberry Pi Zero 2 W**.

- **File:** `ext-monitor-pi0-appliance.img.gz` (Compressed download size: ~19 MB | Uncompressed size: 256 MB)
- **Boot Time:** < 2 seconds directly into RAM (100% tmpfs initramfs)
- **SD Corruption Risk:** 0% (Standard 256MB FAT32 partition with 130,044 clusters, strictly read-only after boot)
- **Universal Hardware Support:** Contains device trees and kernels for both single-core ARMv6 (`BCM2835`) and quad-core ARMv7 (`BCM2710`).
- **Built-in Services:**
  - 100% Native Rust `ext-receiver` (VideoCore IV V4L2 M2M H.264 hardware decode directly to KMS DRM)
  - Native Pure-Rust Zero-Gateway DHCP Server for `usb0` (Hands out `192.168.7.1` to PC with zero internet disruption)
  - Embedded Web Control Dashboard on HTTP port 8080 (`http://192.168.7.2:8080`)
  - Miracast / MS-MICE RTSP server on TCP port 7236 (Native Windows 10/11 `Win + K` wireless casting)

---

## 🔌 Hardware Port Wiring (Crucial Pinout Guide)

```
                            [ 40-Pin GPIO Header ]
  +------------------------------------------------------------------------+
  | [Micro-SD Slot]                                                        |
  | (Flashed Card)                [ BCM2835 SoC ]                          |
  |                                                                 [CSI]  |
  +---------[ mini-HDMI ]-----------[ Micro-USB OTG ]---------[ PWR IN ]---+
                   │                        │                     │
                   │                        │                     └── [EMPTY!] DO NOT CONNECT CABLE
                   │                        │                         (Host PC provides full power)
                   │                        └── Micro-USB Cable to PC / Laptop USB
                   │                            (5V Power + 480 Mbps High-Speed Data)
                   └── mini-HDMI Cable to External Monitor or Living Room TV
```

1. **mini-HDMI Port (Front long edge, LEFT):** Connects to the second monitor or TV HDMI input.
2. **Micro-USB OTG Port (Front long edge, CENTER with USB logo):** Connects directly to a USB port on your host PC/laptop.
3. **Micro-USB PWR IN Port (Front long edge, RIGHT):** **MUST REMAIN EMPTY!** The Raspberry Pi Zero is fully powered via the center OTG port.

---

## 🚀 How to Flash to Micro-SD Card

### Option 1: Direct In-Situ Flash via USB (Without removing card from Pi Zero!)
If the Raspberry Pi Zero is connected to your PC via USB and in bootloader recovery mode:
```bash
sudo rpiboot -v
# The Pi Zero mounts its SD card as a USB Mass Storage disk (/dev/sda)!
sudo dd if=ext-monitor-pi0-appliance.img of=/dev/sda bs=4M status=progress conv=fsync
```

### Option 2: Flash via Card Reader on Linux:
```bash
# 1. Decompress image
gzip -d -k ext-monitor-pi0-appliance.img.gz

# 2. Identify your micro-SD card (e.g., /dev/sdb or /dev/mmcblk0)
lsblk

# 3. Flash to SD card (replace /dev/sdX with your actual SD device)
sudo dd if=ext-monitor-pi0-appliance.img of=/dev/sdX bs=4M status=progress conv=fsync
```

### Option 3: On Windows / macOS:
1. Decompress `ext-monitor-pi0-appliance.img.gz` using 7-Zip or gzip.
2. Use **Raspberry Pi Imager** or **BalenaEtcher** to select `ext-monitor-pi0-appliance.img` and write to your micro-SD card.

---

## 🔌 First Boot & Verification
1. Insert the micro-SD card into the Raspberry Pi Zero.
2. Connect the mini-HDMI cable to your monitor or TV.
3. Connect a Micro-USB cable from the Pi's **center USB (OTG) port** to your laptop or PC.
4. The VideoCore IV rainbow firmware test screen appears immediately on your display, followed by Linux boot in **under 2 seconds**.
5. Your PC automatically receives IP `192.168.7.1` (Zero-Gateway DHCP) with zero disruption to your Wi-Fi/Ethernet internet connection.
6. Open your browser to `http://192.168.7.2:8080` to access the live dashboard!

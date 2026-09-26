# Raspberry Pi Zero / Zero 2 W Appliance SD Card Image (Ready to Flash)

This directory contains the universal, ready-to-flash, ultra-fast bootable appliance image for the **Raspberry Pi Zero v1.2 / v1.3 / W / WH** and **Raspberry Pi Zero 2 W**.

- **File:** `ext-monitor-pi0-appliance.img.gz` (Compressed download size: ~21 MB | Uncompressed size: 32 MB)
- **Boot Time:** < 2 seconds directly into RAM (100% tmpfs initramfs)
- **SD Corruption Risk:** 0% (Standard 32MB FAT16 partition, strictly read-only after boot, 100% BCM2835 Boot ROM compliant)
- **Universal Hardware Support:** Contains device trees and kernels for both single-core ARMv6 (`BCM2835`) and quad-core ARMv7 (`BCM2710`).
- **3 Concurrent Post-Boot Modes:**
  - **Mode 1 (Hybrid Network & Miracast):** Linux Wayland streaming via UDP port 5000 + Windows 10/11 native casting via RTSP port 7236 (`Win + K`) + Zero-Gateway DHCP server assigning `192.168.7.1` to PC + Web Dashboard at `http://192.168.7.2:8080`.
  - **Mode 2 (USB Bulk Direct):** Zero-network 480 Mbps direct streaming over USB FunctionFS (`/dev/usb-ffs/display`).
  - **Mode 3 (USB Serial Console):** Permanent CDC ACM serial terminal (`/dev/ttyGS0` on Pi, `/dev/ttyACM0` on Host) for instant recovery and diagnostics without Wi-Fi.
- **Hardware Acceleration:** Native V4L2 M2M hardware H.264 decode on VideoCore IV (`/dev/video10`) directly into DRM KMS (`/dev/dri/card0`) at 60 FPS with ~0% CPU.

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

## 🔌 First Boot, Verification & Troubleshooting

### Normal Boot Sequence
1. Insert the micro-SD card into the Raspberry Pi Zero (v1.3 Single-Core or Zero 2 W).
2. Connect the mini-HDMI cable to your monitor or TV.
3. Connect a Micro-USB cable from the Pi's **center USB (OTG) port** to your laptop or PC.
4. **Stage 1 (VideoCore IV Firmware):** The 4-color rainbow test screen appears immediately on your display, confirming power, BCM2835 Boot ROM FAT32 mount (`>= 65,525 clusters`), and HDMI signal synchronization.
5. **Stage 2 (Linux Kernel & Userspace):** The Linux kernel initializes KMS/DRM, loads USB gadget drivers, and starts `ext-receiver`.
6. **Stage 3 (USB Network Connection):** The Pi asserts the USB D+ pullup resistor. Your host PC automatically detects the USB network adapter and receives IP `192.168.7.1` (Zero-Gateway DHCP) with zero disruption to your Wi-Fi/Ethernet internet connection.
7. Open your browser to `http://192.168.7.2:8080` to access the live dashboard!

### 🔍 Diagnostic Notes:
- **Rainbow Screen Appears, but Host PC Doesn't Detect USB Network Device:**
  - In standard Raspberry Pi OS kernels, the USB gadget stack (`CONFIG_USB_CONFIGFS=m`, `libcomposite`, `u_ether`, `usb_f_ecm`) is built as loadable `.ko` kernel modules.
  - If the initramfs lacks `/lib/modules/$(uname -r)/`, the gadget cannot register with configfs and the hardware D+ pullup is never asserted. Ensure kernel modules are embedded in the initramfs or run with a kernel featuring monolithic gadget support (`CONFIG_USB_ETH=y`).
- **Target Hardware Reference:**
  - Tested and verified on **Raspberry Pi Zero v1.3 Single-Core (ARMv6 BCM2835 @ 1.0 GHz, 512MB RAM)**.


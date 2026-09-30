# Blueprint 07: Micro-SD Flash Protection in RAM and USB Firmware Upgrades

> [🇧🇷 Versão em Português](../pt/07-cartao-sd-em-ram-e-upgrade-usb.md) | 🇺🇸 English Version

**Project:** `ext-monitor`  
**Author:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Reference Files:** `build-appliance/initramfs/init`, `receiver/src/web.rs`, `receiver/src/web_ui.rs`  
**Date:** September 2026 (Updated for v2.3.0 Universal Appliance)  

---

## 1. Overview and Problem Statement

A frequent failure mode in embedded Raspberry Pi deployments is **micro-SD card filesystem corruption**:
* When standard desktop Linux distributions (Raspberry Pi OS / Ubuntu) run, background disk writes are continuous (systemd journals, swap, filesystem metadata, browser caches).
* If power is abruptly cut or the USB cable is unplugged, an incomplete write block corrupts the filesystem, leaving the board unbootable.

`ext-monitor` resolves this fundamentally: **the operating system executes 100% in RAM**, and the micro-SD card remains completely **unmounted, write-protected, and ready for USB firmware upgrades**.

---

## 2. 100% RAM Execution Mechanics

During device initialization:
1. The Broadcom Boot ROM reads `initramfs.cpio.gz` from the FAT16 boot partition and unpacks it into RAM (`/dev/ram0`).
2. The Linux kernel takes control and launches `/init`.
3. `/init` mounts virtual filesystems (`tmpfs` for `/tmp`, `/var`, `/run`, `procfs` for `/proc`, `sysfs` for `/sys`).
4. **No physical partitions remain mounted during operation.**
5. The MMC card controller (`/dev/mmcblk0`) enters deep idle state.

### Key Architectural Advantages:
* **Zero Corruption on Power Loss:** Users can unplug the USB cable at any time with zero risk of filesystem damage.
* **Extreme Performance:** All binaries and configurations run at full LPDDR2 RAM speed (hundreds of MB/s), eliminating SD flash read latency.
* **Unlimited Flash Lifespan:** Zero disk write cycles ensure the physical micro-SD card lasts indefinitely.

---

## 3. Micro-SD Card as USB Mass Storage Gadget

Because `/dev/mmcblk0p1` is unmounted in RAM, it can be attached to the **USB Mass Storage** gadget function:

```sh
# Probe MMC driver and bind to mass storage function
if [ -b /dev/mmcblk0p1 ] || [ -b /dev/mmcblk0 ]; then
    mkdir -p functions/mass_storage.0
    echo 1 > functions/mass_storage.0/stall
    echo 0 > functions/mass_storage.0/lun.0/cdrom
    echo 0 > functions/mass_storage.0/lun.0/ro
    echo 0 > functions/mass_storage.0/lun.0/nofua
    TARGET_DEV="/dev/mmcblk0p1"
    [ ! -b "$TARGET_DEV" ] && TARGET_DEV="/dev/mmcblk0"
    echo "$TARGET_DEV" > functions/mass_storage.0/lun.0/file
    ln -sf functions/mass_storage.0 configs/c.1/
fi
```

### What Happens on the Host Computer:
1. Connecting the USB cable triggers the host OS to discover a **USB Mass Storage Drive** labeled **`EXTMONITOR`**.
2. Opening File Explorer or Nautilus reveals all firmware files:
   * `config.txt` (Display resolution and HDMI timings)
   * `cmdline.txt` (Kernel parameters)
   * `mode.txt` (Persistent boot mode: e.g. `usb-bulk`)
   * `initramfs.cpio.gz` (Complete embedded appliance image)
   * `kernel.img` (Linux kernel for ARMv6 Pi Zero)
   * `kernel7.img` (Linux kernel for ARMv7 Pi Zero 2 W)
   * Device tree blobs and overlays.

---

## 4. In-Situ Firmware Upgrades Without Removing the SD Card

### Method 1: Host File Manager Drag-and-Drop
1. Connect the Pi Zero via the USB data port.
2. Open the **`EXTMONITOR`** drive.
3. Replace `initramfs.cpio.gz` or `kernel.img` with the updated release artifact.
4. Safely eject the drive in Windows or Linux.
5. Reboot the board—the updated firmware boots immediately from RAM.

### Method 2: Web Dashboard Upload (`http://192.168.7.2:8080`)
1. Click **"Mount SD Card"** in the Web Dashboard. The appliance runs `mount -t vfat /dev/mmcblk0p1 /mnt/boot`.
2. Upload the new firmware file through the web browser.
3. Click **"Safely Unmount"** and **"Reboot Appliance"**.

### Method 3: Scripted OTA via `curl` / `ext-tool`
```bash
# Automated OTA in-RAM firmware update:
ext-tool deploy
```
This transfers the compressed payload via HTTP `POST /api/system/update`, reboots the Pi Zero in RAM in under 3 seconds, and writes zero bytes to physical flash.

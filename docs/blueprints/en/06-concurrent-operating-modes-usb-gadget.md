# Blueprint 06: Concurrent Operating Modes and the USB ConfigFS Super-Gadget

> [🇧🇷 Versão em Português](../pt/06-modos-de-operacao-concorrentes-e-usb-gadget.md) | 🇺🇸 English Version

**Project:** `ext-monitor`  
**Author:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Reference Files:** `build-appliance/initramfs/init`, `receiver/src/wfd.rs`, `receiver/src/usb_bulk.rs`, `sender/src/usb_transport.rs`, `receiver/src/web_ui.rs`  
**Date:** September 2026 (Updated for v2.3.0 Universal Appliance)  

---

## 1. Overview and Design Architecture

A key architectural hallmark of `ext-monitor` is its **universal concurrent capability**:
* A single Raspberry Pi Zero connected to any computer (Linux, Windows, or macOS) concurrently exposes high-speed Ethernet networking, an emergency serial diagnostic console, and a mass-storage firmware upgrade partition.
* The Rust firmware listens simultaneously for incoming video streams from **Linux (UDP/RTP on port 5000)**, **Windows Miracast (RTSP WFD on port 7236)**, and **USB Bulk Direct (FunctionFS)**, enabling zero-reboot switching between host PCs.
* The Web Dashboard provides granular toggles to enable or disable individual listeners for complete resource isolation when required.

---

## 2. USB ConfigFS Super-Gadget Layout

The Linux USB Gadget Subsystem is initialized during boot by `/init` through the kernel **ConfigFS** virtual filesystem (`/sys/kernel/config/usb_gadget/`):

```
/sys/kernel/config/usb_gadget/ext_composite/
├── idVendor (0x1d50 Openmoko / 0x1d6b Linux Foundation)
├── idProduct (0x614d Multifunction Hub / 0x0104 Composite Gadget)
├── strings/0x409/ (Manufacturer: Raspberry Pi | Product: High-Speed Display Hub)
├── configs/c.1/ (Max Power: 500mA)
│    ├── acm.usb0 ----------> [ Link to Function 1: Serial CDC-ACM ]
│    ├── ecm.usb0 ----------> [ Link to Function 2: CDC-ECM Ethernet ]
│    └── ffs.usb0 ----------> [ Link to Function 3: FunctionFS Display ]
└── functions/
     ├── acm.usb0 (Serial console /dev/ttyGS0 -> /dev/ttyACM0 on PC)
     ├── ecm.usb0 (High-Speed USB Ethernet 480 Mbps)
     └── ffs.usb0 (USB Bulk Direct mounted at /dev/usb-ffs/display)
```

### 2.1 Hardware Endpoint Budget on DWC2 Controller
The Broadcom BCM2835 DWC2 OTG hardware controller provides **8 hardware endpoints** (Control EP0 + 7 configurable IN/OUT endpoints):
* `acm.usb0`: Consumes 2 endpoints (1 Bulk IN, 1 Bulk OUT) + 1 Interrupt IN.
* `ecm.usb0`: Consumes 2 endpoints (1 Bulk IN, 1 Bulk OUT) + 1 Interrupt IN.
* `ffs.usb0`: Consumes 2 endpoints (1 Bulk OUT video stream, 1 Bulk IN control channel).
* **Total:** Exactly 7 endpoints allocated + EP0. The super-gadget operates strictly within silicon budget without bus conflicts or descriptor thrashing.

---

## 3. The Three Concurrent Ingress Modes

```
                                [ ext-receiver ]
                                       │
        ┌──────────────────────────────┼──────────────────────────────┐
        ▼                              ▼                              ▼
  [ MODE 1: LINUX ]            [ MODE 2: WINDOWS ]             [ MODE 3: USB BULK ]
Channel: UDP Port 5000       Channel: RTSP Port 7236        Channel: USB FunctionFS
Protocol: RTP H.264          Protocol: Wi-Fi Display / WFD  Protocol: Raw Packet Stream
Client: ext-sender           Client: Native Win+K Cast      Client: Direct USB Driver
Latency: < 15ms              Latency: ~30ms                 Latency: < 1ms
```

### 3.1 Mode 1: Linux UDP High-Performance Stream (Port 5000)
* **Target:** Linux workstations running GNOME Wayland or X11.
* **Mechanism:** Host executes `ext-sender` (or `connect.sh`). Screen frames are captured via PipeWire / KMS and streamed via RTP H.264 directly to `192.168.7.2:5000`.
* **Performance:** 60 FPS continuous CFR, sub-15ms latency, live bitrate scaling (400–6000 kbps), and color mode switching (24-bit TrueColor, 256 colors, monochrome).

### 3.2 Mode 2: Windows Miracast Driverless Display (`Win + K`)
* **Target:** Laptops or desktops running Windows 10 or Windows 11.
* **Mechanism:**
  1. User presses native Windows shortcut: **`Win + K`** (Cast / Connect).
  2. Windows scans local wireless displays and discovers **"ExtMonitor-Pi0"**.
  3. The `wfd.rs` Rust module negotiates RTSP M1–M7 messages over TCP port 7236.
  4. Windows hardware encodes the secondary desktop via GPU (Intel/AMD/Nvidia) and sends MPEG-TS H.264 streams to the Pi Zero.
  5. The Pi Zero decodes and presents HDMI video with **zero third-party software installation required on Windows**.

### 3.3 Mode 3: USB Bulk Direct (FunctionFS / Raw Endpoints)
* **Target:** Ultra-low latency setups (< 1ms transport) or environments where corporate firewalls restrict UDP networking.
* **Mechanism:** Video NALUs are written directly to USB Bulk OUT endpoints (`ep1` on `/dev/usb-ffs/display/ep1`).
* **Native Host Command:**
  ```bash
  ext-sender --usb    # Directly connects to Mode 3 USB Bulk (< 1ms)
  ```
* **Non-Root Udev Permissions (`/etc/udev/rules.d/99-ext-monitor-usb.rules`):**
  Grants regular non-root users read/write access to OpenMoko (`1d50:614d`) and Linux Foundation (`1d6b:0104`) descriptors:
  ```udev
  SUBSYSTEM=="usb", ATTR{idVendor}=="1d50", ATTR{idProduct}=="614d", MODE="0666", GROUP="plugdev"
  SUBSYSTEM=="usb", ATTR{idVendor}=="1d6b", ATTR{idProduct}=="0104", MODE="0666", GROUP="plugdev"
  ```
* **ZLP (Zero-Length Packet) Handling:** `receiver/src/ingress/usb.rs` properly handles `Ok(0)` reads when transfers end on 512-byte boundaries, keeping the ingress thread alive without treating ZLPs as unexpected EOF disconnects.

---

## 4. Web Control Keys & Bidirectional Transport Sync (Zero Black Screen)

The Web Dashboard (`http://192.168.7.2:8080`) provides granular toggles for all modes:
1. **Synchronized Mode Switching:** When toggling Mode 3 (USB Bulk) on, the receiver dispatches a UDP control packet to host port `5001` (`{"action":"start","transport":"usb_bulk"}`). `ext-sender` hot-switches its pipeline to the FunctionFS Bulk OUT endpoint, preventing packet starvation and **eliminating the black screen**.
2. **Seamless Network Return:** When disabling Mode 3 or re-enabling Mode 1, the receiver notifies the sender (`{"action":"start","transport":"network"}`), and both smoothly return to UDP Port 5000.
3. **Session State Persistence:** All toggle changes are persisted across browser reloads via `localStorage` and synchronized with the backend.

---

## 5. Summary

The USB ConfigFS Super-Gadget turns the Raspberry Pi Zero into an agile, multi-role appliance that adapts to any operating system host instantly, delivering maximum compatibility without compromising silicon efficiency.

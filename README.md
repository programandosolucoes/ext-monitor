# ext-monitor: GPU Hardware-Offloaded USB Second Monitor Engine

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Language: Rust](https://img.shields.io/badge/Language-Rust_100%25-orange.svg)](https://www.rust-lang.org/)
[![Hardware: BCM2835 / VideoCore IV](https://img.shields.io/badge/Hardware-Broadcom_BCM2835-red.svg)](https://www.raspberrypi.com/)
[![Latency: < 15ms](https://img.shields.io/badge/Latency-%3C15ms_Drop--on--Late-brightgreen.svg)]()
[![OS: Linux & Windows](https://img.shields.io/badge/OS-Linux_Wayland_%26_Windows_10%2F11-blueviolet.svg)]()

A high-performance, 100% native Rust engine designed to transform a **Raspberry Pi Zero (v1.2 / v1.3 / W / Zero 2 W)** connected via a single standard **Micro-USB 2.0 cable (OTG 480 Mbps)** into a **zero-latency hardware HDMI second display** for Linux (Wayland / GNOME Mutter) and Windows 10/11 (native Miracast / `Win + K`).

Delivers fluid **60 FPS** at native panel resolution (1280x720 / 1600x900) with **sub-15ms latency** and **~0% CPU load** on the Pi Zero through zero-copy GPU pipeline offloading (Broadcom VideoCore IV V4L2 M2M hardware decoder + KMS/DRM overlay scanout, driven by AMD VA-API, NVIDIA NVENC, or Intel QSV hardware encoding on the host).

---

![Raspberry Pi Zero Dual Monitor Desk Setup](docs/assets/hero-setup.jpg)

---

## 🎯 What It Does & What It Is For

Modern operating systems lack low-friction, driverless ways to add a dedicated secondary display over standard USB cables without expensive DisplayLink adapters or sluggish VNC/RDP network mirrors.

`ext-monitor` solves this at the silicon level:
* **True Physical Display Extension:** Exposes a virtual HDMI output in GNOME Wayland / Mutter or native Windows Wireless Display (`Win + K`). Windows and workspaces snap, drag, and maximize natively.
* **Single-Cable Simplicity:** The Raspberry Pi Zero is powered and communicates entirely over a single micro-USB cable plugged into the host PC. No external power bricks, no extra dongles.
* **100% RAM Embedded Appliance:** Boots directly from RAM (`initramfs.cpio.gz`) in **under 1.8 seconds**. The micro-SD card is uncoupled after boot, ensuring **zero risk of filesystem corruption** upon sudden disconnection.
* **Sub-15ms Real-Time Response:** Implements multi-tier *Drop-on-Late* frame decimation (Moonlight/Sunshine architecture) with 3-deep LIFO ring buffers, eliminating buffer bloat and mouse pointer latency.
* **Zero Host Drivers on Windows & 1-Line Setup on Linux:** Works out-of-the-box with Windows 10/11 via native Miracast (RTSP port 7236). On Linux, connect in one click using `curl -sSL http://192.168.7.2:8080/connect.sh | bash`.

---

## 🔌 Hardware Setup & Correct Port Wiring

<p align="center">
  <img src="docs/assets/hardware-macro.jpg" alt="Raspberry Pi Zero BCM2835 with High-Speed USB and HDMI Connections" width="760" />
</p>

### Port Pinout & Anti-Error Guide

```
                            [ 40-Pin GPIO Header ]
  +------------------------------------------------------------------------+
  | [Micro-SD Slot]                                                        |
  | (SanDisk Card)                [ BCM2835 SoC ]                          |
  |                                                                 [CSI]  |
  +---------[ mini-HDMI ]-----------[ Micro-USB OTG ]---------[ PWR IN ]---+
                   │                        │                     │
                   │                        │                     └── [LEAVE EMPTY!] DO NOT PLUG
                   │                        │                         (Host PC supplies 5V via OTG)
                   │                        └── Micro-USB to PC / Laptop USB Port
                   │                            (5V Bus Power + 480 Mbps Data Transport)
                   │
                   └── mini-HDMI Cable to Secondary Monitor or TV
```

* **mini-HDMI Port (Left):** Dedicated video scanout to your secondary monitor or television.
* **Center Micro-USB Port (OTG / Data + Power):** Plugged directly into the host PC or laptop. Delivers 5V power and handles all data traffic (480 Mbps).
* **Right Micro-USB Port (PWR IN):** **MUST REMAIN EMPTY!** Do not attach an external power supply when connected to a computer to avoid ground loops.
* **Power Draw:** Only ~0.8W (safely within any standard USB 2.0 port power spec).
* **Operating Temperature:** ~44.5°C sustained load (zero thermal throttling).

---

## 🏗️ System Architecture

```
[ HOST PC (Linux Wayland GNOME 46 / Ubuntu 24.04) ]
  ├── Kernel HDMI Override: Forces kernel HDMI-A-1 connected with real monitor EDID
  ├── Mutter DisplayConfig D-Bus: Creates side-by-side virtual display (1600x900 / 1280x720)
  ├── Mutter ScreenCast D-Bus: Emits zero-copy DMA-BUF video stream with embedded cursor
  └── Rust `ext-sender` (GPU Hardware Transmission Engine):
        ├── Hardware Encoder Engine:
        │     ├── AMD / Intel: VA-API Direct DMA-BUF -> `vah264enc` (target-usage=5, min-qp=18, max-qp=34)
        │     ├── NVIDIA: NVENC Zero-Latency -> `nvh264enc` (preset=low-latency-hq, zerolatency=true)
        │     ├── Intel: QuickSync -> `qsvh264enc` (rate-control=cbr, target-usage=7)
        │     └── CPU Software: x264 zerolatency ultrafast fallback
        ├── Frame Skipping & Damage Redraw (`videorate drop-only=true`):
        │     └── Skips redundant static frames; focuses entire bitrate budget on active UI redraws
        ├── Pacing & Leaky Queues:
        │     └── Multi-layer drop-on-late queues drop delayed frames before network transmission
        └── Dual Transport: UDP RTP port 5000 (Mode 1) or USB Bulk Direct FunctionFS (Mode 3)
              │
              ▼ [ Micro-USB OTG Cable / Network Link ]
              │
[ RECEIVER APPLIANCE (Raspberry Pi Zero W / BCM2835 VideoCore IV) ]
  └── Rust `ext-receiver` (All-in-One Multi-Mode Display Daemon):
        ├── Built-in Zero-Gateway DHCP Server: Assigns 192.168.7.1 to PC without breaking main Wi-Fi/Ethernet
        ├── Web Dashboard & Control Socket: Serves management UI on port 8080 and handles hot-apply on UDP 5001
        ├── WFD Miracast RTSP Server: Listens on TCP 7236 for native Windows 10/11 Win + K projections
        ├── Broadcom VideoCore IV Hardware VPU Decoder:
        │     └── Decodes RFC 6184 H.264 stream via `/dev/video10` (V4L2 M2M `bcm2835-codec`)
        └── KMS/DRM Direct Scanout:
              └── Commits NV12/RGB planes directly to HDMI without X11 or Wayland compositor overhead
```

---

## 🔀 Three Concurrent Operating Modes

The appliance boots an active composite USB gadget providing three concurrent services:

| Mode | Target Platform | Protocol / Port | Latency | Key Advantage |
| :--- | :--- | :--- | :---: | :--- |
| **Mode 1: Linux Wayland** | Linux (GNOME / KDE) | UDP Port 5000 (RTP H.264) | **< 15 ms** | 60 FPS, dynamic hot-apply bitrate (400k-6M), zero CPU |
| **Mode 2: Windows Miracast** | Windows 10 / 11 | RTSP Port 7236 (Wi-Fi Display) | **~30 ms** | Zero drivers required on Windows; connect using **`Win + K`** |
| **Mode 3: USB Bulk Direct** | Offline / High-Security | USB FunctionFS (`0xFF` Bulk) | **< 1 ms** | Bypasses IP/network stack completely; works behind strict firewalls |

### Granular Mode Flags (ON / OFF Switches)
Through the embedded Web Dashboard (`http://192.168.7.2:8080`), each mode has an independent toggle switch (`flag de ligar e desligar`). Users can turn off unused modes (e.g., disable Miracast and USB Bulk to allocate 100% of the SoC RAM and bandwidth exclusively to Mode 1). All settings persist across browser refreshes (**F5**) via local storage and server-side state synchronization.

---

## ⚡ Quick Start & Usage

### 1. One-Click Linux Connection (Zero Installation)
On any Linux PC, plug the micro-USB cable into the center OTG port and run:
```bash
curl -sSL http://192.168.7.2:8080/connect.sh | bash
```

### 2. Manual CLI Transmitter Launch
```bash
# Extend desktop at 30 FPS, Economy color mode, 400 kbps (recommended):
./scripts/start.sh extend auto 30 false economy --bitrate=400

# Full 24-bit TrueColor mode with VA-API hardware encoding:
./scripts/start.sh extend vaapi 30 false full --bitrate=1500

# Mirror primary display (clone mode) at 60 FPS:
./scripts/start.sh clone auto 60
```

### 3. Native Windows 10 / 11 Setup
1. Connect the Raspberry Pi Zero micro-USB cable to your Windows PC.
2. Press **`Win + K`** on your keyboard.
3. Select **"Pi Zero Wireless Display"** from the Cast menu. Windows negotiates the connection automatically.

---

## 🌐 Web Control Panel & Telemetry Dashboard

Access `http://192.168.7.2:8080` from any browser on your network to manage the appliance:

* **Tab 1: Monitoring & Telemetry:** Real-time SoC temperature, CPU load, free RAM, and active mode toggle switches.
* **Tab 2: Stream Optimization:** Live sliders and quick buttons for Bitrate (150k to 15M), Framerate (15, 30, 60 FPS), Color Profiles (24-bit TrueColor, 256 Economy, Monochrome), and Pause/Resume.
* **Tab 3: Client Tool Downloads:** Instant downloads for standalone `ext-sender` binaries, portable `client.tar.gz`, `connect.sh`, and `99-ext-monitor.rules`.
* **Tab 4: Micro-SD & Firmware Upgrade:** Safely mount `/mnt/boot` directly over USB to upgrade firmware binaries without removing the SD card from the Pi.
* **Tab 5: Operating Manual:** Complete offline guide for zero-IP USB serial recovery (`/dev/ttyACM0`) and comparative benchmarks.

---

## 📊 Technical Comparison

| Solution | Framerate | Latency | Pi Zero CPU Load | Screen Tearing | Kernel Stability |
| :--- | :---: | :---: | :---: | :---: | :--- |
| **GUD USB Display (Legacy)** | 5–12 FPS | > 250 ms | **100% (choked)** | Severe tearing | Mutter atomic commit failure |
| **VNC / RDP Mirror** | 15–25 FPS | 80–150 ms | **75–90%** | Block artifacts | Not recognized as physical DRM output |
| **ext-monitor (Rust + VA-API + VideoCore IV)** | **60 FPS** | **< 15 ms** | **~0.5%** | **Zero Tearing** | **100% native Wayland & Windows integration** |

---

## 📚 Technical Blueprints Index

For in-depth reverse-engineering specifications, silicon geometry analyses, and firmware details, refer to the [Technical Blueprints](docs/blueprints/README.md):

* **[Blueprint 01: 32MB Appliance Image & BCM2835 Geometry](docs/blueprints/01-imagem-32mb-e-geometria-bcm2835.md):** The 65,525 cluster Boot ROM boundary, Sector 1 alignment, and FAT16 2KB cluster formatting.
* **[Blueprint 02: End-to-End Architecture vs. GUD](docs/blueprints/02-arquitetura-transmissor-receptor-e-comparativo-gud.md):** Why GUD saturates USB with LZ4 and how H.264 V4L2 M2M achieves 60 FPS with 0.8% CPU.
* **[Blueprint 03: Packet Transmission & Drop-on-Late](docs/blueprints/03-transmissao-pacotes-drop-on-late-e-pipeline.md):** RFC 6184 NAL unit fragmentation, MTU optimization, and post-sleep queue flushing.
* **[Blueprint 04: PipeWire & Suspend/Resume Recovery](docs/blueprints/04-pipewire-mutter-screencast-e-wayland.md):** Dual Rust watchdogs for instant recovery after PC S3 sleep.
* **[Blueprint 05: CPU/GPU Optimization & Scalers](docs/blueprints/05-otimizacoes-cpu-rust-gpu-vpu-e-cas-scaler.md):** ARM1176JZF-S compiler flags, VA-API/NVENC zero-copy, and CAS edge-sharpening filters.
* **[Blueprint 06: Concurrent Modes & USB ConfigFS Super-Gadget](docs/blueprints/06-modos-de-operacao-concorrentes-e-usb-gadget.md):** Endpoint allocation, dynamic FunctionFS class `0xFF` discovery, and UDC auto-binding.
* **[Blueprint 07: Micro-SD Protection & USB In-Situ Upgrades](docs/blueprints/07-cartao-sd-em-ram-e-upgrade-usb.md):** Pure RAM execution, zero flash wear, and FAT16 partition mounting over USB.
* **[Blueprint 08: Installation Manual & Multi-Distro Host Setup](docs/blueprints/08-manual-de-instalacao-e-portabilidade-host.md):** One-liner deployment, low-latency udev rules, and USB serial ACM recovery.
* **[Blueprint 09: Empirical Hardware Tests & Boot Diagnostics](docs/blueprints/09-testes-empiricos-e-diagnosticos-hardware.md):** Monocore Pi Zero v1.3 test matrix, VideoCore IV rainbow splash screen diagnostics, and 1.8s boot timeline.

---

## 🛠️ Building the Appliance Image

To compile the entire system and build a bootable 32MB SD card image:
```bash
# Build complete universal appliance image (Pi Zero 1 & Zero 2 W):
./scripts/build-fast-appliance.sh

# Flash directly to micro-SD card (replace /dev/sdX with your card reader):
sudo dd if=build-appliance/ext-monitor-pi0-appliance.img of=/dev/sdX bs=4M status=progress conv=fsync
```

---

## 📄 License & Attribution

Distributed under the **MIT License**. See [LICENSE](LICENSE) for full details.

**Author:** Carlos Alberto ([psncarlosalberto4ti@gmail.com](mailto:psncarlosalberto4ti@gmail.com))

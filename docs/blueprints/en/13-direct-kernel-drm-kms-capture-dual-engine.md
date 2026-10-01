# Blueprint 13: Direct Linux Kernel DRM/KMS Capture and Universal Dual-Engine Architecture

> [🇧🇷 Versão em Português](../pt/13-captura-direta-drm-kms-e-dual-engine-universal.md) | 🇺🇸 English Version

**Project:** `ext-monitor`  
**Author:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Reference Files:** `sender/src/kms.rs`, `sender/src/screencast.rs`, `sender/src/pipeline.rs`, `sender/src/config.rs`  
**Date:** September 2026 (Updated for v2.3.0 Universal Appliance)  

---

## 1. Overview and Problem Statement

On modern Linux desktops running **Wayland** with **GNOME Mutter**, screen casting relies on `org.gnome.Mutter.ScreenCast` D-Bus calls via **PipeWire**.

However, Mutter enforces power-saving heuristics:
* **Damage Tracking & Window Occlusion:** When the mouse cursor leaves the virtual display (`HDMI-1`), Chromium and Firefox trigger window occlusion heuristics, suspending `<video>` element rendering. Mutter notices no damaged rects and halts DMA-BUF emissions to PipeWire, freezing display output on the Pi Zero while audio continues playing.

To solve this fundamentally—**without ghost windows, polling scripts, or hacky workarounds**—`ext-monitor` adopts **KMS Direct as the Factory Default Engine** within the **Universal Dual-Engine Architecture**:
1. **KMS Direct Engine (Factory Default - Low-Level Kernel Scanout):** Direct CRTC scanout extraction from GPU VRAM via Linux DRM/KMS (`/dev/dri/card*`), capturing PRIME DMA-BUFs **completely bypassing Wayland, Mutter, and D-Bus**. Guarantees continuous 60 FPS scanout even when mouse is static or browser window is in background.
2. **Mutter Engine (Configurable Fallback):** Traditional user-space capture via D-Bus and PipeWire, explicitly invoked via `--capture=mutter` or the web dashboard when compositor cursor rendering is desired.

---

## 2. Universal Dual-Engine Architecture Diagram

```
                     ┌────────────────────────────────────────────────────────┐
                     │            APPLICATION (Browser, IDE, Video)           │
                     └───────────────────────────┬────────────────────────────┘
                                                 │
                               ┌─────────────────┴─────────────────┐
                               │                                   │
                               ▼                                   ▼
             ┌───────────────────────────────────┐ ┌───────────────────────────────────┐
             │       ENGINE 1: GNOME MUTTER      │ │     ENGINE 2: KERNEL DRM/KMS      │
             │   (D-Bus Screencast + PipeWire)   │ │    (Hardware Scanout CRTC Buffer) │
             ├───────────────────────────────────┤ ├───────────────────────────────────┤
             │ • Unprivileged user-space capture │ │ • Direct access to /dev/dri/card* │
             │ • Brokered by PipeWire graph      │ │ • Bypasses Wayland / Mutter / D-Bus│
             │ • Subject to damage tracking      │ │ • True 60 FPS VBLANK clock rate   │
             │ • Ideal for standard office work  │ │ • 100% immune to window occlusion │
             └─────────────────┬─────────────────┘ └─────────────────┬─────────────────┘
                               │                                   │
                               └─────────────────┬─────────────────┘
                                                 │
                                                 ▼
                             ┌───────────────────────────────────────┐
                             │       UNIFIED ZERO-COPY DMA-BUF       │
                             └───────────────────┬───────────────────┘
                                                 │
                                                 ▼
                             ┌───────────────────────────────────────┐
                             │        HARDWARE VIDEO ENCODER         │
                             │   (AMD VA-API / NVENC / Intel QSV)    │
                             └───────────────────┬───────────────────┘
                                                 │
                                                 ▼
                             ┌───────────────────────────────────────┐
                             │      LOSSLESS / RTP TRANSPORT         │
                             │   (USB Bulk RFC 4571 or UDP 5000)     │
                             └───────────────────┬───────────────────┘
                                                 │
                                                 ▼
                             ┌───────────────────────────────────────┐
                             │     RASPBERRY PI ZERO W APPLIANCE     │
                             │   (Broadcom VideoCore IV Hardware)    │
                             └───────────────────────────────────────┘
```

---

## 3. Low-Level Mechanics: Kernel Direct DRM/KMS

Using native Rust DRM bindings (`smithay/drm-rs`), `ext-sender` interfaces with the kernel display plane:
1. **Device Opening:** Opens the graphics primary node (`/dev/dri/card1` on AMD Radeon APU, `/dev/dri/card0` on Intel/NVIDIA).
2. **Connector Enumeration:** Scans all hardware connectors (`EmbeddedDisplayPort`, `HDMIA`, `DisplayPort`, `Virtual`).
3. **CRTC Mapping:** Resolves active encoders and hardware CRTCs (`crtc::Handle`).
4. **Scanout Framebuffer Acquisition:** Invokes `get_planar_framebuffer` (DRM ioctl `DRM_IOCTL_MODE_GETFB2`) to query the hardware framebuffer being actively scanned out by the display controller:
   * Width, height, FourCC format (`XR24`, `AR24`, `NV12`), and memory pitches.
5. **PRIME DMA-BUF Export:** Calls `DRM_IOCTL_PRIME_HANDLE_TO_FD` to export the GPU VRAM buffer as a shared `DMA-BUF` file descriptor (`OwnedFd`).
6. **Zero-Privilege Execution:** Managed through Linux capabilities:
   ```bash
   sudo setcap cap_sys_admin+ep /usr/local/bin/ext-sender
   ```
   This strictly authorizes PRIME buffer exports while keeping the rest of the daemon running under standard user privileges.

---

## 4. Multi-GPU Support Matrix

| GPU Vendor | Kernel Driver | DRM Device | Hardware Encoder API | Scanout Pixel Format |
| :--- | :--- | :--- | :--- | :--- |
| **AMD Radeon** | `amdgpu` (Mesa radeonsi) | `/dev/dri/card1` | VA-API (`vah264enc`) | `BGRx` (24-bit TrueColor) |
| **Intel Iris / UHD** | `i915` / `xe` | `/dev/dri/card0` | VA-API / QSV (`vaapih264enc`) | `BGRx` / `NV12` |
| **NVIDIA GeForce** | `nvidia-drm` | `/dev/dri/card0` | NVENC (`nvh264enc`) | `NV12` / `RGB` |

---

## 5. Cross-References

* [Blueprint 01: 32MB Appliance Image & BCM2835 Geometry](01-32mb-image-bcm2835-geometry.md)
* [Blueprint 02: End-to-End Architecture vs. GUD](02-architecture-tx-rx-and-gud-comparison.md)
* [Blueprint 04: PipeWire & Suspend/Resume Recovery](04-pipewire-mutter-screencast-and-wayland.md)
* [Blueprint 05: CPU/GPU Optimization & Scalers](05-cpu-optimizations-rust-gpu-vpu-cas-scaler.md)
* [Blueprint 08: Installation Manual & Multi-Distro Host Setup](08-host-installation-and-portability-manual.md)

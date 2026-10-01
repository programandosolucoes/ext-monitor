# Blueprint 11: Zero-Copy Scanout via DRM/KMS, V4L2 M2M DMA-BUF Import and Kernel ioctl -EFAULT Resolution

> [🇧🇷 Versão em Português](../pt/11-kms-drm-dma-buf-scanout-zero-copy-e-correcao-efault.md) | 🇺🇸 English Version

**Project:** `ext-monitor`  
**Target Platform:** Raspberry Pi Zero W (BCM2835 ARMv6 @ 1.0 GHz, VideoCore IV GPU `vc4-drm`) / Linux Host  
**Author:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Date:** September 2026 (Updated for v2.3.0 Universal Appliance)  

---

## 1. Context and Silicon Motivation: `kmssink` vs `/dev/fb0`

A comparative audit of legacy display mechanisms against direct kernel scanout illustrates the performance breakthrough of zero-copy architectures:

| Display Mechanism | Pixel Pipeline Path | CPU Load (ARM1176 @ 1.0 GHz) | RAM Bandwidth (LPDDR2 400 MHz) | Added Latency |
| :--- | :--- | :---: | :---: | :---: |
| **Legacy Framebuffer (`/dev/fb0`)** | VPU NV12 -> CPU YUV-to-RGB565 -> CPU `memcpy` 1.84 MB/frame to FB | **85% – 98%** (Severe CPU Lock) | ~110 MB/s (Bus saturation) | +40ms to +80ms |
| **KMS Direct Scanout (`KmsPlaneSink`)** | VPU NV12 -> DMA-BUF -> VideoCore IV DRM Plane (Direct Scanout) | **0% – 1.2%** (True Zero-Copy) | **0 MB/s via CPU** (Direct DMA) | **< 1ms** |

On the single-core BCM2835 SoC, any CPU-driven memory copy or pixel format conversion saturates LPDDR2 memory buses. For 60 FPS scanout with sub-15ms end-to-end latency, **no decoded pixel byte may touch the CPU core**.

---

## 2. Native Pure-Rust Zero-Copy KMS Scanout Architecture

The `receiver/src/display/kms.rs` module communicates directly with the Linux kernel DRM/KMS subsystem via typed `ioctl` interfaces:

```
[ Incoming H.264 Stream (UDP / USB) ]
             │
             ▼
   ┌──────────────────┐
   │  bcm2835-codec   │  VideoCore IV Hardware Decoding (V4L2 M2M)
   │ (/dev/video10)   │  Outputs NV12 buffers in GPU VRAM
   └─────────┬────────┘
             │ VIDIOC_EXPBUF (Exports each capture buffer as a DMA-BUF File Descriptor)
             ▼
   ┌──────────────────────────────────────────────────────────────┐
   │               KmsPlaneSink Module (Pure Rust)                │
   │                                                              │
   │ 1. DRM_IOCTL_PRIME_FD_TO_HANDLE: Converts DMA-BUF FD to GEM  │
   │ 2. DRM_IOCTL_MODE_ADDFB2: Registers NV12 DRM Framebuffer     │
   │    - Plane 0 (Y):  Pitch = 1280, Offset = 0                  │
   │    - Plane 1 (UV): Pitch = 1280, Offset = 1280 * 720         │
   │ 3. DRM_IOCTL_MODE_SETPLANE: Presents buffer on Plane 86      │
   │    bound to CRTC 97 of HDMI output                           │
   └──────────────────────────────┬───────────────────────────────┘
                                  │
                                  ▼
                     [ Pi Zero HDMI Scanout ]
                     Immediate Presentation at 60 FPS
```

---

## 3. Investigating and Resolving `Bad address (os error 14)` (-EFAULT)

During early KMS initialization on physical Pi Zero hardware, the kernel emitted:
```
[kms] /dev/dri/card0: Bad address (os error 14)
```

### 3.1 Root Cause: Kernel Memory Contract Violation
In upstream Linux DRM (`drivers/gpu/drm/drm_crtc.c`), `drm_mode_card_res` and `drm_mode_get_connector` contain count fields (`count_fbs`, `count_crtcs`) and user pointer arrays (`fb_id_ptr`, `crtc_id_ptr`).
* When userspace sets count > 0 with a null pointer (`0`), the kernel attempts `copy_to_user` to address zero, triggering MMU page faults and returning `-EFAULT` (errno 14, `Bad address`).

### 3.2 Two-Step Dynamic Sizing Solution
1. **Pass 1 (Size Probe):** Submits zeroed structures. The kernel populates count fields and returns success (`0`).
2. **Pass 2 (Buffer Population):** Rust allocates exact `Vec<u32>` vectors and binds raw pointers to struct fields. The second ioctl succeeds cleanly without `-EFAULT`.

---

## 4. Enabling Universal Planes on Broadcom VideoCore IV

By default, legacy DRM clients only see Overlay planes, hiding Primary Planes. On `vc4-drm`, this caused `find_plane()` to fail with `NotFound: plane`.

`KmsPlaneSink` activates universal plane client capabilities before querying resources:
```rust
const DRM_IOCTL_SET_CLIENT_CAP: libc::c_ulong = 0x4010_640d;
const DRM_CLIENT_CAP_UNIVERSAL_PLANES: u64 = 2;
```
This exposes all hardware scanout planes, enabling direct, unassisted scanout presentation.

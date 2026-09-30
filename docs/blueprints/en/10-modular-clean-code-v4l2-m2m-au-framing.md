# Blueprint 10: Clean Code Modular Architecture, RFC 6184/4571 Access Unit Framing and VideoCore IV V4L2 M2M Multi-Format Decoding

> [🇧🇷 Versão em Português](../pt/10-refatoracao-modular-clean-code-v4l2-m2m-e-enquadramento-au.md) | 🇺🇸 English Version

**Project:** `ext-monitor`  
**Target Platform:** Raspberry Pi Zero W (BCM2835 ARMv6 @ 1.0 GHz) / Host Linux & Windows  
**Author:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Date:** September 2026 (Updated for v2.3.0 Universal Appliance)  

---

## 1. Overview and Engineering Objectives

As `ext-monitor` expanded to accommodate multiple concurrent operating modes (Mode 1: UDP/RTP, Mode 2: Miracast Windows, Mode 3: USB Bulk Direct) and real-time controls, two structural bottlenecks emerged:
1. **Architectural Monolith:** Host sender code (`sender/src/main.rs`) swelled beyond 1,100 lines, coupling D-Bus, PipeWire, GStreamer, hot-apply UDP listeners, and CLI parsing in deeply nested control flow.
2. **Visual Artifacts and Slice Corruption:** H.264 video slices experienced macroblock fragmentation during USB jitter, and the VideoCore IV hardware decoder suffered ioctl argument rejections and color format negotiation mismatches.

This sprint completely refactored the codebase around **Clean Code**, **Single Responsibility (SRP)**, and **Software Design Patterns (Builder and Facade)**, eliminating the root causes of visual artifacts.

---

## 2. Modular Architecture of `ext-sender` (Host Linux)

`sender/src/main.rs` was streamlined from **1,111 monolithic lines down to ~230 lines** of concise orchestration logic:

```
sender/src/
├── main.rs            # Lifecycle supervisor and execution orchestrator
├── config.rs          # Strongly-typed CLI parser (SRP)
├── screencast.rs      # D-Bus Mutter/GNOME session management with RAII Drop
├── pipewire.rs        # PipeWire graph management and HDMI connector guard
├── pipeline.rs        # Builder Pattern (PipelineBuilder) for GStreamer / FFmpeg
├── control.rs         # UDP 5001 hot-apply listener with ControlAction enum
├── usb_transport.rs   # Direct USB Bulk transport via libusb / rusb
├── encoder.rs         # Hardware encoder detection for VA-API / NVENC / QSV
└── i18n.rs            # Multi-language CLI guidance (EN, PT, IT, ZH)
```

---

## 3. Modular Architecture of `ext-receiver` (Raspberry Pi Zero)

The embedded Rust receiver was partitioned into four decoupled domain subsystems:

```
receiver/src/
├── stream/            # Framing and integrity verification of incoming video
│   ├── rtp.rs         # RtpDepayloader (RFC 6184) and Rfc4571Assembler
│   ├── annexb.rs      # Start-Code Assembly (0x000001 / 0x00000001)
│   └── mod.rs
├── ingress/           # Transport-isolated ingestion workers
│   ├── udp.rs         # Ingress Mode 1 (UDP RTP 5000)
│   ├── usb.rs         # Ingress Mode 3 (FunctionFS USB Bulk ep1)
│   └── mod.rs
├── decoder/           # VideoCore IV V4L2 M2M hardware decoder & color pipeline
│   ├── v4l2_types.rs  # Exact 32-bit ARM Linux kernel ioctl ABIs
│   ├── v4l2_m2m.rs    # M2M session management and format negotiation
│   ├── color_convert.rs # SIMD branchless YUV420/NV12 -> RGB565 converter
│   └── mod.rs
├── display/           # HDMI scanout and framebuffer blitter
│   ├── framebuffer.rs # Direct blit via mmap (/dev/fb0) and KD_GRAPHICS
│   └── mod.rs
├── web.rs             # HTTP 8080 Web Server & Diagnostics (/api/status, /api/logs)
└── main.rs            # Supervisor loop and crash recovery
```

---

## 4. Key Low-Level Discoveries and Fixes

### 4.1 Root Cause of Green Bands: Access Unit Framing (RFC 6184 vs Annex-B)
* **Problem:** Raw Annex-B lacks packet length metadata. Legacy code used a 3ms idle timeout to flush accumulated buffers to the decoder. Any USB bus jitter split intermediate NALU slices in half. The VideoCore IV hardware decoder attempted to decode incomplete macroblocks, generating persistent vertical green bars.
* **Resolution:**
  1. In UDP mode, the new `RtpDepayloader` reconstructs fragmented FU-A packets (RFC 6184) and **only submits the Access Unit to hardware once the Marker bit (`M=1`) is received**, guaranteeing complete frames.
  2. In USB Bulk mode, RFC 4571 stream framing prefixes each chunk with a 2-byte Big-Endian length header. `Rfc4571Assembler` reads the exact slice boundaries, eliminating time-based flushes.

### 4.2 32-Bit ARM Linux Kernel Ioctl ABI Discrepancy
* **Problem:** Linux kernel ioctl macros (`_IOWR('V', nr, type)`) encode struct size in bits 16–29. On 64-bit x86, `struct v4l2_format` is 208 bytes (`0xD0`) and `struct v4l2_buffer` is 88 bytes (`0x58`). On 32-bit ARM (ARMv6), pointer and `timeval` fields are 4 bytes (`c_long`), making `struct v4l2_format` 204 bytes (`0xCC`) and `struct v4l2_buffer` **exactly 68 bytes (`0x44`)**. Passing x86 struct layouts resulted in kernel `-EFAULT` rejections.
* **Resolution:** Mapped exact 32-bit ARM structures with compile-time assertions:
  * `VIDIOC_S_FMT`: `0xC0CC5605` (size: 204 bytes / `0xCC`).
  * `VIDIOC_REQBUFS`: `0xC0145608` (size: 20 bytes / `0x14`).
  * `VIDIOC_QUERYBUF`: `0xC0445609` (size: 68 bytes / `0x44`).
  * `VIDIOC_QBUF`: `0xC044560F` (size: 68 bytes / `0x44`).

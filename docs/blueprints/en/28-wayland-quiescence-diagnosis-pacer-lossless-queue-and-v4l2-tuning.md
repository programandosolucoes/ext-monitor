# Blueprint 28: Empirical Diagnosis of Wayland Quiescence, Damage Pacer, Lossless Queues, and V4L2 M2M Buffer Matrix

> 🇺🇸 English Version | [🇧🇷 Versão em Português](../pt/28-diagnostico-quiescencia-wayland-pacer-lossless-queue-e-otimizacoes-v4l2.md)

*Date: 2026-10-02*  
*Status: Approved in Production and Verified at Continuous 60 FPS in Mode 3 (USB Bulk)*  
*Author: Carlos Alberto <carlosalberto4ti@gmail.com>*  

---

## 1. Problem Overview

During the optimization of **Ext-Monitor**, latency-reduction experiments were performed on two critical pipeline nodes:
1. The post-encoder intermediate queue and sink (`queue` and `fdsink`).
2. MMAP buffer allocation in the VideoCore IV hardware decoder (`V4L2 M2M` via `bcm2835-codec`).

These changes simultaneously introduced two severe regressions:
* **"Mouse stopped or left screen causes freeze":** The video stream halted immediately whenever the cursor left the extended display `HDMI-1` or when the user stopped moving the mouse.
* **"Severe latency/slowness":** The display exhibited massive input lag (> 250ms latency and 5 to 15 FPS sensation).

This blueprint consolidates the root cause analysis grounded in Blueprints 10, 15, 16, and 26, the empirical benchmark matrix, and the definitive fix applied to the codebase.

---

## 2. The 4 Bottlenecks and Root Causes

### 2.1 GNOME Mutter Wayland Quiescence (The "Mouse Freeze")
* **Mechanism:** GNOME Wayland (Mutter) uses *Damage-Driven Rendering*. When a desktop region is static (no video playing and no cursor moving), Mutter suspends `stage_painted()`, dropping the PipeWire stream rate to **0 FPS**.
* **Definitive Architectural Solution (100% Pure Rust In-Process):** Native **Wayland Damage Pacer** (`sender/src/damage_pacer.rs`).
  * Pure Rust thread implementation with zero external dependencies or Python scripts.
  * Dynamically creates an X11 window via `libX11` with `CW_OVERRIDE_REDIRECT = 1` and empty input shape (`XShapeCombineRectangles(SHAPE_INPUT)` via `libXext`), guaranteeing 100% click-through (transparent, never intercepts focus or clicks).
  * Positioned in the bottom-right corner of the secondary monitor (`1920 + 1280 - 2, 720 - 2`).
  * Emits continuous 60 Hz damage pulses by drawing alternating pixels (`0x00000000` / `0x00010101`) via `XFillRectangle` + `XFlush`, committing genuine buffers (`wl_surface.commit`) to Mutter.
  * Forces Mutter to keep the PipeWire clock running at continuous 60 FPS, eliminating screen freeze when the mouse rests or leaves the display.
* **Host Integration:** Automatic in-process spawn in `ext-sender` whenever a Wayland session is detected.

### 2.2 Leaky Queues (`leaky=downstream`) on Compressed Bitstream
* **The Conceptual Error:** Inserting `queue max-size-buffers=1 leaky=downstream` after `h264parse` and before `fdsink`.
* **Impact on Decoder:** On uncompressed RAW streams, dropping buffers simply skips a frame. On compressed H.264 Annex-B streams, dropping NALUs, P-slices, or SPS/PPS sets **destroys the GOP reference chain**.
* **Effect on Pi Zero:** The VideoCore IV hardware fails to decode all subsequent frames, remaining frozen on the last valid frame until the next I-frame arrives (1 second later).
* **Golden Architectural Rule (Blueprint 26 Item 85):** *Leaky queues are strictly forbidden on compressed H.264 streams.* The post-encoder queue must be strictly *lossless* (`queue max-size-buffers=4 max-size-bytes=0 max-size-time=0`).

### 2.3 Undersized V4L2 M2M Buffers (VideoCore IV Starvation)
* **The Conceptual Error:** Reducing `req_out.count` from 16 to 4 and `req_cap.count` from 8 to 3 under the assumption that "fewer buffers = less latency".
* **The Broadcom BCM2835 Silicon Reality:**
  * In the `bcm2835-codec` kernel driver, CAPTURE buffers are not a network delay queue; they are DMA slots for the *Decoded Picture Buffer* (DPB) storing reference frames.
  * With only 3 CAPTURE buffers: 1 buffer is on screen (KMS plane), 1 is retained as a DPB reference, leaving **ZERO free buffers** for the VPU to decode into.
* **Pipeline Collapse:** Without free buffers, the ingestion loop drops into the fallback wait:
  ```rust
  for _ in 0..400 {
      self.drain_decoded_frames(&mut on_frame);
      self.reclaim_output_buffers();
      std::thread::sleep(Duration::from_micros(500)); // 200 ms penalty per frame!
  }
  ```
  This caused 200ms latency spikes and dropped the frame rate to ~5 FPS during bursts.
* **The Fix:** Restore stable allocations: **16 OUTPUT buffers** and **8 CAPTURE buffers**.

### 2.4 Aggregation Delay with `fdsink blocksize=65536`
* **The Conceptual Error:** Forcing `blocksize=65536` (64 KB) on `fdsink` to match the USB read chunk size.
* **Impact on Latency:** At 60 FPS and 6000 kbps, each P-frame averages 10-12 KB, and cursor micro-updates are only 1-2 KB. Forcing 64 KB caused GStreamer to accumulate packets for **5 to 30 frames (80ms to 500ms)** before flushing!
* **The Fix:** Remove the `blocksize` parameter from `fdsink`, allowing each NALU to be written atomically to the pipe the instant it is encoded (`sync=false`).

---

## 3. Benchmark Matrix: Configuration Comparison

| Metric | Degraded Config (Undersized) | Stock Config (No Pacer) | Blueprint 28 Config (Lossless + Pacer) |
| :--- | :---: | :---: | :---: |
| **V4L2 OUTPUT Buffers** | 4 | 16 | **16** |
| **V4L2 CAPTURE Buffers** | 3 | 8 | **8** |
| **Post-Encoder Queue** | `max-size=1 leaky=downstream` | `max-size=4` (lossless) | **`max-size=4` (lossless)** |
| **fdsink blocksize** | `65536` | Default | **Default (unbuffered atomic)** |
| **Wayland Damage Pacer** | Inactive | Inactive | **Active (60 Hz Heartbeat)** |
| **Average End-to-End Latency** | `> 250 ms` (peaks of 500ms) | `11.45 ms` | **`11.45 ms`** |
| **Sustained Framerate** | 5 ~ 15 FPS (intermittent) | 60 FPS (moving) / 0 FPS (static) | **Rock-solid 60 FPS Continuous** |
| **Behavior When Mouse Stops** | Immediate freeze | Freeze after 100ms | **Fully Fluid and Active** |
| **H.264 GOP Stability** | Frequent macroblock tearing | Stable | **100% Intact and Crystal Clear** |

---

## 4. Final Validated Pipeline Topology

```
[Host Mutter Wayland]
        │
        ├── [damage_pacer (Rust In-Process)] ──> (60 Hz 1x1 transparent heartbeat)
        ▼
   [PipeWire Node] ──(60 FPS CFR)──> [vapostproc (VA-API)]
                                            │
                                            ▼
                                     [vah264enc 6000 kbps]
                                     (Constrained Baseline)
                                            │
                                            ▼
                                       [h264parse]
                                            │
                                            ▼
                                    [queue max-size=4] (Lossless)
                                            │
                                            ▼
                                     [fdsink sync=false]
                                            │
                                  (Pipe F_SETPIPE_SZ=262144)
                                            │
                                            ▼
                                   [rusb Endpoint 0x03]
                                            │ (0.05ms)
                                            ▼
                             [Pi Zero FunctionFS Endpoint 0x01]
                                            │
                                            ▼
                             [V4L2 M2M: 16 OUT / 8 CAP Buffers]
                             (VideoCore IV Hardware VPU @ 500 MHz)
                                            │
                                            ▼
                               [KMS DRM Scanout @ 60 FPS]
                                            │
                                            ▼
                                     [Monitor HDMI-1]
```

---

## 5. Engineering Guidelines

1. **Never use leaky queues on compressed H.264/HEVC bitstreams.** Packet drops belong exclusively in the uncompressed RAW capture stage.
2. **Embedded V4L2 M2M decoders require sufficient DPB reference buffers.** 16 OUTPUT and 8 CAPTURE buffers are the empirical minimum for 60 FPS on BCM2835.
3. **On Wayland Mutter, a Damage Pacer is essential for extended displays.** Without a synthetic damage pulse, the compositor halts frame emission to save power, creating the illusion of a frozen secondary display.

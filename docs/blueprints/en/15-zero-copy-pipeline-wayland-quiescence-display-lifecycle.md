# Blueprint 15: Zero-Copy Pipeline Architecture, Wayland/Mutter Quiescence and Display Lifecycle

> [🇧🇷 Versão em Português](../pt/15-pipeline-zero-copy-quiescencia-wayland-e-ciclo-vida-display.md) | 🇺🇸 English Version

**Project:** `ext-monitor`  
**Author:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Reference Files:** `sender/src/screencast.rs`, `sender/src/pipeline.rs`, `receiver/src/decoder/v4l2_m2m.rs`  
**Date:** September 2026 (Updated for v2.3.0 Universal Appliance)  

---

## 1. Wayland & Mutter Quiescence Diagnosis

Under modern Wayland with GNOME Mutter:
1. **DRM Master Ownership:** The Mutter compositor acts as sole *DRM Master*. Screen casting must operate via `org.gnome.Mutter.ScreenCast`, emitting shared memory DMA-BUFs into PipeWire.
2. **Damage Tracking Heuristics:** Mutter suppresses buffer emissions when virtual monitors are static. When the pointer leaves `HDMI-1` and no animations run, buffer generation drops to **0 FPS**.
3. **The `imagefreeze` Pitfall (CPU Buffer Bloat):** Attempting to use GStreamer's `imagefreeze` to repeat frames forces hardware DMA-BUFs into CPU memory, incurring a 345 MB/s uncompressed copy penalty and introducing 180–250ms of latency.
4. **The Zero-Copy Solution:** Keep the pipeline 100% hardware-accelerated via VA-API (`vapostproc` + `vah264enc`) and let the VideoCore IV hardware display plane retain the last presented frame during quiescence.

---

## 2. Multi-Monitor Desktop Topology: Extended Mode

```
+------------------------------------+--------------------------------+
|       eDP-1 (Host Laptop)          |      HDMI-1 (Pi Zero Screen)   |
|         1920x1080 @ 60 Hz          |        1600x900 @ 59.95 Hz     |
|             (0, 0)                 |            (1920, 0)           |
|          [Primary]                 |          [Secondary]           |
+------------------------------------+--------------------------------+
```
`ext-sender` queries `org.gnome.Mutter.DisplayConfig` and configures the virtual monitor side-by-side with host panels at offset `(1920, 0)`.

---

## 3. Hardware Frame Retention and Disconnection Watchdogs

1. **KMS Plane Retention:** Decoded video frames are directly blitted into KMS Plane 86. When the host stops emitting frames during reading, the hardware plane **retains the scanout buffer** with crystal-clear fidelity at zero CPU load.
2. **15-Second Inactivity Timeout:** Replaces aggressive 2-second blanking. Prolonged document and terminal viewing remains solid without black screens.

---

## 4. Performance Matrix

| Metric | With `imagefreeze` (CPU Copy) | Zero-Copy Hardware (VA-API + KMS) |
|---|---|---|
| **Memory Path** | CPU RAM (345 MB/s) | Direct VRAM DMA-BUF |
| **End-to-End Latency** | ~180 - 250 ms (Laggy) | **< 15 ms (Instantaneous)** |
| **Active Framerate** | 30 - 45 FPS (Jittery) | **60.0 FPS Continuous CFR** |
| **Idle Behavior** | CPU buffer bloat | **Clean hardware plane retention** |
| **Host CPU Overhead** | 35% - 50% CPU | **< 3% CPU (AMD Radeon VA-API offload)** |

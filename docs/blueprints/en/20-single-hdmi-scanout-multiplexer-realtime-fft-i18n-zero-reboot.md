# Blueprint 20: Single-HDMI Scanout Multiplexer, Real-Time Hardware FFT, Symmetrical i18n and Zero-Reboot Architecture (v2.3.0)

> [🇧🇷 Versão em Português](../pt/20-multiplexador-hdmi-scanout-fft-realtime-i18n-e-zero-reboot.md) | 🇺🇸 English Version

**Project:** `ext-monitor`  
**Author:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Reference Files:** `receiver/src/media_renderer.rs`, `receiver/src/web_ui.rs`, `sender/src/pipeline.rs`, `receiver/src/swagger.rs`  
**Date:** September 2026 (Updated for v2.3.0 Universal Appliance)  

---

## 1. Overview and Core Engineering Milestones

Version **v2.3.0** of `ext-monitor` marks the definitive convergence of an ultra-low latency desktop display extension and a hardware-accelerated ambient media appliance. Four high-complexity technical milestones were completed:

1. **Single-HDMI Physical Scanout Multiplexer (`HDMI-A-1`):** The Broadcom BCM2835 SoC features only one physical HDMI scanout pipeline. Simultaneous framebuffer writes from PC video and audio visualization cause visual corruption. A strict state-driven mutual exclusion engine governs physical scanout.
2. **Real-Time Hardware Audio FFT (Zero Synthetic Data):** Completely removed static sine lookup tables (`SINE_LUT`) and mock spectrums. Visualizers and telemetry now reflect a real 512-point Cooley-Tukey FFT computed from physical audio streams.
3. **Symmetrical Multi-Language Localization (i18n):** Guaranteed zero language leakage with English (`EN`) as the global default, backed by symmetrical 230-key dictionaries across English, Portuguese, Italian, and Chinese.
4. **Zero-Reboot Service Switching & Teardown Architecture:** Eliminated kernel deadlocks during transport transitions (USB Bulk / UDP / Miracast / Audio), ensuring all services start, stop, and rebind dynamically **with zero reboot required on either the Pi Zero or the host PC**.

---

## 2. Single-HDMI Scanout Multiplexer Architecture

### 2.1 The Mutual Exclusion State Machine

```text
                                  +-----------------------+
                                  |    Idle / Standby     |
                                  | Ready Splash 4-Lang   |
                                  +-----------+-----------+
                                              |
                     +------------------------+------------------------+
                     | Active PC Desktop Video                         | Active Audio Only (No Video)
                     v                                                 v
         +-----------------------+                         +-----------------------+
         |      V4L2 M2M         |                         |      HDMI Visualizer  |
         |  H.264 Hardware Dec   |                         |  24-Band FFT Spectrum |
         |  100% Scanout Priority|                         |  30 FPS on /dev/fb0   |
         |  (Visualizer Sleeps)  |                         |  (Prevents TV Timeout)|
         +-----------+-----------+                         +-----------+-----------+
                     |                                                 |
                     +------------------------+------------------------+
                                              | Disconnection / Stream End
                                              v
                                  +-----------------------+
                                  |   Automatic Return    |
                                  |   Ready Splash 4-Lang |
                                  +-----------------------+
```

1. **Active Desktop Video:** Hardware H.264 decoding commands 100% of HDMI scanout. The audio visualizer thread sleeps (`was_drawing = false`), freeing GPU and memory bandwidth. PC audio simultaneously outputs via HDMI digital PCM/Opus.
2. **Active Audio without Video (IoT / Music):** When playing music via Bluetooth A2DP, DLNA, or PC audio without screen projection, the visualizer wakes up and renders 24 real frequency bars at 30 FPS on `/dev/fb0`, preventing the TV from entering standby.
3. **Both Idle:** Displays the 4-language Onboarding Ready Splash with IP address, connection mode, and instructions.

---

## 3. Real-Time Audio FFT Engine on Host PC (`ext-sender`)

### 3.1 512-Point Cooley-Tukey FFT in Pure Rust
`sender/src/pipeline.rs` monitors `Raspberry_Pi_HDMI_Audio.monitor` via `parec` at 48,000 Hz, 16-bit stereo:
1. **Hann Windowing:** 512-point samples ($10.7\text{ ms}$) receive $w[n] = 0.5 \times (1 - \cos(2\pi n / N))$ windowing to eradicate spectral leakage.
2. **RMS Power Calculation:** Quantized to a single byte (0 to 255) covering $-60\text{ dBFS}$ to $0\text{ dBFS}$.
3. **Logarithmic 24-Band Binning (94 Hz to 24 kHz):**
   - Bass: 94 Hz to 375 Hz (bands 0 to 3);
   - Midrange: 375 Hz to 3,000 Hz (bands 4 to 11);
   - Treble & Presence: 3,000 Hz to 24,000 Hz (bands 12 to 23).
4. **Low-Overhead UDP Dispatch:** Transmits a compact 25-byte binary packet (24 normalized band levels + 1 RMS byte) directly to UDP port `5006` on the Pi Zero at 50 FPS.

---

## 4. Zero-Reboot Teardown & Anti-Deadlock Architecture

### 4.1 Historical Root Cause of the USB Bulk Freeze
In earlier versions, switching between Mode 3 (USB Bulk) and Mode 2 (Miracast) froze the receiver. Synchronous `libc::read` calls on `/dev/usb-ffs/display/ep1` blocked indefinitely in the kernel `dwc2` driver once the host stopped transmitting. Control threads calling `worker.join()` hung permanently, forcing a physical reboot.

### 4.2 The Universal Anti-Deadlock Architecture
1. **Non-Blocking Polling with Timeouts (`libc::poll` 100ms):**
   Every hardware endpoint or socket read is preceded by `libc::poll` or `set_read_timeout` (10ms to 100ms). Loops periodically check atomic `running.load()`. When a service is stopped or switched, workers exit cleanly in milliseconds without blocking supervisors.
2. **Child Process Lifecycle Management (`Child::kill` & `wait`):**
   `StreamerHandle::Composite` in `ext-sender` explicitly tracks the `parec` spectrum child process alongside audio and video GStreamer pipelines. On pause or mode switch, processes are terminated via `kill()` and reaped via `wait()`, eliminating zombie leaks.
3. **Clean Resource Unmapping (`munmap` and `libc::close`):**
   `FramebufferSink` implements `Drop` to unmap memory (`libc::munmap`) from `/dev/fb0`. `NativeV4l2Decoder` cleanly closes `/dev/video10`, allowing the next service to acquire display hardware immediately.
4. **PipeWire Module Deduplication:**
   Prior to loading `module-null-sink`, the system checks `pactl list sinks short`. Existing sinks are reused without duplicating audio nodes.

---

## 5. Interactive Swagger OpenAPI 3.0 Documentation

Accessing `http://192.168.7.2:8080/swagger` launches Swagger UI based on OpenAPI 3.0.3, documenting:
* Media & Spectrum operations (`/api/media/*`);
* Host PC Remote Control (`/api/host/*`);
* Bluetooth Discovery & Pairing (`/api/bluetooth/*`);
* Digital Audio, Volume and Mute (`/api/audio/*`);
* Network Configuration (`/api/network`).

---

## 6. Network Auto-Discovery Protocol (Wi-Fi & Ethernet LAN) via UDP Broadcast (Port 5002)

To eliminate manual IP configuration when the receiver operates over Wi-Fi or Ethernet dongles:
1. **Receiver Beacon Responder (`receiver/src/discovery.rs`):**
   - The receiver listens on UDP port `5002` for probe datagrams containing `EXT-MONITOR-DISCOVER`.
   - When received, it instantly responds to the sender with `EXT-MONITOR-OFFER http_port=8080 stream_port=5000 version=2.3.0`.
   - In addition, it broadcasts periodic announcements every 10 seconds to `255.255.255.255:5002`.
2. **Sender Dynamic Probing (`sender/src/discovery.rs`):**
   - On startup, if the standard USB OTG IP `192.168.7.2` is unreachable, `ext-sender` broadcasts an `EXT-MONITOR-DISCOVER` probe on port 5002.
   - Upon receiving the `EXT-MONITOR-OFFER`, it extracts the dynamic IP address and connects RTP video and Opus audio channels automatically without manual user intervention.

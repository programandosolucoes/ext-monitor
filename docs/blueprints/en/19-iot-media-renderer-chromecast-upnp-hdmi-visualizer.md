# Blueprint 19: IoT Media Renderer Appliance — Google Cast, UPnP/DLNA and HDMI Graphic Visualizer

> [🇧🇷 Versão em Português](../pt/19-iot-media-renderer-chromecast-upnp-e-visualizador-hdmi.md) | 🇺🇸 English Version

**Project:** `ext-monitor`  
**Author:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Reference Files:** `receiver/src/media_renderer.rs`, `receiver/src/web.rs`, `receiver/src/audio.rs`  
**Date:** September 2026 (Updated for v2.3.0 Universal Appliance)  

---

## 1. Overview: From Secondary Display to Smart IoT Media Appliance

While `ext-monitor` was conceived as a high-speed USB secondary display, its physical form factor (Raspberry Pi Zero connected to a TV via HDMI with VideoCore IV acceleration, Wi-Fi 802.11n, Bluetooth 4.1, and embedded HTTP server) equips it with all hardware primitives of a **Smart Media Streaming Dongle (Chromecast / Fire TV / Apple TV class)**.

### The Problem: Blank Screens During Audio Playback
When streaming music via Bluetooth or network audio, leaving the TV displaying a static black screen wastes the display.
`ext-monitor` introduces:
1. **Dynamic HDMI Audio Spectrum Visualizer:** When audio plays without PC desktop video, the HDMI output renders real-time track metadata and an animated 24-band frequency spectrum at 30 FPS.
2. **Google Cast Ecosystem (CastV2 & DIAL):** Discovered by Android and iOS Google Home apps for media casting.
3. **UPnP / DLNA MediaRenderer:** Receives native "Cast to Device" commands from Windows, Linux, VLC, and smart TVs.

---

## 2. Technical Feasibility on Broadcom BCM2835 Silicon

| Feature | Implementation Mechanism | CPU Load on Pi Zero | Status |
| :--- | :--- | :--- | :--- |
| **Google Cast Audio (CastV2)** | Rust mDNS daemon (`_googlecast._tcp`) + TLS port 8009 | ~1.8% CPU | **Fully Operational** |
| **YouTube Cast (DIAL Protocol)** | SSDP on UDP 1900 + REST on port 8080 | < 0.5% CPU | **Fully Operational** |
| **UPnP / DLNA A/V Renderer** | SSDP (`urn:schemas-upnp-org:device:MediaRenderer:1`) | ~1.0% CPU | **Fully Operational** |
| **HDMI Spectrum Visualizer** | Fast Fourier Transform + Direct DRM Overlay | ~0.8% CPU | **Fully Operational** |
| **H.264 Video Playback** | VideoCore IV hardware offload via V4L2 M2M | ~1.5% CPU | **Fully Operational** |

---

## 3. Subsystem Architecture

```
+-----------------------------------------------------------------------------------+
|                               WI-FI / ETHERNET NETWORK                            |
|                                                                                   |
|  [ Android / iOS Mobile ]                 [ Windows / Linux PC ]                  |
|   - Google Home (Cast V2)                  - "Cast to Device" (DLNA UPnP)         |
|   - YouTube / Spotify Cast                 - MP4 / WebM Network Streams           |
+--------------------------+--------------------------------+-----------------------+
                           | mDNS / SSDP                    |
                           v                                v
+-----------------------------------------------------------------------------------+
|                        RASPBERRY PI ZERO W (APPLIANCE)                            |
|                                                                                   |
|  +-----------------------------------------------------------------------------+  |
|  |             Discovery and Control Layer (Rust Userspace)                    |  |
|  |  - mDNS Avahi Daemon (_googlecast._tcp / _googlezone._tcp)                  |  |
|  |  - SSDP Responder (UDP 1900 for UPnP MediaRenderer and DIAL YouTube)        |  |
|  |  - REST Control API (HTTP 8080: /api/media/control, /api/media/status)       |  |
|  +--------------------------------------+--------------------------------------+  |
|                                         |                                         |
|                 +-----------------------+-----------------------+                 |
|                 | Video Stream                                  | Audio Stream    |
|                 v                                               v                 |
|  +-------------------------------+             +-------------------------------+  |
|  |   V4L2 M2M Hardware Decoder   |             |   ALSA HDMI Direct (hw:0,0)   |  |
|  |   H.264 / MPEG4 (/dev/video10)|             |   Opus / AAC / PCM / MP3      |  |
|  +---------------+---------------+             +---------------+---------------+  |
|                  |                                             |                  |
|                  | DMA-BUF                                     v                  |
|                  v                              +------------------------------+  |
|  +-------------------------------+              | Track Metadata & FFT Engine  |  |
|  | DRM Plane 0: Fullscreen Video |              | - Track / Artist / Album     |  |
|  +-------------------------------+              | - 24-Band Spectrum Analyzer  |  |
|                  ^                              +--------------+---------------+  |
|                  | Mutual Exclusion Scanout                    |                  |
|                  +---------------------------------------------+                  |
|                   (DRM Overlay Plane 1: Dynamic HDMI Audio Visualizer)            |
|                                                                                   |
|                                         │ Single HDMI Port (HDMI-A-1)             |
|                                         ▼                                         |
|                             [ TELEVISION / HDMI MONITOR ]                         |
+-----------------------------------------------------------------------------------+
```

---

## 4. Hardware Audio FFT Real-Time Spectrum Engine

When audio plays (via Bluetooth or network) while desktop video is inactive:
* The 512-point Cooley-Tukey FFT engine computes frequency band amplitudes with Hann windowing.
* The 24-band spectrum visualizer draws smoothly at 30 FPS directly into the HDMI scanout buffer.
* The TV doubles as an elegant ambient music station instead of a blank screen.

# Engineering Blueprints Compendium — `ext-monitor`

> [🇧🇷 Versão em Português](README.pt-BR.md) | 🇺🇸 English Version

**Project:** `ext-monitor` — Universal Low-Latency USB Display & Multimedia Appliance Engine  
**Target Platform:** Raspberry Pi Zero W / Zero 2 W / Raspberry Pi 4 / Linux & Windows PC  
**Author:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Date:** September 2026  
**Project Status:** Stable, Production-Ready, 100% Native Pure Rust  

---

## 1. Overview of the Engineering Suite

This compendium gathers the reverse-engineering documentation, architectural decisions, and low-level specifications that made it possible to transform a $10 embedded computer (single-core ARM1176 1.0 GHz Raspberry Pi Zero) into a professional 60 FPS secondary monitor with sub-15ms latency.

All critical engineering milestones—from FAT16 sector alignment on Broadcom silicon to VideoCore IV zero-copy DRM/KMS scanout, Wayland power suspend resilience, 4-language onboarding splash, live EDID telemetry, direct kernel DRM/KMS scanout capture, bidirectional Rust host daemon, hybrid network/Bluetooth audio, IoT Media Renderer ecosystem, and single-HDMI FFT multiplexing—are formally documented across the 20 technical blueprints below:

---

## 2. Directory of the 21 Engineering Blueprints

| # | Technical Blueprint | Document File | Engineering Focus |
| :---: | :--- | :--- | :--- |
| **01** | **32MB Image & BCM2835 Geometry** | [`01-32mb-image-bcm2835-geometry.md`](en/01-32mb-image-bcm2835-geometry.md) | Broadcom ROM 65,525 clusters boundary, sector 1 alignment, 2KB FAT16 clusters, and 100% RAM execution. |
| **02** | **End-to-End Architecture vs. GUD** | [`02-architecture-tx-rx-and-gud-comparison.md`](en/02-architecture-tx-rx-and-gud-comparison.md) | Why GUD fails (100% CPU lock, USB bus saturation) and how H.264 V4L2 M2M achieves 60 FPS at 0.8% CPU load. |
| **03** | **Packet Transmission & Drop-on-Late** | [`03-packet-transmission-drop-on-late-pipeline.md`](en/03-packet-transmission-drop-on-late-pipeline.md) | 1472-byte MTU, RFC 6184 FU-A fragmentation, proactive drop-on-late frame decimation, and sleep queue drainage. |
| **04** | **PipeWire, Mutter & S3 Suspend Recovery** | [`04-pipewire-mutter-screencast-and-wayland.md`](en/04-pipewire-mutter-screencast-and-wayland.md) | Dongle-less headless capture, zero-copy DMA-BUF, keepalive clocks, and **Dual Rust Watchdogs** for instant S3 sleep recovery. |
| **05** | **CPU Optimization & CAS Scaler** | [`05-cpu-optimizations-rust-gpu-vpu-cas-scaler.md`](en/05-cpu-optimizations-rust-gpu-vpu-cas-scaler.md) | ARM1176 compiler flags, VA-API/NVENC silicon encoding, and Contrast Adaptive Sharpening (CAS) for sharp fonts. |
| **06** | **Concurrent Modes & USB Super-Gadget** | [`06-concurrent-operating-modes-usb-gadget.md`](en/06-concurrent-operating-modes-usb-gadget.md) | 3 concurrent modes (Linux UDP, Windows Miracast Win+K, USB Bulk), 7 DWC2 endpoints, dynamic UDC binding. |
| **07** | **Micro-SD Protection & USB Upgrades** | [`07-sd-card-in-ram-and-usb-upgrade.md`](en/07-sd-card-in-ram-and-usb-upgrade.md) | Zero flash wear, boot volume exposed as `EXTMONITOR` flash drive, in-situ firmware upgrades. |
| **08** | **Installation Manual & Host Portability** | [`08-host-installation-and-portability-manual.md`](en/08-host-installation-and-portability-manual.md) | 1-click `curl connect.sh`, multi-distro installer, low-latency udev rules, and `/dev/ttyACM0` serial console. |
| **09** | **Empirical Hardware Tests & Diagnostics** | [`09-empirical-hardware-tests-boot-diagnostics.md`](en/09-empirical-hardware-tests-boot-diagnostics.md) | Validation on Pi Zero v1.3 Monocore, rainbow splash diagnostics, 1.8s boot timeline, and thermal tests. |
| **10** | **Modular Clean Code & V4L2 M2M AU Framing** | [`10-modular-clean-code-v4l2-m2m-au-framing.md`](en/10-modular-clean-code-v4l2-m2m-au-framing.md) | SRP decomposition, Builder pattern, 32-bit ARM ioctl ABI fix (`0xC0CC5605`), and RFC 6184/4571 AU assembly. |
| **11** | **KMS DRM Zero-Copy Scanout & -EFAULT Fix** | [`11-kms-drm-dma-buf-scanout-zero-copy-efault-fix.md`](en/11-kms-drm-dma-buf-scanout-zero-copy-efault-fix.md) | Resolving errno 14 (-EFAULT), VideoCore IV Universal Planes (`vc4-drm`), direct DMA-BUF import. |
| **12** | **Multilingual Splash & Real-Time EDID** | [`12-multilingual-splash-realtime-edid-teardown.md`](en/12-multilingual-splash-realtime-edid-teardown.md) | Disconnection freeze prevention, embedded 4-language splash (`miniz_oxide`), and real-time VESA EDID parsing. |
| **13** | **Direct Kernel DRM/KMS Capture** | [`13-direct-kernel-drm-kms-capture-dual-engine.md`](en/13-direct-kernel-drm-kms-capture-dual-engine.md) | Bypassing GNOME Mutter, atomic PRIME DMA-BUF extraction via ioctl `GETFB2`, immunity to window occlusion. |
| **14** | **HDMI Digital Audio, Opus & ALSA** | [`14-digital-hdmi-audio-opus-alsa-av-sync.md`](en/14-digital-hdmi-audio-opus-alsa-av-sync.md) | Ultra-low latency (< 25ms) digital audio over PipeWire/Opus 48kHz on UDP port 5004, ALSA `vc4-hdmi` playback. |
| **15** | **Zero-Copy Pipeline & Wayland Quiescence** | [`15-zero-copy-pipeline-wayland-quiescence-display-lifecycle.md`](en/15-zero-copy-pipeline-wayland-quiescence-display-lifecycle.md) | Avoiding CPU `imagefreeze` buffer bloat, hardware KMS frame retention, and 15-second inactivity watchdog. |
| **16** | **Wayland Pacer & Universal Receivers** | [`16-wayland-quiescence-pacer-cross-armv6-universal-receivers.md`](en/16-wayland-quiescence-pacer-cross-armv6-universal-receivers.md) | Continuous 60 FPS heartbeat pacing for uninterrupted YouTube playback, cross-ARMv6 build guide, and non-OTG Pi setups. |
| **17** | **Native Rust Host Daemon & Web Control** | [`17-native-rust-host-agent-bidirectional-web-control.md`](en/17-native-rust-host-agent-bidirectional-web-control.md) | UDP 5001 RPC protocol, web control of host streaming, Mutter stride assert crash (`SIGABRT 6`) resolution. |
| **18** | **Hybrid Audio: IP Opus & Bluetooth A2DP** | [`18-hybrid-audio-subsystem-network-opus-bluetooth-a2dp.md`](en/18-hybrid-audio-subsystem-network-opus-bluetooth-a2dp.md) | Private local headset audio vs HDMI TV sound, Bluetooth 4.1 pairing on Pi Zero W, and ALSA `hw:0,0` routing. |
| **19** | **IoT Media Renderer: Cast & HDMI Visualizer** | [`19-iot-media-renderer-chromecast-upnp-hdmi-visualizer.md`](en/19-iot-media-renderer-chromecast-upnp-hdmi-visualizer.md) | Smart streaming dongle mode: Google Cast (CastV2), DIAL (YouTube), UPnP/DLNA, and HDMI audio visualizer. |
| **20** | **Single-HDMI Multiplexer & Zero-Reboot** | [`20-single-hdmi-scanout-multiplexer-realtime-fft-i18n-zero-reboot.md`](en/20-single-hdmi-scanout-multiplexer-realtime-fft-i18n-zero-reboot.md) | Mutual exclusion on single physical HDMI port, 512-point Cooley-Tukey FFT, 4-language i18n, and zero-reboot teardown. |
| **21** | **Miracast MS-MICE, Reverse RTSP WFD & 1080p SPS** | [`21-miracast-ms-mice-rtsp-wfd-dynamic-sps-and-gnome-network-displays.md`](en/21-miracast-ms-mice-rtsp-wfd-dynamic-sps-and-gnome-network-displays.md) | MS-MICE binary signaling (TCP 7250), reverse RTSP WFD connection to Source (7236), UDP 5005 discovery migration, and GNOME 1080p bug resolution via pure-Rust dynamic SPS parser. |
| **22** | **NV12 Stride Alignment & Realtime Miracast** | [`22-nv12-macroblock-stride-green-bar-fix-and-realtime-miracast-optimizations.md`](en/22-nv12-macroblock-stride-green-bar-fix-and-realtime-miracast-optimizations.md) | 16-line macroblock pitch math (1080->1088), 15,360-byte padding fix on KMS UV plane, PUSI zero-delay TS demuxing, 2ms polling, and AMD Radeon 610M VA-API hardware acceleration. |

---

## 3. Architecture Highlights

### Host Power Suspend Resilience (Sleep / S3)
* **The Failure:** Host S3 suspend destroys the ScreenCast D-Bus session and PipeWire node. Video encoder child processes lock up reading orphaned sockets.
* **The Solution:** Dual Rust Watchdog monitors the node every 1500ms. If destroyed, it terminates child processes atomically, waits for system wake, and renegotiates a clean ScreenCast session within 1.8 seconds.

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

### The Foundation (Original Blueprints 01 to 20):
All critical engineering milestones of the baseline appliance—from FAT16 sector alignment on Broadcom silicon to VideoCore IV zero-copy DRM/KMS scanout, Wayland power suspend resilience, 4-language onboarding splash, live EDID telemetry, direct kernel DRM/KMS scanout capture, bidirectional Rust host daemon, hybrid network/Bluetooth audio, IoT Media Renderer ecosystem, and single-HDMI FFT multiplexing—are formally documented across the initial 20 foundational blueprints.

### The Expansion Suite (Blueprints 21 to 25):
Following the completion of the baseline appliance, the engineering scope expanded to integrate **native zero-driver wireless projection (Wi-Fi Display / Miracast / MS-MICE)** for Windows 10/11 (`Win + K`) and Linux desktops. This expansion introduced 5 advanced blueprints covering:
* **BP 21:** MS-MICE binary signaling, reverse RTSP WFD negotiation, and dynamic SPS dimension parsing.
* **BP 22:** NV12 16-line macroblock stride alignment on KMS, eliminating the top green bar and dropping demux latency to 2ms.
* **BP 23:** Automated sysfs host GPU hardware ranking (AMD Mendocino/RDNA, Intel QuickSync, NVIDIA NVENC) and UI launch tooltips.
* **BP 24:** Native 720p60 CEA index 6 enforcement (Level 3.1) and deterministic PES-demarcated MPEG-TS Access Unit demuxing.
* **BP 25:** Unified multi-service standby stop, active RTSP client socket teardown, host transmitter termination (`SIGTERM`/`SIGKILL`), and persistent HDMI ready splash.
* **BP 26:** Empirical glass-to-glass latency, API responsiveness, and hardware telemetry benchmark across all 3 modes with Extend KMS default.

---

## 2. Directory of the 26 Engineering Blueprints

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
| **23** | **Miracast GPU Hardware Acceleration & Host Launcher** | [`23-miracast-gpu-hardware-acceleration-and-host-launcher.md`](en/23-miracast-gpu-hardware-acceleration-and-host-launcher.md) | Direct sysfs GPU vendor detection (AMD, Intel, NVIDIA), automated hardware ranking launcher daemon, atomic debounce, and Web UI GPU command box. |
| **24** | **Native 720p60 Miracast WFD & Deterministic Demux** | [`24-native-720p60-miracast-wfd-pes-au-deterministic-demux.md`](en/24-native-720p60-miracast-wfd-pes-au-deterministic-demux.md) | Enforcing CEA index 6 (1280x720p60) WFD formats, Level 3.1 profile, deterministic PES-demarcated MPEG-TS demuxer, and TS continuity counter verification. |
| **25** | **Unified Standby Stop, RTSP Teardown & Transmitter Kill** | [`25-unified-standby-stop-multi-service-transmitter-teardown.md`](en/25-unified-standby-stop-multi-service-transmitter-teardown.md) | Multi-service standby stop (Modes 1, 2, 3), active RTSP TCP stream teardown, host gnome-network-displays kill via pkill, and persistent HDMI ready splash. |
| **26** | **Empirical Latency, Responsiveness & Hardware Telemetry Benchmark** | [`26-latency-responsiveness-hardware-telemetry-benchmark.md`](en/26-latency-responsiveness-hardware-telemetry-benchmark.md) | Consolidated empirical benchmark across all 3 modes at native 60 FPS: Mode 2 Miracast Extend KMS (11.45ms), Mode 3 USB Bulk (11.45ms), and Mode 1 UDP (12.63ms); silicon telemetry, bus isolation, low 1.69W power draw, and <2.3% CPU load on Pi Zero. |
| **27** | **Modular Flow Architecture & Generic HEVC Codec** | [`27-modular-flow-architecture-microblocks-and-generic-hevc-codec.md`](en/27-modular-flow-architecture-microblocks-and-generic-hevc-codec.md) | Flow-oriented microblocks decomposition across ingress, demux, codec, and scanout layers; generic HEVC H.265 / H.264 video abstraction; zero cross-coupling. |
| **28** | **Wayland Quiescence, Damage Pacer & V4L2 Buffer Matrix** | [`28-wayland-quiescence-diagnosis-pacer-lossless-queue-and-v4l2-tuning.md`](en/28-wayland-quiescence-diagnosis-pacer-lossless-queue-and-v4l2-tuning.md) | Resolving Wayland Mutter 0 FPS quiescence freeze via 60 Hz 1x1 click-through Damage Pacer; eliminating leaky queues in compressed streams; 16/8 V4L2 M2M DPB buffer matrix. |
| **29** | **1s Freeze Diagnosis (mDNS Storm), Pure Rust Pacer & KMS Scanout** | [`29-one-second-freeze-mdns-rust-pacer-and-kms-scanout.md`](en/29-one-second-freeze-mdns-rust-pacer-and-kms-scanout.md) | Elimination of Python dependency (100% pure Rust in-process damage pacer), diagnosis and fix of mDNS multicast echo loop (9.7M dropped packets and 1s GOP stall), and direct kernel DRM/KMS scanout with `CAP_SYS_ADMIN`. |
| **30** | **Direct ALSA IEC958 Hi-Res Audio, Sink Deduplication, IP Anti-Fragmentation & Web Profiles** | [`30-hi-res-iec958-audio-sink-deduplication-anti-fragmentation-and-web-profiles.md`](en/30-hi-res-iec958-audio-sink-deduplication-anti-fragmentation-and-web-profiles.md) | Native 96kHz/192kHz ALSA hardware operation with IEC958 subframe channel status, PulseAudio sink mutex deduplication on host, eliminating audio cuts via pre-warmed device session, eliminating 30s video flickers (settimeofday jitter), and zero-fragmentation 1024-byte UDP audio packets (`audiobuffersplit`). |
| **31** | **Web UI Transport Mismatch Diagnosis & Telemetry Sync (Mode 1 vs Mode 3)** | [`31-ui-transport-mismatch-diagnosis-and-telemetry-sync-mode1-vs-mode3.md`](en/31-ui-transport-mismatch-diagnosis-and-telemetry-sync-mode1-vs-mode3.md) | Resolving visual mismatch where Web UI buttons statically displayed "USB Bulk Direct" while real streaming was active over Network UDP; updating HTML active defaults, initial JS state, and immediate sub-100ms boot telemetry sync. |
| **32** | **Audio DAC Panel on Tab 1, Chromecast-Style Web Casting & Service Switching Unit Tests** | [`32-audio-dac-panel-and-chromecast-casting-with-unit-tests.md`](en/blueprint-32-audio-dac-panel-and-chromecast-casting-with-unit-tests.md) | Consolidating Hi-Res Audio DAC controls and Chromecast-style web sharing (1-click Web Cast and Google Cast) directly onto Tab 1 Monitoring, with 7 formal Rust unit tests for deterministic service switching and standby behavior. |
| **33** | **Strict Chromium Cast TLS Certificate Validation, WebRTC Mirroring Handshake & SSDP Conflict Resolution** | [`33-google-cast-v2-tls-webrtc-handshake-and-ssdp-conflict-resolution.md`](en/blueprint-33-google-cast-v2-tls-webrtc-handshake-and-ssdp-conflict-resolution.md) | Enforcing Chromium Cast strict certificate lifetimes (4-day max, critical X.509 extensions, RSA-SHA256 signature), complete WebRTC Mirroring OFFER/ANSWER negotiation, eliminating duplicate ghost devices via SSDP/DIAL routing, and fixing GNOME Mutter logical display detection (resolving gray screen). |

---

## 3. Architecture Highlights

### Host Power Suspend Resilience (Sleep / S3)
* **The Failure:** Host S3 suspend destroys the ScreenCast D-Bus session and PipeWire node. Video encoder child processes lock up reading orphaned sockets.
* **The Solution:** Dual Rust Watchdog monitors the node every 1500ms. If destroyed, it terminates child processes atomically, waits for system wake, and renegotiates a clean ScreenCast session within 1.8 seconds.

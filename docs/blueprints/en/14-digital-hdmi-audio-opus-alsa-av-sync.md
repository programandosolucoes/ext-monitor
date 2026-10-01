# Blueprint 14: Digital HDMI Audio Subsystem, Ultra-Low Latency Opus Codec and ALSA Integration

> [🇧🇷 Versão em Português](../pt/14-subsistema-audio-hdmi-digital-opus-alsa-e-sincronismo-av.md) | 🇺🇸 English Version

**Project:** `ext-monitor`  
**Author:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Reference Files:** `receiver/src/audio.rs`, `sender/src/pipeline.rs`, `sender/src/pipewire.rs`  
**Date:** September 2026 (Updated for v2.3.0 Universal Appliance)  

---

## 1. Overview and Engineering Constraints

Earlier versions of `ext-monitor` streamed video only. On real workstations, the lack of audio routed to secondary monitors or TVs forced reliance on host speakers, restricting media consumption and full-screen presentations.

### Engineering Challenges:
1. **Algorithmic Latency and A/V Lip-Sync:** Audio buffering had to strictly match video latency (< 15–20ms). Legacy codecs like MP3 or AAC incur noticeable packetization delays and heavy CPU overhead on the Pi Zero's 1.0 GHz monocore SoC.
2. **Kernel HDMI Audio Activation:** Upstream `vc4-kms-v3d` disabled HDMI audio by default.
3. **Traffic Isolation and Head-of-Line Blocking Prevention:** Audio could not share the H.264 video port (5000), because 64KB video IDR bursts would cause jitter and dropouts in audio streams.

---

## 2. Architectural Decisions

### A. Codec Selection: Opus 48 kHz Stereo (10ms Frames)
* **Tiny Algorithmic Latency:** 10ms frame sizes ensure audio arrives at display speakers in under 15ms.
* **Low Network Overhead:** Streaming at 96 kbps with 48 kHz sampling uses less than 0.02% of USB 2.0 bus capacity.
* **Negligible CPU Consumption:** Decoding Opus packets into PCM consumes < 0.3% of the ARM1176 CPU.

### B. Dedicated Port Allocation
| Service | Protocol / Port | Function |
| :--- | :--- | :--- |
| **Video H.264 (Mode 1)** | UDP 5000 | Video RTP RFC 6184 stream to V4L2 M2M |
| **Control & Telemetry** | UDP 5001 | Dynamic hot-apply and telemetry sockets |
| **Digital HDMI Audio** | **UDP 5004** | **Dedicated RTP Opus audio channel (pt=96)** |
| **Hardware Audio FFT** | UDP 5006 | Realtime spectrum analysis bands for visualizer |

---

## 3. Implementation Details

### 3.1 Kernel Configuration (`config.txt`)
Enables VideoCore IV HDMI audio:
```ini
dtoverlay=vc4-kms-v3d,cma-128,nocomposite
dtparam=audio=on
```

### 3.2 Host Audio Sender (`ext-sender` / PipeWire)
Extracts sound directly from PipeWire and encodes to low-latency Opus:
```bash
gst-launch-1.0 -q pipewiresrc client-name=ext-hdmi-audio do-timestamp=true ! \
  audioconvert ! audioresample ! audio/x-raw,rate=48000,channels=2 ! \
  opusenc bitrate=96000 frame-size=10 complexity=3 ! rtpopuspay pt=96 ! \
  udpsink host=192.168.7.2 port=5004 sync=false
```

### 3.3 Receiver Audio Engine (`receiver/src/audio.rs`)
Plays directly into the Linux ALSA `vc4-hdmi` sound card with live volume interpolation:
```bash
gst-launch-1.0 -q udpsrc port=5004 caps="application/x-rtp,media=audio,clock-rate=48000,encoding-name=OPUS,payload=96" ! \
  rtpopusdepay ! opusdec ! audioconvert ! audioresample ! \
  volume volume=1.0 ! alsasink sync=false buffer-time=20000 latency-time=10000
```

### 3.4 REST API & Dashboard Controls
* `GET /api/audio/status`: Live volume and mute telemetry.
* `POST /api/audio/volume`: Smooth volume adjustment (0–100%) without stream restarts.
* `POST /api/audio/mute`: Instant mute toggle.

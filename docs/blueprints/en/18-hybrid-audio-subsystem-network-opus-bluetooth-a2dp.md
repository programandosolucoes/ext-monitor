# Blueprint 18: Hybrid Audio Subsystem — Network IP Opus and Bluetooth A2DP Sink

> [🇧🇷 Versão em Português](../pt/18-audio-hibrido-rede-opus-e-bluetooth-a2dp.md) | 🇺🇸 English Version

**Project:** `ext-monitor`  
**Author:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Reference Files:** `scripts/bluetooth-audio.sh`, `receiver/src/audio.rs`, `sender/src/pipewire.rs`  
**Date:** September 2026 (Updated for v2.3.0 Universal Appliance)  

---

## 1. Overview: The Multi-Stream Audio Challenge

On multi-monitor developer workstations, audio routing requires intentional separation:
* **The Undesirable Default:** Routing all system audio blindly to the HDMI TV causes Slack notifications, private Zoom meetings, and YouTube sounds to blare through TV speakers.
* **The Hybrid Ideal:** Private calls and system sounds remain isolated in personal headsets (e.g. Yealink UH34 USB), while presentations or media videos are routed to the TV. Additionally, the TV can double as a wireless Bluetooth speaker for smartphones.

`ext-monitor` solves this with a **Dual-Transport Hybrid Audio Subsystem**:
1. **Transport 1 (IP Network):** PipeWire virtual null-sink + Opus 48kHz RTP over UDP.
2. **Transport 2 (Wireless Bluetooth):** BlueZ A2DP Sink on Broadcom BCM43438 hardware routed to ALSA HDMI.

---

## 2. Audio Topology Architecture

```
                                          +-----------------------------------+
                                          |          Host PC (Linux)          |
                                          |                                   |
[ Meetings / Private Audio ] -----------> | [ Default Sink: Yealink Headset ] | (Local Private Audio)
                                          |                                   |
[ Selected Media / Presentation ] ------> | [ Null Sink: Raspberry_Pi_HDMI ]  |
                                          |                 |                 |
                                          |                 v (GStreamer)     |
                                          |         Opus RTP / UDP 5004       |
                                          +-----------------+-----------------+
                                                            |
                                        +-------------------+
                                        | USB Network (192.168.7.2)
                                        v
+-----------------------------------------------------------------------------+
|                          Raspberry Pi Zero W                                |
|                                                                             |
|  [ udpsrc:5004 ] -> [ rtpopusdepay ] -> [ opusdec ] --+                     |
|                                                       |                     |
|                                                       +-> [ ALSA: hw:0,0 ]  |
|                                                       |    (bcm2835 HDMI)   |
|  [ Smartphone / Tablet ] -(Bluetooth A2DP)-> [ BlueZ ]-+          |         |
|                                                                   v         |
|                                                         TV HDMI Speakers    |
+-----------------------------------------------------------------------------+
```

---

## 3. Host PipeWire Virtual Sink

`ext-sender` instantiates an isolated virtual audio sink without modifying system default devices:
```bash
pactl load-module module-null-sink \
    sink_name="Raspberry_Pi_HDMI_Audio" \
    sink_properties=device.description=Raspberry_Pi_HDMI_Audio
```
Individual desktop applications (Chrome, VLC, Spotify) can be routed selectively to `Raspberry_Pi_HDMI_Audio` in GNOME Sound Settings or `pavucontrol`.

---

## 4. Bluetooth A2DP Sink (Broadcom BCM43438)

The wireless chip on the Pi Zero W supports Bluetooth 4.1 classic audio streaming:
* **BlueZ A2DP Profile:** Configured as `audio-sink`.
* **ALSA Bridging:** `bluealsa-aplay` captures SBC/AAC Bluetooth streams and sends them directly to ALSA `hw:0,0`.
* **1-Click Pairing:** The Web Dashboard allows triggering 60-second discoverability mode, broadcasting **`ext-monitor`** to mobile devices.

# Design Specification: Universal Pure-Rust Miracast (Wi-Fi Display) Client

**Date:** 2026-10-01  
**Author:** Carlos Alberto <psncarlosalberto4ti@gmail.com>  
**Status:** Draft / Approved by User  
**Target Branch:** `feature/rust-miracast-client`  
**Target Appliance:** Raspberry Pi Zero W / Linux Laptop (Wayland/X11) / Universal Miracast Sinks  

---

## 1. Executive Summary

This specification defines the architecture, protocol implementation, and modular design for a **100% pure-Rust Miracast (Wi-Fi Display / WFD 1.0) Source (Transmitter)**.

The primary objective is to replace the heavy, GUI-bound, GTK4-dependent `gnome-network-displays` with a lightweight, headless, modular Rust client. The client functions both as:
1. A **standalone universal CLI tool** (`ext-miracast [IP]`) capable of scanning local networks (mDNS/SSDP) and streaming the host desktop to any Miracast sink (our Pi Zero appliance, Smart TVs, Windows/Android receivers).
2. An **integrated native subsystem** inside the `ext-sender` daemon, directly wired to the Web UI ("Modo 2 Miracast") and the unified standby stop engine.

---

## 2. Background & Motivation

While Mode 1 (UDP) and Mode 3 (USB Bulk) in `ext-monitor` are completely native in Rust, Mode 2 (Miracast) on Linux previously relied on launching `gnome-network-displays`. This introduced severe pain points:
* **Forced Graphical UI:** `gnome-network-displays` requires GTK4 and Libadwaita, popping up application windows on the host desktop and preventing true headless or daemonized background operation.
* **Lack of Programmatic Lifecycle Control:** Stopping a stream required sending POSIX termination signals (`SIGTERM`/`SIGKILL` via `pkill`) because the application provides no D-Bus or socket RPC control API.
* **External Heavyweight Dependency:** Users had to install `gnome-network-displays` and a large chain of GNOME runtime packages.
* **Encapsulation Mismatch:** The Raspberry Pi receiver (`ext-receiver`) already has a pure-Rust RTSP/WFD receiver engine ([`receiver/src/wfd.rs`](file:///home/carlos/ide/ext-monitor/receiver/src/wfd.rs)), while the host transmitter lacked a matching native Rust implementation.

---

## 3. High-Level Architecture & Data Flow

```
+-----------------------------------------------------------------------------------+
| Host Laptop: ext-miracast / ext-sender (feature/rust-miracast-client)             |
|                                                                                   |
|  [Discovery Engine]                                                               |
|   mDNS Scanner (_display._tcp, _airplay._tcp) + SSDP Broadcast                    |
|   Interactive Terminal Sink Selector / Direct IP CLI Argument                     |
|                                                                                   |
|  [RTSP WFD Source State Machine]                                                  |
|   TCP Connection (Port 7236 standard WFD / Port 7250 MS-MICE)                     |
|   Exchanges M1 -> M7:                                                             |
|     M1/M2: OPTIONS *                                                              |
|     M3:    GET_PARAMETER (wfd_video_formats, wfd_audio_codecs)                    |
|     M4:    SET_PARAMETER (negotiate CEA index 6 720p60 / 1080p, URL)              |
|     M5:    SET_PARAMETER (trigger SETUP)                                          |
|     M6:    SETUP (negotiate Sink UDP RTP port)                                    |
|     M7:    PLAY (start transmission)                                              |
|     TEARDOWN: Instant atomic disconnect                                           |
|                                                                                   |
|  [100% Pure-Rust Streaming Pipeline]                                              |
|   Screen Capture: PipeWire Screencast Portal / Direct DRM-KMS DMA-BUF             |
|   Video Encoder:  Hardware VA-API / NVENC H.264 (existing ext-sender modules)     |
|   MpegTsPacketizer: Pure-Rust 188-byte TS packetizer with PAT, PMT, PES, PCR/PTS   |
|   RtpSender:        UDP socket streaming TS packets (Payload Type 33) to Sink     |
+-----------------------------------------------------------------------------------+
                                         |
                       RTSP Control (TCP 7236/7250) + RTP MPEG-TS (UDP 5002)
                                         v
+-----------------------------------------------------------------------------------+
| Miracast Sink: Raspberry Pi Zero (ext-receiver) OR Any Universal Miracast Device  |
+-----------------------------------------------------------------------------------+
```

---

## 4. Detailed Component Design

### 4.1 Discovery Engine (`sender/src/miracast/discovery.rs`)
* **Purpose:** Locates compatible Wi-Fi Display / Miracast / MS-MICE sinks on the local network.
* **Mechanism:**
  * Sends multicast UDP packets to:
    * `224.0.0.251:5353` (mDNS) querying `_display._tcp.local` and `_airplay._tcp.local`.
    * `239.255.255.250:1900` (SSDP M-SEARCH) querying `urn:schemas-upnp-org:device:MediaRenderer:1`.
  * Also probes `192.168.7.2:7250` and `192.168.7.2:7236` (the direct USB gadget IP).
* **CLI UX:**
  * If the user runs `ext-miracast 192.168.7.2`, discovery is bypassed and connection begins immediately.
  * If the user runs `ext-miracast` with no arguments, discovery runs for 2.0 seconds and prints:
    ```text
    [ext-miracast] Discovered Miracast Sinks on Local Network:
      [1] ExtMonitor-Pi0 (192.168.7.2:7236) [USB Direct / MS-MICE]
      [2] Living Room Smart TV (192.168.1.150:7236) [Wi-Fi Display]
    Select sink (1-2) or 'q' to quit: 
    ```

### 4.2 RTSP WFD Source State Machine (`sender/src/miracast/wfd_client.rs`)
* **Purpose:** Implements the Wi-Fi Display (WFD 1.0 / MS-MICE) client session in pure Rust over `std::net::TcpStream`.
* **State Machine:**
  * **CONNECTING:** Connects to Sink IP on TCP 7236 (or 7250 if MS-MICE).
  * **NEGOTIATING_OPTIONS (M1/M2):** Sends `OPTIONS * RTSP/1.0`, receives Sink capabilities, handles reverse `OPTIONS`.
  * **QUERYING_PARAMS (M3):** Sends `GET_PARAMETER` for `wfd_video_formats`, `wfd_audio_codecs`, `wfd_client_rtpports`.
  * **SETTING_PARAMS (M4/M5):** Sends `SET_PARAMETER` selecting:
    * `wfd_video_formats`: Native CEA index 6 (1280x720p60 Level 3.1) or highest mutually supported mode.
    * `wfd_presentation_url`: `rtsp://<sink_ip>/wfd1.0/streamid=0 none`.
    * `wfd_trigger_method: SETUP`.
  * **SETUP (M6):** Sends `SETUP rtsp://.../wfd1.0/streamid=0/video RTSP/1.0` with `Transport: RTP/AVP/UDP;unicast;client_port=5002` (or dynamic port). Receives server UDP port from Sink response.
  * **STREAMING (M7):** Sends `PLAY rtsp://...`. Transitions video pipeline to active state.
  * **TEARDOWN:** Sends `TEARDOWN rtsp://...` and closes socket gracefully.

### 4.3 Pure-Rust MPEG-TS / PES Packetizer (`sender/src/miracast/mpegts.rs`)
* **Purpose:** Converts raw H.264 NALUs and PCM/AAC audio frames into standard MPEG-2 Transport Stream (MPEG-TS) packets for transmission over RTP.
* **Technical Specifications:**
  * **Packet Size:** Exactly 188 bytes, starting with sync byte `0x47`.
  * **PAT (Program Association Table):** Fixed PID `0x0000`, points Program 1 to PMT PID `0x1000`. Emitted every 100ms / at every IDR frame.
  * **PMT (Program Map Table):** Fixed PID `0x1000`, defines:
    * Stream Type `0x1B` (H.264 Video) on PID `0x0100`.
    * Stream Type `0x0F` (ADTS AAC) or `0x81` (LPCM Audio) on PID `0x0101`.
  * **PES (Packetized Elementary Stream):**
    * Stream ID `0xE0` for Video.
    * Header contains Presentation Time Stamp (PTS) and Decoding Time Stamp (DTS) based on a 90 kHz clock.
    * Injects Program Clock Reference (PCR) in TS Adaptation Field.
  * **RTP Multiplexing:** Packs 7 TS packets (7 * 188 = 1316 bytes) into an RTP packet (RFC 2250 / RFC 3550, PT=33) with 12-byte header, keeping payload under Ethernet MTU (1500 bytes).

### 4.4 Standalone CLI Binary (`sender/src/bin/ext-miracast.rs`)
* Minimal standalone executable compiled alongside `ext-sender`.
* Syntax: `ext-miracast [--ip <SINK_IP>] [--port <PORT>] [--res 720p|1080p] [--fps 30|60]`
* Handles POSIX signals (`SIGINT`, `SIGTERM`) to trigger immediate RTSP `TEARDOWN` and release all resources cleanly.

### 4.5 Integration into `ext-sender` Daemon
* In `sender/src/main.rs` and `sender/src/control.rs`:
  * Mode 2 ("Miracast") execution path delegates to `miracast::MiracastClient::connect_and_stream()` in a dedicated thread.
  * `ControlAction::StopStreaming` immediately signals the active `MiracastClient` session, sending RTSP `TEARDOWN` and stopping the video capture thread.
  * Eliminates all dependency on `gnome-network-displays` and `pkill`.

---

## 5. Error Handling & Resilience

1. **Sink Connection Failure:** If TCP 7236/7250 is unreachable or times out, report descriptive error with suggested fixes (e.g. check Pi Zero IP or verify firewall) and return cleanly.
2. **Abrupt Disconnection:** If the Sink resets the TCP connection or loses Wi-Fi/USB link, the client detects EOF or `BrokenPipe`, tears down the capture encoder safely, and returns to idle/CLI.
3. **Graceful Cancellation:** Pressing `Ctrl+C` in the CLI or clicking "Standby" in the Web UI completes the RTSP `TEARDOWN` exchange in < 50ms before process exit.

---

## 6. Verification & Test Plan

1. **Unit Tests (`sender/src/miracast/tests/`):**
   * `test_mpegts_pat_pmt_generation`: Verify 188-byte alignment, `0x47` sync bytes, and correct CRC32 on PAT/PMT packets.
   * `test_mpegts_pes_framing`: Verify PES header framing and PTS 90 kHz time calculations.
   * `test_wfd_rtsp_message_builder`: Verify M1-M7 message formatting against Wi-Fi Display specification.
2. **Integration Test with `ext-receiver`:**
   * Run `ext-miracast 192.168.7.2`.
   * Verify Pi Zero logs show valid RTSP M1-M7 handshake and starts H.264 V4L2 M2M hardware decoding.
   * Verify fluid 60 FPS output on physical HDMI display.
   * Verify clean shutdown when pressing `Ctrl+C` or sending Standby from Web UI.

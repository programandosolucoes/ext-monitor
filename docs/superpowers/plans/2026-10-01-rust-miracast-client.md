# Universal Pure-Rust Miracast (Wi-Fi Display) Client Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a 100% pure-Rust Miracast (Wi-Fi Display / WFD 1.0 / MS-MICE) Source (Transmitter) on Linux Wayland to replace `gnome-network-displays` with a headless, universal CLI binary (`ext-miracast`) and native `ext-sender` Web UI integration.

**Architecture:** A modular pure-Rust stack consisting of an MPEG-TS/PES packetizer with RTP payload type 33 encapsulation, an RTSP 1.0 WFD M1-M7 client state machine over TCP, and an mDNS/SSDP network discovery engine. Screen and audio capture reuse `ext-sender`'s existing PipeWire Screencast and hardware VA-API/NVENC encoders.

**Tech Stack:** Rust (2021 edition), `std::net` (TCP/UDP), POSIX sockets, PipeWire DMA-BUF portal, libva/openh264.

## Global Constraints
- Target Git Branch: `feature/rust-miracast-client` (never push directly to `main` until validated).
- Zero GTK or graphical UI dependencies for the Miracast client.
- Zero extra C library dependencies; pure Rust for MPEG-TS, RTSP, and discovery.
- Strict 188-byte alignment for all MPEG-TS packets starting with `0x47`.
- Clean teardown on `Ctrl+C` or Web UI Standby within < 100ms via RTSP `TEARDOWN`.
- Support both direct USB IP (`192.168.7.2`) and local network sinks via discovery.

---

### Task 1: Pure-Rust MPEG-TS / PES Packetizer (`sender/src/miracast/mpegts.rs`)

**Files:**
- Create: `sender/src/miracast/mpegts.rs`
- Modify: `sender/src/miracast/mod.rs`
- Test: Unit tests inside `sender/src/miracast/mpegts.rs`

**Interfaces:**
- Consumes: Raw H.264 NALUs (`&[u8]`), boolean `is_keyframe`, timestamp `pts_90khz: u64`.
- Produces: `MpegTsMuxer::new()`, `mux_h264_nalus(nalus: &[u8], is_keyframe: bool, pts_90khz: u64) -> Vec<[u8; 188]>`, `wrap_rtp(ts_packets: &[[u8; 188]]) -> Vec<Vec<u8>>`.

- [x] **Step 1: Write unit tests for MPEG-TS and RTP framing**

Add tests covering PAT, PMT, PES header generation, and 188-byte alignment verification:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ts_packet_sync_byte() {
        let mut muxer = MpegTsMuxer::new();
        let dummy_nalu = vec![0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, 0x00];
        let packets = muxer.mux_h264_nalus(&dummy_nalu, true, 90000);
        assert!(!packets.is_empty());
        for pkt in &packets {
            assert_eq!(pkt[0], 0x47, "MPEG-TS packet must begin with sync byte 0x47");
            assert_eq!(pkt.len(), 188, "MPEG-TS packet must be exactly 188 bytes");
        }
    }

    #[test]
    fn test_rtp_wrapping_chunking() {
        let dummy_ts = [[0x47u8; 188]; 14];
        let rtp_packets = MpegTsMuxer::wrap_rtp(&dummy_ts, 1, 90000);
        // 14 TS packets should be split into 2 RTP packets (7 TS packets each = 1316 bytes + 12-byte header)
        assert_eq!(rtp_packets.len(), 2);
        assert_eq!(rtp_packets[0].len(), 12 + 7 * 188);
        assert_eq!(rtp_packets[0][1] & 0x7F, 33, "RTP Payload Type must be 33 (MP2T)");
    }
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p ext-sender mpegts`
Expected: FAIL with module or type not found.

- [x] **Step 3: Implement minimal MpegTsMuxer**

Create `sender/src/miracast/mpegts.rs` implementing:
- PAT packet generator (PID `0x0000`, Program 1 -> PMT PID `0x1000`, CRC32).
- PMT packet generator (PID `0x1000`, Video Stream Type `0x1B` on PID `0x0100`, PCR PID `0x0100`).
- PES packetizer with 90 kHz PTS/DTS header framing and Adaptation Field PCR injection on keyframes.
- Continuity counter tracking (`cc & 0x0F`) per PID.
- RTP encapsulation (RFC 2250 / RFC 3550, 12-byte RTP header, PT=33, 7 TS packets per datagram = 1316 bytes payload).

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test -p ext-sender mpegts`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add sender/src/miracast/mpegts.rs sender/src/miracast/mod.rs
git commit -m "feat(miracast): implement pure-rust mpeg-ts and rtp packetizer"
```

---

### Task 2: RTSP 1.0 WFD Source State Machine (`sender/src/miracast/wfd_client.rs`)

**Files:**
- Create: `sender/src/miracast/wfd_client.rs`
- Modify: `sender/src/miracast/mod.rs`
- Test: Unit tests inside `sender/src/miracast/wfd_client.rs`

**Interfaces:**
- Consumes: Target IP and RTSP port (7236 standard, 7250 MS-MICE).
- Produces: `WfdClient::connect(target_ip: &str, target_port: u16) -> Result<WfdClient, io::Error>`, `negotiate_session(&mut self) -> Result<u16, io::Error>` (returns negotiated sink UDP RTP port), `teardown(&mut self) -> Result<(), io::Error>`.

- [x] **Step 1: Write unit tests for RTSP message builders**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtsp_options_builder() {
        let msg = WfdClient::build_options_request("192.168.7.2", 7236, 1);
        assert!(msg.starts_with("OPTIONS * RTSP/1.0\r\n"));
        assert!(msg.contains("CSeq: 1\r\n"));
        assert!(msg.contains("Require: org.wfa.wfd1.0\r\n"));
    }

    #[test]
    fn test_parse_transport_server_port() {
        let response = "RTSP/1.0 200 OK\r\nCSeq: 4\r\nTransport: RTP/AVP/UDP;unicast;client_port=5002;server_port=5002\r\n\r\n";
        let port = WfdClient::parse_rtp_port_from_transport(response);
        assert_eq!(port, Some(5002));
    }
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p ext-sender wfd_client`
Expected: FAIL with missing methods.

- [x] **Step 3: Implement WfdClient state machine**

Implement in `sender/src/miracast/wfd_client.rs`:
- TCP stream connection to Sink on port 7236 (fallback 7250 MS-MICE).
- Message sequencing:
  - M1: `OPTIONS * RTSP/1.0`
  - M2: Handle incoming reverse `OPTIONS` from Sink.
  - M3: `GET_PARAMETER` for `wfd_video_formats`, `wfd_audio_codecs`.
  - M4: `SET_PARAMETER` choosing CEA index 6 (720p60) or 1080p, and presentation URL.
  - M5: `SET_PARAMETER` triggering SETUP.
  - M6: `SETUP` requesting client RTP port. Extracts server port from Transport header.
  - M7: `PLAY` command to initiate streaming.
  - `teardown()` sending `TEARDOWN rtsp://...` and flushing socket.

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test -p ext-sender wfd_client`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add sender/src/miracast/wfd_client.rs sender/src/miracast/mod.rs
git commit -m "feat(miracast): implement pure-rust rtsp wfd source state machine"
```

---

### Task 3: mDNS & SSDP Network Discovery Engine (`sender/src/miracast/discovery.rs`)

**Files:**
- Create: `sender/src/miracast/discovery.rs`
- Modify: `sender/src/miracast/mod.rs`
- Test: Unit tests inside `sender/src/miracast/discovery.rs`

**Interfaces:**
- Produces: `DiscoveredSink { name: String, ip: IpAddr, port: u16, kind: &'static str }`, `scan_sinks(timeout: Duration) -> Vec<DiscoveredSink>`, `interactive_select_sink() -> Option<DiscoveredSink>`.

- [x] **Step 1: Write unit tests for mDNS / SSDP query encoding**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mdns_query_packet() {
        let packet = build_mdns_query("_display._tcp.local");
        assert!(packet.len() > 12);
        assert_eq!(packet[2], 0x00); // Standard query
        assert_eq!(packet[5], 0x01); // 1 Question
    }
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p ext-sender discovery`
Expected: FAIL.

- [x] **Step 3: Implement discovery engine**

Implement in `sender/src/miracast/discovery.rs`:
- Multicast mDNS probe on `224.0.0.251:5353`.
- SSDP M-SEARCH broadcast on `239.255.255.250:1900`.
- Direct USB probe for `192.168.7.2:7236` and `192.168.7.2:7250` (detects Pi Zero in < 50ms).
- Response parser extracting IP, port, and human-readable device name.
- Interactive terminal selector rendering clean numbered list.

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test -p ext-sender discovery`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add sender/src/miracast/discovery.rs sender/src/miracast/mod.rs
git commit -m "feat(miracast): implement pure-rust mdns and ssdp discovery scanner"
```

---

### Task 4: Miracast Session Orchestrator (`sender/src/miracast/client.rs`)

**Files:**
- Create: `sender/src/miracast/client.rs`
- Modify: `sender/src/miracast/mod.rs`

**Interfaces:**
- Consumes: `WfdClient`, `MpegTsMuxer`, existing `PipeWireCapturer` / `encoder::H264Encoder`.
- Produces: `MiracastSession::start(target_ip: &str, target_port: u16, running: Arc<AtomicBool>) -> Result<(), io::Error>`, `MiracastSession::stop(&self)`.

- [x] **Step 1: Implement session loop**

In `sender/src/miracast/client.rs`:
- Coordinates the WFD RTSP handshake with `WfdClient`.
- Binds local UDP socket for RTP streaming.
- Spawns video encoding and multiplexing thread: reads frames from existing encoder, sends to `MpegTsMuxer`, sends RTP datagrams to `target_ip:sink_rtp_port`.
- Monitors `running` atomic boolean. Upon cancellation, calls `wfd_client.teardown()` and closes sockets.

- [x] **Step 2: Verify compilation**

Run: `cargo check -p ext-sender`
Expected: Compiles with no warnings or errors.

- [x] **Step 3: Commit**

```bash
git add sender/src/miracast/client.rs sender/src/miracast/mod.rs
git commit -m "feat(miracast): implement pure-rust miracast session orchestrator"
```

---

### Task 5: Standalone CLI Binary (`sender/src/bin/ext-miracast.rs`)

**Files:**
- Create: `sender/src/bin/ext-miracast.rs`
- Modify: `sender/Cargo.toml` (declare `[[bin]] name = "ext-miracast"`)

- [x] **Step 1: Write the standalone CLI executable**

In `sender/src/bin/ext-miracast.rs`:
- Parse CLI arguments (`--ip`, `--port`, `--help`).
- If no IP is given, invoke `interactive_select_sink()`.
- Register `SIGINT` / `SIGTERM` handlers via `libc::signal` or `signal_hook` to ensure clean RTSP `TEARDOWN`.
- Call `MiracastSession::start(...)`.

- [x] **Step 2: Build the binary**

Run: `cargo build -p ext-sender --bin ext-miracast`
Expected: Binary `./target/debug/ext-miracast` created successfully.

- [x] **Step 3: Run `--help` test**

Run: `./target/debug/ext-miracast --help`
Expected: Prints clean usage information and exits with code 0.

- [x] **Step 4: Commit**

```bash
git add sender/src/bin/ext-miracast.rs sender/Cargo.toml
git commit -m "feat(miracast): add standalone ext-miracast cli binary"
```

---

### Task 6: Native Integration into `ext-sender` Daemon & Web UI Control

**Files:**
- Modify: `sender/src/miracast_launcher.rs` (redirect launch to internal pure-Rust session or standalone binary)
- Modify: `sender/src/control.rs` (wire start/stop to `MiracastSession`)
- Modify: `sender/src/main.rs` (handle Mode 2 cleanly)

- [x] **Step 1: Connect Miracast session into control loop**

- In `sender/src/control.rs`:
  - When `ControlAction::StartStreaming` specifies Mode 2 (`miracast`), launch `MiracastSession` directly in background thread instead of launching `gnome-network-displays`.
  - When `ControlAction::StopStreaming` is triggered (e.g. via Standby button in Web UI), call `MiracastSession::stop()` directly, sending atomic RTSP `TEARDOWN` without calling `pkill`.

- [x] **Step 2: Verify compilation and tests**

Run: `cargo check -p ext-sender && cargo test -p ext-sender`
Expected: PASS.

- [x] **Step 3: Commit**

```bash
git add sender/src/miracast_launcher.rs sender/src/control.rs sender/src/main.rs
git commit -m "feat(miracast): integrate native pure-rust miracast into ext-sender daemon"
```

---

### Task 7: End-to-End Live Validation with `ext-receiver` on Pi Zero

**Files:**
- Hardware Test on local Raspberry Pi Zero W (`192.168.7.2`)

- [ ] **Step 1: Test discovery**

Run: `./target/debug/ext-miracast`
Expected: Detects `ExtMonitor-Pi0 (192.168.7.2)` via direct USB probe or mDNS.

- [ ] **Step 2: Test live streaming**

Run: `./target/debug/ext-miracast 192.168.7.2`
Expected:
- Pi Zero terminal logs show: `Incoming RTSP WFD connection`, `Negotiated 720p60`, starts decoding.
- Physical HDMI screen transitions from Standby/Ready splash to live fluid desktop stream.

- [ ] **Step 3: Test clean teardown**

Press `Ctrl+C` in `ext-miracast`.
Expected:
- Source sends `TEARDOWN`.
- Pi Zero receives EOF/teardown and immediately renders Standby ready splash screen without crashing.

- [ ] **Step 4: Commit and finalize branch documentation**

```bash
git add docs/
git commit -m "docs(plan): complete implementation and validation of pure-rust miracast client"
```

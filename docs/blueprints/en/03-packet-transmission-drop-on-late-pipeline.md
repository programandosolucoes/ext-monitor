# Blueprint 03: Packet Transmission, NALU Fragmentation and the Drop-on-Late Algorithm

> [🇧🇷 Versão em Português](../pt/03-transmissao-pacotes-drop-on-late-e-pipeline.md) | 🇺🇸 English Version

**Project:** `ext-monitor`  
**Author:** Carlos Alberto <carlosalberto4ti@gmail.com>  
**Reference Files:** `sender/src/pipeline.rs`, `receiver/src/stream/rtp.rs`, `receiver/src/stream/annexb.rs`, `receiver/src/ingress/udp.rs`, `receiver/src/ingress/usb.rs`  
**Date:** September 2026 (Updated with RFC 6184 / RFC 4571 Access Unit Framing)  

---

## 1. Overview and Design Principles

The stability and fluidity of a secondary monitor depend critically on the network dispatching strategy. In interactive display pipelines, **constant low latency (< 15ms) is infinitely more critical than 100% reliable packet retransmission**.

If a streaming protocol queues delayed packets in buffers to guarantee lossless delivery (as TCP or on-demand video streaming engines like YouTube and Netflix do), mouse cursor movement suffers devastating buffer bloat (rubber-banding), destroying the user experience.

This blueprint details RTP packetization, FU-A fragmentation (RFC 6184), RFC 4571 stream framing for USB Bulk transport, and the **Drop-on-Late Leaky Queue** algorithm.

---

## 2. Packet Anatomy and USB Network MTU (1472 Bytes)

The virtual network interface emulated by the USB Gadget (`cdc_ether` / `usb0`) operates with a standard MTU of **1500 bytes**.

To avoid IP-layer fragmentation (which incurs CPU overhead and escalates dropped frames), the maximum UDP payload size is precisely calculated:

```
+------------------------------------------------------------------------+
| Ethernet Header (14 bytes)                                             |
|  +-------------------------------------------------------------------+ |
|  | IPv4 Header (20 bytes)                                            | |
|  |  +--------------------------------------------------------------+ | |
|  |  | UDP Header (8 bytes)                                         | | |
|  |  |  +---------------------------------------------------------+ | | |
|  |  |  | RTP Header (12 bytes)                                   | | | |
|  |  |  |  +----------------------------------------------------+ | | | |
|  |  |  |  | H.264 Payload (NALU chunk - Max 1460 bytes)         | | | | |
|  |  |  |  +----------------------------------------------------+ | | | |
|  |  |  +---------------------------------------------------------+ | | |
|  |  +--------------------------------------------------------------+ | |
|  +-------------------------------------------------------------------+ |
+------------------------------------------------------------------------+
```

* **Maximum UDP Payload:** `1500 (MTU) - 20 (IP) - 8 (UDP) = 1472 bytes`.
* **Usable H.264 Video Payload:** `1472 - 12 (RTP Header) = 1460 bytes`.

---

## 3. H.264 NALU Fragmentation and Assembly (RFC 6184)

Compressed H.264 frames at 720p or 900p vary from small P-frames (static desktop: hundreds of bytes) to large IDR Keyframes (full screen redraw: 50–120 KB).

The networking subsystem implements all three packet modes defined in **RFC 6184**:

### 3.1 Single NAL Unit Mode
When the NALU size is ≤ 1460 bytes:
* Transmitted whole in a single RTP packet.
* Original NAL header byte is preserved intact.

### 3.2 STAP-A Mode (Aggregation of Small NALUs)
When consecutive NALUs are tiny (such as 25-byte SPS and 8-byte PPS parameter sets):
* `ext-sender` aggregates them into a single RTP packet stamped with type `24` (STAP-A), preceded by 16-bit length fields.
* Eliminates network protocol header waste for stream metadata.

### 3.3 FU-A Mode (Fragmentation Unit Type A)
When a NALU (typically an IDR or complex P-frame) exceeds 1460 bytes:
* Segmented into fragments up to 1458 bytes.
* Byte 1 is the **FU Indicator** (Type 28).
* Byte 2 is the **FU Header**:
  * Bit `S` (Start): Marks the first fragment of the NALU.
  * Bit `E` (End): Marks the final fragment of the NALU.
  * Bit `R` (Reserved): Set to 0.
  * 5 bits: Original NALU type (e.g., 5 for IDR slice, 1 for non-IDR).

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|F|NRI|  Type=28|S|E|R|  Type   |          Payload ...          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
  FU Indicator      FU Header
```

---

## 4. The Drop-on-Late Algorithm & The Leaky Queue

The fundamental flaw in naive video streaming architectures is using unbounded FIFO queues. If the network experiences a transient 50ms hiccup, queued frames are played back sequentially in past time, introducing permanent lagging.

### 4.1 Leaky Queue Execution Model
The receiver enforces a queue depth of **maximum 1 frame**:

```
Incoming Video Packet
         │
         ▼
[ Packet Timestamp ] < [ Timestamp of Last Frame Sent to Hardware VPU ] ?
         │
         ├── YES: STALE / LATE PACKET! -> Immediate Drop (DROP)
         │
         └── NO: Query VPU Hardware Status
                     │
                     ├── Hardware VPU Busy Decoding?
                     │     └── If secondary P-frame: DROP!
                     │     └── If IDR Keyframe: Preempt Hardware!
                     │
                     └── Hardware VPU Idle -> Immediate Submission to V4L2 M2M
```

### 4.2 Self-Healing Mechanism with Periodic IDRs
* Host encoder (`ext-sender`) outputs a complete IDR frame every **1 second** (`key-int-max=30` at 30 FPS / `key-int-max=60` at 60 FPS).
* If packet corruption occurs on the USB bus, downstream dependent P-frames are dropped, avoiding visual artifact smearing.
* The next IDR rebuilds the display completely within < 16ms without perceptible user delay.

### 4.3 Sleep Drain and S3 Suspend Recovery
* **During S3 Sleep:** Host frame generation halts and USB network traffic stops. The receiver UDP socket descriptor stays open with a 2MB `SO_RCVBUF` without memory leaks.
* **Orphaned Chunk Pruning:** Partial or residual packets stranded in network buffers during power transitions are evicted immediately by the Leaky Queue's timestamp validation.
* **Atomic Resynchronization:** When the `ext-sender` supervisor recreates the pipeline and transmits the fresh IDR frame with fresh SPS/PPS headers, the VideoCore IV locks sync on the very first frame cycle (< 16ms).

---

## 5. Architectural Evolution: Shell Scripts to 100% Pure Native Rust

The `ext-monitor` pipeline evolved through three major engineering generations:

```
PHASE 1 (Legacy Prototype):
  Bash Scripts -> gst-launch-1.0 -> Python Flask -> heavy child subprocesses
  - RAM Footprint: ~160 MB
  - Boot Time: ~15 to 22 seconds
  - Runtime Dependencies: Python, GStreamer, GLib, libcairo, libgirepository
  - End-to-End Latency: ~90ms to 140ms
                      │
                      ▼
PHASE 2 (Hybrid Rust Prototype):
  Rust ext-sender invoking GStreamer C-API + ffmpeg on receiver
  - RAM Footprint: ~65 MB
  - Failure: ffmpeg on Pi Zero fell back to software decoding (100% CPU lockup)
                      │
                      ▼
PHASE 3 (Current Production - 100% Native Pure Rust & Silicon V4L2 M2M):
  Fully static binaries with zero external runtime dependencies.
  Direct ioctls to the V4L2 M2M hardware decoder (/dev/video10).
  Embedded Web, DHCP, and Swagger servers in pure Rust.
  - RAM Footprint: < 8 MB
  - Binary Size: ~700 KB (stripped)
  - Boot Time: < 1.8 seconds
  - End-to-End Latency: < 15ms (UDP) / < 1ms (USB Bulk)
  - CPU Load on Pi Zero: 0.8% to 1.2%
```

Eliminating intermediate processes and Unix pipes reduced OS context switching to near zero, providing the determinism demanded by the Pi Zero's silicon.

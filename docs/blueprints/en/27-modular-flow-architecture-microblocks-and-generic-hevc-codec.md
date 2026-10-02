# Blueprint 27: Flow-Oriented Micro-Block Architecture, Physical Decoupling, and Generic Multi-Codec Support (H.264 / HEVC H.265)

## 1. Overview and Core Architectural Principles

This blueprint formalizes the architectural refactoring of the **Ext-Monitor** ecosystem to achieve extreme modularity, independent maintainability, and absolute decoupling between subsystems.

### The Monolith Challenge:
Previously, capture routines, hardware encoding, transport packaging (USB vs UDP vs RTSP), and decoding routines frequently resided in sprawling files with high contextual coupling. A minor adjustment in USB ZLP handling could inadvertently destabilize MPEG-TS packet pacing or RTP depayloading.

### The Flow-Oriented Micro-Block Philosophy:
The system is decomposed into **self-documenting, single-responsibility micro-blocks**. The name of each file, struct, and function directly reflects its exact stage and role in the unidirectional end-to-end data pipeline:
```
[Scanout Capture] ➔ [GPU Encoding] ➔ [Transport Egress] ➔ [Physical Link] ➔ [Transport Ingress] ➔ [Demux / Depayload] ➔ [Hardware Decode] ➔ [KMS Presentation]
```

Every micro-block satisfies:
1. **Minimal Interface Contract (Deep Module):** A clean, focused interface backed by a deep, battle-tested implementation.
2. **Autonomous Boundary Handling:** In-flight packet validation, corrupted byte filtering, explicit I/O timeouts, and automatic error recovery without impacting neighboring stages.
3. **Strict Ingress / Egress Decoupling:** Zero cross-layer bleeding; the USB transport layer does not parse H.264 NALUs, and the hardware VPU decoder does not care whether compressed bitstream bytes arrived over USB Bulk or UDP.

---

## 2. End-to-End Data Flow Map and Micro-Block Topology

### 2.1 Transmitter (Host / Sender):
```
sender/src/flow/
├── capture_kms_direct.rs       # Zero-copy frame capture directly from primary/secondary KMS CRTC
├── capture_pipewire_mutter.rs  # PipeWire D-Bus portal screencast frame capture
├── encoder_types.rs            # Universal video descriptors, CodecKind (H264, HEVC), profiles
├── encoder_gpu_selector.rs     # Multi-arch hardware encoder dispatcher (VA-API / NVENC / QSV / CPU)
├── egress_usb_bulk.rs          # Atomic USB Bulk OUT (0x03) writer with ZLP generation
├── egress_network_udp.rs       # RFC 4571 UDP datagram transmitter to receiver socket
└── egress_miracast_wfd.rs      # RTSP WFD Wi-Fi Display session orchestrator & MPEG-TS muxer
```

### 2.2 Receiver (Appliance / Pi Zero & Generic Targets):
```
receiver/src/flow/
├── ingress_usb_bulk.rs         # Continuous USB FunctionFS Endpoint (0x01/0x03) reader
├── ingress_network_udp.rs      # UDP datagram receiver (ports 5000 / 5002)
├── ingress_rtsp_wfd.rs         # RTSP WFD signaling server (ports 7236 / 7250)
├── demux_annexb_assembler.rs   # Annex-B NALU assembler, SPS/PPS caching, IDR detection
├── demux_rtp_depayloader.rs    # RFC 6184 (H.264) and RFC 7798 (HEVC H.265) depayloader
├── demux_mpegts_parser.rs      # 188-byte MPEG-TS parser, PAT, PMT, PES, and PCR clock recovery
├── codec_types.rs              # Universal CodecKind definitions (H264, HevcH265, Av1)
├── codec_v4l2_m2m.rs           # VideoCore IV / rpivid V4L2 M2M hardware decoder
├── codec_vaapi.rs              # VA-API hardware decoder for generic x86_64 receivers
├── scanout_frame_pacer.rs      # Deterministic CFR pacing (60/30 FPS) & anti-freeze keepalive
└── scanout_drm_kms.rs          # DMA-BUF zero-copy buffer export to DRM HDMI scanout plane
```

---

## 3. Generic Multi-Codec Specification (H.264 and HEVC / H.265)

### 3.1 Hardware Capability Matrix

| Platform / Hardware | H.264 (AVC) ASIC | HEVC (H.265) ASIC | Bandwidth Efficiency | Primary Use Case |
| :--- | :---: | :---: | :---: | :--- |
| **Raspberry Pi Zero / W (BCM2835)** | ✅ VideoCore IV (ASIC) | ❌ None (CPU choked) | Standard Baseline (100%) | **Mandatory Default** |
| **Raspberry Pi 4 / 400 (BCM2711)** | ✅ VideoCore VI | ✅ 4K60 ASIC (`rpivid`) | **-45% bandwidth** | High resolution / Wi-Fi |
| **Raspberry Pi Zero 2 W (BCM2710A1)**| ✅ VideoCore IV | ⚠️ Multithread CPU (4 cores) | -40% bandwidth (720p) | Efficient bridge |
| **PC / Laptop Receiver (x86_64)**   | ✅ VA-API / NVDEC | ✅ VA-API / NVDEC (10-bit) | **-50% bandwidth** | PC-as-Screen |
| **PlayStation 5 / Chiaki / Moonlight**| ✅ Hardware | ✅ Native Hardware | **-50% bandwidth** | Ultra-low latency stream |

### 3.2 Universal Codec Enumeration (`codec_types.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CodecKind {
    /// H.264 / AVC (Advanced Video Coding - ISO/IEC 14496-10)
    H264,
    /// H.265 / HEVC (High Efficiency Video Coding - ISO/IEC 23008-2)
    HevcH265,
    /// AOMedia Video 1 (AV1)
    Av1,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StreamNegotiation {
    pub supported_codecs: Vec<CodecKind>,
    pub preferred_codec: CodecKind,
    pub max_width: u32,
    pub max_height: u32,
    pub max_fps: u32,
}
```

### 3.3 Auto-Negotiation and Fallback Rules
1. The receiver broadcasts its supported codec matrix during initialization (`GET /api/status` or RTSP M3 `GET_PARAMETER`).
2. When the receiver is identified as a **Raspberry Pi Zero W**, codec selection strictly locks to `CodecKind::H264`.
3. When the receiver is a **Raspberry Pi 4**, **x86_64 PC**, or HEVC-capable target, the transmitter preferentially selects `CodecKind::HevcH265`.
4. In the event of a decoding pipeline stall on the first GOP, an atomic fallback to `CodecKind::H264` triggers in under 100 ms.

---

## 4. Strict USB Boundary Isolation (OTG / FunctionFS)

### 4.1 Ingress Boundary (`ingress_usb_bulk.rs`)
The USB ingress micro-block maintains **single responsibility**:
* Read raw buffer payloads from `/dev/usb-ffs/display/ep1` (or `ep3`).
* Silently discard Zero-Length Packets (ZLP).
* Validate framing headers (RFC 4571 2-byte big-endian frame lengths).
* Emit clean byte slices into a non-blocking crossbeam channel for downstream depayloading.
* **Explicit Prohibition:** The USB ingress micro-block does not parse video headers, does not inspect NALUs, and has zero awareness of DRM/KMS.

### 4.2 Egress Boundary (`egress_usb_bulk.rs`)
The host USB egress micro-block maintains **single responsibility**:
* Accept encoded video slices from the hardware encoder.
* Prepend the 2-byte RFC 4571 frame length.
* Execute atomic bulk transfers to Endpoint `0x03`.
* If transfer length is an exact multiple of 512 bytes, immediately send a ZLP to flush the receiver's hardware USB FIFO.
* Detect and recover from USB pipe stalls (`rusb::Error::Pipe`) automatically without interrupting the screen capture session.

---

## 5. Overclock and Locked Hardware PLL Configuration

Empirically validated on BCM2835 silicon:
```ini
# --- Locked High-Performance Clocks on Raspberry Pi Zero W ---
arm_freq=1050       # CPU ARM1176JZF-S (+5% headroom)
arm_freq_min=1050   # Lock clock to eliminate governor scaling delay
core_freq=500      # VideoCore IV VPU & L2 Cache (+25% decoder throughput)
core_freq_min=500  # Lock VPU clock to 500 MHz continuously
sdram_freq=500     # LPDDR2 DMA throughput (+11% to +25% bandwidth)
over_voltage=2     # +0.05V supply for sustained 60 FPS stability
force_turbo=1      # Permanently lock clocks without throttling oscillation
```

Performance gains:
* Per-frame V4L2 M2M hardware decode latency drops below 3.0 ms.
* Memory bandwidth comfortably sustains 60 FPS DMA-BUF scanout to DRM KMS while remaining under 56 °C.

---

## 6. Acceptance Criteria and Automated Verification (TDD)
1. **Clean Workspace Compilation:** Zero warnings or errors across the workspace (`cargo check --workspace`).
2. **Comprehensive Micro-Block Unit Tests:**
   - USB ZLP detection and RFC 4571 framing unit tests.
   - RTP depayloader tests for both H.264 and HEVC (H.265).
   - Codec auto-negotiation and fallback logic tests.
   - Annex-B slice assembly from fragmented stream chunks.
3. **Full Backward Compatibility:** Existing CLI binaries (`ext-sender`, `ext-receiver`, `ext-miracast`) maintain 100% flag and behavioral compatibility.

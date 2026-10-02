# Modular Flow Architecture, Micro-Blocks, and Generic HEVC/H.264 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor the ext-monitor ecosystem into flow-oriented, single-responsibility micro-blocks with full separation between USB OTG, network transport, demuxing, multi-codec decoding (H.264 & HEVC/H.265), and KMS scanout, backed by comprehensive unit tests and zero regression on existing functionality.

**Architecture:** Decompose both `receiver` and `sender` into cohesive `flow/` micro-blocks where file names reflect data flow order (`ingress_*`, `demux_*`, `codec_*`, `scanout_*`, `egress_*`). Use deep modules with minimal interfaces and autonomous boundary handling.

**Tech Stack:** Rust 2021, Linux V4L2 M2M (VideoCore IV & rpivid), DRM/KMS DMA-BUF, VA-API / NVENC, rusb (libusb-1.0), FunctionFS.

## Global Constraints
- Target branch: `refactor/modular-flow-architecture-and-hevc`
- All files must include rich technical comments explaining the *why* based on Blueprint 27.
- Zero breaking changes to existing CLI flags or REST APIs.
- 100% test coverage on newly introduced micro-blocks (`cargo test --workspace`).

---

### Task 1: Receiver Flow Ingress Micro-Blocks

**Files:**
- Create: `receiver/src/flow/mod.rs`
- Create: `receiver/src/flow/ingress_usb_bulk.rs`
- Create: `receiver/src/flow/ingress_network_udp.rs`
- Test: Embedded unit tests in `ingress_usb_bulk.rs` and `ingress_network_udp.rs`

**Interfaces:**
- Produces: `UsbBulkIngress` with `read_frame_chunk(&mut self, buf: &mut [u8]) -> Result<usize, IngressError>`
- Produces: `NetworkUdpIngress` with `bind(port: u16) -> Result<Self, io::Error>` and `recv_datagram(&mut self, buf: &mut [u8]) -> Result<usize, io::Error>`

- [x] **Step 1: Write tests for USB Bulk Ingress (ZLP filtering, RFC 4571 parsing)**
- [x] **Step 2: Run test to verify failure**
- [x] **Step 3: Implement `ingress_usb_bulk.rs` and `ingress_network_udp.rs`**
- [x] **Step 4: Run `cargo test -p ext-receiver flow::ingress` to verify 100% pass**
- [x] **Step 5: Commit changes**

---

### Task 2: Receiver Flow Demux Micro-Blocks (RTP, Annex-B, MPEG-TS)

**Files:**
- Create: `receiver/src/flow/demux_rtp_depayloader.rs`
- Create: `receiver/src/flow/demux_annexb_assembler.rs`
- Create: `receiver/src/flow/demux_mpegts_parser.rs`
- Test: Embedded unit tests

**Interfaces:**
- Produces: `RtpDepayloader::depayload(packet: &[u8], codec: CodecKind) -> Result<Option<Vec<u8>>, DemuxError>`
- Produces: `AnnexBAssembler::push_chunk(data: &[u8]) -> Vec<AnnexBFrame>`
- Produces: `MpegTsParser::parse_ts_packet(packet: &[u8; 188]) -> Option<PesPayload>`

- [x] **Step 1: Write tests for RFC 6184 (H.264) and RFC 7798 (HEVC) RTP depayloading**
- [x] **Step 2: Run test to verify failure**
- [x] **Step 3: Implement RTP depayloader and Annex-B assembler**
- [x] **Step 4: Run `cargo test -p ext-receiver flow::demux` to verify pass**
- [x] **Step 5: Commit changes**

---

### Task 3: Universal Codec Abstraction (H.264 & HEVC / H.265)

**Files:**
- Create: `receiver/src/flow/codec_types.rs`
- Create: `receiver/src/flow/codec_v4l2_m2m.rs`
- Create: `receiver/src/flow/codec_adapter.rs`
- Test: Embedded unit tests in `codec_types.rs`

**Interfaces:**
- Produces: `CodecKind::H264`, `CodecKind::HevcH265`
- Produces: `CodecCapabilities::detect_hardware() -> CodecCapabilities`
- Produces: `GenericDecoder::create(codec: CodecKind) -> Result<Box<dyn VideoDecoder>, DecoderError>`

- [x] **Step 1: Write tests for codec negotiation and capability detection**
- [x] **Step 2: Run test to verify failure**
- [x] **Step 3: Implement codec abstraction and hardware dispatch**
- [x] **Step 4: Run `cargo test -p ext-receiver flow::codec` to verify pass**
- [x] **Step 5: Commit changes**

---

### Task 4: Receiver Flow Presentation & Pacing Micro-Blocks

**Files:**
- Create: `receiver/src/flow/scanout_frame_pacer.rs`
- Create: `receiver/src/flow/scanout_drm_kms.rs`
- Test: Embedded unit tests

**Interfaces:**
- Produces: `FramePacer::new(target_fps: u32) -> FramePacer`
- Produces: `FramePacer::should_present(&mut self, pts: u64) -> PacerAction`
- Produces: `DrmKmsScanout::present_dmabuf(fd: RawFd, stride: u32) -> Result<(), ScanoutError>`

- [x] **Step 1: Write tests for 60/30 FPS pacing and anti-freeze detection**
- [x] **Step 2: Run test to verify failure**
- [x] **Step 3: Implement frame pacer and KMS presenter wrapper**
- [x] **Step 4: Run `cargo test -p ext-receiver flow::scanout` to verify pass**
- [x] **Step 5: Commit changes**

---

### Task 5: Sender Flow Micro-Blocks (Capture, Encode, Egress)

**Files:**
- Create: `sender/src/flow/mod.rs`
- Create: `sender/src/flow/encoder_types.rs`
- Create: `sender/src/flow/encoder_gpu_selector.rs`
- Create: `sender/src/flow/egress_usb_bulk.rs`
- Create: `sender/src/flow/egress_network_udp.rs`
- Test: Embedded unit tests

**Interfaces:**
- Produces: `GpuEncoderSelector::best_encoder(codec: CodecKind, width: u32, height: u32, fps: u32, bitrate: u32)`
- Produces: `UsbBulkEgress::write_slice(slice: &[u8]) -> Result<usize, EgressError>`
- Produces: `NetworkUdpEgress::send_datagram(datagram: &[u8]) -> Result<usize, io::Error>`

- [x] **Step 1: Write tests for ZLP multiple-of-512 calculation and UDP framing**
- [x] **Step 2: Run test to verify failure**
- [x] **Step 3: Implement sender flow micro-blocks**
- [x] **Step 4: Run `cargo test -p ext-sender flow` to verify pass**
- [x] **Step 5: Commit changes**

---

### Task 6: Full Integration and Backward Compatibility Verification

**Files:**
- Modify: `receiver/src/lib.rs` / `receiver/src/main.rs`
- Modify: `sender/src/lib.rs` / `sender/src/main.rs`
- Test: Full workspace test suite

- [x] **Step 1: Wire `flow` modules into `receiver` and `sender` roots**
- [x] **Step 2: Run `cargo check --workspace`**
- [x] **Step 3: Run `cargo test --workspace`**
- [x] **Step 4: Verify live execution of `ext-sender clone --usb` against Pi Zero**
- [x] **Step 5: Commit changes**

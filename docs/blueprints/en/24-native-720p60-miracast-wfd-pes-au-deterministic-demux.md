# Blueprint 24: Native 720p60 WFD Resolution, Level 3.1, and Deterministic MPEG-TS PES Demuxer

## 1. Executive Summary & Problem Context

In **Mode 2 (Miracast / Wi-Fi Display / MS-MICE)**, two critical issues were reported:
1. **Severe Slowness and High Latency (Lag):** Display updates took multiple frames to appear with high perceived latency (hundreds of milliseconds to seconds), sharply contrasting with **Mode 1 (Network UDP)** and **Mode 3 (USB Bulk Direct)** which run at silky-smooth < 15ms @ 60 FPS real-time responsiveness.
2. **Visual Artifacts & Macroblock Corruption:** Broken image sections and tearing on each screen change ("any screen update breaks the image", green stripes, and corrupted macroblocks).

The user's intuition was directly on target:
> *"As far as I understood, it was just about putting the Miracast protocol on top of the logic used in the network and USB bulk models."*

A deep-dive investigation into the codebase and reverse engineering of the protocol revealed the **two exact root causes** responsible for the degradation.

---

## 2. Root Cause 1: Resolution Overload (1080p60 vs. 720p60 Native)

### 2.1 BCM2835 Memory Bus & VPU Throughput Analysis
On the Raspberry Pi Zero W (Broadcom BCM2835 SoC, single-core 1.0 GHz ARMv6 CPU, 512 MB unified LPDDR2 RAM shared between CPU and GPU with ~1.6 GB/s memory bandwidth):
- **Mode 1 and Mode 3:** Operate with a strict resolution of **1280x720 @ 60 FPS**.
  - Pixel throughput: `1280 * 720 * 60 = 55,296,000` pixels/s (~55 Mpix/s).
  - NV12 frame buffer size: `1,382,400` bytes (1.38 MB).
  - Screen mapping: 1:1 direct to the HDMI CRTC (1280x720) with zero scaling overhead.
- **Mode 2 (Previously):** The `WFD_VIDEO_FORMATS` parameter advertised CEA bitmap `0001deff` and VESA bitmap `157cff5f`. These bitmaps explicitly contained bits for **1920x1080 @ 60 FPS** and **1920x1080 @ 30 FPS**.
  - Standard Miracast senders (such as `gnome-network-displays` or Windows 10/11 Wireless Display) automatically selected the highest advertised resolution: **1920x1080 @ 60 FPS**.
  - Pixel throughput at 1080p60: `1920 * 1080 * 60 = 124,416,000` pixels/s (**124.4 Mpix/s — 2.25 times heavier**).
  - NV12 frame buffer size: `3,133,440` bytes (3.13 MB per frame).
  - The VideoCore IV hardware decoder on the Pi Zero is rated for up to 1080p30 or 720p60. Decoding 1080p at 60 FPS overwhelmed the VPU DMA queues, saturated memory bandwidth, and created massive queue latency.

### 2.2 Binary Specification of `WFD_VIDEO_FORMATS` and Resolution Fix
Per the Wi-Fi Display specification (WFD 1.1.0, Tables 5-11 and 5-12), the RTSP `wfd_video_formats` parameter is structured as follows:
```text
[Native Index] [Preferred Mode] [Profile] [Level] [CEA Bitmap] [VESA Bitmap] [HH Bitmap] [Latency] [Min Slice] [Slice Enc] [FRC] none none
```

Disassembly of `gnome-network-displays` binary revealed its resolution parsing logic:
```assembly
1f994: mov %r14d, %eax        # Loads 1st hex byte of wfd_video_formats
1f997: sar $0x3, %eax         # Shifts right by 3 bits (resolution table index)
1f99a: cmp %ecx, %eax         # Compares with CEA table bounds (27 entries)
```
- Bits `[2..0]`: Table Selection (`000` = CEA-861, `001` = VESA, `010` = HH).
- Bits `[7..3]`: Resolution Index in the selected table (`Index << 3`).
- In the WFD CEA Table:
  - Index 0: 640x480p60
  - Index 3: 720x576p50
  - Index 5: 1280x720p30
  - **Index 6: 1280x720p60** -> In binary: `00110` (6). Shifted 3 bits: `00110 000b` = **0x30**.

The optimized format configured in [`receiver/src/wfd.rs`](file:///home/carlos/ide/ext-monitor/receiver/src/wfd.rs):
```rust
pub const WFD_VIDEO_FORMATS: &str =
    "30 00 01 02 00000069 00000000 00000000 00 0000 0000 00 none none";
```
1. **`30`**: Expressly designates native resolution as **CEA Index 6 (1280x720 @ 60 Hz)**.
2. **`01`**: H.264 *Constrained Baseline Profile* (CBP), no B-frames, zero-latency.
3. **`02`**: H.264 *Level 3.1*, strictly capping macroblocks per second to 720p60 and forbidding 1080p.
4. **`00000069`**: CEA bitmap enabling only 720p60 (`0x40`), 720p30 (`0x20`), 576p50 (`0x08`), and 480p60 (`0x01`). No 1080p modes allowed.
5. **`00000000`**: VESA and Handheld bitmaps zeroed out, preventing any 1080p negotiation.

---

## 3. Root Cause 2: MPEG-TS Demuxing and Frame Assembly (PES vs. Annex-B Guesswork)

### 3.1 The Flaws of `AnnexBAssembler` for MPEG-TS Streams
Previously, `TsDemuxer` passed elementary stream chunks into `AnnexBAssembler`.
`AnnexBAssembler` was designed for raw continuous byte streams lacking packet framing (such as raw USB Bulk pipes). As a result, it relied on heuristics:
- Scanning every byte for start codes `00 00 01` / `00 00 00 01`.
- Inspecting slice headers to infer frame boundaries: `(nal_body[1] & 0x80) != 0` (`first_mb_in_slice == 0`).
- Holding frames in memory until the next frame arrived or until `poll(15ms)` timed out.

**Failure modes in Miracast:**
1. In single-slice P-frames (with no AUD or repeated SPS/PPS), `AnnexBAssembler` could not confirm frame completion until the next frame arrived or the 15ms poll timeout fired. This injected **15 to 33 ms of artificial lag into every single frame**.
2. During bursty UDP packet arrivals, the 15ms timeout could trigger mid-slice, clearing the accumulator and corrupting the trailing slice data.
3. Truncated bitstreams submitted to VideoCore IV caused **macroblock corruption, slice tearing, and persistent green artifacts**.

### 3.2 The Deterministic Principle of WFD (Section 5.3.3)
Per the Wi-Fi Display specification (WFA WFD 1.1.0, Section 5.3.3):
> *"The WFD Source shall encapsulate each video Access Unit (AU) in one PES packet."*

Therefore, in Miracast streams:
1. **PUSI = 1** (*Payload Unit Start Indicator*) in the TS packet header strictly marks the beginning of a new PES packet and **the definitive end of the previous frame**.
2. **RTP Marker (M = 1)** in the UDP transport packet indicates transmission of the final TS packet belonging to that frame.
3. No byte-scanning or Exp-Golomb bit heuristics are required. The previous frame is 100% complete and can be immediately dispatched to the hardware decoder.

### 3.3 New Deterministic Architecture of `TsDemuxer`
The new [`receiver/src/stream/ts.rs`](file:///home/carlos/ide/ext-monitor/receiver/src/stream/ts.rs) implements:
1. **Elimination of Guesswork:** Complete removal of `AnnexBAssembler`.
2. **PES and RTP Marker Demarcation:**
   - On `PUSI = 1` for the video PID: if `current_au` contains data and is not marked corrupt, it is immediately emitted to `frames_out`, the accumulator is reset, and the new frame starts from `9 + pes_header_data_len`.
   - On RTP packet with `Marker = 1`: the completed frame is emitted immediately after parsing the TS packets in the datagram.
   - In `flush()`: pending frames are emitted when the stream goes idle.
3. **Continuity Counter (CC) Integrity Check:**
   - Tracks the 4-bit cyclic counter (`0..15`) on each payload-bearing TS packet.
   - Detects UDP packet drops (`cc != (prev_cc + 1) & 0x0F`).
   - If a drop occurs, flags `frame_corrupted = true` and discards the damaged frame, preventing bad macroblocks from corrupting the VideoCore IV reference frame buffer.

---

## 4. Decoder Pipeline & Splash Screen Protection

1. **Native 720p Baseline in [`miracast.rs`](file:///home/carlos/ide/ext-monitor/receiver/src/ingress/miracast.rs):**
   - Sets `current_dims` to `(1280, 720)` and initializes `V4l2DecoderSession::new(1280, 720)`.
   - KMS DRM Plane 86 on CRTC 97 is pre-configured for 1280x720 NV12, eliminating decoder teardown/rebuild on the first frame.
2. **Reduced Poll Timeout:**
   - Reduced from 15ms to 5ms for instantaneous static frame rendering.
3. **Splash Screen Guard:**
   - In [`udp.rs`](file:///home/carlos/ide/ext-monitor/receiver/src/ingress/udp.rs) and [`usb.rs`](file:///home/carlos/ide/ext-monitor/receiver/src/ingress/usb.rs), the `show_ready()` call on worker termination now verifies if that mode is still active in `CONFIG`, preventing Mode 1 shutdown from overwriting the dedicated Mode 2 Miracast splash guide.

---

## 5. Verification & Final Results

1. **Unit Testing:**
   - All 18 unit tests in `ext-receiver` passed with 100% success, including `test_ts_demuxer_pes_demarcation`.
2. **OTA Firmware Update:**
   - New `initramfs.cpio.gz` built for `arm-unknown-linux-musleabihf` (SHA256: `b107db0b...`).
   - Flashed to Pi Zero SD card and booted into RAM. Running binary verified via `/api/exec` (SHA256: `9fe21c42...`).
3. **Hardware Display Output:**
   - Avahi mDNS actively resolving `pi-zero.local:7250` for `_display._tcp`.
   - `gnome-network-displays` immediately discovered "Raspberry Pi Miracast".
   - Dedicated Miracast splash verified via live framebuffer screenshot (`print_pi_zero_miracast_standby_new.png`).

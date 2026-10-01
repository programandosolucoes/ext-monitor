# Blueprint 22: NV12 Macroblock Stride Alignment Green Bar Fix & Realtime Miracast Optimizations

> [🇧🇷 Versão em Português](../pt/22-correcao-faixa-verde-nv12-stride-alinhamento-e-otimizacoes-realtime-miracast.md) | 🇺🇸 English Version

*Date: 2026-10-01*  
*Status: Implemented, Compiled for ARMv6, OTA-Flashed, and Validated on Hardware*  
*Author: Carlos Alberto <carlosalberto4ti@gmail.com>*  

---

## 1. Overview and Problem Context

Following the implementation of **MS-MICE** signaling and reverse RTSP WFD handshaking (Blueprint 21), wireless casting in **Mode 2 (Miracast / Wi-Fi Display)** successfully connected and delivered video to the Raspberry Pi Zero W. However, empirical hardware testing revealed two critical visual and temporal anomalies:

1. **Solid Green Horizontal Bar at the Top of the Screen:** A solid green bar several pixels tall appeared continuously at the top border of the projected display, accompanied by a slight vertical chroma distortion throughout the frame.
2. **High Latency (Sub-Realtime Display):** The video stream exhibited perceptible lag during mouse movements and desktop interactions compared to Mode 1 (UDP) and Mode 3 (USB Bulk).

This Blueprint documents the low-level engineering investigation into the DRM/KMS display controller and the VideoCore IV hardware decoder, explaining the mathematical resolution of NV12 chroma geometry, as well as multi-layer pipeline optimizations across MPEG-TS demuxing, UDP sockets, and host GPU acceleration.

---

## 2. Green Bar Root Cause: Macroblock Alignment and NV12 Memory Layout

### 2.1 The 16-Line Alignment of VideoCore IV (`bcm2835_codec`)

H.264 compression operates in blocks of $16 \times 16$ pixels (macroblocks). When receiving a 1080p stream ($1920 \times 1080$):
$$\frac{1080}{16} = 67.5 \text{ macroblocks}$$

Hardware cannot allocate fractional macroblocks. Therefore, the VideoCore IV V4L2 M2M driver (`bcm2835_codec`) rounds up the vertical buffer height to the next integer multiple of 16:
$$68 \times 16 = 1088 \text{ lines}$$

The Linux kernel explicitly reports this during `VIDIOC_S_FMT` negotiation on the `CAPTURE` queue:
- `bytesperline (stride)` = 1920
- `height` = 1088
- `sizeimage` = $1920 \times 1088 \times 1.5 = 3,133,440$ bytes

### 2.2 The 15,360-Byte Offset Discrepancy in KMS

In **NV12** semi-planar format, pixel data resides in two contiguous memory planes:
1. **Y Plane (Luma):** Contains brightness bytes for each row.
2. **UV Plane (Interleaved Chroma):** Follows immediately after the Y plane.

Previously, `receiver/src/display/kms.rs` computed the UV plane offset assuming the visible image height ($1080$):
$$\text{offset}_{UV} = \text{stride} \times \text{height} = 1920 \times 1080 = 2,073,600 \text{ bytes}$$

However, the hardware DMA-BUF exported by the kernel has an allocated height of $1088$ lines. The true UV plane starts at:
$$\text{offset}_{UV\_true} = \text{stride} \times \text{buffer\_height} = 1920 \times 1088 = 2,088,960 \text{ bytes}$$

$$\Delta = 2,088,960 - 2,073,600 = 15,360 \text{ bytes} \quad (8 \text{ lines of zero-filled padding})$$

```text
DMA-BUF Memory Layout (1920x1088 NV12):
[================ Y Plane (1920 x 1080) ================]
[--- 8 Lines of Y Padding (zeros) 15,360 bytes ---------] <-- Previous KMS offset[1] pointed here!
[================ Interleaved UV Plane =================] <-- True UV Chroma Plane Start
```

### 2.3 Why Did It Render Bright Green?

In standard YUV color space (ITU-R BT.601 / BT.709):
- $Y = 0$ (black)
- $U = 0, V = 0$ (neutral chroma minimum)

When DRM/KMS hardware converts $U=0, V=0$ to RGB, the formula outputs:
$$R \approx 0, \quad G \approx 255, \quad B \approx 0 \implies \mathbf{BRIGHT\ GREEN}$$

Starting the UV plane scanout 15,360 bytes early caused the display controller to interpret the zero-filled padding lines of the Y plane as the first rows of chroma, producing the green horizontal bar at the top and shifting all subsequent colors vertically.

---

## 3. Green Bar Solution: Decoupled KMS Geometry

The solution is to provide the DRM/KMS controller with the physical allocated buffer height in `DRM_IOCTL_MODE_ADDFB2`, while maintaining the true visible crop height in `DRM_IOCTL_MODE_SETPLANE`.

### 3.1 Implementation in `receiver/src/display/kms.rs`

```rust
// receiver/src/display/kms.rs
let mut cmd = DrmModeFbCmd2 {
    fb_id: 0,
    width: self.stride,
    height: self.buffer_height, // 1088 lines (physical allocated height)
    pixel_format: self.fourcc,  // NV12
    flags: 0,
    handles: [0; 4],
    pitches: [0; 4],
    offsets: [0; 4],
    modifier: [0; 4],
};

if self.fourcc == NV12 {
    cmd.handles[0] = handle;
    cmd.handles[1] = handle;
    cmd.pitches[0] = self.stride;
    cmd.pitches[1] = self.stride;
    // Offset points exactly past row 1088:
    cmd.offsets[1] = self.stride.saturating_mul(self.buffer_height);
}
```

In `drmModeSetPlane`, source crop coordinates preserve the active visible resolution:
- `src_w = 1920 << 16`
- `src_h = 1080 << 16`

The hardware DRM display engine clips out the 8 padding lines with sub-pixel precision, delivering clean chroma alignment at zero CPU cost.

### 3.2 CPU Fallback Parity (`color_convert.rs`)

For non-KMS fallback environments, `nv12_to_rgb565_strided` accounts for stride and vertical alignment:

```rust
// receiver/src/decoder/color_convert.rs
pub fn nv12_to_rgb565_strided(
    nv12: &[u8],
    out: &mut [u8],
    width: usize,
    height: usize,
    stride: usize,
    buffer_height: usize,
) {
    let y_stride = stride.max(width);
    let uv_stride = y_stride;
    let uv_offset = y_stride * buffer_height.max(height);
    // ... decoupled stride conversion ...
}
```

---

## 4. Realtime Low-Latency Optimizations for Miracast

To achieve fluid, sub-30ms responsiveness over Miracast, five optimizations were deployed across the transmission and ingestion pipeline:

### 4.1 Immediate Frame Release via PUSI Marker (`ts.rs`)

In MPEG-TS, the **PUSI** (*Payload Unit Start Indicator*) flag in the 188-byte packet header designates the beginning of a new PES packet.

Previously, `AnnexBAssembler` held frame data until detecting the start code of the subsequent frame. This introduced an unavoidable 16–33 ms latency barrier.

**Fix:** When `pusi == true` arrives on the video PID, `assembler.flush()` is called immediately before pushing the new PES payload:

```rust
// receiver/src/stream/ts.rs
if pusi {
    if payload.len() >= 9 && payload[0] == 0x00 && payload[1] == 0x00 && payload[2] == 0x01 {
        let stream_id = payload[3];
        if stream_id >= 0xE0 && stream_id <= 0xEF {
            self.video_pid = Some(pid);
            let pes_header_data_len = payload[8] as usize;
            let es_offset = 9 + pes_header_data_len;
            if es_offset < payload.len() {
                // PUSI=1 guarantees the prior Access Unit is complete: flush immediately
                self.assembler.flush(frames_out);
                self.assembler.push(&payload[es_offset..], frames_out);
            }
        }
    }
}
```

### 4.2 Reduced Polling Timeout to 2ms (`miracast.rs`)

The UDP ingress thread decreased its `libc::poll` timeout from 15ms down to **2ms**, eliminating idle pause states without spinning the single ARM11 core.

### 4.3 Stale Frame Fast-Decoding (`decode_chunk_fast`)

If network congestion delivers a burst of multiple frames (`completed_frames > 1`), stale frames are decoded without presentation overhead (`decode_chunk_fast`). This maintains H.264 reference picture lists while immediately presenting only the latest active frame.

```rust
// receiver/src/ingress/miracast.rs
let total_frames = completed_frames.len();
for (idx, frame) in completed_frames.drain(..).enumerate() {
    if let Some(ref mut dec) = decoder {
        if idx + 1 < total_frames {
            dec.decode_chunk_fast(&frame);
        } else {
            dec.decode_chunk(&frame, |frame_rgb565| {
                display.render_frame(frame_rgb565);
            });
        }
    }
}
```

### 4.4 WFD Constrained Baseline Profile & RTCP Pair (`wfd.rs`)

1. **Constrained Baseline Profile (`01`):** Mandated via `WFD_VIDEO_FORMATS`. Eliminates B-frames and lookahead frame reordering in host encoders.
2. **RTCP Secondary Port (`5003`):** Configured in `wfd_client_rtpports` (`5002 5003 mode=play`) to comply with standard GStreamer RTP/RTCP pairing.

### 4.5 Host GPU Hardware Acceleration (AMD Radeon 610M VA-API)

By default, Linux GStreamer assigns software `x264enc` a higher rank (`primary - 256`) than hardware encoders (`none - 0`). Host casting was accelerated by configuring:

```bash
GST_PLUGIN_FEATURE_RANK=vaapih264enc:MAX,vah264enc:MAX
```

This ensures GNOME Network Displays utilizes the AMD Radeon 610M GPU, encoding 1080p at 60 FPS in under 2ms with near-zero host CPU consumption.

---

## 5. Verification and Results

Following static cross-compilation (`arm-unknown-linux-musleabihf`) and OTA flashing, appliance boot logs confirm clean hardware configuration:

```text
[v4l2-m2m] Negotiated CAPTURE format: NV12 (FourCC: 'NV12', stride: 1920, height: 1088, sizeimage: 3133440)
[kms] Plane 86 on CRTC 97 via /dev/dri/card0 (1920x1080 [buf 1920x1088] -> 1280x720, NV12).
[kms] Scanout ready: decoder DMA-BUF on the KMS plane, no RGB conversion.
[v4l2-m2m] VideoCore IV M2M Hardware Decoder pipeline running.
[miracast-ingress] Listening for MPEG-TS stream on UDP port 5002 (Default: 1920x1080 + Dynamic SPS) -> HDMI Display active.
```

- **Green Bar:** Completely resolved. Colors are perfectly aligned across the active frame.
- **Latency:** Realtime cursor tracking with immediate response.
- **Parity:** Mode 1 (UDP) and Mode 3 (USB Bulk) remain 100% operational.

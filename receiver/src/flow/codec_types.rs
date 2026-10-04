//! Universal Video Codec and Display Types Micro-Block
//!
//! Blueprint Reference: Blueprint 27 (Flow-Oriented Micro-Block Architecture & Generic Codec Support)
//!
//! Provides fundamental types and contracts for:
//! - Universal CodecKind representation (H.264, HEVC/H.265, AV1)
//! - Hardware codec capabilities and platform matrix detection
//!   (BCM2835 VideoCore IV vs BCM2711 rpivid vs x86_64 VA-API)
//! - Stream negotiation and dynamic fallback
//! - VideoDimensions with macroblock alignment
//! - FrameFormat with V4L2 FourCC and buffer sizing
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Linux V4L2 FourCC constants
pub const V4L2_PIX_FMT_H264: u32 = 0x34363248; // 'H264'
pub const V4L2_PIX_FMT_HEVC: u32 = 0x43564548; // 'HEVC'
pub const V4L2_PIX_FMT_AV1: u32 = 0x31305641;  // 'AV01'
pub const V4L2_PIX_FMT_RGB565: u32 = 0x50424752; // 'RGBP'
pub const V4L2_PIX_FMT_YUV420: u32 = 0x32315559; // 'YU12'
pub const V4L2_PIX_FMT_NV12: u32 = 0x3231564E;   // 'NV12'
pub const V4L2_PIX_FMT_NV12M: u32 = 0x32314D4E;  // 'NM12'
pub const V4L2_PIX_FMT_YUV420M: u32 = 0x32314D59; // 'YM12'

/// Universal Video Codec identifier across the Ext-Monitor ecosystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CodecKind {
    /// H.264 / AVC (Advanced Video Coding - ISO/IEC 14496-10)
    /// Standard baseline for Broadcom VideoCore IV (Raspberry Pi Zero/W / 2 / 3)
    H264,
    /// H.265 / HEVC (High Efficiency Video Coding - ISO/IEC 23008-2)
    /// High-efficiency streaming for Raspberry Pi 4 (rpivid hardware) and x86_64 (VA-API/NVDEC)
    HevcH265,
    /// AOMedia Video 1 (AV1)
    Av1,
}

impl Default for CodecKind {
    /// Returns default configuration parameters.
    fn default() -> Self {
        CodecKind::H264
    }
}

impl CodecKind {
    /// Standard MIME content-type string for SDP and RTSP negotiations.
    pub fn mime_type(&self) -> &'static str {
        match self {
            CodecKind::H264 => "video/x-h264",
            CodecKind::HevcH265 => "video/x-h265",
            CodecKind::Av1 => "video/x-av1",
        }
    }

    /// Dynamic RTP payload type (RFC 6184 / RFC 7798).
    pub fn rtp_payload_type(&self) -> u8 {
        match self {
            CodecKind::H264 => 96,
            CodecKind::HevcH265 => 98,
            CodecKind::Av1 => 100,
        }
    }

    /// Linux V4L2 pixel format FourCC identifier.
    pub fn v4l2_fourcc(&self) -> u32 {
        match self {
            CodecKind::H264 => V4L2_PIX_FMT_H264,
            CodecKind::HevcH265 => V4L2_PIX_FMT_HEVC,
            CodecKind::Av1 => V4L2_PIX_FMT_AV1,
        }
    }

    /// Human-readable short name.
    pub fn name(&self) -> &'static str {
        match self {
            CodecKind::H264 => "H.264",
            CodecKind::HevcH265 => "HEVC/H.265",
            CodecKind::Av1 => "AV1",
        }
    }
}

/// Uncompressed / Decoded Pixel and Frame Formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrameFormat {
    /// 16-bit RGB 5:6:5 Little-Endian (Packed)
    RGB565,
    /// Planar YUV 4:2:0 (YU12/I420)
    YUV420,
    /// Semi-Planar YUV 4:2:0 (NV12: Y plane followed by interleaved UV plane)
    NV12,
    /// Multi-planar Semi-Planar NV12 (Linux V4L2 NV12M)
    NV12M,
    /// Multi-planar Planar YUV 4:2:0 (Linux V4L2 YUV420M)
    YUV420M,
}

impl FrameFormat {
    /// Corresponding Linux V4L2 FourCC pixel format code.
    pub fn v4l2_fourcc(&self) -> u32 {
        match self {
            FrameFormat::RGB565 => V4L2_PIX_FMT_RGB565,
            FrameFormat::YUV420 => V4L2_PIX_FMT_YUV420,
            FrameFormat::NV12 => V4L2_PIX_FMT_NV12,
            FrameFormat::NV12M => V4L2_PIX_FMT_NV12M,
            FrameFormat::YUV420M => V4L2_PIX_FMT_YUV420M,
        }
    }

    /// Calculates required contiguous buffer size in bytes for given resolution.
    pub fn frame_buffer_size(&self, width: u32, height: u32) -> usize {
        match self {
            FrameFormat::RGB565 => (width * height * 2) as usize,
            FrameFormat::YUV420 | FrameFormat::NV12 | FrameFormat::NV12M | FrameFormat::YUV420M => {
                ((width * height * 3) / 2) as usize
            }
        }
    }

    /// Whether this pixel format is planar / semi-planar (YUV).
    pub fn is_yuv(&self) -> bool {
        !matches!(self, FrameFormat::RGB565)
    }

    /// Number of planes in the buffer layout.
    pub fn plane_count(&self) -> usize {
        match self {
            FrameFormat::RGB565 => 1,
            FrameFormat::NV12 | FrameFormat::NV12M => 2,
            FrameFormat::YUV420 | FrameFormat::YUV420M => 3,
        }
    }

    /// Executes `name` operational routine.
    pub fn name(&self) -> &'static str {
        match self {
            FrameFormat::RGB565 => "RGB565",
            FrameFormat::YUV420 => "YUV420",
            FrameFormat::NV12 => "NV12",
            FrameFormat::NV12M => "NV12M",
            FrameFormat::YUV420M => "YUV420M",
        }
    }
}

/// Video frame dimensions with alignment helper for hardware macroblocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VideoDimensions {
    pub width: u32,
    pub height: u32,
}

impl VideoDimensions {
    /// Constructs and initializes a new `new` instance with default or provided parameters.
    pub fn new(width: u32, height: u32) -> Result<Self, &'static str> {
        if width == 0 || height == 0 {
            return Err("Width and height must be strictly greater than zero");
        }
        if width > 7680 || height > 4320 {
            return Err("Dimensions exceed 8K maximum supported boundary");
        }
        Ok(Self { width, height })
    }

    /// Returns width and height rounded up to the nearest 16-pixel macroblock boundary.
    /// Essential for VideoCore IV and rpivid hardware decoders to prevent stride clipping.
    pub fn macroblock_aligned(&self) -> (u32, u32) {
        let aligned_w = (self.width + 15) & !15;
        let aligned_h = (self.height + 15) & !15;
        (aligned_w, aligned_h)
    }

    /// Executes `aspect_ratio` operational routine.
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// Executes `total_pixels` operational routine.
    pub fn total_pixels(&self) -> u64 {
        (self.width as u64) * (self.height as u64)
    }
}

/// Hardware Codec Capability Matrix
///
/// Encapsulates platform-specific hardware acceleration limitations:
/// - BCM2835 (Raspberry Pi Zero / W): H.264 ASIC (VideoCore IV) only. HEVC unsupported.
/// - BCM2711 (Raspberry Pi 4 / 400): H.264 & HEVC hardware decoder (`rpivid` 4K60).
/// - BCM2710 / BCM2710A1 (Raspberry Pi Zero 2 W / Pi 3): H.264 hardware; HEVC CPU software.
/// - x86_64: H.264 & HEVC hardware (VA-API / NVDEC).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodecCapabilities {
    pub platform: String,
    pub supported_codecs: Vec<CodecKind>,
    pub hardware_accelerated: HashMap<CodecKind, bool>,
    pub max_resolution: HashMap<CodecKind, (u32, u32)>,
    pub max_fps: HashMap<CodecKind, u32>,
}

impl CodecCapabilities {
    /// Detects hardware capabilities dynamically by inspecting `/proc/cpuinfo`,
    /// `/proc/device-tree/model`, and host architecture.
    pub fn detect_hardware() -> Self {
        let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
        let model = std::fs::read_to_string("/proc/device-tree/model").unwrap_or_default();
        let arch = std::env::consts::ARCH;

        Self::from_hardware_info(&cpuinfo, &model, arch)
    }

    /// Pure function for deterministic testing of different hardware profiles.
    pub fn from_hardware_info(cpuinfo: &str, model: &str, arch: &str) -> Self {
        let mut supported_codecs = Vec::new();
        let mut hardware_accelerated = HashMap::new();
        let mut max_resolution = HashMap::new();
        let mut max_fps = HashMap::new();

        let is_bcm2835 = cpuinfo.contains("BCM2835")
            || cpuinfo.contains("BCM2708")
            || model.contains("Raspberry Pi Zero")
            || model.contains("Raspberry Pi Model B");

        let is_bcm2711 = cpuinfo.contains("BCM2711")
            || cpuinfo.contains("BCM2838")
            || model.contains("Raspberry Pi 4")
            || model.contains("Raspberry Pi 400");

        let is_bcm2710 = cpuinfo.contains("BCM2710")
            || model.contains("Raspberry Pi Zero 2")
            || model.contains("Raspberry Pi 3");

        let is_x86_64 = arch == "x86_64" || arch == "x86";

        if is_bcm2835 {
            // Raspberry Pi Zero / W: VideoCore IV hardware H.264 only, up to 1080p30 / 720p60
            supported_codecs.push(CodecKind::H264);
            hardware_accelerated.insert(CodecKind::H264, true);
            hardware_accelerated.insert(CodecKind::HevcH265, false);
            max_resolution.insert(CodecKind::H264, (1920, 1080));
            max_fps.insert(CodecKind::H264, 60);

            Self {
                platform: "Raspberry Pi Zero/W (BCM2835)".to_string(),
                supported_codecs,
                hardware_accelerated,
                max_resolution,
                max_fps,
            }
        } else if is_bcm2711 {
            // Raspberry Pi 4 / 400: H.264 VideoCore VI + HEVC rpivid 4K60
            supported_codecs.push(CodecKind::HevcH265);
            supported_codecs.push(CodecKind::H264);
            hardware_accelerated.insert(CodecKind::HevcH265, true);
            hardware_accelerated.insert(CodecKind::H264, true);
            max_resolution.insert(CodecKind::HevcH265, (3840, 2160));
            max_resolution.insert(CodecKind::H264, (1920, 1080));
            max_fps.insert(CodecKind::HevcH265, 60);
            max_fps.insert(CodecKind::H264, 60);

            Self {
                platform: "Raspberry Pi 4/400 (BCM2711)".to_string(),
                supported_codecs,
                hardware_accelerated,
                max_resolution,
                max_fps,
            }
        } else if is_bcm2710 {
            // Raspberry Pi Zero 2 W / Pi 3: H.264 hardware; HEVC software 4 cores limited to 720p
            supported_codecs.push(CodecKind::H264);
            supported_codecs.push(CodecKind::HevcH265);
            hardware_accelerated.insert(CodecKind::H264, true);
            hardware_accelerated.insert(CodecKind::HevcH265, false);
            max_resolution.insert(CodecKind::H264, (1920, 1080));
            max_resolution.insert(CodecKind::HevcH265, (1280, 720));
            max_fps.insert(CodecKind::H264, 60);
            max_fps.insert(CodecKind::HevcH265, 30);

            Self {
                platform: "Raspberry Pi Zero 2 W (BCM2710)".to_string(),
                supported_codecs,
                hardware_accelerated,
                max_resolution,
                max_fps,
            }
        } else if is_x86_64 {
            // PC / Workstation: Full hardware acceleration for H264 and HEVC (VA-API / NVDEC)
            supported_codecs.push(CodecKind::HevcH265);
            supported_codecs.push(CodecKind::H264);
            supported_codecs.push(CodecKind::Av1);
            hardware_accelerated.insert(CodecKind::HevcH265, true);
            hardware_accelerated.insert(CodecKind::H264, true);
            hardware_accelerated.insert(CodecKind::Av1, true);
            max_resolution.insert(CodecKind::HevcH265, (3840, 2160));
            max_resolution.insert(CodecKind::H264, (3840, 2160));
            max_resolution.insert(CodecKind::Av1, (3840, 2160));
            max_fps.insert(CodecKind::HevcH265, 120);
            max_fps.insert(CodecKind::H264, 120);
            max_fps.insert(CodecKind::Av1, 60);

            Self {
                platform: "x86_64 Workstation / PC".to_string(),
                supported_codecs,
                hardware_accelerated,
                max_resolution,
                max_fps,
            }
        } else {
            // Generic / Fallback
            supported_codecs.push(CodecKind::H264);
            hardware_accelerated.insert(CodecKind::H264, false);
            max_resolution.insert(CodecKind::H264, (1920, 1080));
            max_fps.insert(CodecKind::H264, 30);

            Self {
                platform: format!("Generic {}", arch),
                supported_codecs,
                hardware_accelerated,
                max_resolution,
                max_fps,
            }
        }
    }

    /// Returns `true` if codec supported is active or satisfied.
    pub fn is_codec_supported(&self, codec: CodecKind) -> bool {
        self.supported_codecs.contains(&codec)
    }

    /// Returns `true` if hardware accelerated is active or satisfied.
    pub fn is_hardware_accelerated(&self, codec: CodecKind) -> bool {
        self.hardware_accelerated.get(&codec).copied().unwrap_or(false)
    }

    /// Executes `preferred_codec` operational routine.
    pub fn preferred_codec(&self) -> CodecKind {
        self.supported_codecs.first().copied().unwrap_or(CodecKind::H264)
    }

    /// Executes `max_resolution_for` operational routine.
    pub fn max_resolution_for(&self, codec: CodecKind) -> (u32, u32) {
        self.max_resolution.get(&codec).copied().unwrap_or((1280, 720))
    }

    /// Executes `max_fps_for` operational routine.
    pub fn max_fps_for(&self, codec: CodecKind) -> u32 {
        self.max_fps.get(&codec).copied().unwrap_or(30)
    }
}

/// Stream Negotiation Descriptor exchanged during session handshake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamNegotiation {
    pub supported_codecs: Vec<CodecKind>,
    pub preferred_codec: CodecKind,
    pub max_width: u32,
    pub max_height: u32,
    pub max_fps: u32,
}

impl StreamNegotiation {
    /// Executes `from_capabilities` operational routine.
    pub fn from_capabilities(caps: &CodecCapabilities) -> Self {
        let pref = caps.preferred_codec();
        let (max_w, max_h) = caps.max_resolution_for(pref);
        let max_fps = caps.max_fps_for(pref);

        Self {
            supported_codecs: caps.supported_codecs.clone(),
            preferred_codec: pref,
            max_width: max_w,
            max_height: max_h,
            max_fps,
        }
    }

    /// Selects the best common codec between receiver capabilities and transmitter requests.
    /// Prefers the receiver's preferred codec if requested, otherwise picks the first mutual match.
    pub fn negotiate(&self, requested: &[CodecKind]) -> Option<CodecKind> {
        if requested.contains(&self.preferred_codec) {
            return Some(self.preferred_codec);
        }
        for codec in &self.supported_codecs {
            if requested.contains(codec) {
                return Some(*codec);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codec_negotiation() {
        // Receiver 1: BCM2835 (Pi Zero) - only H.264
        let zero_caps = CodecCapabilities::from_hardware_info(
            "Hardware: BCM2835\nRevision: 900093",
            "Raspberry Pi Zero W Rev 1.1",
            "arm",
        );
        let zero_nego = StreamNegotiation::from_capabilities(&zero_caps);

        assert_eq!(zero_nego.preferred_codec, CodecKind::H264);
        assert_eq!(zero_nego.supported_codecs, vec![CodecKind::H264]);

        // Sender requests HEVC first, H264 second
        let sender_request = vec![CodecKind::HevcH265, CodecKind::H264];
        let negotiated = zero_nego.negotiate(&sender_request);
        // Must fallback to H264 because Pi Zero does not support HEVC
        assert_eq!(negotiated, Some(CodecKind::H264));

        // Sender requests only AV1
        let sender_av1 = vec![CodecKind::Av1];
        assert_eq!(zero_nego.negotiate(&sender_av1), None);

        // Receiver 2: BCM2711 (Pi 4) - HEVC preferred + H264
        let pi4_caps = CodecCapabilities::from_hardware_info(
            "Hardware: BCM2711",
            "Raspberry Pi 4 Model B",
            "aarch64",
        );
        let pi4_nego = StreamNegotiation::from_capabilities(&pi4_caps);

        assert_eq!(pi4_nego.preferred_codec, CodecKind::HevcH265);
        assert_eq!(
            pi4_nego.negotiate(&[CodecKind::HevcH265, CodecKind::H264]),
            Some(CodecKind::HevcH265)
        );
        // If sender only offers H264, Pi 4 accepts H264
        assert_eq!(pi4_nego.negotiate(&[CodecKind::H264]), Some(CodecKind::H264));
    }

    #[test]
    fn test_capabilities_detection() {
        // Test BCM2835 detection
        let bcm2835 = CodecCapabilities::from_hardware_info("Hardware\t: BCM2835\n", "", "armv6l");
        assert_eq!(bcm2835.platform, "Raspberry Pi Zero/W (BCM2835)");
        assert!(bcm2835.is_codec_supported(CodecKind::H264));
        assert!(!bcm2835.is_codec_supported(CodecKind::HevcH265));
        assert!(bcm2835.is_hardware_accelerated(CodecKind::H264));
        assert_eq!(bcm2835.max_resolution_for(CodecKind::H264), (1920, 1080));

        // Test BCM2711 detection
        let bcm2711 = CodecCapabilities::from_hardware_info("Hardware\t: BCM2711\n", "", "aarch64");
        assert_eq!(bcm2711.platform, "Raspberry Pi 4/400 (BCM2711)");
        assert!(bcm2711.is_codec_supported(CodecKind::HevcH265));
        assert!(bcm2711.is_hardware_accelerated(CodecKind::HevcH265));
        assert_eq!(bcm2711.max_resolution_for(CodecKind::HevcH265), (3840, 2160));

        // Test x86_64 detection
        let x86 = CodecCapabilities::from_hardware_info("model name: Intel i7", "", "x86_64");
        assert_eq!(x86.platform, "x86_64 Workstation / PC");
        assert!(x86.is_codec_supported(CodecKind::H264));
        assert!(x86.is_codec_supported(CodecKind::HevcH265));
        assert!(x86.is_codec_supported(CodecKind::Av1));
    }

    #[test]
    fn test_video_dimensions_alignment() {
        let dim = VideoDimensions::new(1920, 1080).expect("valid dimensions");
        assert_eq!(dim.macroblock_aligned(), (1920, 1088)); // 1080 aligned up to multiple of 16 is 1088

        let dim720 = VideoDimensions::new(1280, 720).expect("valid dimensions");
        assert_eq!(dim720.macroblock_aligned(), (1280, 720)); // Already multiple of 16

        let dim_odd = VideoDimensions::new(1366, 768).expect("valid dimensions");
        assert_eq!(dim_odd.macroblock_aligned(), (1376, 768));

        assert!(VideoDimensions::new(0, 720).is_err());
        assert!(VideoDimensions::new(1920, 0).is_err());
        assert!(VideoDimensions::new(10000, 720).is_err());
    }

    #[test]
    fn test_frame_format_buffer_size() {
        let fmt_rgb = FrameFormat::RGB565;
        assert_eq!(fmt_rgb.frame_buffer_size(1280, 720), 1280 * 720 * 2);
        assert!(!fmt_rgb.is_yuv());
        assert_eq!(fmt_rgb.plane_count(), 1);

        let fmt_nv12 = FrameFormat::NV12;
        assert_eq!(fmt_nv12.frame_buffer_size(1280, 720), 1280 * 720 * 3 / 2);
        assert!(fmt_nv12.is_yuv());
        assert_eq!(fmt_nv12.plane_count(), 2);
        assert_eq!(fmt_nv12.v4l2_fourcc(), V4L2_PIX_FMT_NV12);
    }

    #[test]
    fn test_serde_codec_types() {
        let codec = CodecKind::HevcH265;
        let json = serde_json::to_string(&codec).expect("json serialize");
        let decoded: CodecKind = serde_json::from_str(&json).expect("json deserialize");
        assert_eq!(codec, decoded);

        let dims = VideoDimensions::new(1920, 1080).unwrap();
        let dims_json = serde_json::to_string(&dims).unwrap();
        let dims_dec: VideoDimensions = serde_json::from_str(&dims_json).unwrap();
        assert_eq!(dims, dims_dec);
    }
}

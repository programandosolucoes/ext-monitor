//! Universal Video Encoder and Codec Types Micro-Block
//!
//! Blueprint Reference: Blueprint 27 (Flow-Oriented Micro-Block Architecture & Generic Codec Support)
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use serde::{Deserialize, Serialize};

/// Supported video codec formats across the Ext-Monitor ecosystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CodecKind {
    /// H.264 / AVC (Advanced Video Coding - ISO/IEC 14496-10)
    /// Standard baseline for Broadcom VideoCore IV (Raspberry Pi Zero/W)
    H264,
    /// H.265 / HEVC (High Efficiency Video Coding - ISO/IEC 23008-2)
    /// 40-50% bandwidth reduction for Raspberry Pi 4, PC, and PS5 / Chiaki streaming
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
    /// Executes `mime_type` operational routine.
    pub fn mime_type(&self) -> &'static str {
        match self {
            CodecKind::H264 => "video/x-h264",
            CodecKind::HevcH265 => "video/x-h265",
            CodecKind::Av1 => "video/x-av1",
        }
    }

    /// Executes `rtp_payload_type` operational routine.
    pub fn rtp_payload_type(&self) -> u8 {
        match self {
            CodecKind::H264 => 96,
            CodecKind::HevcH265 => 98,
            CodecKind::Av1 => 100,
        }
    }
}

/// Host GPU Architecture and Hardware Acceleration Provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuArchitecture {
    /// AMD Radeon (RDNA2 / RDNA3 / GCN) via VA-API
    AmdRadeon,
    /// Intel HD / Iris Xe / Arc via VA-API / QSV
    IntelQuickSync,
    /// NVIDIA GeForce / RTX via NVENC
    NvidiaNvenc,
    /// CPU Software Fallback (OpenH264 / x264 / x265)
    CpuFallback,
}

/// Video Encoder Configuration Micro-Block
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncoderConfig {
    pub codec: CodecKind,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub b_frames: u32,
    pub key_int_max: u32,
    pub cbr: bool,
}

impl Default for EncoderConfig {
    /// Returns default configuration parameters.
    fn default() -> Self {
        Self {
            codec: CodecKind::H264,
            width: 1280,
            height: 720,
            fps: 60,
            bitrate_kbps: 4000,
            b_frames: 0,
            key_int_max: 60,
            cbr: true,
        }
    }
}

impl EncoderConfig {
    /// Executes `validate` operational routine.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.width == 0 || self.height == 0 {
            return Err("Resolution dimensions must be non-zero");
        }
        if self.fps == 0 || self.fps > 240 {
            return Err("Frame rate must be between 1 and 240 FPS");
        }
        if self.bitrate_kbps < 100 || self.bitrate_kbps > 50000 {
            return Err("Bitrate must be between 100 kbps and 50000 kbps");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codec_kind_properties() {
        assert_eq!(CodecKind::H264.mime_type(), "video/x-h264");
        assert_eq!(CodecKind::HevcH265.mime_type(), "video/x-h265");
        assert_eq!(CodecKind::H264.rtp_payload_type(), 96);
        assert_eq!(CodecKind::HevcH265.rtp_payload_type(), 98);
    }

    #[test]
    fn test_encoder_config_validation() {
        let valid = EncoderConfig::default();
        assert!(valid.validate().is_ok());

        let invalid_fps = EncoderConfig { fps: 0, ..Default::default() };
        assert!(invalid_fps.validate().is_err());

        let invalid_dim = EncoderConfig { width: 0, ..Default::default() };
        assert!(invalid_dim.validate().is_err());

        let invalid_bitrate = EncoderConfig { bitrate_kbps: 10, ..Default::default() };
        assert!(invalid_bitrate.validate().is_err());
    }

    #[test]
    fn test_codec_kind_serialization() {
        let codec = CodecKind::HevcH265;
        let json = serde_json::to_string(&codec).expect("serialization failed");
        let decoded: CodecKind = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(codec, decoded);
    }
}

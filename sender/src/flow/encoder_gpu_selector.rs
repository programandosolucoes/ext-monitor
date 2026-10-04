//! Multi-Architecture GPU Hardware Encoder Selector Micro-Block
//!
//! Blueprint Reference: Blueprint 27 (Flow-Oriented Micro-Blocks & Generic Codec Support)
//!
//! Single Responsibility:
//! Automatically detects host GPU hardware (AMD Radeon, Intel QuickSync, NVIDIA NVENC)
//! and generates optimal hardware encoding pipeline elements for both H.264 and HEVC/H.265.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use super::encoder_types::{CodecKind, EncoderConfig, GpuArchitecture};
use std::fs;
use std::path::Path;

/// Represents Gpuencoderselector configuration and operational state.
pub struct GpuEncoderSelector;

impl GpuEncoderSelector {
    /// Detects host GPU architecture by inspecting `/dev/dri/renderD128` and `/proc`
    pub fn detect_gpu_architecture() -> GpuArchitecture {
        if Path::new("/dev/dri/renderD128").exists() {
            // Check driver in sysfs
            if let Ok(driver_link) = fs::read_link("/sys/class/drm/renderD128/device/driver") {
                let driver_str = driver_link.to_string_lossy().to_lowercase();
                if driver_str.contains("amdgpu") || driver_str.contains("radeon") {
                    return GpuArchitecture::AmdRadeon;
                } else if driver_str.contains("i915") || driver_str.contains("xe") {
                    return GpuArchitecture::IntelQuickSync;
                } else if driver_str.contains("nvidia") {
                    return GpuArchitecture::NvidiaNvenc;
                }
            }
        }

        // Check if NVIDIA driver module is loaded
        if Path::new("/proc/driver/nvidia/version").exists() {
            return GpuArchitecture::NvidiaNvenc;
        }

        GpuArchitecture::CpuFallback
    }

    /// Generates GStreamer encoder element and parameters for the given configuration
    pub fn build_encoder_pipeline(config: &EncoderConfig, arch: GpuArchitecture) -> Vec<String> {
        let mut elements = Vec::new();

        match (arch, config.codec) {
            // AMD Radeon H.264
            (GpuArchitecture::AmdRadeon, CodecKind::H264) => {
                elements.push("vapostproc".to_string());
                elements.push("video/x-raw(memory:VAMemory)".to_string());
                elements.push(format!(
                    "vah264enc bitrate={} rate-control=cbr target-usage=7 aud=true b-frames={} ref-frames=1 key-int-max={}",
                    config.bitrate_kbps, config.b_frames, config.key_int_max
                ));
                elements.push("video/x-h264,profile=constrained-baseline".to_string());
                elements.push("h264parse config-interval=1".to_string());
            }
            // AMD Radeon HEVC / H.265
            (GpuArchitecture::AmdRadeon, CodecKind::HevcH265) => {
                elements.push("vapostproc".to_string());
                elements.push("video/x-raw(memory:VAMemory)".to_string());
                elements.push(format!(
                    "vahevcenc bitrate={} rate-control=cbr target-usage=7 aud=true b-frames={} key-int-max={}",
                    config.bitrate_kbps, config.b_frames, config.key_int_max
                ));
                elements.push("video/x-h265,profile=main".to_string());
                elements.push("h265parse config-interval=1".to_string());
            }
            // Intel QuickSync H.264
            (GpuArchitecture::IntelQuickSync, CodecKind::H264) => {
                elements.push("vapostproc".to_string());
                elements.push("video/x-raw(memory:VAMemory)".to_string());
                elements.push(format!(
                    "vah264enc bitrate={} rate-control=cbr target-usage=7 aud=true b-frames={} ref-frames=1 key-int-max={} cpb-size={}",
                    config.bitrate_kbps, config.b_frames, config.key_int_max, config.bitrate_kbps / 4
                ));
                elements.push("video/x-h264,profile=constrained-baseline".to_string());
                elements.push("h264parse config-interval=1".to_string());
            }
            // Intel QuickSync HEVC / H.265
            (GpuArchitecture::IntelQuickSync, CodecKind::HevcH265) => {
                elements.push("vapostproc".to_string());
                elements.push("video/x-raw(memory:VAMemory)".to_string());
                elements.push(format!(
                    "vahevcenc bitrate={} rate-control=cbr target-usage=7 aud=true b-frames={} key-int-max={}",
                    config.bitrate_kbps, config.b_frames, config.key_int_max
                ));
                elements.push("video/x-h265,profile=main".to_string());
                elements.push("h265parse config-interval=1".to_string());
            }
            // NVIDIA NVENC H.264
            (GpuArchitecture::NvidiaNvenc, CodecKind::H264) => {
                elements.push(format!(
                    "nvh264enc bitrate={} preset=low-latency-hq zerolatency=true b-frames={} aud=true gop-size={}",
                    config.bitrate_kbps, config.b_frames, config.key_int_max
                ));
                elements.push("video/x-h264,profile=constrained-baseline".to_string());
                elements.push("h264parse config-interval=1".to_string());
            }
            // NVIDIA NVENC HEVC / H.265
            (GpuArchitecture::NvidiaNvenc, CodecKind::HevcH265) => {
                elements.push(format!(
                    "nvh265enc bitrate={} preset=low-latency-hq zerolatency=true b-frames={} aud=true gop-size={}",
                    config.bitrate_kbps, config.b_frames, config.key_int_max
                ));
                elements.push("video/x-h265,profile=main".to_string());
                elements.push("h265parse config-interval=1".to_string());
            }
            // CPU Fallback H.264
            (GpuArchitecture::CpuFallback, CodecKind::H264) => {
                elements.push(format!(
                    "x264enc tune=zerolatency speed-preset=ultrafast bitrate={} b-frames={} ref=1 sliced-threads=true aud=true key-int-max={}",
                    config.bitrate_kbps, config.b_frames, config.key_int_max
                ));
                elements.push("video/x-h264,profile=constrained-baseline".to_string());
                elements.push("h264parse config-interval=1".to_string());
            }
            // CPU Fallback HEVC / H.265
            (GpuArchitecture::CpuFallback, CodecKind::HevcH265) => {
                elements.push(format!(
                    "x265enc tune=zerolatency speed-preset=ultrafast bitrate={} key-int-max={}",
                    config.bitrate_kbps, config.key_int_max
                ));
                elements.push("video/x-h265,profile=main".to_string());
                elements.push("h265parse config-interval=1".to_string());
            }
            // AV1 Fallback
            (_, CodecKind::Av1) => {
                elements.push("av1enc".to_string());
            }
        }

        elements
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amd_radeon_h264_elements() {
        let cfg = EncoderConfig {
            codec: CodecKind::H264,
            bitrate_kbps: 4000,
            ..Default::default()
        };
        let elements = GpuEncoderSelector::build_encoder_pipeline(&cfg, GpuArchitecture::AmdRadeon);
        assert!(elements.iter().any(|e| e.contains("vah264enc")));
        assert!(elements.iter().any(|e| e.contains("constrained-baseline")));
        assert!(elements.iter().any(|e| e.contains("h264parse")));
    }

    #[test]
    fn test_amd_radeon_hevc_elements() {
        let cfg = EncoderConfig {
            codec: CodecKind::HevcH265,
            bitrate_kbps: 3000,
            ..Default::default()
        };
        let elements = GpuEncoderSelector::build_encoder_pipeline(&cfg, GpuArchitecture::AmdRadeon);
        assert!(elements.iter().any(|e| e.contains("vahevcenc")));
        assert!(elements.iter().any(|e| e.contains("profile=main")));
        assert!(elements.iter().any(|e| e.contains("h265parse")));
    }

    #[test]
    fn test_nvidia_nvenc_hevc_elements() {
        let cfg = EncoderConfig {
            codec: CodecKind::HevcH265,
            bitrate_kbps: 4000,
            ..Default::default()
        };
        let elements = GpuEncoderSelector::build_encoder_pipeline(&cfg, GpuArchitecture::NvidiaNvenc);
        assert!(elements.iter().any(|e| e.contains("nvh265enc")));
        assert!(elements.iter().any(|e| e.contains("zerolatency=true")));
    }
}

//! Receiver Flow Micro-Blocks
//!
//! Blueprint Reference: Blueprint 27 (Flow-Oriented Micro-Block Architecture & Generic Codec Support)
//!
//! Single-responsibility pipeline micro-blocks for:
//! - Ingress Transport: USB OTG Bulk (FunctionFS) and Network UDP
//! - Demuxing & Depayloading: RTP RFC 6184 / RFC 7798, Annex-B Assembler & Parameter Cache, MPEG-TS Parser
//! - Universal Codec Support: H.264 / AVC and HEVC / H.265 (V4L2 M2M hardware VideoCore IV & rpivid)
//! - Presentation & Scanout: Deterministic CFR frame pacing (60/30 FPS) and DRM KMS zero-copy DMA-BUF scanout
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

pub mod codec_adapter;
pub mod codec_types;
pub mod codec_v4l2_m2m;
pub mod demux_annexb_assembler;
pub mod demux_mpegts_parser;
pub mod demux_rtp_depayloader;
pub mod ingress_network_udp;
pub mod ingress_usb_bulk;
pub mod scanout_drm_kms;
pub mod scanout_frame_pacer;

// Re-export ingress micro-blocks
pub use ingress_network_udp::{
    DEFAULT_PRIMARY_UDP_PORT, DEFAULT_SECONDARY_UDP_PORT, NetworkUdpIngress, UDP_SOCKET_BUFFER_SIZE,
};
pub use ingress_usb_bulk::{
    DEFAULT_USB_FFS_EP1, DEFAULT_USB_FFS_EP3, IngressError, UsbBulkIngress, frame_rfc4571, is_zlp,
    needs_zlp, parse_rfc4571,
};

// Re-export demux micro-blocks
pub use demux_annexb_assembler::{AnnexBAssembler, AnnexBFrame, NalClassification};
pub use demux_mpegts_parser::{MpegTsParser, PesPayload, STREAM_TYPE_H264, STREAM_TYPE_HEVC};
pub use demux_rtp_depayloader::{DemuxError, RtpDepayloader, RtpHeader};

// Re-export universal codec micro-blocks
pub use codec_types::{
    CodecCapabilities, CodecKind, FrameFormat, StreamNegotiation, VideoDimensions,
    V4L2_PIX_FMT_AV1, V4L2_PIX_FMT_H264, V4L2_PIX_FMT_HEVC, V4L2_PIX_FMT_NV12,
    V4L2_PIX_FMT_NV12M, V4L2_PIX_FMT_RGB565, V4L2_PIX_FMT_YUV420, V4L2_PIX_FMT_YUV420M,
};
pub use codec_v4l2_m2m::{
    BCM2835_H264_DEVICE, RPIVID_HEVC_DEVICE, V4l2DecodedFrame, V4l2M2mConfig, V4l2M2mDecoder,
    V4l2M2mError,
};
pub use codec_adapter::{
    DecodedFrame, DecoderError, DecoderStats, FallbackDecoder, GenericDecoder,
    MockVideoDecoder, V4l2VideoDecoder, VideoDecoder,
};

// Re-export presentation and scanout micro-blocks
pub use scanout_frame_pacer::{
    DEFAULT_KEEPALIVE_TIMEOUT_SECS, FramePacer, PacerAction, PacerStats,
};
pub use scanout_drm_kms::{
    calculate_destination_rect, DrmKmsScanout, HeldDrmBuffer, KmsScanoutPresenter,
    MockScanoutPresenter, ScanoutError,
};

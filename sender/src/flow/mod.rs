//! Sender Flow Micro-Blocks
//!
//! Blueprint Reference: Blueprint 27 (Flow-Oriented Micro-Block Architecture)
//!
//! Exposes decoupled single-responsibility components for:
//! - Hardware GPU Encoder selection and multi-codec support (H.264 / HEVC)
//! - USB Bulk Egress with ZLP management
//! - Network UDP Egress
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

pub mod encoder_types;
pub mod encoder_gpu_selector;
pub mod egress_usb_bulk;
pub mod egress_network_udp;

pub use encoder_types::{CodecKind, EncoderConfig, GpuArchitecture};
pub use encoder_gpu_selector::GpuEncoderSelector;
pub use egress_usb_bulk::{UsbBulkEgress, needs_zlp, frame_rfc4571};
pub use egress_network_udp::NetworkUdpEgress;

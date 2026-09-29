//! Stream parsing and frame assembly module
//!
//! Provides clean, decoupled parsers for Annex-B byte streams and RTP H.264 streams.
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

pub mod annexb;
pub mod rtp;
pub mod ts;

pub use annexb::AnnexBAssembler;
pub use rtp::{Rfc4571Assembler, RtpDepayloader};
pub use ts::TsDemuxer;

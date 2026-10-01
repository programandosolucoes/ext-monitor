//! Miracast (Wi-Fi Display / WFD 1.0) Pure-Rust Protocol Implementation
//!
//! Submodules:
//! - `mpegts`: Pure-Rust MPEG-TS / PES packetizer and RTP type 33 encapsulation.
//! - `wfd_client`: Pure-Rust RTSP 1.0 Wi-Fi Display (WFD 1.0 / MS-MICE) Source State Machine.
//! - `discovery`: Pure-Rust mDNS, SSDP, and Direct-USB Network Discovery Engine.

pub mod discovery;
pub mod mpegts;
pub mod wfd_client;

pub use discovery::{DiscoveredSink, interactive_select_sink, scan_sinks};



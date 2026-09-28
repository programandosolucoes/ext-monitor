//! Display output sink abstractions
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

pub mod edid;
pub mod framebuffer;
pub mod kms;
pub mod splash;

pub use edid::MonitorInfo;
pub use framebuffer::FramebufferSink;
pub use kms::KmsPlaneSink;
pub use splash::SplashEngine;

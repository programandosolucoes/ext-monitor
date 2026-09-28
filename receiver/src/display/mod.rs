//! Display output sink abstractions
//!
//! License: MIT
//! Author: Carlos Alberto <psncarlosalberto4ti@gmail.com>

pub mod framebuffer;
pub mod kms;

pub use framebuffer::FramebufferSink;
pub use kms::KmsPlaneSink;

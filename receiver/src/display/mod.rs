//! Display output sink abstractions
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

pub mod edid;
pub mod kms;
pub mod splash;

pub use edid::MonitorInfo;
pub use kms::KmsPlaneSink;
pub use splash::SplashEngine;

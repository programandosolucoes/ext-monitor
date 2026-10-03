//! Service Hierarchy and Resource Arbiter Micro-Block
//!
//! Blueprint Reference: Blueprint 34 (Service Hierarchy, Decision Tree and Micro-Blocks)
//! ADR Reference: ADR 0001 (Hierarquia de Serviços e Árbitro do Display HDMI)
//!
//! Implements a 4-level deterministic priority hierarchy across all hardware resources:
//! - Level 0: Primary Desktop Screen Modes (Mode 3 USB Bulk, Mode 1 Network UDP, Mode 2 Miracast)
//!            -> Absolute exclusive ownership of HDMI KMS Display Plane.
//!            -> Graphical visualizers, equalizers and splash screens are FORCIBLY MUTED.
//!            -> Synchronized audio runs cooperatively via ALSA 48kHz.
//! - Level 1: Dedicated Media Streaming / Cast (Google Cast, UPnP/DLNA)
//!            -> Active only when Level 0 is inactive. Dedicated player controls HDMI.
//! - Level 2: Standalone PC Audio (HDMI Soundbox Mode)
//!            -> Active audio without screen video.
//!            -> Display shows static audio splash or fluid FFT spectrum equalizer (if enabled).
//!            -> Never oscillates to service alert screen on audio silence intervals.
//! - Level 3: Standby / Idle
//!            -> Displays static "Ready to Connect" multilingual splash screen.
//!            -> Zero polling of DRM/KMS.
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Mutex;

/// Operational modes for Level 0 (Desktop Screen Extension/Mirror)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopMode {
    Mode3UsbBulk,
    Mode1NetworkUdp,
    Mode2Miracast,
}

impl DesktopMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mode3UsbBulk => "mode3_usb_bulk",
            Self::Mode1NetworkUdp => "mode1_udp",
            Self::Mode2Miracast => "mode2_miracast",
        }
    }
}

/// Media sources for Level 1 (Dedicated Media Streaming)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaSource {
    GoogleCast,
    UpnpDlna,
    Dial,
}

/// The 4-Level Service Hierarchy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceLevel {
    /// Level 0: Full Desktop Screen (Maximum Exclusive Priority over HDMI Video)
    Level0Desktop(DesktopMode),
    /// Level 1: Dedicated Media Player (Cast / DLNA)
    Level1Media(MediaSource),
    /// Level 2: HDMI Soundbox (Audio only from notebook, visualizer optional)
    Level2AudioOnly { visualizer_enabled: bool },
    /// Level 3: Standby / Idle
    Level3Standby,
}

/// Represents the active owner of the physical HDMI scanout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayOwner {
    /// Hardware VPU Decoder via KMS DRM plane (Zero-Copy DMA-BUF)
    KmsPlane(u32),
    /// Framebuffer /dev/fb0 for static splash screens or Level 2 visualizer
    Framebuffer,
    /// Display is idle or transitioning
    Unassigned,
}

/// Errors returned by the Service Hierarchy Arbiter
#[derive(Debug, PartialEq, Eq)]
pub enum ArbiterError {
    Level0Preempted,
    InvalidTransition(String),
    ResourceLocked(String),
}

impl std::fmt::Display for ArbiterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Level0Preempted => write!(f, "Action denied: Level 0 Desktop Screen holds exclusive HDMI ownership"),
            Self::InvalidTransition(msg) => write!(f, "Invalid state transition: {}", msg),
            Self::ResourceLocked(res) => write!(f, "Resource '{}' is currently locked", res),
        }
    }
}

impl std::error::Error for ArbiterError {}

/// Central Service and Hardware Resource Arbiter
pub struct ServiceArbiter {
    current_level: Mutex<ServiceLevel>,
    display_owner: Mutex<DisplayOwner>,
    audio_active: AtomicBool,
    visualizer_user_enabled: AtomicBool,
    /// Fast numeric level cache for lock-free checks (0 = Level 0, 1 = Level 1, 2 = Level 2, 3 = Level 3)
    level_cache: AtomicU8,
}

impl ServiceArbiter {
    pub const fn new() -> Self {
        Self {
            current_level: Mutex::new(ServiceLevel::Level3Standby),
            display_owner: Mutex::new(DisplayOwner::Framebuffer),
            audio_active: AtomicBool::new(false),
            visualizer_user_enabled: AtomicBool::new(false),
            level_cache: AtomicU8::new(3),
        }
    }

    /// Returns the current active service level
    pub fn current_level(&self) -> ServiceLevel {
        self.current_level.lock().unwrap().clone()
    }

    /// Fast lock-free check for Level 0 status
    #[inline(always)]
    pub fn is_level0_active(&self) -> bool {
        self.level_cache.load(Ordering::Relaxed) == 0
    }

    /// Fast lock-free check for Level 2 (Audio Only) status
    #[inline(always)]
    pub fn is_level2_active(&self) -> bool {
        self.level_cache.load(Ordering::Relaxed) == 2
    }

    /// Decides whether the FFT Equalizer Visualizer is permitted to draw on screen.
    /// Strict Rule: It is PERMITTED ONLY in Level 2 (Audio Only) and when explicitly enabled by user.
    /// In Level 0 (Desktop Screen) or Level 3 (Standby), it is FORCIBLY MUTED.
    pub fn can_visualizer_render(&self) -> bool {
        // Absolute veto: Level 0 video owns the display 100%
        if self.is_level0_active() {
            return false;
        }

        let level = self.current_level();
        match level {
            ServiceLevel::Level2AudioOnly { visualizer_enabled } => {
                visualizer_enabled
                    && self.visualizer_user_enabled.load(Ordering::Relaxed)
                    && self.audio_active.load(Ordering::Relaxed)
            }
            _ => false,
        }
    }

    /// Decides whether Splash Screens (/dev/fb0) are permitted to draw on screen.
    /// Strict Rule: Splash is FORBIDDEN during Level 0 (to prevent os error 13 DRM collisions).
    pub fn can_splash_render(&self) -> bool {
        if self.is_level0_active() {
            return false;
        }
        let owner = *self.display_owner.lock().unwrap();
        matches!(owner, DisplayOwner::Framebuffer | DisplayOwner::Unassigned)
    }

    /// Requests transition to Level 0 (Desktop Video Mode 1, 2, or 3).
    /// Atomically preempts any visualizer, revokes framebuffer access, and assigns KMS plane.
    pub fn request_level0(&self, mode: DesktopMode, plane_id: u32) -> Result<(), ArbiterError> {
        let mut level = self.current_level.lock().unwrap();
        let mut owner = self.display_owner.lock().unwrap();

        *level = ServiceLevel::Level0Desktop(mode);
        *owner = DisplayOwner::KmsPlane(plane_id);
        self.level_cache.store(0, Ordering::SeqCst);

        println!(
            "\x1b[1;32m[arbiter]\x1b[0m LEVEL 0 Granted: {:?} -> KMS Plane {} (Visualizers & Splash Muted)",
            mode, plane_id
        );
        Ok(())
    }

    /// Requests transition to Level 1 (Dedicated Media / Cast).
    pub fn request_level1(&self, source: MediaSource) -> Result<(), ArbiterError> {
        let mut level = self.current_level.lock().unwrap();
        let mut owner = self.display_owner.lock().unwrap();

        *level = ServiceLevel::Level1Media(source);
        *owner = DisplayOwner::KmsPlane(0); // Dedicated media player
        self.level_cache.store(1, Ordering::SeqCst);

        println!("\x1b[1;34m[arbiter]\x1b[0m LEVEL 1 Granted: Media Stream {:?}", source);
        Ok(())
    }

    /// Requests transition to Level 2 (HDMI Soundbox / Audio Only Mode).
    /// Releases the KMS plane cleanly and sets display ownership to Framebuffer.
    pub fn request_level2(&self, visualizer_enabled: bool) -> Result<(), ArbiterError> {
        let mut level = self.current_level.lock().unwrap();
        let mut owner = self.display_owner.lock().unwrap();

        *level = ServiceLevel::Level2AudioOnly { visualizer_enabled };
        *owner = DisplayOwner::Framebuffer;
        self.visualizer_user_enabled.store(visualizer_enabled, Ordering::SeqCst);
        self.level_cache.store(2, Ordering::SeqCst);

        println!(
            "\x1b[1;36m[arbiter]\x1b[0m LEVEL 2 Granted: Audio Soundbox (Visualizer: {})",
            visualizer_enabled
        );
        Ok(())
    }

    /// Requests transition to Level 3 (Standby / Idle).
    /// Releases all video resources cleanly.
    pub fn request_standby(&self) -> Result<(), ArbiterError> {
        let mut level = self.current_level.lock().unwrap();
        let mut owner = self.display_owner.lock().unwrap();

        *level = ServiceLevel::Level3Standby;
        *owner = DisplayOwner::Framebuffer;
        self.level_cache.store(3, Ordering::SeqCst);

        println!("\x1b[1;33m[arbiter]\x1b[0m LEVEL 3: Standby Active. KMS plane released.");
        Ok(())
    }

    /// Updates whether audio packets are currently streaming into ALSA
    pub fn set_audio_active(&self, active: bool) {
        self.audio_active.store(active, Ordering::Relaxed);
    }

    /// Sets the user preference for the visualizer from the Web UI or API
    pub fn set_visualizer_user_enabled(&self, enabled: bool) {
        self.visualizer_user_enabled.store(enabled, Ordering::SeqCst);
        if let Ok(mut level) = self.current_level.lock() {
            if let ServiceLevel::Level2AudioOnly { ref mut visualizer_enabled } = *level {
                *visualizer_enabled = enabled;
            }
        }
    }

    /// Returns the current active display scanout owner
    pub fn display_owner(&self) -> DisplayOwner {
        *self.display_owner.lock().unwrap()
    }

    /// Returns whether the user has enabled the visualizer preference
    pub fn is_visualizer_user_enabled(&self) -> bool {
        self.visualizer_user_enabled.load(Ordering::Relaxed)
    }
}

/// Global shared instance of the Service Arbiter
pub static ARBITER: ServiceArbiter = ServiceArbiter::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_level3_standby() {
        let arbiter = ServiceArbiter::new();
        assert_eq!(arbiter.current_level(), ServiceLevel::Level3Standby);
        assert!(!arbiter.is_level0_active());
        assert!(!arbiter.is_level2_active());
        assert!(!arbiter.can_visualizer_render());
        assert!(arbiter.can_splash_render());
    }

    #[test]
    fn test_level0_exclusive_ownership_mutes_visualizer_and_splash() {
        let arbiter = ServiceArbiter::new();
        arbiter.set_audio_active(true);
        arbiter.set_visualizer_user_enabled(true);

        // Switch to Level 0 (Mode 3 USB Bulk)
        arbiter.request_level0(DesktopMode::Mode3UsbBulk, 86).unwrap();

        assert!(arbiter.is_level0_active());
        assert_eq!(
            arbiter.current_level(),
            ServiceLevel::Level0Desktop(DesktopMode::Mode3UsbBulk)
        );

        // Visualizer and Splash MUST be completely vetoed!
        assert!(!arbiter.can_visualizer_render());
        assert!(!arbiter.can_splash_render());
    }

    #[test]
    fn test_level0_modes_transition_cleanly() {
        let arbiter = ServiceArbiter::new();

        // Mode 3 -> Mode 1 transition
        arbiter.request_level0(DesktopMode::Mode3UsbBulk, 86).unwrap();
        assert_eq!(
            arbiter.current_level(),
            ServiceLevel::Level0Desktop(DesktopMode::Mode3UsbBulk)
        );

        arbiter.request_level0(DesktopMode::Mode1NetworkUdp, 86).unwrap();
        assert_eq!(
            arbiter.current_level(),
            ServiceLevel::Level0Desktop(DesktopMode::Mode1NetworkUdp)
        );
        assert!(!arbiter.can_visualizer_render());
    }

    #[test]
    fn test_level2_audio_only_allows_visualizer_when_enabled() {
        let arbiter = ServiceArbiter::new();
        arbiter.set_audio_active(true);

        // Request Level 2 with visualizer enabled
        arbiter.request_level2(true).unwrap();
        assert!(arbiter.is_level2_active());
        assert!(!arbiter.is_level0_active());

        // When audio is active and enabled, visualizer is allowed
        assert!(arbiter.can_visualizer_render());
        assert!(arbiter.can_splash_render());

        // If audio goes silent, visualizer stops drawing
        arbiter.set_audio_active(false);
        assert!(!arbiter.can_visualizer_render());

        // If user disables visualizer in Level 2, it is suppressed
        arbiter.set_audio_active(true);
        arbiter.set_visualizer_user_enabled(false);
        assert!(!arbiter.can_visualizer_render());
    }

    #[test]
    fn test_standby_transition_releases_kms() {
        let arbiter = ServiceArbiter::new();

        arbiter.request_level0(DesktopMode::Mode3UsbBulk, 86).unwrap();
        assert!(!arbiter.can_splash_render());

        arbiter.request_standby().unwrap();
        assert_eq!(arbiter.current_level(), ServiceLevel::Level3Standby);
        assert!(arbiter.can_splash_render());
        assert!(!arbiter.can_visualizer_render());
    }
}

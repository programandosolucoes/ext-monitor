//! Deterministic CFR Frame Pacer and Anti-Freeze Watchdog Micro-Block
//!
//! Blueprint Reference: Blueprint 27 (Flow-Oriented Micro-Block Architecture)
//!
//! Provides deterministic frame rate pacing, duplicate frame mitigation, and
//! display anti-freeze watchdog:
//! - Deterministic CFR pacing (60 FPS: 16.66ms / 30 FPS: 33.33ms)
//! - Mitigation of duplicate frames to conserve VPU and LPDDR2 memory bus bandwidth
//! - Anti-freeze keepalive watchdog (60s) preventing HDMI display sleep
//!
//! License: MIT
//! Author: Carlos Alberto <carlosalberto4ti@gmail.com>

use std::time::{Duration, Instant};

/// Default anti-freeze watchdog timeout (60 seconds)
pub const DEFAULT_KEEPALIVE_TIMEOUT_SECS: u64 = 60;

/// Action decided by the Frame Pacer for an incoming frame
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacerAction {
    /// Frame is on schedule, present immediately to KMS plane
    Present,
    /// Frame arrived ahead of schedule, sleep for duration before scanout
    Wait(Duration),
    /// Frame arrived past the deadline tolerance, drop to maintain low latency
    DropLate { delay: Duration },
    /// Frame is identical to previously scanned frame, skip presentation
    Duplicate,
}

/// Real-time statistics for frame pacing
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PacerStats {
    pub frames_evaluated: u64,
    pub frames_presented: u64,
    pub frames_dropped_late: u64,
    pub frames_waited: u64,
    pub duplicates_detected: u64,
    pub keepalives_triggered: u64,
}

/// Deterministic CFR Frame Pacer
pub struct FramePacer {
    target_fps: u32,
    frame_interval: Duration,
    jitter_tolerance: Duration,
    keepalive_timeout: Duration,
    last_presentation_time: Option<Instant>,
    last_pts: Option<u64>,
    last_frame_hash: Option<u64>,
    last_activity_time: Instant,
    stats: PacerStats,
}

impl FramePacer {
    /// Creates a new FramePacer with given target FPS (e.g. 60 or 30).
    pub fn new(target_fps: u32) -> Self {
        Self::with_keepalive(target_fps, Duration::from_secs(DEFAULT_KEEPALIVE_TIMEOUT_SECS))
    }

    /// Creates a new FramePacer with custom target FPS and keepalive watchdog duration.
    pub fn with_keepalive(target_fps: u32, keepalive_timeout: Duration) -> Self {
        let fps = target_fps.max(1);
        let frame_interval = Duration::from_nanos(1_000_000_000 / (fps as u64));
        // Default tolerance: half a frame interval or at least 2ms
        let jitter_tolerance = (frame_interval / 2).max(Duration::from_millis(2));

        Self {
            target_fps: fps,
            frame_interval,
            jitter_tolerance,
            keepalive_timeout,
            last_presentation_time: None,
            last_pts: None,
            last_frame_hash: None,
            last_activity_time: Instant::now(),
            stats: PacerStats::default(),
        }
    }

    /// Exact frame interval required for CFR pacing.
    pub fn frame_interval(&self) -> Duration {
        self.frame_interval
    }

    pub fn target_fps(&self) -> u32 {
        self.target_fps
    }

    pub fn set_jitter_tolerance(&mut self, tolerance: Duration) {
        self.jitter_tolerance = tolerance;
    }

    /// Evaluates whether an incoming frame should be presented, waited for, dropped,
    /// or identified as duplicate.
    pub fn evaluate(
        &mut self,
        pts: u64,
        frame_hash: Option<u64>,
        now: Instant,
    ) -> PacerAction {
        self.stats.frames_evaluated += 1;
        self.last_activity_time = now;

        // 1. Check for duplicate frames (same PTS or identical content hash)
        if let Some(last_pts) = self.last_pts {
            if pts == last_pts && pts != 0 {
                self.stats.duplicates_detected += 1;
                return PacerAction::Duplicate;
            }
        }
        if let (Some(cur_hash), Some(last_hash)) = (frame_hash, self.last_frame_hash) {
            if cur_hash == last_hash {
                self.stats.duplicates_detected += 1;
                return PacerAction::Duplicate;
            }
        }

        // 2. Pace based on elapsed time since last presentation
        match self.last_presentation_time {
            None => {
                // First frame: present immediately
                PacerAction::Present
            }
            Some(last_time) => {
                let elapsed = now.saturating_duration_since(last_time);
                if elapsed < self.frame_interval {
                    let wait_time = self.frame_interval - elapsed;
                    // If arrived slightly early within tolerance, present immediately
                    if wait_time <= Duration::from_micros(500) {
                        PacerAction::Present
                    } else {
                        self.stats.frames_waited += 1;
                        PacerAction::Wait(wait_time)
                    }
                } else {
                    let delay = elapsed - self.frame_interval;
                    // If frame is delayed beyond deadline + tolerance, drop it to catch up
                    if delay > (self.frame_interval + self.jitter_tolerance) {
                        self.stats.frames_dropped_late += 1;
                        PacerAction::DropLate { delay }
                    } else {
                        PacerAction::Present
                    }
                }
            }
        }
    }

    /// Records that a frame was actually scanned out to update pacing timestamps.
    pub fn record_presented(
        &mut self,
        pts: u64,
        frame_hash: Option<u64>,
        now: Instant,
    ) {
        self.last_presentation_time = Some(now);
        self.last_pts = Some(pts);
        self.last_frame_hash = frame_hash;
        self.last_activity_time = now;
        self.stats.frames_presented += 1;
    }

    /// Checks if the anti-freeze watchdog has expired (no frames presented within keepalive timeout).
    pub fn should_trigger_keepalive(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.last_activity_time) >= self.keepalive_timeout
    }

    /// Resets activity watchdog without presenting a full frame (e.g. heartbeat or splash refresh).
    pub fn touch(&mut self, now: Instant) {
        self.last_activity_time = now;
    }

    /// Increments keepalive trigger counter.
    pub fn notify_keepalive_triggered(&mut self, now: Instant) {
        self.stats.keepalives_triggered += 1;
        self.last_activity_time = now;
    }

    pub fn stats(&self) -> &PacerStats {
        &self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_interval() {
        let pacer_60 = FramePacer::new(60);
        // 1,000,000,000 / 60 = 16,666,666 ns = 16.666 ms
        assert_eq!(pacer_60.frame_interval(), Duration::from_nanos(16_666_666));
        assert_eq!(pacer_60.target_fps(), 60);

        let pacer_30 = FramePacer::new(30);
        // 1,000,000,000 / 30 = 33,333,333 ns = 33.333 ms
        assert_eq!(pacer_30.frame_interval(), Duration::from_nanos(33_333_333));
        assert_eq!(pacer_30.target_fps(), 30);

        let pacer_120 = FramePacer::new(120);
        assert_eq!(pacer_120.frame_interval(), Duration::from_nanos(8_333_333));
    }

    #[test]
    fn test_keepalive_trigger() {
        let keepalive = Duration::from_millis(50);
        let mut pacer = FramePacer::with_keepalive(60, keepalive);

        let start = Instant::now();
        assert!(!pacer.should_trigger_keepalive(start));

        // Before timeout
        let before_timeout = start + Duration::from_millis(25);
        assert!(!pacer.should_trigger_keepalive(before_timeout));

        // After timeout
        let after_timeout = start + Duration::from_millis(60);
        assert!(pacer.should_trigger_keepalive(after_timeout));

        // Touch resets watchdog
        pacer.touch(after_timeout);
        assert!(!pacer.should_trigger_keepalive(after_timeout));

        let after_second_timeout = after_timeout + Duration::from_millis(60);
        assert!(pacer.should_trigger_keepalive(after_second_timeout));
        pacer.notify_keepalive_triggered(after_second_timeout);
        assert_eq!(pacer.stats().keepalives_triggered, 1);
    }

    #[test]
    fn test_duplicate_frame_mitigation() {
        let mut pacer = FramePacer::new(60);
        let t0 = Instant::now();

        // First frame
        let action1 = pacer.evaluate(100, Some(0xAAAA), t0);
        assert_eq!(action1, PacerAction::Present);
        pacer.record_presented(100, Some(0xAAAA), t0);

        // Same PTS frame
        let action_dup_pts = pacer.evaluate(100, None, t0 + Duration::from_millis(16));
        assert_eq!(action_dup_pts, PacerAction::Duplicate);

        // Same Hash frame
        let action_dup_hash = pacer.evaluate(101, Some(0xAAAA), t0 + Duration::from_millis(16));
        assert_eq!(action_dup_hash, PacerAction::Duplicate);

        // Distinct frame
        let action_distinct = pacer.evaluate(102, Some(0xBBBB), t0 + Duration::from_millis(17));
        assert_eq!(action_distinct, PacerAction::Present);
    }

    #[test]
    fn test_pacer_pacing_actions() {
        let mut pacer = FramePacer::new(60);
        let t0 = Instant::now();

        // Frame 1 presented
        pacer.record_presented(1, None, t0);

        // Frame 2 arrives 5ms later (too early for 60fps / 16.6ms)
        let t1 = t0 + Duration::from_millis(5);
        let action_early = pacer.evaluate(2, None, t1);
        match action_early {
            PacerAction::Wait(d) => {
                assert!(d > Duration::from_millis(10));
            }
            other => panic!("Expected Wait action, got {:?}", other),
        }

        // Frame 3 arrives 17ms later (on schedule)
        let t2 = t0 + Duration::from_millis(17);
        let action_ontime = pacer.evaluate(3, None, t2);
        assert_eq!(action_ontime, PacerAction::Present);
        pacer.record_presented(3, None, t2);

        // Frame 4 arrives 100ms later (too late, dropped)
        let t3 = t2 + Duration::from_millis(100);
        let action_late = pacer.evaluate(4, None, t3);
        match action_late {
            PacerAction::DropLate { delay } => {
                assert!(delay > Duration::from_millis(50));
            }
            other => panic!("Expected DropLate, got {:?}", other),
        }
    }
}

//! Stage 2 — Prompt 11: Batching Scheduler
//!
//! Formal scheduler with:
//! - 1-hour timer window
//! - 100-reading volume threshold (whichever first)
//! - Force-batch triggers: sensor offline, critical error, manual
//! - Configurable timeouts and event callbacks

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

// ── Trigger types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SealTrigger {
    /// 1-hour time window elapsed.
    TimerExpired { elapsed_secs: i64 },
    /// Reading volume reached threshold.
    VolumeThreshold { count: usize },
    /// Sensor went offline — seal current partial batch.
    SensorOffline { sensor_did: String },
    /// Critical validation error — seal and quarantine.
    CriticalError { reason: String },
    /// Explicit operator command.
    ManualTrigger { operator: String },
}

// ── Scheduler state ───────────────────────────────────────────────────────────

/// Tracks scheduler conditions without owning the readings buffer.
/// Drives `BatchEngine` (which owns the buffer) from outside.
pub struct BatchScheduler {
    pub window_secs: i64,
    pub volume_threshold: usize,
    pub window_start: DateTime<Utc>,
    pub ingested: usize,
}

impl BatchScheduler {
    pub fn new(window_secs: i64, volume_threshold: usize) -> Self {
        Self { window_secs, volume_threshold, window_start: Utc::now(), ingested: 0 }
    }

    /// Default: 1-hour window, 100-reading volume (Prompt 11 spec).
    pub fn default_config() -> Self { Self::new(3600, 100) }

    /// Record one more reading ingested.
    pub fn tick(&mut self) { self.ingested += 1; }

    /// Elapsed seconds since the window opened.
    pub fn elapsed_secs(&self) -> i64 {
        (Utc::now() - self.window_start).num_seconds()
    }

    /// Check if either sealing condition is met.
    /// Returns the first matching trigger, or None.
    pub fn check_seal(&self) -> Option<SealTrigger> {
        if self.ingested >= self.volume_threshold {
            return Some(SealTrigger::VolumeThreshold { count: self.ingested });
        }
        let elapsed = self.elapsed_secs();
        if elapsed >= self.window_secs {
            return Some(SealTrigger::TimerExpired { elapsed_secs: elapsed });
        }
        None
    }

    /// Force-seal due to sensor going offline.
    pub fn trigger_offline(&self, sensor_did: impl Into<String>) -> SealTrigger {
        SealTrigger::SensorOffline { sensor_did: sensor_did.into() }
    }

    /// Force-seal due to a critical error.
    pub fn trigger_error(&self, reason: impl Into<String>) -> SealTrigger {
        SealTrigger::CriticalError { reason: reason.into() }
    }

    /// Manual operator trigger.
    pub fn trigger_manual(&self, operator: impl Into<String>) -> SealTrigger {
        SealTrigger::ManualTrigger { operator: operator.into() }
    }

    /// Reset the scheduler after a seal.
    pub fn reset(&mut self) {
        self.window_start = Utc::now();
        self.ingested = 0;
    }

    /// Time remaining until the window expires. Returns 0 if already expired.
    pub fn remaining_secs(&self) -> i64 {
        (self.window_secs - self.elapsed_secs()).max(0)
    }

    /// Progress 0.0–1.0 toward next scheduled seal
    /// (either volume or timer, whichever is closer).
    pub fn progress(&self) -> f64 {
        let vol_progress = self.ingested as f64 / self.volume_threshold as f64;
        let time_progress = self.elapsed_secs() as f64 / self.window_secs as f64;
        vol_progress.max(time_progress).min(1.0)
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test 1: volume trigger fires at threshold ──────────────────────────────

    #[test]
    fn test_volume_trigger_at_threshold() {
        let mut sched = BatchScheduler::new(3600, 5);
        for _ in 0..4 { sched.tick(); }
        assert!(sched.check_seal().is_none(), "Should not seal at 4/5");
        sched.tick();
        assert!(matches!(sched.check_seal(), Some(SealTrigger::VolumeThreshold { count: 5 })));
    }

    // ── Test 2: timer trigger fires when window expires ───────────────────────

    #[test]
    fn test_timer_trigger_when_expired() {
        let mut sched = BatchScheduler::new(0, 100); // 0-second window → immediately expired
        sched.window_start = Utc::now() - Duration::seconds(1);
        assert!(matches!(sched.check_seal(), Some(SealTrigger::TimerExpired { .. })));
    }

    // ── Test 3: volume takes priority over timer ──────────────────────────────

    #[test]
    fn test_volume_priority_over_timer() {
        let mut sched = BatchScheduler::new(0, 3); // both conditions met
        for _ in 0..3 { sched.tick(); }
        assert!(matches!(sched.check_seal(), Some(SealTrigger::VolumeThreshold { .. })));
    }

    // ── Test 4: reset clears state ────────────────────────────────────────────

    #[test]
    fn test_reset_clears_state() {
        let mut sched = BatchScheduler::new(3600, 3);
        for _ in 0..3 { sched.tick(); }
        sched.reset();
        assert_eq!(sched.ingested, 0);
        assert!(sched.check_seal().is_none(), "No seal after reset");
    }

    // ── Test 5: offline trigger ───────────────────────────────────────────────

    #[test]
    fn test_offline_trigger() {
        let sched = BatchScheduler::new(3600, 100);
        let t = sched.trigger_offline("did:cardano:sensor:biochar-001");
        assert!(matches!(t, SealTrigger::SensorOffline { .. }));
    }

    // ── Test 6: critical error trigger ───────────────────────────────────────

    #[test]
    fn test_critical_error_trigger() {
        let sched = BatchScheduler::new(3600, 100);
        let t = sched.trigger_error("signature validation failed");
        assert!(matches!(t, SealTrigger::CriticalError { .. }));
    }

    // ── Test 7: manual trigger ────────────────────────────────────────────────

    #[test]
    fn test_manual_trigger() {
        let sched = BatchScheduler::new(3600, 100);
        let t = sched.trigger_manual("ops-admin");
        assert!(matches!(t, SealTrigger::ManualTrigger { .. }));
    }

    // ── Test 8: progress increases monotonically ──────────────────────────────

    #[test]
    fn test_progress_increases() {
        let mut sched = BatchScheduler::new(3600, 10);
        let p0 = sched.progress();
        sched.tick();
        sched.tick();
        let p2 = sched.progress();
        assert!(p2 > p0, "Progress must increase as readings are added");
        assert!(p2 <= 1.0);
    }

    // ── Test 9: remaining_secs non-negative ───────────────────────────────────

    #[test]
    fn test_remaining_secs_non_negative() {
        let mut sched = BatchScheduler::new(0, 100);
        sched.window_start = Utc::now() - Duration::seconds(999);
        assert_eq!(sched.remaining_secs(), 0, "Must clamp at 0");
    }

    // ── Test 10: default config is 3600s / 100 readings ──────────────────────

    #[test]
    fn test_default_config() {
        let sched = BatchScheduler::default_config();
        assert_eq!(sched.window_secs, 3600);
        assert_eq!(sched.volume_threshold, 100);
    }
}

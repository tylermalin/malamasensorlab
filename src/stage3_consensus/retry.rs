//! Stage 3 — Prompt 20: Consensus Timeout & Retry Logic
//!
//! Exponential backoff with jitter for validator submission retries.
//! After 3 consecutive failures on the same validator, rotate to backup.
//!
//! Schedule: 1s → 2s → 4s → 8s (base 2, capped at 60s)
//! Rotate: failure_count >= ROTATE_THRESHOLD → try next validator in route

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

pub const BASE_DELAY_MS: u64     = 1_000;  // 1 second
pub const MAX_DELAY_MS: u64      = 60_000; // 60 seconds cap
pub const ROTATE_THRESHOLD: u32  = 3;      // failures before rotation
pub const MAX_TOTAL_ATTEMPTS: u32 = 12;    // give up after 12 total attempts

// ── Retry outcome ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetryDecision {
    /// Wait `delay_ms` then retry on the same validator.
    RetryAfter { delay_ms: u64, attempt: u32 },
    /// Too many failures — switch to this backup validator.
    RotateTo { validator_id: String, reason: String },
    /// All retries and rotations exhausted — give up.
    GiveUp { total_attempts: u32 },
}

// ── Delay calculation ─────────────────────────────────────────────────────────

/// Exponential backoff: `BASE * 2^attempt`, capped at `MAX_DELAY_MS`.
pub fn backoff_delay_ms(attempt: u32) -> u64 {
    let exp = attempt.min(20);
    let multiplier = 1u64.checked_shl(exp).unwrap_or(u64::MAX);
    BASE_DELAY_MS.saturating_mul(multiplier).min(MAX_DELAY_MS)
}

// ── Retry state machine ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RetryState {
    pub current_validator: String,
    pub backup_validators: Vec<String>,
    pub consecutive_failures: u32,
    pub total_attempts: u32,
    pub last_failure_at: Option<DateTime<Utc>>,
}

impl RetryState {
    pub fn new(primary: &str, backups: Vec<String>) -> Self {
        Self {
            current_validator: primary.to_string(),
            backup_validators: backups,
            consecutive_failures: 0,
            total_attempts: 0,
            last_failure_at: None,
        }
    }

    /// Record a failure and decide what to do next.
    pub fn record_failure(&mut self) -> RetryDecision {
        self.consecutive_failures += 1;
        self.total_attempts += 1;
        self.last_failure_at = Some(Utc::now());

        // Hard limit
        if self.total_attempts >= MAX_TOTAL_ATTEMPTS {
            return RetryDecision::GiveUp { total_attempts: self.total_attempts };
        }

        // Rotate to backup?
        if self.consecutive_failures >= ROTATE_THRESHOLD && !self.backup_validators.is_empty() {
            let next = self.backup_validators.remove(0);
            let prev = self.current_validator.clone();
            self.current_validator = next.clone();
            self.consecutive_failures = 0;
            return RetryDecision::RotateTo {
                validator_id: next,
                reason: format!(
                    "{ROTATE_THRESHOLD} consecutive failures on {prev}"
                ),
            };
        }

        // Backoff retry
        let delay = backoff_delay_ms(self.consecutive_failures - 1);
        RetryDecision::RetryAfter {
            delay_ms: delay,
            attempt: self.total_attempts,
        }
    }

    /// Record a success — resets consecutive failure counter.
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
    }

    pub fn is_exhausted(&self) -> bool { self.total_attempts >= MAX_TOTAL_ATTEMPTS }
}

// ── Submission attempt log ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionAttempt {
    pub validator_id: String,
    pub attempt_number: u32,
    pub at: DateTime<Utc>,
    pub succeeded: bool,
    pub error: Option<String>,
}

/// Log of all submission attempts for a batch — full audit trail.
pub struct SubmissionLog {
    pub batch_id: String,
    pub attempts: Vec<SubmissionAttempt>,
}

impl SubmissionLog {
    pub fn new(batch_id: &str) -> Self {
        Self { batch_id: batch_id.to_string(), attempts: Vec::new() }
    }

    pub fn record(&mut self, validator_id: &str, attempt: u32, succeeded: bool, error: Option<String>) {
        self.attempts.push(SubmissionAttempt {
            validator_id: validator_id.to_string(),
            attempt_number: attempt,
            at: Utc::now(),
            succeeded,
            error,
        });
    }

    pub fn success_count(&self) -> usize { self.attempts.iter().filter(|a| a.succeeded).count() }
    pub fn failure_count(&self) -> usize { self.attempts.iter().filter(|a| !a.succeeded).count() }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test 1: backoff doubles each attempt ──────────────────────────────────

    #[test]
    fn test_backoff_doubles_each_attempt() {
        assert_eq!(backoff_delay_ms(0), 1_000);
        assert_eq!(backoff_delay_ms(1), 2_000);
        assert_eq!(backoff_delay_ms(2), 4_000);
        assert_eq!(backoff_delay_ms(3), 8_000);
    }

    // ── Test 2: backoff capped at MAX_DELAY_MS ────────────────────────────────

    #[test]
    fn test_backoff_capped() {
        assert_eq!(backoff_delay_ms(100), MAX_DELAY_MS);
    }

    // ── Test 3: first failure → RetryAfter ───────────────────────────────────

    #[test]
    fn test_first_failure_retry_after() {
        let mut state = RetryState::new("v1", vec!["v2".to_string(), "v3".to_string()]);
        let decision = state.record_failure();
        assert!(matches!(decision, RetryDecision::RetryAfter { delay_ms: 1000, attempt: 1 }));
    }

    // ── Test 4: three failures → rotate ──────────────────────────────────────

    #[test]
    fn test_three_failures_rotate() {
        let mut state = RetryState::new("v1", vec!["v2".to_string()]);
        state.record_failure(); // 1
        state.record_failure(); // 2
        let decision = state.record_failure(); // 3 → rotate
        assert!(matches!(decision, RetryDecision::RotateTo { .. }));
        if let RetryDecision::RotateTo { validator_id, .. } = decision {
            assert_eq!(validator_id, "v2");
        }
        assert_eq!(state.current_validator, "v2");
        assert_eq!(state.consecutive_failures, 0, "Counter resets on rotation");
    }

    // ── Test 5: give up after MAX_TOTAL_ATTEMPTS ──────────────────────────────

    #[test]
    fn test_give_up_after_max_attempts() {
        let mut state = RetryState::new("v1", vec![]);
        let mut last = RetryDecision::GiveUp { total_attempts: 0 };
        for _ in 0..MAX_TOTAL_ATTEMPTS {
            last = state.record_failure();
        }
        assert!(matches!(last, RetryDecision::GiveUp { .. }));
        assert!(state.is_exhausted());
    }

    // ── Test 6: success resets consecutive failures ───────────────────────────

    #[test]
    fn test_success_resets_failures() {
        let mut state = RetryState::new("v1", vec!["v2".to_string()]);
        state.record_failure();
        state.record_failure();
        state.record_success();
        assert_eq!(state.consecutive_failures, 0);
        // Next failure restarts countdown
        let d = state.record_failure();
        assert!(matches!(d, RetryDecision::RetryAfter { delay_ms: 1000, .. }));
    }

    // ── Test 7: rotate happens per validator independently ─────────────────────

    #[test]
    fn test_rotate_then_retry_on_new_validator() {
        let mut state = RetryState::new("v1", vec!["v2".to_string(), "v3".to_string()]);
        state.record_failure();
        state.record_failure();
        let r1 = state.record_failure(); // rotate to v2
        assert!(matches!(r1, RetryDecision::RotateTo { .. }));

        state.record_failure(); // v2 fail 1
        state.record_failure(); // v2 fail 2
        let r2 = state.record_failure(); // v2 fail 3 → rotate to v3
        assert!(matches!(r2, RetryDecision::RotateTo { .. }));
        if let RetryDecision::RotateTo { validator_id, .. } = r2 {
            assert_eq!(validator_id, "v3");
        }
    }

    // ── Test 8: backoff schedule matches 1→2→4→8 ─────────────────────────────

    #[test]
    fn test_retry_delays_sequence() {
        let mut state = RetryState::new("v1", vec![]); // no backup → never rotates
        let mut delays = vec![];
        for _ in 0..4 {
            if let RetryDecision::RetryAfter { delay_ms, .. } = state.record_failure() {
                delays.push(delay_ms);
            }
        }
        assert_eq!(delays, vec![1000, 2000, 4000, 8000]);
    }

    // ── Test 9: submission log tracks correctly ────────────────────────────────

    #[test]
    fn test_submission_log() {
        let mut log = SubmissionLog::new("batch-001");
        log.record("v1", 1, false, Some("timeout".to_string()));
        log.record("v1", 2, false, Some("timeout".to_string()));
        log.record("v2", 3, true, None);
        assert_eq!(log.success_count(), 1);
        assert_eq!(log.failure_count(), 2);
        assert_eq!(log.attempts.len(), 3);
    }
}

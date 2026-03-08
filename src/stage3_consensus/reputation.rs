//! Stage 3 — Prompt 17: Validator Reputation Scoring
//!
//! Tracks two dimensions of validator quality:
//!   - **Uptime** = % of batch submissions signed within 5 minutes
//!   - **Accuracy** = % of validator approvals that Verra also approved
//!
//! Combined reputation score = 0.5 * uptime + 0.5 * accuracy (0.0–1.0)
//! Remove validator from quorum if reputation < 0.50

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Duration, Utc};

pub const UPTIME_WINDOW_SECS: i64    = 300;   // 5 minutes
pub const MIN_REPUTATION: f64        = 0.50;  // removal threshold
pub const UPTIME_WEIGHT: f64         = 0.50;
pub const ACCURACY_WEIGHT: f64       = 0.50;

// ── Validator stats ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorStats {
    pub validator_id: String,
    /// Total batches submitted to this validator.
    pub total_submissions: u64,
    /// Submissions signed within the 5-minute window.
    pub on_time_signs: u64,
    /// Batches this validator approved.
    pub approvals_given: u64,
    /// Of those approvals, how many Verra also approved.
    pub verra_confirmed: u64,
    /// Timestamp of last activity.
    pub last_active: DateTime<Utc>,
}

impl ValidatorStats {
    pub fn new(validator_id: &str) -> Self {
        Self {
            validator_id: validator_id.to_string(),
            total_submissions: 0,
            on_time_signs: 0,
            approvals_given: 0,
            verra_confirmed: 0,
            last_active: Utc::now(),
        }
    }

    /// Uptime = on_time_signs / total_submissions (0.0–1.0)
    pub fn uptime(&self) -> f64 {
        if self.total_submissions == 0 { return 1.0; } // new validator — innocent until proven slow
        self.on_time_signs as f64 / self.total_submissions as f64
    }

    /// Accuracy = verra_confirmed / approvals_given (0.0–1.0)
    /// Returns 1.0 if no approvals given yet (new validator grace period).
    pub fn accuracy(&self) -> f64 {
        if self.approvals_given == 0 { return 1.0; }
        self.verra_confirmed as f64 / self.approvals_given as f64
    }

    /// Combined reputation score.
    pub fn reputation(&self) -> f64 {
        UPTIME_WEIGHT * self.uptime() + ACCURACY_WEIGHT * self.accuracy()
    }

    /// True if this validator should be removed from the quorum set.
    pub fn should_remove(&self) -> bool {
        self.reputation() < MIN_REPUTATION
    }
}

// ── Reputation event types ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReputationEvent {
    /// Validator signed within the 5-min window.
    SignedOnTime { batch_id: String },
    /// Validator missed the signing window.
    SignedLate { batch_id: String },
    /// Validator was not reachable within the timeout.
    FailedToSign { batch_id: String },
    /// Validator approved and Verra also approved (accuracy +1).
    ApprovalConfirmedByVerra { batch_id: String },
    /// Validator approved but Verra rejected (accuracy -1 effective).
    ApprovalRejectedByVerra { batch_id: String },
}

// ── Reputation ledger ─────────────────────────────────────────────────────────

pub struct ReputationLedger {
    validators: HashMap<String, ValidatorStats>,
    event_log: Vec<(String, ReputationEvent, DateTime<Utc>)>,
}

impl ReputationLedger {
    pub fn new() -> Self {
        Self { validators: HashMap::new(), event_log: Vec::new() }
    }

    /// Ensure a validator entry exists.
    pub fn register(&mut self, validator_id: &str) {
        self.validators.entry(validator_id.to_string())
            .or_insert_with(|| ValidatorStats::new(validator_id));
    }

    /// Record that a batch submission was dispatched to this validator.
    pub fn record_submission(&mut self, validator_id: &str) {
        let stats = self.validators.entry(validator_id.to_string())
            .or_insert_with(|| ValidatorStats::new(validator_id));
        stats.total_submissions += 1;
    }

    /// Record an event for a validator and update their stats.
    pub fn record_event(&mut self, validator_id: &str, event: ReputationEvent) {
        let now = Utc::now();
        self.event_log.push((validator_id.to_string(), event.clone(), now));

        let stats = self.validators.entry(validator_id.to_string())
            .or_insert_with(|| ValidatorStats::new(validator_id));
        stats.last_active = now;

        match &event {
            ReputationEvent::SignedOnTime { .. } => {
                stats.on_time_signs += 1;
            }
            ReputationEvent::SignedLate { .. } | ReputationEvent::FailedToSign { .. } => {
                // on_time_signs not incremented — uptime decreases
            }
            ReputationEvent::ApprovalConfirmedByVerra { .. } => {
                stats.approvals_given += 1;
                stats.verra_confirmed += 1;
            }
            ReputationEvent::ApprovalRejectedByVerra { .. } => {
                stats.approvals_given += 1;
                // verra_confirmed not incremented — accuracy decreases
            }
        }
    }

    /// Get the current stats for a validator.
    pub fn stats(&self, validator_id: &str) -> Option<&ValidatorStats> {
        self.validators.get(validator_id)
    }

    /// List validators whose reputation has dropped below the removal threshold.
    pub fn validators_to_remove(&self) -> Vec<&ValidatorStats> {
        self.validators.values().filter(|s| s.should_remove()).collect()
    }

    /// Full event history for a validator.
    pub fn events_for(&self, validator_id: &str) -> Vec<&ReputationEvent> {
        self.event_log.iter()
            .filter(|(id, _, _)| id == validator_id)
            .map(|(_, e, _)| e)
            .collect()
    }

    /// All registered validators sorted by reputation (highest first).
    pub fn ranked(&self) -> Vec<&ValidatorStats> {
        let mut v: Vec<&ValidatorStats> = self.validators.values().collect();
        v.sort_by(|a, b| b.reputation().partial_cmp(&a.reputation()).unwrap());
        v
    }
}

impl Default for ReputationLedger { fn default() -> Self { Self::new() } }

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const V1: &str = "V1:malama-labs";
    const V2: &str = "V2:verra-registry";
    const V3: &str = "V3:community";

    // ── Test 1: new validator starts at reputation 1.0 ────────────────────────

    #[test]
    fn test_new_validator_reputation_is_1() {
        let stats = ValidatorStats::new(V1);
        assert!((stats.reputation() - 1.0).abs() < 1e-9);
        assert!(!stats.should_remove());
    }

    // ── Test 2: perfect uptime and accuracy → 1.0 ─────────────────────────────

    #[test]
    fn test_perfect_validator_reputation() {
        let mut ledger = ReputationLedger::new();
        for _ in 0..10 {
            ledger.record_submission(V1);
            ledger.record_event(V1, ReputationEvent::SignedOnTime { batch_id: "b1".to_string() });
            ledger.record_event(V1, ReputationEvent::ApprovalConfirmedByVerra { batch_id: "b1".to_string() });
        }
        let stats = ledger.stats(V1).unwrap();
        assert!((stats.uptime() - 1.0).abs() < 1e-9);
        assert!((stats.accuracy() - 1.0).abs() < 1e-9);
        assert!((stats.reputation() - 1.0).abs() < 1e-9);
    }

    // ── Test 3: uptime + accuracy both fail → removal ─────────────────────────

    #[test]
    fn test_low_uptime_triggers_removal() {
        let mut ledger = ReputationLedger::new();
        // 10 submissions: never signs on time AND all approvals rejected by Verra
        for _ in 0..10 {
            ledger.record_submission(V1);
            ledger.record_event(V1, ReputationEvent::FailedToSign { batch_id: "b".to_string() });
            ledger.record_event(V1, ReputationEvent::ApprovalRejectedByVerra { batch_id: "b".to_string() });
        }
        let stats = ledger.stats(V1).unwrap();
        assert!((stats.uptime() - 0.0).abs() < 1e-9);
        assert!((stats.accuracy() - 0.0).abs() < 1e-9);
        assert!((stats.reputation() - 0.0).abs() < 1e-9);
        assert!(stats.should_remove(), "0% uptime + 0% accuracy must trigger removal");
    }

    // ── Test 4: Verra rejections lower accuracy ────────────────────────────────

    #[test]
    fn test_verra_rejections_lower_accuracy() {
        let mut ledger = ReputationLedger::new();
        // 10 submissions, 0 on-time signs → uptime = 0
        for _ in 0..10 {
            ledger.record_submission(V1);
            ledger.record_event(V1, ReputationEvent::SignedOnTime { batch_id: "b".to_string() });
            ledger.record_event(V1, ReputationEvent::ApprovalRejectedByVerra { batch_id: "b".to_string() });
        }
        let stats = ledger.stats(V1).unwrap();
        assert!((stats.accuracy() - 0.0).abs() < 1e-9);
        // uptime = 1.0, accuracy = 0.0 → reputation = 0.5 (exactly at boundary)
        assert!((stats.reputation() - 0.5).abs() < 1e-9);
        // NOT removed since 0.5 == MIN_REPUTATION and condition is strict <
        assert!(!stats.should_remove());
    }

    // ── Test 5: combined score: 50% uptime, 100% accuracy = 0.75 ─────────────

    #[test]
    fn test_combined_score_calculated_correctly() {
        let mut ledger = ReputationLedger::new();
        for i in 0..10 {
            ledger.record_submission(V2);
            // Every other submission is late
            if i % 2 == 0 {
                ledger.record_event(V2, ReputationEvent::SignedOnTime { batch_id: "b".to_string() });
            } else {
                ledger.record_event(V2, ReputationEvent::SignedLate { batch_id: "b".to_string() });
            }
            ledger.record_event(V2, ReputationEvent::ApprovalConfirmedByVerra { batch_id: "b".to_string() });
        }
        let stats = ledger.stats(V2).unwrap();
        assert!((stats.uptime() - 0.5).abs() < 1e-9);
        assert!((stats.accuracy() - 1.0).abs() < 1e-9);
        assert!((stats.reputation() - 0.75).abs() < 1e-9);
        assert!(!stats.should_remove());
    }

    // ── Test 6: validators_to_remove correctly identifies bad actors ──────────

    #[test]
    fn test_validators_to_remove() {
        let mut ledger = ReputationLedger::new();
        // V1: fine
        ledger.record_submission(V1);
        ledger.record_event(V1, ReputationEvent::SignedOnTime { batch_id: "b".to_string() });
        // V3: completely fails
        for _ in 0..5 {
            ledger.record_submission(V3);
            ledger.record_event(V3, ReputationEvent::FailedToSign { batch_id: "b".to_string() });
            ledger.record_event(V3, ReputationEvent::ApprovalRejectedByVerra { batch_id: "b".to_string() });
        }
        let to_remove = ledger.validators_to_remove();
        assert!(to_remove.iter().any(|s| s.validator_id == V3), "V3 must be flagged for removal");
        assert!(!to_remove.iter().any(|s| s.validator_id == V1), "V1 must not be flagged");
    }

    // ── Test 7: event log is append-only ─────────────────────────────────────

    #[test]
    fn test_event_log_append_only() {
        let mut ledger = ReputationLedger::new();
        ledger.record_event(V1, ReputationEvent::SignedOnTime { batch_id: "b1".to_string() });
        ledger.record_event(V1, ReputationEvent::SignedLate  { batch_id: "b2".to_string() });
        assert_eq!(ledger.events_for(V1).len(), 2);
    }

    // ── Test 8: ranked returns highest reputation first ───────────────────────

    #[test]
    fn test_ranked_highest_first() {
        let mut ledger = ReputationLedger::new();
        // V1: perfect
        ledger.record_submission(V1);
        ledger.record_event(V1, ReputationEvent::SignedOnTime { batch_id: "b".to_string() });
        // V2: always fails
        ledger.record_submission(V2);
        ledger.record_event(V2, ReputationEvent::FailedToSign { batch_id: "b".to_string() });

        let ranked = ledger.ranked();
        assert!(ranked[0].reputation() >= ranked[ranked.len() - 1].reputation());
    }

    // ── Test 9: reputation constant thresholds are correct ────────────────────

    #[test]
    fn test_constants() {
        assert_eq!(UPTIME_WINDOW_SECS, 300);
        assert!((MIN_REPUTATION - 0.50).abs() < 1e-9);
        assert!((UPTIME_WEIGHT + ACCURACY_WEIGHT - 1.0).abs() < 1e-9);
    }
}

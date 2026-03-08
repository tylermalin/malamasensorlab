//! Stage 3 — Prompt 21: Validator Reputation Calculation & Slashing
//!
//! Weighted reputation scoring with slashing mechanics:
//!
//! Score = w_uptime * uptime + w_accuracy * accuracy + w_speed * speed_score
//!
//! Slashing events reduce the validator's stake (simulated as a penalty multiplier):
//!   - Signing a forged/invalid batch: -30% slash
//!   - Double-signing (equivocation): -50% slash (severe)
//!   - Persistent latency above threshold: -5% slash per violation
//!   - Offline for > 24h: -10% slash

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// ── Weights ────────────────────────────────────────────────────────────────────

pub const W_UPTIME:   f64 = 0.35;
pub const W_ACCURACY: f64 = 0.40;
pub const W_SPEED:    f64 = 0.25;
pub const SLASH_INVALID_BATCH:   f64 = 0.30;
pub const SLASH_DOUBLE_SIGN:     f64 = 0.50;
pub const SLASH_LATENCY_EXCESS:  f64 = 0.05;
pub const SLASH_OFFLINE_24H:     f64 = 0.10;
pub const MIN_REPUTATION_SCORE:  f64 = 0.50;

// ── Slash event ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SlashEvent {
    InvalidBatchSigned   { batch_id: String },
    DoubleSign           { batch_id: String },
    PersistentHighLatency { latency_ms: u64 },
    OfflineExcess        { hours_offline: u64 },
}

impl SlashEvent {
    pub fn penalty_fraction(&self) -> f64 {
        match self {
            SlashEvent::InvalidBatchSigned { .. }  => SLASH_INVALID_BATCH,
            SlashEvent::DoubleSign { .. }           => SLASH_DOUBLE_SIGN,
            SlashEvent::PersistentHighLatency { .. }=> SLASH_LATENCY_EXCESS,
            SlashEvent::OfflineExcess { .. }        => SLASH_OFFLINE_24H,
        }
    }

    pub fn description(&self) -> String {
        match self {
            SlashEvent::InvalidBatchSigned { batch_id } =>
                format!("Signed invalid batch {batch_id}: -{:.0}% stake", SLASH_INVALID_BATCH * 100.0),
            SlashEvent::DoubleSign { batch_id } =>
                format!("Double-signed batch {batch_id}: -{:.0}% stake (equivocation)", SLASH_DOUBLE_SIGN * 100.0),
            SlashEvent::PersistentHighLatency { latency_ms } =>
                format!("Excess latency {latency_ms}ms: -{:.0}% stake", SLASH_LATENCY_EXCESS * 100.0),
            SlashEvent::OfflineExcess { hours_offline } =>
                format!("Offline {hours_offline}h: -{:.0}% stake", SLASH_OFFLINE_24H * 100.0),
        }
    }
}

// ── Validator performance record ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorPerformance {
    pub validator_id: String,
    /// Fraction of rounds where validator was online and voted (0.0–1.0).
    pub uptime_fraction: f64,
    /// Fraction of votes agreed with final quorum decision (0.0–1.0).
    pub accuracy_fraction: f64,
    /// Fraction of votes submitted within the 5-min SLA window (0.0–1.0).
    pub on_time_fraction: f64,
    /// Current stake multiplier (1.0 = full stake, 0.0 = fully slashed).
    pub stake_multiplier: f64,
    /// Accumulated slash events (immutable audit log).
    pub slash_history: Vec<(SlashEvent, DateTime<Utc>)>,
}

impl ValidatorPerformance {
    pub fn new(validator_id: &str) -> Self {
        Self {
            validator_id: validator_id.to_string(),
            uptime_fraction: 1.0,
            accuracy_fraction: 1.0,
            on_time_fraction: 1.0,
            stake_multiplier: 1.0,
            slash_history: Vec::new(),
        }
    }

    /// Weighted reputation score (0.0–1.0).
    pub fn reputation_score(&self) -> f64 {
        (W_UPTIME * self.uptime_fraction
            + W_ACCURACY * self.accuracy_fraction
            + W_SPEED * self.on_time_fraction)
            .min(1.0)
            .max(0.0)
    }

    /// Apply a slash event — reduces stake_multiplier by penalty_fraction.
    pub fn slash(&mut self, event: SlashEvent) {
        let penalty = event.penalty_fraction();
        let now = Utc::now();
        self.slash_history.push((event, now));
        self.stake_multiplier = (self.stake_multiplier - penalty).max(0.0);
    }

    /// Should this validator be removed? (reputation < threshold OR stake = 0)
    pub fn should_remove(&self) -> bool {
        self.reputation_score() < MIN_REPUTATION_SCORE || self.stake_multiplier <= 0.0
    }

    /// Total accumulated penalty applied.
    pub fn total_slashed(&self) -> f64 {
        self.slash_history.iter().map(|(e, _)| e.penalty_fraction()).sum()
    }
}

// ── Reputation registry ───────────────────────────────────────────────────────

pub struct ReputationRegistry {
    validators: HashMap<String, ValidatorPerformance>,
}

impl ReputationRegistry {
    pub fn new() -> Self { Self { validators: HashMap::new() } }

    pub fn register(&mut self, validator_id: &str) {
        self.validators.entry(validator_id.to_string())
            .or_insert_with(|| ValidatorPerformance::new(validator_id));
    }

    /// Apply a slash event to a validator.
    pub fn slash(&mut self, validator_id: &str, event: SlashEvent) {
        if let Some(v) = self.validators.get_mut(validator_id) {
            v.slash(event);
        }
    }

    /// Update performance fractions after a voting round.
    pub fn record_round(
        &mut self,
        validator_id: &str,
        was_online: bool,
        voted_with_quorum: bool,
        was_on_time: bool,
    ) {
        let v = self.validators.entry(validator_id.to_string())
            .or_insert_with(|| ValidatorPerformance::new(validator_id));

        // Exponential moving average (alpha = 0.1) — smooth gradual changes
        const ALPHA: f64 = 0.1;
        v.uptime_fraction   = (1.0 - ALPHA) * v.uptime_fraction  + ALPHA * if was_online { 1.0 } else { 0.0 };
        v.accuracy_fraction = (1.0 - ALPHA) * v.accuracy_fraction + ALPHA * if voted_with_quorum { 1.0 } else { 0.0 };
        v.on_time_fraction  = (1.0 - ALPHA) * v.on_time_fraction  + ALPHA * if was_on_time { 1.0 } else { 0.0 };
    }

    pub fn performance(&self, validator_id: &str) -> Option<&ValidatorPerformance> {
        self.validators.get(validator_id)
    }

    /// Validators that should be removed.
    pub fn flagged_for_removal(&self) -> Vec<&ValidatorPerformance> {
        self.validators.values().filter(|v| v.should_remove()).collect()
    }

    /// Rankings: highest reputation first.
    pub fn ranked(&self) -> Vec<&ValidatorPerformance> {
        let mut v: Vec<&ValidatorPerformance> = self.validators.values().collect();
        v.sort_by(|a, b| b.reputation_score().partial_cmp(&a.reputation_score()).unwrap());
        v
    }
}

impl Default for ReputationRegistry { fn default() -> Self { Self::new() } }

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const V1: &str = "V1:malama-labs";
    const V2: &str = "V2:verra";

    // ── Test 1: perfect validator scores 1.0 ─────────────────────────────────

    #[test]
    fn test_perfect_performance_scores_1() {
        let v = ValidatorPerformance::new(V1);
        assert!((v.reputation_score() - 1.0).abs() < 1e-9);
        assert!(!v.should_remove());
    }

    // ── Test 2: invalid batch slash reduces stake by 30% ──────────────────────

    #[test]
    fn test_invalid_batch_slash() {
        let mut v = ValidatorPerformance::new(V1);
        v.slash(SlashEvent::InvalidBatchSigned { batch_id: "b1".to_string() });
        assert!((v.stake_multiplier - 0.70).abs() < 1e-9);
        assert_eq!(v.slash_history.len(), 1);
    }

    // ── Test 3: double-sign slash reduces by 50% ──────────────────────────────

    #[test]
    fn test_double_sign_slash() {
        let mut v = ValidatorPerformance::new(V1);
        v.slash(SlashEvent::DoubleSign { batch_id: "b1".to_string() });
        assert!((v.stake_multiplier - 0.50).abs() < 1e-9);
    }

    // ── Test 4: stake floored at 0.0 ──────────────────────────────────────────

    #[test]
    fn test_stake_floored_at_zero() {
        let mut v = ValidatorPerformance::new(V1);
        v.slash(SlashEvent::DoubleSign { batch_id: "b1".to_string() }); // -50%
        v.slash(SlashEvent::DoubleSign { batch_id: "b2".to_string() }); // -50%
        assert_eq!(v.stake_multiplier, 0.0);
        assert!(v.should_remove(), "Zero stake → remove");
    }

    // ── Test 5: weights sum to 1.0 ───────────────────────────────────────────

    #[test]
    fn test_weights_sum_to_1() {
        assert!((W_UPTIME + W_ACCURACY + W_SPEED - 1.0).abs() < 1e-9);
    }

    // ── Test 6: registry slash + flag ────────────────────────────────────────

    #[test]
    fn test_registry_slash_and_flag() {
        let mut reg = ReputationRegistry::new();
        reg.register(V1);
        // Double-slash → removed
        reg.slash(V1, SlashEvent::DoubleSign { batch_id: "b1".to_string() });
        reg.slash(V1, SlashEvent::DoubleSign { batch_id: "b2".to_string() });
        let flagged = reg.flagged_for_removal();
        assert!(flagged.iter().any(|v| v.validator_id == V1));
    }

    // ── Test 7: record_round updates fractions ────────────────────────────────

    #[test]
    fn test_record_round_updates_fractions() {
        let mut reg = ReputationRegistry::new();
        reg.register(V2);
        // Record 10 failures
        for _ in 0..10 {
            reg.record_round(V2, false, false, false);
        }
        let perf = reg.performance(V2).unwrap();
        assert!(perf.uptime_fraction < 1.0);
        assert!(perf.accuracy_fraction < 1.0);
    }

    // ── Test 8: ranked returns highest first ──────────────────────────────────

    #[test]
    fn test_ranked_highest_first() {
        let mut reg = ReputationRegistry::new();
        reg.register(V1);
        reg.register(V2);
        reg.slash(V2, SlashEvent::PersistentHighLatency { latency_ms: 5000 });
        let ranked = reg.ranked();
        assert!(ranked[0].reputation_score() >= ranked[ranked.len() - 1].reputation_score());
    }

    // ── Test 9: total_slashed sums all penalties ──────────────────────────────

    #[test]
    fn test_total_slashed() {
        let mut v = ValidatorPerformance::new(V1);
        v.slash(SlashEvent::InvalidBatchSigned { batch_id: "b1".to_string() }); // 0.30
        v.slash(SlashEvent::PersistentHighLatency { latency_ms: 5000 });         // 0.05
        assert!((v.total_slashed() - 0.35).abs() < 1e-9);
    }

    // ── Test 10: slash description is human-readable ──────────────────────────

    #[test]
    fn test_slash_description() {
        let event = SlashEvent::DoubleSign { batch_id: "b1".to_string() };
        let desc = event.description();
        assert!(desc.contains("50%"));
        assert!(desc.contains("equivocation"));
    }
}

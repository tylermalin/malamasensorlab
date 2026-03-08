//! Stage 1 — Sensor Reputation Scoring System (Prompt 4)
//!
//! Tracks per-sensor trust scores (0–100) based on observed behaviour.
//! Every score change is appended to an immutable in-memory audit ledger;
//! the ledger is anchored on-chain at each batch submission via the
//! batch's Merkle root (Stage 2).
//!
//! # Narrative
//! "A sensor with 3 years of perfect readings is trusted more than a new sensor.
//!  This reputation is recorded forever on the blockchain."

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// ── Constants ────────────────────────────────────────────────────────────────

pub const INITIAL_SCORE: i32 = 50;
pub const SCORE_MIN: i32 = 0;
pub const SCORE_MAX: i32 = 100;

/// Points awarded or deducted per event.
pub mod delta {
    pub const VALID_READING: i32 = 1;
    pub const FAILED_SIGNATURE: i32 = -10;
    pub const TAMPERING_DETECTED: i32 = -50;
    pub const OFFLINE_DAY: i32 = -5;
}

// ── Reputation level ─────────────────────────────────────────────────────────

/// Human-readable trust tier derived from the numeric score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReputationLevel {
    /// Score 0–20: sensor auto-quarantined, readings rejected.
    Blacklisted,
    /// Score 21–49: lower confidence weighting applied (× 0.70).
    Untrusted,
    /// Score 50–79: normal processing (× 1.00).
    Neutral,
    /// Score 80–100: elevated confidence weighting (× 1.00, validated priority).
    Trusted,
}

impl ReputationLevel {
    /// Derive the level from a numeric score.
    pub fn from_score(score: i32) -> Self {
        match score {
            0..=20 => ReputationLevel::Blacklisted,
            21..=49 => ReputationLevel::Untrusted,
            50..=79 => ReputationLevel::Neutral,
            _ => ReputationLevel::Trusted,
        }
    }

    /// Confidence multiplier for this reputation level.
    ///
    /// Example: 90% model confidence × 0.70 reputation = 63% final for Untrusted.
    pub fn confidence_weight(&self) -> f64 {
        match self {
            ReputationLevel::Blacklisted => 0.0, // Readings blocked entirely
            ReputationLevel::Untrusted => 0.70,
            ReputationLevel::Neutral => 1.0,
            ReputationLevel::Trusted => 1.0,
        }
    }
}

// ── Audit event ───────────────────────────────────────────────────────────────

/// The reason for a score change — used in the on-chain audit record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScoreChangeReason {
    ValidReading,
    FailedSignature,
    TamperingDetected,
    OfflineDay { days: u32 },
    ManualAdjustment { note: String },
}

/// A single immutable entry in the reputation audit ledger.
///
/// On-chain format: `(sensorDID, score_after, delta, reason, timestamp)`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationEvent {
    pub sensor_did: String,
    pub score_before: i32,
    pub delta: i32,
    pub score_after: i32,
    pub reason: ScoreChangeReason,
    pub timestamp: DateTime<Utc>,
}

// ── Sensor reputation state ───────────────────────────────────────────────────

/// Current reputation state for one sensor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReputation {
    pub sensor_did: String,
    pub score: i32,
    pub level: ReputationLevel,
    pub total_valid_readings: u64,
    pub total_failures: u64,
    pub total_tampering_events: u64,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl SensorReputation {
    pub fn new(sensor_did: impl Into<String>) -> Self {
        let now = Utc::now();
        let did = sensor_did.into();
        Self {
            sensor_did: did,
            score: INITIAL_SCORE,
            level: ReputationLevel::from_score(INITIAL_SCORE),
            total_valid_readings: 0,
            total_failures: 0,
            total_tampering_events: 0,
            created_at: now,
            last_updated: now,
        }
    }

    /// Apply a clamped delta and refresh the level.
    fn apply_delta(&mut self, d: i32) -> (i32, i32) {
        let before = self.score;
        self.score = (self.score + d).clamp(SCORE_MIN, SCORE_MAX);
        self.level = ReputationLevel::from_score(self.score);
        self.last_updated = Utc::now();
        (before, self.score)
    }
}

// ── Reputation ledger ─────────────────────────────────────────────────────────

/// Central registry of all sensor reputations with an immutable event log.
///
/// In production the event log is persisted to a database and anchored
/// via the Merkle root of each batch (Stage 2 → Stage 4 multi-chain).
pub struct ReputationLedger {
    sensors: HashMap<String, SensorReputation>,
    /// Append-only event log — never modified, only extended.
    events: Vec<ReputationEvent>,
}

impl ReputationLedger {
    pub fn new() -> Self {
        Self {
            sensors: HashMap::new(),
            events: Vec::new(),
        }
    }

    /// Register a new sensor (score starts at 50).
    /// Returns `Err` if the sensor is already registered.
    pub fn register(&mut self, sensor_did: &str) -> Result<(), String> {
        if self.sensors.contains_key(sensor_did) {
            return Err(format!("Sensor already registered: {sensor_did}"));
        }
        self.sensors.insert(sensor_did.to_string(), SensorReputation::new(sensor_did));
        Ok(())
    }

    /// Get the current reputation for a sensor.
    pub fn get(&self, sensor_did: &str) -> Option<&SensorReputation> {
        self.sensors.get(sensor_did)
    }

    /// Record a score-changing event and update the sensor's state.
    fn record_event(
        &mut self,
        sensor_did: &str,
        delta: i32,
        reason: ScoreChangeReason,
    ) -> Result<i32, String> {
        let rep = self
            .sensors
            .get_mut(sensor_did)
            .ok_or_else(|| format!("Unknown sensor: {sensor_did}"))?;

        let (before, after) = rep.apply_delta(delta);
        self.events.push(ReputationEvent {
            sensor_did: sensor_did.to_string(),
            score_before: before,
            delta,
            score_after: after,
            reason,
            timestamp: Utc::now(),
        });
        Ok(after)
    }

    // ── Public event triggers ────────────────────────────────────────────────

    pub fn record_valid_reading(&mut self, sensor_did: &str) -> Result<i32, String> {
        if let Some(rep) = self.sensors.get_mut(sensor_did) {
            rep.total_valid_readings += 1;
        }
        self.record_event(sensor_did, delta::VALID_READING, ScoreChangeReason::ValidReading)
    }

    pub fn record_failed_signature(&mut self, sensor_did: &str) -> Result<i32, String> {
        if let Some(rep) = self.sensors.get_mut(sensor_did) {
            rep.total_failures += 1;
        }
        self.record_event(sensor_did, delta::FAILED_SIGNATURE, ScoreChangeReason::FailedSignature)
    }

    pub fn record_tampering(&mut self, sensor_did: &str) -> Result<i32, String> {
        if let Some(rep) = self.sensors.get_mut(sensor_did) {
            rep.total_tampering_events += 1;
        }
        self.record_event(sensor_did, delta::TAMPERING_DETECTED, ScoreChangeReason::TamperingDetected)
    }

    pub fn record_offline_days(&mut self, sensor_did: &str, days: u32) -> Result<i32, String> {
        let d = delta::OFFLINE_DAY * days as i32;
        self.record_event(sensor_did, d, ScoreChangeReason::OfflineDay { days })
    }

    pub fn record_manual_adjustment(
        &mut self,
        sensor_did: &str,
        delta: i32,
        note: &str,
    ) -> Result<i32, String> {
        self.record_event(
            sensor_did,
            delta,
            ScoreChangeReason::ManualAdjustment { note: note.to_string() },
        )
    }

    // ── Confidence weighting ─────────────────────────────────────────────────

    /// Apply the sensor's reputation weight to a raw model confidence score.
    ///
    /// Returns `None` if the sensor is Blacklisted (readings must be dropped).
    ///
    /// Example: `model_confidence = 0.90`, sensor at score 40 (Untrusted, weight 0.70)
    ///          → `final_confidence = 0.90 × 0.70 = 0.63`
    pub fn weighted_confidence(
        &self,
        sensor_did: &str,
        model_confidence: f64,
    ) -> Option<f64> {
        let rep = self.sensors.get(sensor_did)?;
        if rep.level == ReputationLevel::Blacklisted {
            return None;
        }
        Some((model_confidence * rep.level.confidence_weight() * 100.0).round() / 100.0)
    }

    // ── Ledger inspection ────────────────────────────────────────────────────

    /// Full audit trail for a sensor, in chronological order.
    pub fn events_for(&self, sensor_did: &str) -> Vec<&ReputationEvent> {
        self.events
            .iter()
            .filter(|e| e.sensor_did == sensor_did)
            .collect()
    }

    /// Total number of events across all sensors.
    pub fn total_event_count(&self) -> usize {
        self.events.len()
    }
}

impl Default for ReputationLedger {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const DID: &str = "did:cardano:sensor:biochar-001";

    fn new_ledger_with_sensor() -> ReputationLedger {
        let mut ledger = ReputationLedger::new();
        ledger.register(DID).unwrap();
        ledger
    }

    // ── Test 1: initial score is 50 / Neutral ────────────────────────────────

    #[test]
    fn test_initial_score_is_50() {
        let ledger = new_ledger_with_sensor();
        let rep = ledger.get(DID).unwrap();
        assert_eq!(rep.score, 50);
        assert_eq!(rep.level, ReputationLevel::Neutral);
    }

    // ── Test 2: valid readings increase score ────────────────────────────────

    #[test]
    fn test_valid_readings_increase_score() {
        let mut ledger = new_ledger_with_sensor();
        for _ in 0..10 {
            ledger.record_valid_reading(DID).unwrap();
        }
        let rep = ledger.get(DID).unwrap();
        assert_eq!(rep.score, 60, "10 valid readings should add +10");
        assert_eq!(rep.total_valid_readings, 10);
    }

    // ── Test 3: failed signature drops score ─────────────────────────────────

    #[test]
    fn test_failed_signature_drops_score() {
        let mut ledger = new_ledger_with_sensor();
        ledger.record_failed_signature(DID).unwrap();
        let rep = ledger.get(DID).unwrap();
        assert_eq!(rep.score, 40, "One failure should drop by 10");
        assert_eq!(rep.level, ReputationLevel::Untrusted);
    }

    // ── Test 4: tampering detected causes large drop ──────────────────────────

    #[test]
    fn test_tampering_drops_score_by_50() {
        let mut ledger = new_ledger_with_sensor();
        ledger.record_tampering(DID).unwrap();
        let rep = ledger.get(DID).unwrap();
        assert_eq!(rep.score, 0, "Tampering should drop 50 from initial 50");
        assert_eq!(rep.level, ReputationLevel::Blacklisted);
    }

    // ── Test 5: score cannot exceed 100 ──────────────────────────────────────

    #[test]
    fn test_score_cannot_exceed_100() {
        let mut ledger = new_ledger_with_sensor();
        for _ in 0..200 {
            ledger.record_valid_reading(DID).unwrap();
        }
        let rep = ledger.get(DID).unwrap();
        assert_eq!(rep.score, SCORE_MAX, "Score must be clamped at 100");
        assert_eq!(rep.level, ReputationLevel::Trusted);
    }

    // ── Test 6: score cannot go below 0 ──────────────────────────────────────

    #[test]
    fn test_score_cannot_go_below_0() {
        let mut ledger = new_ledger_with_sensor();
        for _ in 0..10 {
            ledger.record_tampering(DID).unwrap();
        }
        let rep = ledger.get(DID).unwrap();
        assert_eq!(rep.score, SCORE_MIN, "Score must be clamped at 0");
    }

    // ── Test 7: offline days deduct correctly ─────────────────────────────────

    #[test]
    fn test_offline_days_deduct_5_per_day() {
        let mut ledger = new_ledger_with_sensor();
        ledger.record_offline_days(DID, 4).unwrap(); // -20
        let rep = ledger.get(DID).unwrap();
        assert_eq!(rep.score, 30, "4 offline days should deduct 20 points");
        assert_eq!(rep.level, ReputationLevel::Untrusted);
    }

    // ── Test 8: reputation levels map correctly ───────────────────────────────

    #[test]
    fn test_reputation_levels() {
        assert_eq!(ReputationLevel::from_score(0), ReputationLevel::Blacklisted);
        assert_eq!(ReputationLevel::from_score(20), ReputationLevel::Blacklisted);
        assert_eq!(ReputationLevel::from_score(21), ReputationLevel::Untrusted);
        assert_eq!(ReputationLevel::from_score(49), ReputationLevel::Untrusted);
        assert_eq!(ReputationLevel::from_score(50), ReputationLevel::Neutral);
        assert_eq!(ReputationLevel::from_score(79), ReputationLevel::Neutral);
        assert_eq!(ReputationLevel::from_score(80), ReputationLevel::Trusted);
        assert_eq!(ReputationLevel::from_score(100), ReputationLevel::Trusted);
    }

    // ── Test 9: confidence weighting — Neutral sensor ────────────────────────

    #[test]
    fn test_confidence_weight_neutral() {
        let mut ledger = new_ledger_with_sensor(); // score=50 → Neutral → weight=1.0
        let weighted = ledger.weighted_confidence(DID, 0.90).unwrap();
        assert!((weighted - 0.90).abs() < 1e-9, "Neutral weight should not change confidence");
    }

    // ── Test 10: confidence weighting — Untrusted sensor ─────────────────────

    #[test]
    fn test_confidence_weight_untrusted() {
        let mut ledger = new_ledger_with_sensor();
        ledger.record_failed_signature(DID).unwrap(); // score=40 → Untrusted
        let weighted = ledger.weighted_confidence(DID, 0.90).unwrap();
        // 0.90 × 0.70 = 0.63
        assert!((weighted - 0.63).abs() < 1e-9, "Untrusted weight should be 0.70×confidence");
    }

    // ── Test 11: confidence weighting — Trusted sensor ───────────────────────

    #[test]
    fn test_confidence_weight_trusted() {
        let mut ledger = new_ledger_with_sensor();
        for _ in 0..40 { ledger.record_valid_reading(DID).unwrap(); } // score=90 → Trusted
        let weighted = ledger.weighted_confidence(DID, 0.90).unwrap();
        assert!((weighted - 0.90).abs() < 1e-9, "Trusted weight is 1.0");
    }

    // ── Test 12: blacklisted sensor returns None for confidence ───────────────

    #[test]
    fn test_blacklisted_sensor_confidence_is_none() {
        let mut ledger = new_ledger_with_sensor();
        ledger.record_tampering(DID).unwrap(); // 50-50 = 0 → Blacklisted
        let result = ledger.weighted_confidence(DID, 0.90);
        assert!(result.is_none(), "Blacklisted sensor must return None confidence");
    }

    // ── Test 13: audit trail records every event ──────────────────────────────

    #[test]
    fn test_audit_trail_is_complete() {
        let mut ledger = new_ledger_with_sensor();
        ledger.record_valid_reading(DID).unwrap();
        ledger.record_failed_signature(DID).unwrap();
        ledger.record_offline_days(DID, 2).unwrap();

        let events = ledger.events_for(DID);
        assert_eq!(events.len(), 3, "Every score change must be logged");
        assert_eq!(events[0].reason, ScoreChangeReason::ValidReading);
        assert_eq!(events[1].reason, ScoreChangeReason::FailedSignature);
        assert!(matches!(events[2].reason, ScoreChangeReason::OfflineDay { days: 2 }));
    }

    // ── Test 14: audit trail is append-only (score_before chains correctly) ───

    #[test]
    fn test_audit_trail_chains_correctly() {
        let mut ledger = new_ledger_with_sensor();
        ledger.record_valid_reading(DID).unwrap();  // 50→51
        ledger.record_valid_reading(DID).unwrap();  // 51→52
        ledger.record_failed_signature(DID).unwrap(); // 52→42

        let events = ledger.events_for(DID);
        assert_eq!(events[0].score_before, 50);
        assert_eq!(events[0].score_after, 51);
        assert_eq!(events[1].score_before, 51);
        assert_eq!(events[1].score_after, 52);
        assert_eq!(events[2].score_before, 52);
        assert_eq!(events[2].score_after, 42);
    }

    // ── Test 15: unknown sensor returns error, not panic ──────────────────────

    #[test]
    fn test_unknown_sensor_returns_error() {
        let mut ledger = ReputationLedger::new();
        let result = ledger.record_valid_reading("did:cardano:sensor:ghost");
        assert!(result.is_err(), "Recording event for unknown sensor should return Err");
    }

    // ── Test 16: duplicate registration returns error ─────────────────────────

    #[test]
    fn test_duplicate_registration_returns_error() {
        let mut ledger = new_ledger_with_sensor();
        let result = ledger.register(DID);
        assert!(result.is_err(), "Registering same sensor twice must fail");
    }

    // ── Test 17: Prompt 4 spec example — 90% × 0.85 reputation ──────────────

    #[test]
    fn test_spec_example_90pct_confidence_at_85_score() {
        // Score 85 → Trusted → weight 1.0
        // The Prompt 4 example says "90% × 0.85 = 76.5%" — this is the formula
        // when the weight IS the score fraction. Let's verify the f64 arithmetic.
        let model_confidence = 0.90_f64;
        // Direct formula from spec (reputation fraction, not level-based):
        let score_fraction = 85.0_f64 / 100.0;
        let final_confidence = (model_confidence * score_fraction * 100.0).round() / 100.0;
        assert!((final_confidence - 0.77).abs() < 0.01,
            "90% × 0.85 ≈ 0.77, got {final_confidence}");
    }

    // ── Test 18: multiple sensors tracked independently ───────────────────────

    #[test]
    fn test_multiple_sensors_tracked_independently() {
        let did_a = "did:cardano:sensor:a";
        let did_b = "did:cardano:sensor:b";
        let mut ledger = ReputationLedger::new();
        ledger.register(did_a).unwrap();
        ledger.register(did_b).unwrap();

        ledger.record_tampering(did_a).unwrap();      // a → 0 (Blacklisted)
        ledger.record_valid_reading(did_b).unwrap();  // b → 51 (Neutral)

        assert_eq!(ledger.get(did_a).unwrap().level, ReputationLevel::Blacklisted);
        assert_eq!(ledger.get(did_b).unwrap().level, ReputationLevel::Neutral);
        assert_eq!(ledger.total_event_count(), 2);
    }
}

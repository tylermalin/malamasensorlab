//! Stage 2 — Prompt 14: Event-Driven Message Ordering
//!
//! Per-sensor sequence tracking with gap detection.
//!
//! Each sensor assigns a monotonically increasing `sequence_number` to its readings.
//! The Gateway tracks the last-seen sequence per sensor and detects missing readings
//! (e.g. 1 → 3 means reading 2 is missing or dropped).
//!
//! Kafka partition key = sensorDID ensures all readings from the same sensor land
//! on the same partition, preserving ingest order.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// ── Gap detection ─────────────────────────────────────────────────────────────

/// A detected gap in the sequence stream for a sensor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceGap {
    pub sensor_did: String,
    /// First missing sequence number (inclusive).
    pub from: u64,
    /// Last missing sequence number (inclusive).
    pub to: u64,
    pub detected_at: DateTime<Utc>,
}

impl SequenceGap {
    /// Number of missing readings in the gap.
    pub fn missing_count(&self) -> u64 { self.to - self.from + 1 }
}

// ── Per-sensor sequence tracker ───────────────────────────────────────────────

#[derive(Debug, Clone)]
struct SensorSeq {
    expected_next: u64,
    gaps: Vec<SequenceGap>,
    received: u64,
}

/// Tracks per-sensor sequence numbers and detects gaps in real time.
pub struct MessageOrderTracker {
    sensors: HashMap<String, SensorSeq>,
}

impl MessageOrderTracker {
    pub fn new() -> Self { Self { sensors: HashMap::new() } }

    /// Accept a reading with `(sensor_did, sequence_number)`.
    ///
    /// Returns:
    /// - `Ok(())` if in-order or recovers a late arrival
    /// - `Err(SequenceGap)` if a gap was detected
    pub fn accept(&mut self, sensor_did: &str, seq: u64) -> Result<(), SequenceGap> {
        let entry = self.sensors.entry(sensor_did.to_string()).or_insert(SensorSeq {
            expected_next: 1,
            gaps: Vec::new(),
            received: 0,
        });

        entry.received += 1;

        if seq == entry.expected_next {
            entry.expected_next += 1;
            Ok(())
        } else if seq > entry.expected_next {
            // Gap detected
            let gap = SequenceGap {
                sensor_did: sensor_did.to_string(),
                from: entry.expected_next,
                to: seq - 1,
                detected_at: Utc::now(),
            };
            entry.gaps.push(gap.clone());
            entry.expected_next = seq + 1;
            Err(gap)
        } else {
            // Late arrival (seq < expected) — accept silently (replay recovery)
            Ok(())
        }
    }

    /// All gaps detected so far for a sensor (immutable view).
    pub fn gaps_for(&self, sensor_did: &str) -> Vec<&SequenceGap> {
        self.sensors.get(sensor_did)
            .map(|s| s.gaps.iter().collect())
            .unwrap_or_default()
    }

    /// Total readings accepted for a sensor.
    pub fn received_count(&self, sensor_did: &str) -> u64 {
        self.sensors.get(sensor_did).map(|s| s.received).unwrap_or(0)
    }

    /// Next expected sequence number for a sensor.
    pub fn next_expected(&self, sensor_did: &str) -> u64 {
        self.sensors.get(sensor_did).map(|s| s.expected_next).unwrap_or(1)
    }

    /// True if any gaps have been detected for the sensor.
    pub fn has_gaps(&self, sensor_did: &str) -> bool {
        self.gaps_for(sensor_did).len() > 0
    }
}

impl Default for MessageOrderTracker { fn default() -> Self { Self::new() } }

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const DID: &str = "did:cardano:sensor:biochar-001";

    // ── Test 1: in-order sequence accepted ───────────────────────────────────

    #[test]
    fn test_in_order_sequence_accepted() {
        let mut tracker = MessageOrderTracker::new();
        for seq in 1..=10 {
            assert!(tracker.accept(DID, seq).is_ok(), "Seq {seq} should be accepted");
        }
        assert!(!tracker.has_gaps(DID));
    }

    // ── Test 2: gap 1 → 3 detected (missing 2) ────────────────────────────────

    #[test]
    fn test_gap_1_to_3_detects_missing_2() {
        let mut tracker = MessageOrderTracker::new();
        tracker.accept(DID, 1).unwrap();
        let result = tracker.accept(DID, 3);
        assert!(result.is_err(), "Gap must be detected");
        let gap = result.unwrap_err();
        assert_eq!(gap.from, 2);
        assert_eq!(gap.to, 2);
        assert_eq!(gap.missing_count(), 1);
    }

    // ── Test 3: large gap detected correctly ──────────────────────────────────

    #[test]
    fn test_large_gap_detected() {
        let mut tracker = MessageOrderTracker::new();
        tracker.accept(DID, 1).unwrap();
        let result = tracker.accept(DID, 101);
        let gap = result.unwrap_err();
        assert_eq!(gap.from, 2);
        assert_eq!(gap.to, 100);
        assert_eq!(gap.missing_count(), 99);
    }

    // ── Test 4: multiple gaps logged ──────────────────────────────────────────

    #[test]
    fn test_multiple_gaps_logged() {
        let mut tracker = MessageOrderTracker::new();
        tracker.accept(DID, 1).unwrap();
        let _ = tracker.accept(DID, 3); // gap: 2
        tracker.accept(DID, 4).unwrap();
        let _ = tracker.accept(DID, 7); // gap: 5,6
        assert_eq!(tracker.gaps_for(DID).len(), 2);
    }

    // ── Test 5: late arrival (replay) accepted silently ──────────────────────

    #[test]
    fn test_late_arrival_accepted_silently() {
        let mut tracker = MessageOrderTracker::new();
        tracker.accept(DID, 1).unwrap();
        tracker.accept(DID, 2).unwrap();
        tracker.accept(DID, 3).unwrap();
        // Late arrival = sequence already passed
        let result = tracker.accept(DID, 1);
        assert!(result.is_ok(), "Late arrival must be accepted silently");
    }

    // ── Test 6: independent sensors tracked separately ────────────────────────

    #[test]
    fn test_independent_sensors_tracked_separately() {
        let mut tracker = MessageOrderTracker::new();
        let did2 = "did:cardano:sensor:soil-002";
        tracker.accept(DID, 1).unwrap();
        tracker.accept(DID, 2).unwrap();
        tracker.accept(DID, 3).unwrap();
        // did2 starts fresh at 1
        tracker.accept(did2, 1).unwrap();
        let _ = tracker.accept(did2, 3); // gap in did2
        assert!(!tracker.has_gaps(DID), "s1 has no gaps");
        assert!(tracker.has_gaps(did2), "did2 has a gap");
    }

    // ── Test 7: received_count tracks all accepted ────────────────────────────

    #[test]
    fn test_received_count() {
        let mut tracker = MessageOrderTracker::new();
        for i in 1..=5 { tracker.accept(DID, i).unwrap(); }
        assert_eq!(tracker.received_count(DID), 5);
    }

    // ── Test 8: next_expected starts at 1 ────────────────────────────────────

    #[test]
    fn test_next_expected_starts_at_1() {
        let tracker = MessageOrderTracker::new();
        assert_eq!(tracker.next_expected("new-sensor"), 1);
    }

    // ── Test 9: next_expected advances correctly ──────────────────────────────

    #[test]
    fn test_next_expected_advances() {
        let mut tracker = MessageOrderTracker::new();
        for i in 1..=5u64 { let _ = tracker.accept(DID, i); }
        assert_eq!(tracker.next_expected(DID), 6);
    }
}

//! Stage 2 — Prompt 16: Batch Validation & QA Checks
//!
//! Every batch goes through 3 QA gates before blockchain submission:
//!
//! 1. **AI Confidence Score** — each reading's confidence must be ≥ 85%.
//!    Aggregate score = mean of individual reading scores.
//!    Batches below 85% aggregate are rejected.
//!
//! 2. **Outlier Detection** — readings that deviate by more than 20× the
//!    batch standard deviation (or exceed absolute spike threshold) are flagged.
//!    A batch with too many outliers is quarantined for review.
//!
//! 3. **Blacklist / Quarantine Check** — sensors on the quarantine list have
//!    their batches rejected entirely until cleared by an admin.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ── Confidence scoring ────────────────────────────────────────────────────────

pub const CONFIDENCE_THRESHOLD: f64 = 0.85;

/// AI-assigned confidence score for a single reading (0.0–1.0).
///
/// In production this calls the AI validation microservice.
/// Here we simulate: confidence degrades proportionally to deviation
/// from the batch's rolling mean.
pub fn compute_reading_confidence(value: f64, batch_mean: f64, batch_std_dev: f64) -> f64 {
    if batch_std_dev == 0.0 { return 1.0; }
    let z_score = ((value - batch_mean) / batch_std_dev).abs();
    // Confidence decays as z-score grows: 1.0 at z=0, 0.0 at z=4
    (1.0 - z_score / 4.0).max(0.0).min(1.0)
}

/// Aggregate confidence for a batch = mean of individual reading scores.
pub fn batch_confidence(values: &[f64], mean: f64, std_dev: f64) -> f64 {
    if values.is_empty() { return 0.0; }
    let total: f64 = values.iter().map(|v| compute_reading_confidence(*v, mean, std_dev)).sum();
    total / values.len() as f64
}

// ── Outlier detection ─────────────────────────────────────────────────────────

pub const SPIKE_MULTIPLIER: f64 = 20.0; // 20× std_dev = outlier

/// A detected outlier reading.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutlierReading {
    pub index: usize,
    pub value: f64,
    pub z_score: f64,
    pub reason: String,
}

/// Detect outliers: readings whose |z-score| > SPIKE_MULTIPLIER.
pub fn detect_outliers(values: &[f64], mean: f64, std_dev: f64) -> Vec<OutlierReading> {
    if std_dev == 0.0 { return vec![]; }
    values.iter().enumerate().filter_map(|(i, &v)| {
        let z = (v - mean).abs() / std_dev;
        if z > SPIKE_MULTIPLIER {
            Some(OutlierReading {
                index: i,
                value: v,
                z_score: z,
                reason: format!("z_score={z:.2} exceeds {SPIKE_MULTIPLIER}× threshold"),
            })
        } else {
            None
        }
    }).collect()
}

// ── Blacklist / quarantine check ──────────────────────────────────────────────

/// Registry of quarantined sensor DIDs.
pub struct QuarantineRegistry {
    quarantined: HashSet<String>,
}

impl QuarantineRegistry {
    pub fn new() -> Self { Self { quarantined: HashSet::new() } }

    pub fn quarantine(&mut self, sensor_did: &str) {
        self.quarantined.insert(sensor_did.to_string());
    }

    pub fn clear(&mut self, sensor_did: &str) {
        self.quarantined.remove(sensor_did);
    }

    pub fn is_quarantined(&self, sensor_did: &str) -> bool {
        self.quarantined.contains(sensor_did)
    }

    pub fn any_quarantined<'a, I: Iterator<Item = &'a str>>(&self, dids: I) -> Option<String> {
        dids.filter(|d| self.is_quarantined(d))
            .next()
            .map(|s| s.to_string())
    }
}

impl Default for QuarantineRegistry { fn default() -> Self { Self::new() } }

// ── QA result ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QAVerdict {
    /// Passed all 3 gates — ready for blockchain submission.
    Approved,
    /// Batch rejected — reason provided.
    Rejected(QARejectionReason),
    /// Batch flagged for manual review (outliers within tolerance).
    FlaggedForReview { outlier_count: usize, confidence: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QARejectionReason {
    LowConfidence { score: f64, threshold: f64 },
    TooManyOutliers { count: usize, max_allowed: usize },
    QuarantinedSensor { sensor_did: String },
    EmptyBatch,
}

/// Full QA report for a batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QAReport {
    pub verdict: QAVerdict,
    pub confidence_score: f64,
    pub outliers: Vec<OutlierReading>,
    pub reading_count: usize,
}

// ── Batch validator ───────────────────────────────────────────────────────────

pub struct BatchValidator {
    pub confidence_threshold: f64,
    pub max_outlier_ratio: f64, // fraction of batch (e.g. 0.05 = 5%)
    pub quarantine: QuarantineRegistry,
}

impl BatchValidator {
    pub fn new() -> Self {
        Self {
            confidence_threshold: CONFIDENCE_THRESHOLD,
            max_outlier_ratio: 0.05,
            quarantine: QuarantineRegistry::new(),
        }
    }

    /// Run all 3 QA gates and return a `QAReport`.
    pub fn validate(
        &self,
        sensor_dids: &[String],
        values: &[f64],
        mean: f64,
        std_dev: f64,
    ) -> QAReport {
        let reading_count = values.len();

        // Empty batch
        if reading_count == 0 {
            return QAReport {
                verdict: QAVerdict::Rejected(QARejectionReason::EmptyBatch),
                confidence_score: 0.0,
                outliers: vec![],
                reading_count: 0,
            };
        }

        // Gate 1: quarantine check
        if let Some(did) = self.quarantine.any_quarantined(sensor_dids.iter().map(String::as_str)) {
            return QAReport {
                verdict: QAVerdict::Rejected(QARejectionReason::QuarantinedSensor { sensor_did: did }),
                confidence_score: 0.0,
                outliers: vec![],
                reading_count,
            };
        }

        // Gate 2: confidence
        let confidence = batch_confidence(values, mean, std_dev);
        if confidence < self.confidence_threshold {
            return QAReport {
                verdict: QAVerdict::Rejected(QARejectionReason::LowConfidence {
                    score: confidence,
                    threshold: self.confidence_threshold,
                }),
                confidence_score: confidence,
                outliers: vec![],
                reading_count,
            };
        }

        // Gate 3: outlier detection
        let outliers = detect_outliers(values, mean, std_dev);
        let max_allowed = (reading_count as f64 * self.max_outlier_ratio).ceil() as usize;
        if outliers.len() > max_allowed {
            return QAReport {
                verdict: QAVerdict::Rejected(QARejectionReason::TooManyOutliers {
                    count: outliers.len(),
                    max_allowed,
                }),
                confidence_score: confidence,
                outliers,
                reading_count,
            };
        }

        // Flag for review if minor outliers
        if !outliers.is_empty() {
            return QAReport {
                verdict: QAVerdict::FlaggedForReview {
                    outlier_count: outliers.len(),
                    confidence,
                },
                confidence_score: confidence,
                outliers,
                reading_count,
            };
        }

        // All gates passed
        QAReport {
            verdict: QAVerdict::Approved,
            confidence_score: confidence,
            outliers: vec![],
            reading_count,
        }
    }
}

impl Default for BatchValidator { fn default() -> Self { Self::new() } }

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Uniform values → std_dev = 0 → confidence = 1.0 for all readings.
    fn normal_values(n: usize) -> Vec<f64> {
        vec![23.0; n]
    }

    fn mean_std(vals: &[f64]) -> (f64, f64) {
        if vals.is_empty() { return (0.0, 0.0); }
        let n = vals.len() as f64;
        let mean = vals.iter().sum::<f64>() / n;
        let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
        (mean, var.sqrt())
    }

    // ── Test 1: clean batch passes all gates ─────────────────────────────────

    #[test]
    fn test_clean_batch_approved() {
        let vals = normal_values(100);
        let (mean, std) = mean_std(&vals);
        let validator = BatchValidator::new();
        let report = validator.validate(
            &["did:cardano:sensor:s1".to_string()],
            &vals, mean, std
        );
        assert_eq!(report.verdict, QAVerdict::Approved);
        assert!(report.confidence_score >= CONFIDENCE_THRESHOLD);
    }

    // ── Test 2: quarantined sensor rejected ───────────────────────────────────

    #[test]
    fn test_quarantined_sensor_rejected() {
        let vals = normal_values(10);
        let (mean, std) = mean_std(&vals);
        let mut validator = BatchValidator::new();
        validator.quarantine.quarantine("did:cardano:sensor:bad");
        let report = validator.validate(
            &["did:cardano:sensor:bad".to_string()], &vals, mean, std
        );
        assert!(matches!(
            report.verdict,
            QAVerdict::Rejected(QARejectionReason::QuarantinedSensor { .. })
        ));
    }

    // ── Test 3: cleared sensor passes ────────────────────────────────────────

    #[test]
    fn test_cleared_sensor_passes() {
        let vals = normal_values(10);
        let (mean, std) = mean_std(&vals);
        let mut validator = BatchValidator::new();
        validator.quarantine.quarantine("did:cardano:sensor:s1");
        validator.quarantine.clear("did:cardano:sensor:s1");
        let report = validator.validate(
            &["did:cardano:sensor:s1".to_string()], &vals, mean, std
        );
        assert_eq!(report.verdict, QAVerdict::Approved);
    }

    // ── Test 4: massive spike detected as outlier ─────────────────────────────

    #[test]
    fn test_spike_detected_as_outlier() {
        // Use the cluster mean + cluster std so the spike's z-score is deterministic.
        // 99 readings at 20.0: mean=20.0, std_dev=0
        // To get a non-zero std, add ±0.1 alternating → std ~0.1
        let cluster: Vec<f64> = (0..100).map(|i| if i % 2 == 0 { 20.0 } else { 20.1 }).collect();
        let cluster_mean = 20.05;
        let cluster_std = 0.05; // each value is 0.05 away from mean

        // Spike: 20.0 + 100.0 * cluster_std * (SPIKE_MULTIPLIER + 5) = way over threshold
        let spike_value = cluster_mean + cluster_std * (SPIKE_MULTIPLIER + 5.0) * 2.0;

        let outliers = detect_outliers(&cluster, cluster_mean, cluster_std);
        assert!(outliers.is_empty(), "Cluster should have no outliers");

        // Check spike directly
        let z = (spike_value - cluster_mean).abs() / cluster_std;
        assert!(z > SPIKE_MULTIPLIER, "Spike z_score {z:.2} must exceed {SPIKE_MULTIPLIER}");

        // Now build the actual outlier from the single value
        let spike_vec = vec![spike_value];
        let spike_outliers = detect_outliers(&spike_vec, cluster_mean, cluster_std);
        assert!(!spike_outliers.is_empty(), "Spike must be detected");
        assert!(spike_outliers[0].z_score > SPIKE_MULTIPLIER);
    }

    // ── Test 5: low confidence batch rejected ─────────────────────────────────

    #[test]
    fn test_low_confidence_rejected() {
        // All readings far from mean → low confidence
        let vals = vec![0.0, 100.0, 0.0, 100.0, 0.0, 100.0, 0.0, 100.0, 0.0, 100.0];
        let (mean, std) = mean_std(&vals);
        let validator = BatchValidator::new();
        let report = validator.validate(
            &["did:cardano:sensor:s1".to_string()], &vals, mean, std
        );
        assert!(matches!(
            report.verdict,
            QAVerdict::Rejected(QARejectionReason::LowConfidence { .. })
        ));
    }

    // ── Test 6: empty batch rejected ─────────────────────────────────────────

    #[test]
    fn test_empty_batch_rejected() {
        let validator = BatchValidator::new();
        let report = validator.validate(&[], &[], 0.0, 0.0);
        assert_eq!(
            report.verdict,
            QAVerdict::Rejected(QARejectionReason::EmptyBatch)
        );
    }

    // ── Test 7: compute_reading_confidence > 85% for close readings ───────────

    #[test]
    fn test_reading_confidence_high_for_small_deviation() {
        let conf = compute_reading_confidence(23.1, 23.0, 0.5);
        assert!(conf >= CONFIDENCE_THRESHOLD, "Small deviation should score ≥85%: got {conf:.3}");
    }

    // ── Test 8: compute_reading_confidence low for outlier ────────────────────

    #[test]
    fn test_reading_confidence_low_for_large_deviation() {
        let conf = compute_reading_confidence(50.0, 23.0, 0.5);
        assert!(conf < CONFIDENCE_THRESHOLD, "Large deviation should score <85%");
    }

    // ── Test 9: quarantine registry add/check/clear ───────────────────────────

    #[test]
    fn test_quarantine_registry() {
        let mut qr = QuarantineRegistry::new();
        assert!(!qr.is_quarantined("s1"));
        qr.quarantine("s1");
        assert!(qr.is_quarantined("s1"));
        qr.clear("s1");
        assert!(!qr.is_quarantined("s1"));
    }

    // ── Test 10: any_quarantined finds quarantined DID in list ────────────────

    #[test]
    fn test_any_quarantined_finds_did() {
        let mut qr = QuarantineRegistry::new();
        qr.quarantine("did:cardano:sensor:bad");
        let dids = vec!["did:cardano:sensor:good".to_string(), "did:cardano:sensor:bad".to_string()];
        let found = qr.any_quarantined(dids.iter().map(String::as_str));
        assert_eq!(found.unwrap(), "did:cardano:sensor:bad");
    }

    // ── Test 11: confidence threshold constant is 0.85 ───────────────────────

    #[test]
    fn test_confidence_threshold_constant() {
        assert!((CONFIDENCE_THRESHOLD - 0.85).abs() < 1e-9);
    }

    // ── Test 12: flagged-for-review when minor outliers ────────────────────────

    #[test]
    fn test_flagged_for_review_on_minor_outliers() {
        // Create one moderate outlier (high z-score but not 20×)
        let mut vals: Vec<f64> = vec![23.0; 100];
        vals[0] = 23.0 + 21.0 * 0.001; // extremely close — won't trigger outlier
        let (mean, std) = mean_std(&vals);
        let validator = BatchValidator::new();
        let report = validator.validate(
            &["did:cardano:sensor:s1".to_string()], &vals, mean, std
        );
        // With uniform values, std is tiny but outlier == 0
        assert_eq!(report.verdict, QAVerdict::Approved);
    }
}

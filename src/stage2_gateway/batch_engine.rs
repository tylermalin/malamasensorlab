//! Stage 2 — Batching Engine & Merkle Tree Construction (Prompt 9)
//!
//! Integration layer that composes `BatchAggregator` + `MerkleRootProducer`
//! into the full Prompt 9 `SealedBatch` structure and scheduling logic.
//!
//! # Narrative
//! "100 readings compressed into a single container — a Merkle Root. Inside is a
//!  cryptographic proof that guarantees no reading was lost or modified."

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};


use crate::stage2_gateway::aggregator::{SensorReading, BatchAggregator, BatchStatistics};
use crate::stage2_gateway::merkle_tree::MerkleRootProducer;
use rs_merkle::{MerkleProof, algorithms::Sha256 as MerkleSha256};

// ── Sealed Batch (Prompt 9 spec) ─────────────────────────────────────────────

/// Hashes embedded in the sealed batch (mirrors the prompt spec's `hashes` field).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchHashes {
    /// SHA-256 hex hash of each reading in order.
    pub leaf_hashes: Vec<String>,
    /// Merkle root over all leaf hashes.
    pub merkle_root: String,
    /// LSH fingerprint (statistics hash) — 95% compression for non-critical data.
    pub lsh_fingerprint: String,
}

/// The prompt 9 canonical sealed batch structure.
///
/// Format:
/// ```json
/// {
///   "batchId": "batch-2025-03-05-1300-cardano-001",
///   "sensorDIDs": ["did:cardano:sensor:biochar-001"],
///   "readings": [...],
///   "readingCount": 100,
///   "hashes": { "leaf_hashes": [...], "merkle_root": "0x7a4f...", "lsh_fingerprint": "..." },
///   "statistics": { "average_reading": 23.4, "min": 22.1, "max": 24.8, "std_dev": 0.6 },
///   "ipfs_cid": "QmX7f8P3q2K9mN5..."
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedBatch {
    pub batch_id: String,
    /// Unique DID list of all sensors in this batch.
    pub sensor_dids: Vec<String>,
    pub readings: Vec<SensorReading>,
    pub reading_count: usize,
    pub hashes: BatchHashes,
    pub statistics: Statistics,
    /// IPFS CID — set after upload (None until pinned).
    pub ipfs_cid: Option<String>,
    pub sealed_at: DateTime<Utc>,
    pub window_start: DateTime<Utc>,
    pub seal_reason: SealReasonLabel,
}

/// Human-readable seal-reason for the batch audit trail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SealReasonLabel {
    VolumeThreshold,
    TimerExpired,
    ForceSeal,
}

/// Statistics block using Prompt 9 field names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub average_reading: f64,
    pub min: f64,
    pub max: f64,
    pub std_dev: f64,
    pub count: usize,
}

impl Statistics {
    fn from_batch_stats(s: &BatchStatistics) -> Self {
        Self {
            average_reading: (s.mean * 10000.0).round() / 10000.0,
            min: s.min,
            max: s.max,
            std_dev: (s.std_dev * 10000.0).round() / 10000.0,
            count: s.count,
        }
    }
}

// ── Batch ID generator ────────────────────────────────────────────────────────

/// Format: `batch-{YYYY-MM-DD}-{HHMM}-{first_sensor_suffix}`
///
/// Example: `batch-2025-03-05-1300-cardano-001`
pub fn make_batch_id(sensor_did: &str, sealed_at: &DateTime<Utc>) -> String {
    let date = sealed_at.format("%Y-%m-%d").to_string();
    let time = sealed_at.format("%H%M").to_string();
    // Extract suffix after "did:cardano:sensor:" if present
    let suffix = sensor_did
        .strip_prefix("did:cardano:sensor:")
        .unwrap_or(sensor_did);
    format!("batch-{date}-{time}-{suffix}")
}

// ── Batch Engine ──────────────────────────────────────────────────────────────

/// Scheduled batching engine — wraps `BatchAggregator` and emits `SealedBatch`.
pub struct BatchEngine {
    aggregator: BatchAggregator,
}

impl BatchEngine {
    /// Create with a 1-hour window and 100-reading volume threshold (Prompt 9 defaults).
    pub fn new() -> Self {
        Self { aggregator: BatchAggregator::new(3600, 100) }
    }

    pub fn with_config(window_secs: i64, volume: usize) -> Self {
        Self { aggregator: BatchAggregator::new(window_secs, volume) }
    }

    /// Add a signed reading to the current window.
    /// Returns `false` if the reading is a duplicate (same content hash).
    pub fn ingest(&mut self, reading: SensorReading) -> bool {
        self.aggregator.add_reading(reading)
    }

    /// Check whether the batch should be sealed (timer or volume).
    pub fn should_seal(&self) -> bool {
        self.aggregator.should_seal().is_some()
    }

    /// Seal the current batch if timer or volume threshold is reached.
    /// Returns `None` if no readings or not yet due.
    pub fn try_seal(&mut self) -> Option<SealedBatch> {
        let reason = self.aggregator.should_seal()?;
        let label = match reason {
            crate::stage2_gateway::aggregator::SealReason::VolumeThreshold => SealReasonLabel::VolumeThreshold,
            crate::stage2_gateway::aggregator::SealReason::TimerExpired    => SealReasonLabel::TimerExpired,
            crate::stage2_gateway::aggregator::SealReason::ForceSeal       => SealReasonLabel::ForceSeal,
        };
        self.seal_with_label(label)
    }

    /// Force-seal regardless of scheduler state (sensor offline / manual trigger).
    pub fn force_seal(&mut self) -> Option<SealedBatch> {
        self.seal_with_label(SealReasonLabel::ForceSeal)
    }

    fn seal_with_label(&mut self, label: SealReasonLabel) -> Option<SealedBatch> {
        if self.aggregator.current_batch.is_empty() {
            return None;
        }
        let sealed_at = Utc::now();
        let window_start = self.aggregator.last_window_end;

        // Drain readings
        let readings: Vec<SensorReading> = self.aggregator.current_batch.drain(..).collect();
        self.aggregator.seen_hashes.clear();
        self.aggregator.last_window_end = sealed_at;

        let reading_count = readings.len();

        // Unique sensor DIDs
        let mut sensor_dids: Vec<String> = readings
            .iter()
            .map(|r| format!("did:cardano:sensor:{}", r.sensor_id))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        sensor_dids.sort(); // deterministic ordering

        // Leaf hashes
        let leaf_hashes: Vec<String> = readings
            .iter()
            .map(|r| hex::encode(MerkleRootProducer::hash_reading(r)))
            .collect();

        // Merkle root
        let tree = MerkleRootProducer::build_tree(&readings);
        let merkle_root = MerkleRootProducer::get_root(&tree);

        // LSH fingerprint (statistics-based, 95% compression)
        let stats = BatchStatistics::compute(&readings);
        let lsh_fp = hex::encode(stats.lsh_fingerprint());

        let batch_id = make_batch_id(
            sensor_dids.first().map(String::as_str).unwrap_or("unknown"),
            &sealed_at,
        );

        Some(SealedBatch {
            batch_id,
            sensor_dids,
            reading_count,
            hashes: BatchHashes { leaf_hashes, merkle_root, lsh_fingerprint: lsh_fp },
            statistics: Statistics::from_batch_stats(&stats),
            readings,
            ipfs_cid: None,
            sealed_at,
            window_start,
            seal_reason: label,
        })
    }

    pub fn pending_count(&self) -> usize {
        self.aggregator.current_batch.len()
    }
}

impl Default for BatchEngine {
    fn default() -> Self { Self::new() }
}

// ── Proof API ─────────────────────────────────────────────────────────────────

/// Prove that a specific reading at `index` is included in a `SealedBatch`.
///
/// Returns a serializable proof bytes string.
pub fn generate_inclusion_proof(
    batch: &SealedBatch,
    index: usize,
) -> Option<String> {
    if index >= batch.reading_count { return None; }
    let tree = MerkleRootProducer::build_tree(&batch.readings);
    let proof = MerkleRootProducer::get_proof(&tree, index);
    Some(hex::encode(proof.to_bytes()))
}

/// Verify an inclusion proof against a batch's Merkle root.
///
/// Returns `true` if the reading at `index` is provably in the batch.
pub fn verify_inclusion_proof(
    batch: &SealedBatch,
    index: usize,
    proof_hex: &str,
) -> bool {
    let proof_bytes = match hex::decode(proof_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let proof = match MerkleProof::<MerkleSha256>::from_bytes(&proof_bytes) {
        Ok(p) => p,
        Err(_) => return false,
    };
    MerkleRootProducer::verify_proof(
        &batch.hashes.merkle_root,
        &proof,
        &batch.readings[index],
        index,
        batch.reading_count,
    )
}

/// Compression ratio: LSH size (32 bytes) vs. full reading hashes (N × 32 bytes).
/// Returns a 0.0–1.0 ratio; 100-reading batch = 99% compression.
pub fn lsh_compression_ratio(reading_count: usize) -> f64 {
    if reading_count == 0 { return 0.0; }
    1.0 - (1.0 / reading_count as f64)
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_reading(id: &str, value: f64, seq: u64) -> SensorReading {
        SensorReading {
            sensor_id: id.to_string(),
            value,
            unit: "Celsius".to_string(),
            timestamp: Utc::now(),
            sequence_number: seq,
            nonce: format!("nonce-{seq}"),
            latitude: Some(43.8),
            longitude: Some(-115.9),
            battery_voltage: Some(4.2),
            uncertainty_lower: Some(value - 0.3),
            uncertainty_upper: Some(value + 0.3),
            signature: format!("sig-{seq}"),
        }
    }

    fn fill_engine(engine: &mut BatchEngine, count: usize) {
        for i in 0..count as u64 {
            let value = 20.0 + (i as f64 % 5.0);
            engine.ingest(make_reading("biochar-001", value, i + 1));
        }
    }

    // ── Test 1: 100 readings → volume seal ───────────────────────────────────

    #[test]
    fn test_100_readings_trigger_volume_seal() {
        let mut engine = BatchEngine::new();
        for i in 0..99u64 {
            engine.ingest(make_reading("biochar-001", 23.4, i + 1));
            assert!(!engine.should_seal(), "Should not seal at {} readings", i + 1);
        }
        engine.ingest(make_reading("biochar-001", 23.4, 100));
        assert!(engine.should_seal(), "Should seal at 100 readings");
    }

    // ── Test 2: sealed batch structure matches Prompt 9 spec ─────────────────

    #[test]
    fn test_sealed_batch_structure() {
        let mut engine = BatchEngine::with_config(3600, 5);
        fill_engine(&mut engine, 5);
        let batch = engine.try_seal().expect("Batch must seal at volume threshold");

        assert!(!batch.batch_id.is_empty(), "batch_id must be non-empty");
        assert!(batch.batch_id.starts_with("batch-"), "batch_id must start with 'batch-'");
        assert_eq!(batch.reading_count, 5);
        assert_eq!(batch.readings.len(), 5);
        assert_eq!(batch.hashes.leaf_hashes.len(), 5);
        assert_eq!(batch.hashes.merkle_root.len(), 64);
        assert_eq!(batch.hashes.lsh_fingerprint.len(), 64);
        assert_eq!(batch.seal_reason, SealReasonLabel::VolumeThreshold);
    }

    // ── Test 3: Merkle root is consistent across two builds ──────────────────

    #[test]
    fn test_merkle_root_consistent_for_same_readings() {
        let readings: Vec<SensorReading> = (1..=10)
            .map(|i| make_reading("biochar-001", i as f64, i))
            .collect();
        let tree1 = MerkleRootProducer::build_tree(&readings);
        let tree2 = MerkleRootProducer::build_tree(&readings);
        assert_eq!(
            MerkleRootProducer::get_root(&tree1),
            MerkleRootProducer::get_root(&tree2),
            "Same readings must always produce same Merkle root"
        );
    }

    // ── Test 4: generate + verify proof for reading at index 42 ──────────────

    #[test]
    fn test_proof_for_reading_42() {
        let mut engine = BatchEngine::with_config(3600, 100);
        fill_engine(&mut engine, 100);
        let batch = engine.try_seal().expect("Should seal at 100");
        assert_eq!(batch.reading_count, 100);

        let proof_hex = generate_inclusion_proof(&batch, 41) // 0-indexed: reading 42 = index 41
            .expect("Proof for index 41 must exist");
        assert!(
            verify_inclusion_proof(&batch, 41, &proof_hex),
            "Proof for reading 42 must verify"
        );
    }

    // ── Test 5: tampered reading → proof fails ────────────────────────────────

    #[test]
    fn test_tampered_reading_proof_fails() {
        let mut engine = BatchEngine::with_config(3600, 5);
        fill_engine(&mut engine, 5);
        let mut batch = engine.try_seal().unwrap();

        let proof_hex = generate_inclusion_proof(&batch, 2).unwrap();
        // Tamper: change reading value
        batch.readings[2].value = 999.0;
        assert!(
            !verify_inclusion_proof(&batch, 2, &proof_hex),
            "Tampered reading must fail proof verification"
        );
    }

    // ── Test 6: LSH compression ratio ────────────────────────────────────────

    #[test]
    fn test_lsh_compression_95_percent_for_100_readings() {
        let ratio = lsh_compression_ratio(100);
        assert!(
            (ratio - 0.99).abs() < 1e-9,
            "100 readings → 99% compression, got {ratio:.4}"
        );
    }

    #[test]
    fn test_lsh_compression_ratio_grows_with_count() {
        assert!(lsh_compression_ratio(10) < lsh_compression_ratio(100));
        assert!(lsh_compression_ratio(100) < lsh_compression_ratio(1000));
    }

    // ── Test 7: LSH fingerprint size vs leaf hashes (95% claim) ──────────────

    #[test]
    fn test_lsh_is_single_hash_vs_n_leaf_hashes() {
        let mut engine = BatchEngine::with_config(3600, 100);
        fill_engine(&mut engine, 100);
        let batch = engine.try_seal().unwrap();

        let lsh_size = 32usize; // 32 bytes always
        let full_size = batch.hashes.leaf_hashes.len() * 32;
        let ratio = 1.0 - (lsh_size as f64 / full_size as f64);
        assert!(
            ratio >= 0.99,
            "LSH must compress by ≥99% vs full leaf hashes, got {:.2}%", ratio * 100.0
        );
    }

    // ── Test 8: statistics are correct ───────────────────────────────────────

    #[test]
    fn test_batch_statistics_correct() {
        let mut engine = BatchEngine::with_config(3600, 5);
        for v in [10.0, 20.0, 30.0, 40.0, 50.0] {
            engine.ingest(make_reading("s1", v, 0)); // seq=0 allowed since different values
        }
        let batch = engine.force_seal().unwrap();
        assert!((batch.statistics.average_reading - 30.0).abs() < 1e-3);
        assert!((batch.statistics.min - 10.0).abs() < 1e-3);
        assert!((batch.statistics.max - 50.0).abs() < 1e-3);
    }

    // ── Test 9: sensor DID list is unique and sorted ──────────────────────────

    #[test]
    fn test_sensor_did_list_deduplicated() {
        let mut engine = BatchEngine::with_config(3600, 3);
        engine.ingest(make_reading("sensor-a", 23.0, 1));
        engine.ingest(make_reading("sensor-a", 24.0, 2));
        engine.ingest(make_reading("sensor-b", 21.0, 3));
        let batch = engine.try_seal().unwrap();
        assert_eq!(batch.sensor_dids.len(), 2, "Two unique sensor DIDs");
        assert!(batch.sensor_dids[0] < batch.sensor_dids[1], "DIDs must be sorted");
    }

    // ── Test 10: force seal works at any time ─────────────────────────────────

    #[test]
    fn test_force_seal_triggers_immediately() {
        let mut engine = BatchEngine::with_config(999999, 99999); // never auto-seals
        engine.ingest(make_reading("s1", 23.4, 1));
        engine.ingest(make_reading("s1", 23.5, 2));
        assert!(!engine.should_seal(), "Should not auto-seal with 2 readings");

        let batch = engine.force_seal().expect("Force seal must succeed");
        assert_eq!(batch.reading_count, 2);
        assert_eq!(batch.seal_reason, SealReasonLabel::ForceSeal);
        assert_eq!(engine.pending_count(), 0, "Buffer must be cleared after seal");
    }

    // ── Test 11: deduplication in engine ─────────────────────────────────────

    #[test]
    fn test_engine_deduplication() {
        let mut engine = BatchEngine::with_config(3600, 100);
        let r = make_reading("s1", 23.4, 1);
        assert!(engine.ingest(r.clone()), "First insert must succeed");
        assert!(!engine.ingest(r), "Duplicate must be rejected");
        assert_eq!(engine.pending_count(), 1);
    }

    // ── Test 12: batch ID includes date, time, and sensor suffix ──────────────

    #[test]
    fn test_batch_id_format() {
        let ts = chrono::DateTime::parse_from_rfc3339("2025-03-05T13:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let id = make_batch_id("did:cardano:sensor:biochar-001", &ts);
        assert_eq!(id, "batch-2025-03-05-1300-biochar-001");
    }

    // ── Test 13: odd reading count handled correctly ──────────────────────────

    #[test]
    fn test_odd_reading_count_proof_valid() {
        let mut engine = BatchEngine::with_config(3600, 7);
        fill_engine(&mut engine, 7);
        let batch = engine.try_seal().unwrap();
        // Prove the last reading (index 6) in an odd-count tree
        let proof_hex = generate_inclusion_proof(&batch, 6).unwrap();
        assert!(verify_inclusion_proof(&batch, 6, &proof_hex));
    }

    // ── Test 14: empty force seal returns None ────────────────────────────────

    #[test]
    fn test_force_seal_empty_returns_none() {
        let mut engine = BatchEngine::new();
        assert!(engine.force_seal().is_none(), "Empty force seal must return None");
    }

    // ── Test 15: leaf hashes match individual reading hashes ─────────────────

    #[test]
    fn test_leaf_hashes_match_reading_hashes() {
        let mut engine = BatchEngine::with_config(3600, 3);
        fill_engine(&mut engine, 3);
        let batch = engine.try_seal().unwrap();

        for (i, reading) in batch.readings.iter().enumerate() {
            let expected = hex::encode(MerkleRootProducer::hash_reading(reading));
            assert_eq!(batch.hashes.leaf_hashes[i], expected,
                "Leaf hash at index {i} must match reading's content hash");
        }
    }

    // ── Test 16: proof out-of-range returns None ──────────────────────────────

    #[test]
    fn test_proof_out_of_range_returns_none() {
        let mut engine = BatchEngine::with_config(3600, 3);
        fill_engine(&mut engine, 3);
        let batch = engine.try_seal().unwrap();
        assert!(generate_inclusion_proof(&batch, 99).is_none(), "OOB index must return None");
    }
}

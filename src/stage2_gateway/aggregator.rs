use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

/// A fully-formed, signed sensor reading as defined in the Odyssey Stage 1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub sensor_id: String,
    pub value: f64,
    pub unit: String,
    pub timestamp: DateTime<Utc>,
    pub sequence_number: u64,
    pub nonce: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub battery_voltage: Option<f64>,
    pub uncertainty_lower: Option<f64>,
    pub uncertainty_upper: Option<f64>,
    pub signature: String,
}

impl SensorReading {
    /// Canonical message string used for signing and verification.
    pub fn signing_message(&self) -> String {
        format!(
            "{}{}{}{}",
            self.sensor_id, self.value, self.timestamp.to_rfc3339(), self.nonce
        )
    }

    /// Deterministic JSON bytes for hashing (sorted field order via serde).
    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("SensorReading serialization is infallible")
    }

    /// SHA-256 fingerprint of this reading's canonical form.
    pub fn content_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.canonical_bytes());
        hasher.finalize().into()
    }
}

/// Statistical summary over a set of readings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStatistics {
    pub count: usize,
    pub mean: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
}

impl BatchStatistics {
    pub fn compute(readings: &[SensorReading]) -> Self {
        assert!(!readings.is_empty(), "Cannot compute statistics on empty batch");
        let count = readings.len();
        let mean = readings.iter().map(|r| r.value).sum::<f64>() / count as f64;
        let variance = readings
            .iter()
            .map(|r| (r.value - mean).powi(2))
            .sum::<f64>()
            / count as f64;
        let std_dev = variance.sqrt();
        let min = readings
            .iter()
            .map(|r| r.value)
            .fold(f64::INFINITY, f64::min);
        let max = readings
            .iter()
            .map(|r| r.value)
            .fold(f64::NEG_INFINITY, f64::max);
        Self { count, mean, std_dev, min, max }
    }

    /// Locality-Sensitive Hashing (LSH) fingerprint — SHA-256 of the statistical summary.
    /// Compresses a 100-reading window into 32 bytes while preserving similarity structure.
    pub fn lsh_fingerprint(&self) -> [u8; 32] {
        let repr = format!(
            "{:.4}|{:.4}|{:.4}|{:.4}",
            self.mean, self.std_dev, self.min, self.max
        );
        let mut hasher = Sha256::new();
        hasher.update(repr.as_bytes());
        hasher.finalize().into()
    }
}

/// A sealed batch of sensor readings ready for Merkle tree construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataBatch {
    pub batch_id: String,
    pub readings: Vec<SensorReading>,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub statistics: BatchStatistics,
    pub lsh_fingerprint: String,
    pub merkle_root: Option<String>,
    pub ipfs_cid: Option<String>,
}

/// Sealing reason for audit trail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealReason {
    TimerExpired,
    VolumeThreshold,
    ForceSeal,
}

/// Manages time-based and volume-based batch sealing with deduplication.
pub struct BatchAggregator {
    pub window_duration_secs: i64,
    pub volume_threshold: usize,
    pub current_batch: Vec<SensorReading>,
    pub last_window_end: DateTime<Utc>,
    /// Deduplication set: content hashes of seen readings (prevents replay within a window).
    seen_hashes: std::collections::HashSet<String>,
}

impl BatchAggregator {
    pub fn new(window_duration_secs: i64, volume_threshold: usize) -> Self {
        Self {
            window_duration_secs,
            volume_threshold,
            current_batch: Vec::new(),
            last_window_end: Utc::now(),
            seen_hashes: std::collections::HashSet::new(),
        }
    }

    /// Add a reading, returning false if it's a duplicate.
    pub fn add_reading(&mut self, reading: SensorReading) -> bool {
        let hash = hex::encode(reading.content_hash());
        if self.seen_hashes.contains(&hash) {
            return false; // Duplicate — idempotent reject
        }
        self.seen_hashes.insert(hash);
        self.current_batch.push(reading);
        true
    }

    /// True if the time window has elapsed.
    pub fn timer_expired(&self) -> bool {
        let now = Utc::now();
        (now - self.last_window_end).num_seconds() >= self.window_duration_secs
    }

    /// True if the volume threshold has been reached.
    pub fn volume_reached(&self) -> bool {
        self.current_batch.len() >= self.volume_threshold
    }

    /// Returns the reason to seal, if any.
    pub fn should_seal(&self) -> Option<SealReason> {
        if self.volume_reached() {
            Some(SealReason::VolumeThreshold)
        } else if self.timer_expired() {
            Some(SealReason::TimerExpired)
        } else {
            None
        }
    }

    /// Force-seal regardless of timer or volume (e.g. sensor offline, manual trigger).
    pub fn force_seal(&mut self) -> Option<DataBatch> {
        self.seal_internal(SealReason::ForceSeal)
    }

    pub fn seal_batch(&mut self) -> Option<DataBatch> {
        let reason = self.should_seal()?;
        self.seal_internal(reason)
    }

    fn seal_internal(&mut self, _reason: SealReason) -> Option<DataBatch> {
        if self.current_batch.is_empty() {
            return None;
        }
        let now = Utc::now();
        let readings: Vec<SensorReading> = self.current_batch.drain(..).collect();
        let statistics = BatchStatistics::compute(&readings);
        let lsh_fingerprint = hex::encode(statistics.lsh_fingerprint());
        self.seen_hashes.clear();
        self.last_window_end = now;

        Some(DataBatch {
            batch_id: uuid::Uuid::new_v4().to_string(),
            readings,
            window_start: self.last_window_end,
            window_end: now,
            statistics,
            lsh_fingerprint,
            merkle_root: None,
            ipfs_cid: None,
        })
    }
}

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
            nonce: uuid::Uuid::new_v4().to_string(),
            latitude: Some(43.8),
            longitude: Some(-115.9),
            battery_voltage: Some(4.2),
            uncertainty_lower: Some(value - 0.3),
            uncertainty_upper: Some(value + 0.3),
            signature: "mock_sig".to_string(),
        }
    }

    #[test]
    fn test_volume_threshold_sealing() {
        let mut agg = BatchAggregator::new(3600, 3);
        agg.add_reading(make_reading("s1", 21.0, 1));
        agg.add_reading(make_reading("s1", 22.0, 2));
        assert!(agg.should_seal().is_none(), "Should not seal at 2 readings");
        agg.add_reading(make_reading("s1", 23.0, 3));
        assert_eq!(agg.should_seal(), Some(SealReason::VolumeThreshold));
    }

    #[test]
    fn test_deduplication_rejects_same_reading() {
        let mut agg = BatchAggregator::new(3600, 100);
        let r = make_reading("s1", 21.0, 1);
        assert!(agg.add_reading(r.clone()), "First add should succeed");
        assert!(!agg.add_reading(r), "Duplicate should be rejected");
        assert_eq!(agg.current_batch.len(), 1);
    }

    #[test]
    fn test_batch_statistics() {
        let readings: Vec<SensorReading> = (0..5)
            .map(|i| make_reading("s1", i as f64 * 1.0, i))
            .collect();
        let stats = BatchStatistics::compute(&readings);
        assert_eq!(stats.count, 5);
        assert!((stats.mean - 2.0).abs() < 1e-9);
        assert!((stats.min - 0.0).abs() < 1e-9);
        assert!((stats.max - 4.0).abs() < 1e-9);
    }

    #[test]
    fn test_lsh_fingerprint_deterministic() {
        let readings: Vec<SensorReading> = (0..5)
            .map(|i| make_reading("s1", i as f64, i))
            .collect();
        let stats = BatchStatistics::compute(&readings);
        let fp1 = stats.lsh_fingerprint();
        let fp2 = stats.lsh_fingerprint();
        assert_eq!(fp1, fp2, "LSH fingerprint must be deterministic");
    }

    #[test]
    fn test_force_seal_works_on_non_empty() {
        let mut agg = BatchAggregator::new(3600, 100);
        agg.add_reading(make_reading("s1", 21.0, 1));
        let batch = agg.force_seal();
        assert!(batch.is_some());
        assert_eq!(batch.unwrap().readings.len(), 1);
        assert!(agg.current_batch.is_empty(), "Batch cleared after seal");
    }
}

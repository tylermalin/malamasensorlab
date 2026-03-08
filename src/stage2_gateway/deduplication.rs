//! Stage 2 — Prompt 15: Data Deduplication & Idempotency
//!
//! Deduplication key = (sensorDID, timestamp, value_hash)
//!
//! The dedup cache stores SHA-256 fingerprints of processed readings with a TTL.
//! Any reading with the same fingerprint within the TTL window is silently dropped.
//!
//! Idempotency guarantee: processing the same reading twice produces identical output.

use std::collections::HashMap;
use sha2::{Digest, Sha256};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

// ── Dedup key ─────────────────────────────────────────────────────────────────

/// Derive a dedup key: SHA-256( sensorDID || timestamp_rfc3339 || value_hex ).
pub fn dedup_key(sensor_did: &str, timestamp: &DateTime<Utc>, value: f64) -> String {
    let mut h = Sha256::new();
    h.update(sensor_did.as_bytes());
    h.update(timestamp.to_rfc3339().as_bytes());
    h.update(value.to_bits().to_le_bytes()); // bit-exact float serialization
    hex::encode(h.finalize())
}

// ── Cache entry ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct CacheEntry {
    pub(crate) inserted_at: DateTime<Utc>,
    pub(crate) ttl_secs: i64,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        (Utc::now() - self.inserted_at).num_seconds() >= self.ttl_secs
    }
}

// ── Dedup cache ───────────────────────────────────────────────────────────────

/// In-memory deduplication cache with TTL eviction.
///
/// In production this would be backed by Redis with TTL-keyed entries.
/// Default TTL: 24 hours (86400 seconds).
pub struct DedupCache {
    pub ttl_secs: i64,
    entries: HashMap<String, CacheEntry>,
}

impl DedupCache {
    pub fn new(ttl_secs: i64) -> Self {
        Self { ttl_secs, entries: HashMap::new() }
    }

    pub fn with_24h_ttl() -> Self { Self::new(86400) }

    /// Check if a reading with the given key has already been processed.
    /// Expired entries are treated as absent.
    pub fn is_duplicate(&self, key: &str) -> bool {
        self.entries.get(key).map(|e| !e.is_expired()).unwrap_or(false)
    }

    /// Mark a reading as processed. Returns `false` if it was already present (duplicate).
    pub fn mark_processed(&mut self, key: &str) -> bool {
        // Evict expired entries lazily
        self.entries.retain(|_, e| !e.is_expired());

        if self.is_duplicate(key) {
            return false; // duplicate
        }
        self.entries.insert(key.to_string(), CacheEntry {
            inserted_at: Utc::now(),
            ttl_secs: self.ttl_secs,
        });
        true // newly inserted
    }

    /// Convenience: derive key and check/mark in one call.
    /// Returns `true` if the reading is new (should be processed).
    pub fn accept_reading(
        &mut self,
        sensor_did: &str,
        timestamp: &DateTime<Utc>,
        value: f64,
    ) -> bool {
        let key = dedup_key(sensor_did, timestamp, value);
        self.mark_processed(&key)
    }

    /// Number of entries currently in cache (including not-yet-expired).
    pub fn len(&self) -> usize { self.entries.len() }

    /// Explicitly purge all expired entries.
    pub fn evict_expired(&mut self) {
        self.entries.retain(|_, e| !e.is_expired());
    }
}

// ── Idempotent processing ────────────────────────────────────────────────────

/// Result of idempotent processing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessingResult {
    /// Reading is new — proceed with full processing.
    New,
    /// Reading was already processed — skip (idempotent).
    Duplicate { dedup_key: String },
}

/// Idempotent reading processor: wrap any processing function.
pub struct IdempotentProcessor {
    cache: DedupCache,
}

impl IdempotentProcessor {
    pub fn new(ttl_secs: i64) -> Self {
        Self { cache: DedupCache::new(ttl_secs) }
    }

    /// Attempt to process a reading. Returns `New` on first occurrence,
    /// `Duplicate` on all subsequent calls with the same key.
    pub fn process(
        &mut self,
        sensor_did: &str,
        timestamp: &DateTime<Utc>,
        value: f64,
    ) -> ProcessingResult {
        let key = dedup_key(sensor_did, timestamp, value);
        if self.cache.mark_processed(&key) {
            ProcessingResult::New
        } else {
            ProcessingResult::Duplicate { dedup_key: key }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    const DID: &str = "did:cardano:sensor:biochar-001";

    // ── Test 1: dedup key is deterministic ────────────────────────────────────

    #[test]
    fn test_dedup_key_deterministic() {
        let ts = Utc::now();
        let k1 = dedup_key(DID, &ts, 23.4);
        let k2 = dedup_key(DID, &ts, 23.4);
        assert_eq!(k1, k2, "Same inputs must produce same dedup key");
    }

    // ── Test 2: different values → different keys ─────────────────────────────

    #[test]
    fn test_different_values_different_keys() {
        let ts = Utc::now();
        let k1 = dedup_key(DID, &ts, 23.4);
        let k2 = dedup_key(DID, &ts, 23.5);
        assert_ne!(k1, k2);
    }

    // ── Test 3: different sensors → different keys ────────────────────────────

    #[test]
    fn test_different_sensors_different_keys() {
        let ts = Utc::now();
        let k1 = dedup_key("did:cardano:sensor:s1", &ts, 23.4);
        let k2 = dedup_key("did:cardano:sensor:s2", &ts, 23.4);
        assert_ne!(k1, k2);
    }

    // ── Test 4: first reading accepted ───────────────────────────────────────

    #[test]
    fn test_first_reading_accepted() {
        let mut cache = DedupCache::new(3600);
        assert!(cache.accept_reading(DID, &Utc::now(), 23.4), "First reading must be accepted");
    }

    // ── Test 5: duplicate rejected within TTL ─────────────────────────────────

    #[test]
    fn test_duplicate_rejected_within_ttl() {
        let mut cache = DedupCache::new(3600);
        let ts = Utc::now();
        assert!(cache.accept_reading(DID, &ts, 23.4), "First should succeed");
        assert!(!cache.accept_reading(DID, &ts, 23.4), "Duplicate must be rejected");
    }

    // ── Test 6: expired entry allows re-processing ────────────────────────────

    #[test]
    fn test_expired_entry_allows_reprocessing() {
        let mut cache = DedupCache::new(0); // 0-second TTL → always expired
        let ts = Utc::now();
        cache.accept_reading(DID, &ts, 23.4);
        // After expiry (0 TTL), same reading should be re-accepted
        assert!(cache.accept_reading(DID, &ts, 23.4),
            "Expired entry must allow re-processing");
    }

    // ── Test 7: 24h TTL config ────────────────────────────────────────────────

    #[test]
    fn test_24h_ttl() {
        let cache = DedupCache::with_24h_ttl();
        assert_eq!(cache.ttl_secs, 86400);
    }

    // ── Test 8: idempotent processor returns New then Duplicate ──────────────

    #[test]
    fn test_idempotent_processor() {
        let mut proc = IdempotentProcessor::new(3600);
        let ts = Utc::now();
        assert_eq!(proc.process(DID, &ts, 23.4), ProcessingResult::New);
        let result = proc.process(DID, &ts, 23.4);
        assert!(matches!(result, ProcessingResult::Duplicate { .. }), "Second call must be Duplicate");
    }

    // ── Test 9: different timestamps → both accepted ──────────────────────────

    #[test]
    fn test_different_timestamps_both_accepted() {
        let mut cache = DedupCache::new(3600);
        let ts1 = Utc::now();
        let ts2 = ts1 + chrono::Duration::seconds(60);
        assert!(cache.accept_reading(DID, &ts1, 23.4));
        assert!(cache.accept_reading(DID, &ts2, 23.4), "Different timestamp → new reading");
    }

    // ── Test 10: evict_expired reduces cache size ─────────────────────────────

    #[test]
    fn test_evict_expired_reduces_size() {
        use chrono::Duration;
        let mut cache = DedupCache::new(3600);
        // Manually insert entries that are already past their TTL
        let old_time = Utc::now() - Duration::seconds(7200); // 2 hours ago
        cache.entries.insert("key-a".to_string(), super::CacheEntry {
            inserted_at: old_time,
            ttl_secs: 3600, // 1h TTL → expired 1h ago
        });
        cache.entries.insert("key-b".to_string(), super::CacheEntry {
            inserted_at: old_time,
            ttl_secs: 3600,
        });
        assert_eq!(cache.len(), 2, "Before eviction: 2 entries");
        cache.evict_expired();
        assert_eq!(cache.len(), 0, "All expired entries must be evicted");
    }
}

//! Stage 2 — Prompt 10: Batch Serialization & Deterministic JSON
//!
//! Canonical JSON serialization guarantees that the same batch always produces
//! the same SHA-256 hash regardless of platform, time zone, or field insertion order.
//!
//! Rules:
//! 1. Object keys sorted alphabetically (recursive)
//! 2. Float values rounded to 6 significant figures before serialization
//! 3. Timestamps normalized to RFC-3339 UTC ("Z" suffix)
//! 4. Arrays preserve insertion order

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use crate::stage2_gateway::batch_engine::SealedBatch;

// ── Canonical float formatting ────────────────────────────────────────────────

/// Round a float to `sig_figs` significant figures for deterministic serialization.
pub fn round_sig_figs(x: f64, sig_figs: u32) -> f64 {
    if x == 0.0 { return 0.0; }
    let d = sig_figs as f64 - x.abs().log10().floor() - 1.0;
    let factor = 10f64.powi(d as i32);
    (x * factor).round() / factor
}

// ── Sorted canonical JSON ─────────────────────────────────────────────────────

/// Sort all JSON object keys alphabetically, recursively.
/// Arrays are left in original order (content-preserving).
pub fn sort_keys(v: &Value) -> Value {
    match v {
        Value::Object(m) => {
            let mut sorted = serde_json::Map::new();
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            for k in keys { sorted.insert(k.clone(), sort_keys(&m[k])); }
            Value::Object(sorted)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(sort_keys).collect()),
        other => other.clone(),
    }
}

/// Produce canonical JSON bytes for any serializable type:
/// 1. Serialize to serde_json Value
/// 2. Sort object keys alphabetically (recursive)
/// 3. Serialize to compact JSON bytes (no whitespace)
pub fn canonical_json<T: Serialize>(val: &T) -> Vec<u8> {
    let v: Value = serde_json::to_value(val).expect("Serialization must not fail");
    serde_json::to_vec(&sort_keys(&v)).expect("Re-serialization must not fail")
}

/// SHA-256 of the canonical JSON bytes — the "batch fingerprint".
pub fn canonical_hash<T: Serialize>(val: &T) -> [u8; 32] {
    let bytes = canonical_json(val);
    let mut h = Sha256::new();
    h.update(&bytes);
    h.finalize().into()
}

pub fn canonical_hash_hex<T: Serialize>(val: &T) -> String {
    hex::encode(canonical_hash(val))
}

// ── SealedBatch canonical fingerprint ────────────────────────────────────────

/// Compute the canonical content hash of a complete sealed batch.
/// This is the value anchored on-chain as proof of batch integrity.
pub fn batch_fingerprint(batch: &SealedBatch) -> String {
    // Exclude mutable fields (ipfs_cid) so the fingerprint is stable pre- and post-IPFS upload
    #[derive(Serialize)]
    struct BatchCore<'a> {
        batch_id: &'a str,
        sensor_dids: &'a [String],
        reading_count: usize,
        merkle_root: &'a str,
        lsh_fingerprint: &'a str,
        average_reading: f64,
        min: f64,
        max: f64,
        std_dev: f64,
    }
    let core = BatchCore {
        batch_id: &batch.batch_id,
        sensor_dids: &batch.sensor_dids,
        reading_count: batch.reading_count,
        merkle_root: &batch.hashes.merkle_root,
        lsh_fingerprint: &batch.hashes.lsh_fingerprint,
        average_reading: round_sig_figs(batch.statistics.average_reading, 6),
        min: round_sig_figs(batch.statistics.min, 6),
        max: round_sig_figs(batch.statistics.max, 6),
        std_dev: round_sig_figs(batch.statistics.std_dev, 6),
    };
    canonical_hash_hex(&core)
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct Doc { z: String, a: i32, m: Vec<i32> }

    // ── Test 1: sorted keys produce same hash regardless of insertion order ───

    #[test]
    fn test_canonical_hash_key_order_independent() {
        let v1 = json!({"z": 1, "a": 2, "m": 3});
        let v2 = json!({"a": 2, "m": 3, "z": 1});
        let h1 = hex::encode(canonical_hash(&v1));
        let h2 = hex::encode(canonical_hash(&v2));
        assert_eq!(h1, h2, "Key order must not affect canonical hash");
    }

    // ── Test 2: same struct serialized twice → same hash ─────────────────────

    #[test]
    fn test_same_struct_same_hash() {
        let doc = Doc { z: "hello".to_string(), a: 42, m: vec![1, 2, 3] };
        assert_eq!(canonical_hash_hex(&doc), canonical_hash_hex(&doc));
    }

    // ── Test 3: different values → different hashes ───────────────────────────

    #[test]
    fn test_different_values_different_hashes() {
        let d1 = Doc { z: "a".to_string(), a: 1, m: vec![] };
        let d2 = Doc { z: "b".to_string(), a: 1, m: vec![] };
        assert_ne!(canonical_hash_hex(&d1), canonical_hash_hex(&d2));
    }

    // ── Test 4: array order matters ───────────────────────────────────────────

    #[test]
    fn test_array_order_preserved() {
        let v1 = json!({"a": [1, 2, 3]});
        let v2 = json!({"a": [3, 2, 1]});
        assert_ne!(
            hex::encode(canonical_hash(&v1)),
            hex::encode(canonical_hash(&v2)),
            "Array order must be preserved"
        );
    }

    // ── Test 5: nested objects sorted recursively ─────────────────────────────

    #[test]
    fn test_nested_keys_sorted() {
        let v1 = json!({"outer": {"z": 1, "a": 2}});
        let v2 = json!({"outer": {"a": 2, "z": 1}});
        assert_eq!(
            hex::encode(canonical_hash(&v1)),
            hex::encode(canonical_hash(&v2)),
            "Nested keys must be sorted"
        );
    }

    // ── Test 6: canonical_json output is compact (no whitespace) ─────────────

    #[test]
    fn test_canonical_json_is_compact() {
        let doc = Doc { z: "x".to_string(), a: 1, m: vec![1] };
        let bytes = canonical_json(&doc);
        let s = String::from_utf8(bytes).unwrap();
        assert!(!s.contains("  "), "Canonical JSON must not have extra whitespace");
        assert!(!s.contains('\n'), "Canonical JSON must not have newlines");
    }

    // ── Test 7: round_sig_figs works correctly ────────────────────────────────

    #[test]
    fn test_round_sig_figs() {
        assert!((round_sig_figs(23.456789, 6) - 23.4568).abs() < 1e-4);
        assert!((round_sig_figs(0.000123456, 3) - 0.000123).abs() < 1e-9);
        assert_eq!(round_sig_figs(0.0, 6), 0.0);
    }

    // ── Test 8: batch_fingerprint is deterministic ────────────────────────────

    #[test]
    fn test_batch_fingerprint_deterministic() {
        use crate::stage2_gateway::batch_engine::BatchEngine;
        use crate::stage2_gateway::aggregator::SensorReading;
        use chrono::Utc;

        let mut engine = BatchEngine::with_config(3600, 3);
        for i in 0..3u64 {
            engine.ingest(SensorReading {
                sensor_id: "biochar-001".to_string(),
                value: 23.4 + i as f64,
                unit: "Celsius".to_string(),
                timestamp: Utc::now(),
                sequence_number: i + 1,
                nonce: format!("n{i}"),
                latitude: None, longitude: None, battery_voltage: None,
                uncertainty_lower: None, uncertainty_upper: None,
                signature: "sig".to_string(),
            });
        }
        let batch = engine.try_seal().unwrap();
        let fp1 = batch_fingerprint(&batch);
        let fp2 = batch_fingerprint(&batch);
        assert_eq!(fp1, fp2, "Batch fingerprint must be identical on repeated calls");
        assert_eq!(fp1.len(), 64);
    }
}

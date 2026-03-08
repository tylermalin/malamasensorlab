//! Stage 1 — Sensor Reading Signing System (Prompt 3)
//!
//! Every data point carries a cryptographic birth certificate: an ECDSA signature
//! bound to the sensor's DID, a unique nonce (replay prevention), a monotonic
//! sequence number, and a SHA-256 hash of the entire canonical payload.
//!
//! # Narrative
//! "This reading came from sensor biochar-001, taken at 12:30 UTC on March 5,
//!  and has never been modified."

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use k256::ecdsa::{SigningKey, VerifyingKey, Signature, signature::{Signer, Verifier}};
use std::collections::{HashSet, VecDeque};
use std::str::FromStr;

// ── Data structures ──────────────────────────────────────────────────────────

/// 3-D location recorded at the moment of each reading.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReadingLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: Option<f64>,
}

/// Uncertainty interval for the measured value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncertaintyBounds {
    pub lower: f64,
    pub upper: f64,
    /// 0–1 confidence level (e.g. 0.95 = 95%).
    pub confidence: f64,
}

/// The raw, unsigned payload of a sensor reading.
///
/// All fields are ordered deterministically by serde (struct field order)
/// so `serde_json::to_vec` always produces the same canonical bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingPayload {
    #[serde(rename = "sensorDID")]
    pub sensor_did: String,
    pub reading: f64,
    pub unit: String,
    pub timestamp: DateTime<Utc>,
    #[serde(rename = "sequenceNumber")]
    pub sequence_number: u64,
    /// Hex-encoded 16-byte random value — prevents replay attacks.
    pub nonce: String,
    pub location: ReadingLocation,
    #[serde(rename = "batteryVoltage")]
    pub battery_voltage: f64,
    #[serde(rename = "uncertaintyBounds")]
    pub uncertainty_bounds: UncertaintyBounds,
}

impl ReadingPayload {
    /// Canonical JSON bytes used as the pre-image for signing.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("ReadingPayload serialization is infallible")
    }

    /// SHA-256 of the canonical bytes.
    pub fn sha256_hash(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.canonical_bytes());
        h.finalize().into()
    }
}

/// A reading that has been signed by the sensor's ECDSA key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedReading {
    pub payload: ReadingPayload,
    /// Hex-encoded SHA-256 of the canonical payload.
    pub payload_hash: String,
    /// Hex-encoded ECDSA signature over `payload_hash`.
    pub signature: String,
    /// Hex-encoded SEC-1 compressed public key.
    pub public_key: String,
}

// ── Signing engine ────────────────────────────────────────────────────────────

/// Generate a 16-byte cryptographically random hex nonce.
pub fn generate_nonce() -> String {
    let bytes: Vec<u8> = (0..16).map(|_| rand::random::<u8>()).collect();
    hex::encode(bytes)
}

/// Create and sign a `ReadingPayload` with the sensor's ECDSA private key.
///
/// Steps (mirroring the Odyssey Prompt 3 spec):
/// 1. Serialize payload to canonical JSON
/// 2. Hash with SHA-256
/// 3. Sign the hash with ECDSA (secp256k1)
/// 4. Return `SignedReading` with hash + signature + public key appended
pub fn sign_reading(payload: ReadingPayload, signing_key: &SigningKey) -> SignedReading {
    let hash_bytes = payload.sha256_hash();
    let hash_hex = hex::encode(hash_bytes);

    // ECDSA signs the raw hash bytes
    let sig: Signature = signing_key.sign(&hash_bytes);
    let sig_hex = hex::encode(sig.to_bytes());

    let vk = VerifyingKey::from(signing_key);
    let pk_hex = hex::encode(vk.to_encoded_point(true).as_bytes());

    SignedReading {
        payload,
        payload_hash: hash_hex,
        signature: sig_hex,
        public_key: pk_hex,
    }
}

// ── Verification chain ────────────────────────────────────────────────────────

/// Possible outcomes when verifying a signed reading.
#[derive(Debug, PartialEq)]
pub enum VerificationResult {
    /// Signature is valid and public key matches DID document.
    Valid,
    /// Payload hash doesn't match — content was tampered with.
    TamperedPayload,
    /// ECDSA signature is invalid for this payload hash.
    InvalidSignature,
    /// Public key in the signed reading doesn't match the registered DID key.
    DIDMismatch,
    /// Malformed hex or key encoding.
    EncodingError(String),
}

/// Verify a `SignedReading` against the sensor's registered `VerifyingKey`.
///
/// Chain:
/// 1. Recompute SHA-256 hash → must match `payload_hash`
/// 2. Verify ECDSA signature over hash
/// 3. Confirm public key in reading matches the DID-registered key
pub fn verify_reading(
    signed: &SignedReading,
    registered_key: &VerifyingKey,
) -> VerificationResult {
    // Step 1: Hash integrity
    let expected_hash = hex::encode(signed.payload.sha256_hash());
    if expected_hash != signed.payload_hash {
        return VerificationResult::TamperedPayload;
    }

    // Step 2: ECDSA verification
    let hash_bytes = match hex::decode(&signed.payload_hash) {
        Ok(b) => b,
        Err(e) => return VerificationResult::EncodingError(e.to_string()),
    };
    let sig = match Signature::from_str(&signed.signature) {
        Ok(s) => s,
        Err(e) => return VerificationResult::EncodingError(e.to_string()),
    };
    if registered_key.verify(&hash_bytes, &sig).is_err() {
        return VerificationResult::InvalidSignature;
    }

    // Step 3: Public key matches DID document
    let registered_pk_hex = hex::encode(registered_key.to_encoded_point(true).as_bytes());
    if registered_pk_hex != signed.public_key {
        return VerificationResult::DIDMismatch;
    }

    VerificationResult::Valid
}

// ── Replay attack prevention ──────────────────────────────────────────────────

/// In-memory nonce tracker with a rolling TTL window.
///
/// Production systems would use Redis with a 24h key TTL; this implementation
/// uses a bounded `VecDeque` ordered by insertion time for deterministic testing.
pub struct NonceTracker {
    /// Nonces currently considered "live".
    seen: HashSet<String>,
    /// Insertion-ordered nonces for TTL eviction (oldest first).
    queue: VecDeque<(String, DateTime<Utc>)>,
    /// How long a nonce is remembered (seconds).
    ttl_secs: i64,
}

impl NonceTracker {
    pub fn new(ttl_secs: i64) -> Self {
        Self {
            seen: HashSet::new(),
            queue: VecDeque::new(),
            ttl_secs,
        }
    }

    /// Evict nonces older than `ttl_secs`.
    fn evict_expired(&mut self, now: DateTime<Utc>) {
        while let Some((nonce, inserted_at)) = self.queue.front() {
            if (now - *inserted_at).num_seconds() >= self.ttl_secs {
                self.seen.remove(nonce);
                self.queue.pop_front();
            } else {
                break;
            }
        }
    }

    /// Try to accept a nonce.
    ///
    /// Returns `true` (accepted) if the nonce is new.
    /// Returns `false` (rejected) if the nonce was already seen within the TTL window.
    pub fn accept(&mut self, nonce: &str) -> bool {
        let now = Utc::now();
        self.evict_expired(now);
        if self.seen.contains(nonce) {
            return false; // Replay attack
        }
        self.seen.insert(nonce.to_string());
        self.queue.push_back((nonce.to_string(), now));
        true
    }

    /// Number of live nonces currently tracked.
    pub fn live_count(&self) -> usize {
        self.seen.len()
    }
}

// ── Sequence number validator ─────────────────────────────────────────────────

/// Validates that readings from a sensor have monotonically increasing sequence numbers.
pub struct SequenceTracker {
    last_seen: std::collections::HashMap<String, u64>,
}

impl SequenceTracker {
    pub fn new() -> Self {
        Self { last_seen: std::collections::HashMap::new() }
    }

    /// Accept a sequence number for a sensor DID.
    ///
    /// Returns `Ok(())` if the sequence is valid (greater than the last seen).
    /// Returns `Err` if the sequence number is a replay or out-of-order.
    pub fn accept(&mut self, sensor_did: &str, seq: u64) -> Result<(), String> {
        let last = self.last_seen.entry(sensor_did.to_string()).or_insert(0);
        if seq <= *last {
            return Err(format!(
                "Sequence number {seq} is not greater than last seen {last} for sensor {sensor_did}"
            ));
        }
        *last = seq;
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::SigningKey;
    use rand::rngs::OsRng;
    use chrono::Utc;

    fn make_payload(did: &str, value: f64, seq: u64) -> ReadingPayload {
        ReadingPayload {
            sensor_did: did.to_string(),
            reading: value,
            unit: "Celsius".to_string(),
            timestamp: Utc::now(),
            sequence_number: seq,
            nonce: generate_nonce(),
            location: ReadingLocation {
                latitude: 43.8123,
                longitude: -115.9456,
                altitude_m: Some(1234.0),
            },
            battery_voltage: 4.2,
            uncertainty_bounds: UncertaintyBounds {
                lower: value - 0.3,
                upper: value + 0.3,
                confidence: 0.95,
            },
        }
    }

    // ── Test 1: happy path — sign and verify ──────────────────────────────────

    #[test]
    fn test_sign_and_verify_valid_reading() {
        let key = SigningKey::random(&mut OsRng);
        let vk = VerifyingKey::from(&key);
        let payload = make_payload("did:cardano:sensor:biochar-001", 23.4, 1);
        let signed = sign_reading(payload, &key);

        let result = verify_reading(&signed, &vk);
        assert_eq!(result, VerificationResult::Valid, "Valid reading should verify");
    }

    // ── Test 2: payload hash is correct ──────────────────────────────────────

    #[test]
    fn test_payload_hash_is_sha256_of_canonical_json() {
        let key = SigningKey::random(&mut OsRng);
        let payload = make_payload("did:cardano:sensor:biochar-001", 23.4, 1);
        let expected_hash = hex::encode(payload.sha256_hash());
        let signed = sign_reading(payload, &key);
        assert_eq!(signed.payload_hash, expected_hash, "payload_hash must equal SHA-256 of canonical JSON");
    }

    // ── Test 3: tampered reading value → TamperedPayload ─────────────────────

    #[test]
    fn test_tampered_reading_value_detected() {
        let key = SigningKey::random(&mut OsRng);
        let vk = VerifyingKey::from(&key);
        let payload = make_payload("did:cardano:sensor:biochar-001", 23.4, 1);
        let mut signed = sign_reading(payload, &key);

        // Attacker changes the temperature value
        signed.payload.reading = 999.0;

        let result = verify_reading(&signed, &vk);
        assert_eq!(result, VerificationResult::TamperedPayload, "Modified value must be caught");
    }

    // ── Test 4: tampered timestamp → TamperedPayload ─────────────────────────

    #[test]
    fn test_tampered_timestamp_detected() {
        let key = SigningKey::random(&mut OsRng);
        let vk = VerifyingKey::from(&key);
        let payload = make_payload("did:cardano:sensor:biochar-001", 23.4, 1);
        let mut signed = sign_reading(payload, &key);

        // Attacker backdates the timestamp
        signed.payload.timestamp = chrono::DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let result = verify_reading(&signed, &vk);
        assert_eq!(result, VerificationResult::TamperedPayload, "Modified timestamp must be caught");
    }

    // ── Test 5: wrong verifying key → InvalidSignature ───────────────────────

    #[test]
    fn test_wrong_key_gives_invalid_signature() {
        let key1 = SigningKey::random(&mut OsRng);
        let key2 = SigningKey::random(&mut OsRng);
        let vk2 = VerifyingKey::from(&key2); // Different key from signer
        let payload = make_payload("did:cardano:sensor:biochar-001", 23.4, 1);
        let signed = sign_reading(payload, &key1);

        let result = verify_reading(&signed, &vk2);
        // The hash matches (payload unchanged) but the sig was made with key1
        assert_eq!(result, VerificationResult::InvalidSignature);
    }

    // ── Test 6: replay attack prevention — same nonce rejected ───────────────

    #[test]
    fn test_replay_attack_same_nonce_rejected() {
        let mut tracker = NonceTracker::new(86400); // 24h TTL
        let nonce = generate_nonce();

        assert!(tracker.accept(&nonce), "First use of nonce must succeed");
        assert!(!tracker.accept(&nonce), "Second use of same nonce must be rejected");
    }

    // ── Test 7: different nonces all accepted ─────────────────────────────────

    #[test]
    fn test_unique_nonces_all_accepted() {
        let mut tracker = NonceTracker::new(86400);
        for _ in 0..100 {
            let nonce = generate_nonce();
            assert!(tracker.accept(&nonce), "Each unique nonce must be accepted");
        }
        assert_eq!(tracker.live_count(), 100);
    }

    // ── Test 8: nonce TTL eviction ────────────────────────────────────────────

    #[test]
    fn test_expired_nonce_can_be_reused() {
        let mut tracker = NonceTracker::new(0); // Immediate TTL — everything expires instantly
        let nonce = generate_nonce();
        // First acceptance
        tracker.seen.insert(nonce.clone());
        tracker.queue.push_back((nonce.clone(), Utc::now() - chrono::Duration::seconds(1)));
        // After eviction (TTL=0 s) the nonce should be gone
        assert!(tracker.accept(&nonce), "Expired nonce should be re-accepted after TTL");
    }

    // ── Test 9: monotonic sequence numbers ────────────────────────────────────

    #[test]
    fn test_sequence_numbers_monotonic() {
        let mut seq = SequenceTracker::new();
        let did = "did:cardano:sensor:biochar-001";

        assert!(seq.accept(did, 1).is_ok());
        assert!(seq.accept(did, 2).is_ok());
        assert!(seq.accept(did, 100).is_ok());

        let err = seq.accept(did, 100); // replay
        assert!(err.is_err(), "Repeated sequence must be rejected");

        let err2 = seq.accept(did, 50); // out-of-order
        assert!(err2.is_err(), "Out-of-order sequence must be rejected");
    }

    // ── Test 10: independent sequences per sensor DID ────────────────────────

    #[test]
    fn test_sequence_tracks_independently_per_did() {
        let mut seq = SequenceTracker::new();
        let did_a = "did:cardano:sensor:a";
        let did_b = "did:cardano:sensor:b";

        assert!(seq.accept(did_a, 5).is_ok());
        assert!(seq.accept(did_b, 1).is_ok()); // sensor B can start at 1 even after A is at 5
        assert!(seq.accept(did_a, 6).is_ok());
        assert!(seq.accept(did_b, 2).is_ok());
    }

    // ── Test 11: nonce is 32 hex chars (16 bytes) ─────────────────────────────

    #[test]
    fn test_nonce_format() {
        for _ in 0..20 {
            let nonce = generate_nonce();
            assert_eq!(nonce.len(), 32, "Nonce must be 32 hex chars (16 bytes)");
            assert!(nonce.chars().all(|c| c.is_ascii_hexdigit()), "Nonce must be hex");
        }
    }

    // ── Test 12: canonical hash is deterministic ──────────────────────────────

    #[test]
    fn test_canonical_hash_deterministic() {
        let payload = make_payload("did:cardano:sensor:biochar-001", 23.4, 1);
        let h1 = payload.sha256_hash();
        let h2 = payload.sha256_hash();
        assert_eq!(h1, h2, "SHA-256 of same payload must be identical");
    }

    // ── Test 13: full pipeline — sign → nonce-check → seq-check → verify ─────

    #[test]
    fn test_full_pipeline_integration() {
        let key = SigningKey::random(&mut OsRng);
        let vk = VerifyingKey::from(&key);
        let mut nonce_tracker = NonceTracker::new(86400);
        let mut seq_tracker = SequenceTracker::new();
        let did = "did:cardano:sensor:biochar-001";

        for seq_num in 1..=5u64 {
            let payload = make_payload(did, 20.0 + seq_num as f64, seq_num);
            let nonce = payload.nonce.clone();

            // Nonce check
            assert!(nonce_tracker.accept(&nonce), "Nonce must be fresh");
            // Sequence check
            assert!(seq_tracker.accept(did, seq_num).is_ok(), "Sequence must be monotonic");
            // Sign and verify
            let signed = sign_reading(payload, &key);
            assert_eq!(verify_reading(&signed, &vk), VerificationResult::Valid);
        }
    }
}

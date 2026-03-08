//! Stage 1 — DID Metadata Hashing (Prompt 6)
//!
//! Deterministic canonical JSON serialization + SHA-256 fingerprinting
//! of DID documents. Any field change — even whitespace — changes the hash,
//! enabling tamper detection without a blockchain round-trip.
//!
//! # Narrative
//! "The sensor's DID document is fingerprinted at birth. Re-hash at any time
//!  to answer: 'Has this document been modified since it was first recorded?'"

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use serde_json::Value;

// ── Canonical serialization ───────────────────────────────────────────────────

/// Produce a deterministic byte representation of any serializable value.
///
/// Uses `serde_json::to_vec` whose field order mirrors the Rust struct's field
/// declaration order — consistent across all platforms and Rust versions.
/// Arrays are preserved in their original order (JSON spec compliance).
pub fn canonical_bytes<T: Serialize>(doc: &T) -> Vec<u8> {
    serde_json::to_vec(doc).expect("Serialization of well-formed document must not fail")
}

/// Sort all JSON object keys recursively (alphabetical) and re-serialize.
///
/// Use this when the document arrives from an external source (e.g. IPFS)
/// with unpredictable field ordering, before computing a fingerprint.
pub fn canonical_bytes_sorted(value: &Value) -> Vec<u8> {
    serde_json::to_vec(&sort_json(value))
        .expect("Serialization must not fail")
}

fn sort_json(val: &Value) -> Value {
    match val {
        Value::Object(map) => {
            let mut sorted: serde_json::Map<String, Value> = serde_json::Map::new();
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for k in keys {
                sorted.insert(k.clone(), sort_json(&map[k]));
            }
            Value::Object(sorted)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(sort_json).collect()),
        other => other.clone(),
    }
}

// ── Fingerprint type ──────────────────────────────────────────────────────────

/// A SHA-256 document fingerprint — 32 bytes, displayed as 64 hex chars.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentFingerprint {
    pub hex: String,
    pub algorithm: String,
}

impl DocumentFingerprint {
    /// Compute the SHA-256 fingerprint of a serializable DID document.
    pub fn compute<T: Serialize>(doc: &T) -> Self {
        Self::from_bytes(&canonical_bytes(doc))
    }


    /// Compute from pre-sorted JSON bytes (for external documents).
    pub fn compute_sorted(value: &Value) -> Self {
        Self::from_bytes(&canonical_bytes_sorted(value))
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let mut h = Sha256::new();
        h.update(bytes);
        DocumentFingerprint {
            hex: hex::encode(h.finalize()),
            algorithm: "SHA-256".to_string(),
        }
    }

    /// Verify a document matches this fingerprint.
    pub fn verify<T: Serialize>(&self, doc: &T) -> bool {
        Self::compute(doc).hex == self.hex
    }

    /// Verify a sorted JSON value matches this fingerprint.
    pub fn verify_sorted(&self, value: &Value) -> bool {
        Self::compute_sorted(value).hex == self.hex
    }
}

// ── DID document fingerprinting ───────────────────────────────────────────────

/// A fingerprinted DID document wrapper — the SHA-256 is stored alongside
/// the document and re-checked on every access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintedDocument<T: Serialize + Clone> {
    pub document: T,
    pub fingerprint: DocumentFingerprint,
}

impl<T: Serialize + Clone> FingerprintedDocument<T> {
    /// Wrap a document and compute its fingerprint.
    pub fn new(document: T) -> Self {
        let fingerprint = DocumentFingerprint::compute(&document);
        Self { document, fingerprint }
    }

    /// Verify the document has not been modified since wrapping.
    pub fn is_intact(&self) -> bool {
        self.fingerprint.verify(&self.document)
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestDoc {
        id: String,
        value: f64,
        created: String,
    }

    fn make_doc() -> TestDoc {
        TestDoc {
            id: "did:cardano:sensor:biochar-001".to_string(),
            value: 23.4,
            created: "2025-03-05T12:30:00Z".to_string(),
        }
    }

    // ── Test 1: fingerprint is 64 hex chars (SHA-256) ─────────────────────────

    #[test]
    fn test_fingerprint_is_64_hex_chars() {
        let fp = DocumentFingerprint::compute(&make_doc());
        assert_eq!(fp.hex.len(), 64);
        assert!(fp.hex.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(fp.algorithm, "SHA-256");
    }

    // ── Test 2: same document → same fingerprint ──────────────────────────────

    #[test]
    fn test_fingerprint_deterministic() {
        let doc = make_doc();
        let fp1 = DocumentFingerprint::compute(&doc);
        let fp2 = DocumentFingerprint::compute(&doc);
        assert_eq!(fp1, fp2, "Same document must produce same fingerprint");
    }

    // ── Test 3: different documents → different fingerprints ─────────────────

    #[test]
    fn test_different_documents_different_fingerprints() {
        let doc1 = make_doc();
        let mut doc2 = doc1.clone();
        doc2.value = 99.9;
        assert_ne!(
            DocumentFingerprint::compute(&doc1),
            DocumentFingerprint::compute(&doc2),
            "Modified document must have different fingerprint"
        );
    }

    // ── Test 4: verify passes on unmodified document ──────────────────────────

    #[test]
    fn test_verify_passes_on_clean_document() {
        let doc = make_doc();
        let fp = DocumentFingerprint::compute(&doc);
        assert!(fp.verify(&doc), "Fingerprint must match original document");
    }

    // ── Test 5: verify fails on tampered document ──────────────────────────────

    #[test]
    fn test_verify_fails_on_tampered_document() {
        let doc = make_doc();
        let fp = DocumentFingerprint::compute(&doc);
        let mut tampered = doc.clone();
        tampered.id = "did:cardano:sensor:fake".to_string();
        assert!(!fp.verify(&tampered), "Tampered document must fail fingerprint check");
    }

    // ── Test 6: sorted JSON fingerprint is stable across key orders ───────────

    #[test]
    fn test_sorted_json_fingerprint_stable() {
        let v1 = json!({"b": 2, "a": 1, "c": 3});
        let v2 = json!({"a": 1, "c": 3, "b": 2}); // different insertion order
        let fp1 = DocumentFingerprint::compute_sorted(&v1);
        let fp2 = DocumentFingerprint::compute_sorted(&v2);
        assert_eq!(fp1, fp2, "Key-sorted fingerprints must match regardless of initial order");
    }

    // ── Test 7: nested object sorting ────────────────────────────────────────

    #[test]
    fn test_nested_object_sorted_correctly() {
        let v1 = json!({"z": {"b": 2, "a": 1}, "a": true});
        let v2 = json!({"a": true, "z": {"a": 1, "b": 2}});
        let fp1 = DocumentFingerprint::compute_sorted(&v1);
        let fp2 = DocumentFingerprint::compute_sorted(&v2);
        assert_eq!(fp1, fp2, "Nested objects must be sorted recursively");
    }

    // ── Test 8: any single character change → different fingerprint ───────────

    #[test]
    fn test_single_char_change_detected() {
        let doc = make_doc();
        let fp = DocumentFingerprint::compute(&doc);
        let mut changed = doc.clone();
        // Change just the last character of the DID
        changed.id.push('X');
        assert_ne!(fp.hex, DocumentFingerprint::compute(&changed).hex);
    }

    // ── Test 9: FingerprintedDocument is_intact passes on fresh doc ───────────

    #[test]
    fn test_fingerprinted_document_is_intact() {
        let wrapped = FingerprintedDocument::new(make_doc());
        assert!(wrapped.is_intact(), "Fresh wrapped document must pass integrity");
    }

    // ── Test 10: FingerprintedDocument detects modification ────────────────────

    #[test]
    fn test_fingerprinted_document_detects_modification() {
        let mut wrapped = FingerprintedDocument::new(make_doc());
        wrapped.document.value = 0.0; // silently modified
        assert!(!wrapped.is_intact(), "Modified wrapped document must fail integrity");
    }

    // ── Test 11: array ordering preserved in canonical bytes ──────────────────

    #[test]
    fn test_array_order_preserved() {
        let v1 = json!({"items": [1, 2, 3]});
        let v2 = json!({"items": [3, 2, 1]});
        let fp1 = DocumentFingerprint::compute_sorted(&v1);
        let fp2 = DocumentFingerprint::compute_sorted(&v2);
        assert_ne!(fp1, fp2, "Array order must be preserved — different arrays → different fingerprints");
    }
}

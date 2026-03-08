//! Stage 1 — DID Document IPFS Storage (Prompt 7)
//!
//! Uploads the full DID document to IPFS, pins it to Pinata for persistence,
//! and returns a CID that is then embedded in the on-chain NFT metadata.
//!
//! # Architecture
//! ```text
//!   DID Document (Rust struct)
//!        │
//!        ▼  canonical JSON
//!   ipfs_storage::upload()
//!        │
//!        ├──► IPFS node  → raw CID (content-addressed, append-only)
//!        └──► Pinata API → pin CID (ensures persistence, not just availability)
//!        │
//!        ▼
//!   StorageReceipt { cid, pinata_pin_id, fingerprint }
//!        │
//!        └──► Stored in SensorNFTMetadata.metadata_cid (on-chain)
//! ```
//!
//! In production set `PINATA_JWT` env var. Tests use `MockIPFSClient`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use chrono::{DateTime, Utc};
use crate::stage1_birth_identity::did_hasher::{DocumentFingerprint, canonical_bytes};

// ── Types ─────────────────────────────────────────────────────────────────────

/// IPFS Content Identifier (v1 CIDv0 compatible).
/// In production this is the actual CID returned by `ipfs add`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentID(pub String);

impl ContentID {
    /// Simulate a deterministic CID from content bytes
    /// (production systems use SHA2-256 multihash → base58btc encoding).
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut h = Sha256::new();
        h.update(bytes);
        // Prefix "Qm" mimics CIDv0 / base58btc multihash style
        ContentID(format!("Qm{}", &hex::encode(h.finalize())[..44]))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The result of a successful upload + pin operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageReceipt {
    pub cid: ContentID,
    /// Pinata pin job ID (UUID in production).
    pub pinata_pin_id: String,
    /// SHA-256 fingerprint of the uploaded content (independent tamper check).
    pub content_fingerprint: DocumentFingerprint,
    pub uploaded_at: DateTime<Utc>,
    pub size_bytes: usize,
}

/// Error variants from upload or pin operations.
#[derive(Debug, Clone, PartialEq)]
pub enum StorageError {
    SerializationError(String),
    UploadFailed(String),
    PinFailed(String),
    VerificationFailed { cid: String, expected: String, got: String },
    NotFound(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::SerializationError(e) => write!(f, "SerializationError: {e}"),
            StorageError::UploadFailed(e)       => write!(f, "UploadFailed: {e}"),
            StorageError::PinFailed(e)          => write!(f, "PinFailed: {e}"),
            StorageError::VerificationFailed { cid, expected, got } =>
                write!(f, "VerificationFailed CID={cid}: expected {expected}, got {got}"),
            StorageError::NotFound(cid)         => write!(f, "NotFound: {cid}"),
        }
    }
}

// ── IPFS client trait ─────────────────────────────────────────────────────────

/// Abstraction over IPFS + Pinata operations.
/// Swap `MockIPFSClient` for `PinataClient` in production.
pub trait IPFSClient: Send + Sync {
    /// Upload raw bytes and return a CID.
    fn upload(&self, content: &[u8]) -> Result<ContentID, StorageError>;
    /// Pin a CID to ensure persistence, returning a pin job ID.
    fn pin(&self, cid: &ContentID) -> Result<String, StorageError>;
    /// Retrieve content by CID.
    fn retrieve(&self, cid: &ContentID) -> Result<Vec<u8>, StorageError>;
    /// Check whether a CID is pinned.
    fn is_pinned(&self, cid: &ContentID) -> bool;
}

// ── Mock client (for tests + offline dev) ────────────────────────────────────

pub struct MockIPFSClient {
    store: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
    pinned: std::sync::Mutex<std::collections::HashSet<String>>,
    /// Simulate upload failure.
    pub fail_upload: bool,
    /// Simulate pin failure.
    pub fail_pin: bool,
}

impl MockIPFSClient {
    pub fn new() -> Self {
        Self {
            store: std::sync::Mutex::new(std::collections::HashMap::new()),
            pinned: std::sync::Mutex::new(std::collections::HashSet::new()),
            fail_upload: false,
            fail_pin: false,
        }
    }
}

impl IPFSClient for MockIPFSClient {
    fn upload(&self, content: &[u8]) -> Result<ContentID, StorageError> {
        if self.fail_upload {
            return Err(StorageError::UploadFailed("Simulated upload failure".to_string()));
        }
        let cid = ContentID::from_bytes(content);
        self.store.lock().unwrap().insert(cid.0.clone(), content.to_vec());
        Ok(cid)
    }

    fn pin(&self, cid: &ContentID) -> Result<String, StorageError> {
        if self.fail_pin {
            return Err(StorageError::PinFailed("Simulated pin failure".to_string()));
        }
        if !self.store.lock().unwrap().contains_key(&cid.0) {
            return Err(StorageError::NotFound(cid.0.clone()));
        }
        self.pinned.lock().unwrap().insert(cid.0.clone());
        Ok(format!("pin-{}", &cid.0[..8]))
    }

    fn retrieve(&self, cid: &ContentID) -> Result<Vec<u8>, StorageError> {
        self.store.lock().unwrap()
            .get(&cid.0)
            .cloned()
            .ok_or_else(|| StorageError::NotFound(cid.0.clone()))
    }

    fn is_pinned(&self, cid: &ContentID) -> bool {
        self.pinned.lock().unwrap().contains(&cid.0)
    }
}

// ── Storage engine ────────────────────────────────────────────────────────────

/// Upload a serializable DID document to IPFS and pin it to Pinata.
///
/// Steps:
/// 1. Serialize to canonical JSON bytes
/// 2. Compute SHA-256 fingerprint (independent tamper check)
/// 3. Upload to IPFS → receive CID
/// 4. Pin CID to Pinata → receive pin job ID
/// 5. Return StorageReceipt
pub fn upload_did_document<T: Serialize>(
    doc: &T,
    client: &dyn IPFSClient,
) -> Result<StorageReceipt, StorageError> {
    let bytes = canonical_bytes(doc);
    let size_bytes = bytes.len();
    let fingerprint = DocumentFingerprint::compute(doc);

    let cid = client.upload(&bytes)?;
    let pinata_pin_id = client.pin(&cid)?;

    Ok(StorageReceipt {
        cid,
        pinata_pin_id,
        content_fingerprint: fingerprint,
        uploaded_at: Utc::now(),
        size_bytes,
    })
}

/// Retrieve a DID document from IPFS and verify its content fingerprint.
///
/// Returns `Ok(bytes)` if the CID exists and the content matches the stored fingerprint.
/// Returns `Err(VerificationFailed)` if the content has changed (CID collision attack).
pub fn retrieve_and_verify(
    receipt: &StorageReceipt,
    client: &dyn IPFSClient,
) -> Result<Vec<u8>, StorageError> {
    let bytes = client.retrieve(&receipt.cid)?;

    // Verify content integrity independently of IPFS CID
    let mut h = Sha256::new();
    h.update(&bytes);
    let retrieved_hash = hex::encode(h.finalize());

    if retrieved_hash != receipt.content_fingerprint.hex {
        return Err(StorageError::VerificationFailed {
            cid: receipt.cid.0.clone(),
            expected: receipt.content_fingerprint.hex.clone(),
            got: retrieved_hash,
        });
    }

    Ok(bytes)
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Serialize, Deserialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct MockDIDDoc {
        id: String,
        public_key: String,
        created: String,
    }

    fn make_doc() -> MockDIDDoc {
        MockDIDDoc {
            id: "did:cardano:sensor:biochar-001".to_string(),
            public_key: "02".to_string() + &"ab".repeat(32),
            created: "2025-03-05T12:30:00Z".to_string(),
        }
    }

    // ── Test 1: upload returns a valid CID ────────────────────────────────────

    #[test]
    fn test_upload_returns_cid() {
        let client = MockIPFSClient::new();
        let doc = make_doc();
        let result = upload_did_document(&doc, &client);
        assert!(result.is_ok(), "Upload should succeed");
        let receipt = result.unwrap();
        assert!(receipt.cid.0.starts_with("Qm"), "CID should start with Qm");
        assert_eq!(receipt.cid.0.len(), 46, "CID should be 46 chars (Qm + 44)");
    }

    // ── Test 2: CID is content-addressed (same content → same CID) ───────────

    #[test]
    fn test_cid_is_deterministic() {
        let client = MockIPFSClient::new();
        let doc = make_doc();
        let r1 = upload_did_document(&doc, &client).unwrap();
        let r2 = upload_did_document(&doc, &client).unwrap();
        assert_eq!(r1.cid, r2.cid, "Same content must produce same CID");
    }

    // ── Test 3: document is pinned after upload ───────────────────────────────

    #[test]
    fn test_document_is_pinned() {
        let client = MockIPFSClient::new();
        let receipt = upload_did_document(&make_doc(), &client).unwrap();
        assert!(client.is_pinned(&receipt.cid), "Document must be pinned");
        assert!(!receipt.pinata_pin_id.is_empty(), "Pinata pin ID must not be empty");
    }

    // ── Test 4: fingerprint matches document ──────────────────────────────────

    #[test]
    fn test_fingerprint_matches_document() {
        let client = MockIPFSClient::new();
        let doc = make_doc();
        let receipt = upload_did_document(&doc, &client).unwrap();
        assert!(receipt.content_fingerprint.verify(&doc), "Fingerprint must match document");
    }

    // ── Test 5: retrieve and verify succeeds on clean document ────────────────

    #[test]
    fn test_retrieve_and_verify_succeeds() {
        let client = MockIPFSClient::new();
        let receipt = upload_did_document(&make_doc(), &client).unwrap();
        let bytes = retrieve_and_verify(&receipt, &client);
        assert!(bytes.is_ok(), "Retrieval + verification should succeed");
    }

    // ── Test 6: size_bytes is correct ─────────────────────────────────────────

    #[test]
    fn test_size_bytes_correct() {
        let client = MockIPFSClient::new();
        let doc = make_doc();
        let expected_size = serde_json::to_vec(&doc).unwrap().len();
        let receipt = upload_did_document(&doc, &client).unwrap();
        assert_eq!(receipt.size_bytes, expected_size);
    }

    // ── Test 7: upload failure propagated ────────────────────────────────────

    #[test]
    fn test_upload_failure_propagated() {
        let mut client = MockIPFSClient::new();
        client.fail_upload = true;
        let result = upload_did_document(&make_doc(), &client);
        assert!(matches!(result.unwrap_err(), StorageError::UploadFailed(_)));
    }

    // ── Test 8: pin failure propagated ───────────────────────────────────────

    #[test]
    fn test_pin_failure_propagated() {
        let mut client = MockIPFSClient::new();
        client.fail_pin = true;
        let result = upload_did_document(&make_doc(), &client);
        assert!(matches!(result.unwrap_err(), StorageError::PinFailed(_)));
    }

    // ── Test 9: retrieve not-found returns NotFound error ─────────────────────

    #[test]
    fn test_retrieve_not_found() {
        let client = MockIPFSClient::new();
        // Upload doc A, create a receipt pointing to a different CID
        let receipt = upload_did_document(&make_doc(), &client).unwrap();
        let bad_receipt = StorageReceipt {
            cid: ContentID("QmGHOSTCIDthatdoesnotexist00000000000000000000".to_string()),
            ..receipt
        };
        let result = retrieve_and_verify(&bad_receipt, &client);
        assert!(matches!(result.unwrap_err(), StorageError::NotFound(_)));
    }

    // ── Test 10: different documents → different CIDs ─────────────────────────

    #[test]
    fn test_different_docs_different_cids() {
        let client = MockIPFSClient::new();
        let mut doc2 = make_doc();
        doc2.public_key = "03".to_string() + &"cd".repeat(32);
        let r1 = upload_did_document(&make_doc(), &client).unwrap();
        let r2 = upload_did_document(&doc2, &client).unwrap();
        assert_ne!(r1.cid, r2.cid, "Different docs must produce different CIDs");
    }

    // ── Test 11: ContentID from_bytes is deterministic ────────────────────────

    #[test]
    fn test_content_id_deterministic() {
        let bytes = b"hello malama";
        let cid1 = ContentID::from_bytes(bytes);
        let cid2 = ContentID::from_bytes(bytes);
        assert_eq!(cid1, cid2);
        assert!(cid1.0.starts_with("Qm"));
    }
}

//! Stage 1 — Cardano Sensor NFT Registry (Prompt 5)
//!
//! Implements the Rust side of the DID NFT Registration contract.
//! In production this module submits transactions to Cardano via the
//! Blockfrost HTTP API. Here we provide:
//!
//! 1. The canonical `SensorNFTMetadata` struct (mirrors Plutus datum)
//! 2. An in-memory `NFTRegistry` that enforces the same rules as the
//!    on-chain Plutus validator (for unit tests and offline development)
//! 3. A `BlockfrostAdapter` stub (with trait) for production calls
//!
//! # Narrative
//! "The sensor announces its birth to the world. It mints an NFT on Cardano
//!  bearing its DID. From this moment forward, the sensor exists in the
//!  global ledger — unforgeable."

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

// ── On-chain NFT Metadata ─────────────────────────────────────────────────────
// Mirrors the `SensorNFTDatum` in contracts/cardano/SensorNFT.hs

/// DID prefix enforced both by the Plutus validator and this Rust guard.
pub const CARDANO_DID_PREFIX: &str = "did:cardano:sensor:";

/// CIP-721 compatible NFT metadata structure for a Sensor DID NFT.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SensorNFTMetadata {
    pub nft_id: String,
    pub sensor_did: String,
    /// Hex-encoded SEC-1 compressed ECDSA public key (33 bytes → 66 hex chars).
    pub public_key: String,
    pub latitude: f64,
    pub longitude: f64,
    /// POSIX timestamp (seconds) when the NFT was minted.
    pub minted_at: i64,
    /// IPFS CID of the full DID document JSON (off-chain storage).
    pub metadata_cid: String,
}

impl SensorNFTMetadata {
    /// SHA-256 of the canonical JSON — used as the on-chain content hash.
    pub fn content_hash(&self) -> [u8; 32] {
        let bytes = serde_json::to_vec(self).expect("SensorNFTMetadata serialization infallible");
        let mut h = Sha256::new();
        h.update(bytes);
        h.finalize().into()
    }

    /// Content hash as a hex string.
    pub fn content_hash_hex(&self) -> String {
        hex::encode(self.content_hash())
    }
}

// ── Minting result ────────────────────────────────────────────────────────────

/// The outcome of a successful NFT mint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintReceipt {
    /// Simulated Cardano transaction ID (SHA-256 of metadata + timestamp).
    pub tx_id: String,
    pub nft_id: String,
    pub sensor_did: String,
    pub minted_at: DateTime<Utc>,
    pub metadata: SensorNFTMetadata,
    /// Content hash of the on-chain datum for immutability verification.
    pub datum_hash: String,
}

// ── Validation errors ─────────────────────────────────────────────────────────

/// Mirrors the `traceIfFalse` guards in the Plutus contract.
#[derive(Debug, Clone, PartialEq)]
pub enum RegistrationError {
    /// DID does not begin with `did:cardano:sensor:`.
    InvalidDIDFormat(String),
    /// Public key is not 33 bytes (66 hex chars) compressed SEC-1.
    InvalidPublicKey(String),
    /// A sensor NFT with this DID already exists (duplicate registration).
    AlreadyRegistered(String),
    /// IPFS metadata CID is missing or empty.
    MissingMetadataCID,
}

impl std::fmt::Display for RegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistrationError::InvalidDIDFormat(d)  => write!(f, "InvalidDID: {d}"),
            RegistrationError::InvalidPublicKey(k)  => write!(f, "InvalidKey: {k}"),
            RegistrationError::AlreadyRegistered(d) => write!(f, "AlreadyRegistered: {d}"),
            RegistrationError::MissingMetadataCID   => write!(f, "NoCID"),
        }
    }
}

// ── In-memory NFT Registry ────────────────────────────────────────────────────

/// Enforces the same invariants as the Plutus on-chain validator,
/// including: unique DID, valid format, valid key, valid CID.
///
/// In production, replace `mint()` with a Blockfrost API call that
/// submits the compiled Plutus transaction.
pub struct NFTRegistry {
    /// sensor_did → MintReceipt
    registry: HashMap<String, MintReceipt>,
}

impl NFTRegistry {
    pub fn new() -> Self {
        Self { registry: HashMap::new() }
    }

    /// Register a new sensor NFT.
    ///
    /// Validates all Plutus guards before "minting":
    /// - Valid DID prefix
    /// - Valid public key length (33 bytes compressed)
    /// - DID not already registered
    /// - Non-empty IPFS CID
    ///
    /// Returns a `MintReceipt` on success.
    pub fn mint(
        &mut self,
        metadata: SensorNFTMetadata,
    ) -> Result<MintReceipt, RegistrationError> {
        // Guard 1: assertValidDIDFormat
        self.assert_valid_did(&metadata.sensor_did)?;
        // Guard 2: assertValidECDSAKey (33 bytes = 66 hex chars)
        self.assert_valid_key(&metadata.public_key)?;
        // Guard 3: assertDIDNotRegistered
        if self.registry.contains_key(&metadata.sensor_did) {
            return Err(RegistrationError::AlreadyRegistered(metadata.sensor_did.clone()));
        }
        // Guard 4: non-empty IPFS CID
        if metadata.metadata_cid.is_empty() {
            return Err(RegistrationError::MissingMetadataCID);
        }

        let now = Utc::now();
        let datum_hash = metadata.content_hash_hex();

        // Simulate tx_id = SHA-256(datum_hash || timestamp)
        let tx_preimage = format!("{}{}", datum_hash, now.timestamp_nanos_opt().unwrap_or(0));
        let mut h = Sha256::new();
        h.update(tx_preimage.as_bytes());
        let tx_id = hex::encode(h.finalize());

        let receipt = MintReceipt {
            tx_id,
            nft_id: metadata.nft_id.clone(),
            sensor_did: metadata.sensor_did.clone(),
            minted_at: now,
            datum_hash,
            metadata: metadata.clone(),
        };

        self.registry.insert(metadata.sensor_did.clone(), receipt.clone());
        Ok(receipt)
    }

    /// Query NFT by sensor DID. Returns `None` if not registered.
    ///
    /// Maps to the Plutus `ValidateQuery` redeemer — available to any caller.
    pub fn has_nft_for_did(&self, sensor_did: &str) -> bool {
        self.registry.contains_key(sensor_did)
    }

    /// Retrieve the full NFT metadata for a registered sensor.
    pub fn get_nft(&self, sensor_did: &str) -> Option<&MintReceipt> {
        self.registry.get(sensor_did)
    }

    /// Total number of registered sensors.
    pub fn sensor_count(&self) -> usize {
        self.registry.len()
    }

    /// Verify that an existing NFT's metadata has not been tampered with
    /// (compares stored datum_hash against recomputed hash).
    pub fn verify_immutability(&self, sensor_did: &str) -> Option<bool> {
        let receipt = self.registry.get(sensor_did)?;
        Some(receipt.metadata.content_hash_hex() == receipt.datum_hash)
    }

    // ── Private guards (mirror Plutus validators) ────────────────────────────

    fn assert_valid_did(&self, did: &str) -> Result<(), RegistrationError> {
        if did.starts_with(CARDANO_DID_PREFIX) && did.len() > CARDANO_DID_PREFIX.len() {
            Ok(())
        } else {
            Err(RegistrationError::InvalidDIDFormat(did.to_string()))
        }
    }

    fn assert_valid_key(&self, pubkey_hex: &str) -> Result<(), RegistrationError> {
        // Compressed SEC-1 secp256k1 key = 33 bytes = 66 hex characters
        if pubkey_hex.len() == 66 && pubkey_hex.chars().all(|c| c.is_ascii_hexdigit()) {
            Ok(())
        } else {
            Err(RegistrationError::InvalidPublicKey(pubkey_hex.to_string()))
        }
    }
}

impl Default for NFTRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Blockfrost adapter trait ──────────────────────────────────────────────────

/// Trait for submitting Cardano transactions via Blockfrost.
/// Swap the `MockBlockfrost` implementation for `BlockfrostClient` in production.
pub trait CardanoAdapter {
    fn submit_mint_tx(&self, metadata: &SensorNFTMetadata) -> Result<String, String>;
    fn query_nft(&self, sensor_did: &str) -> Option<SensorNFTMetadata>;
}

/// Offline stub — used in tests and CI without a live Blockfrost key.
pub struct MockBlockfrost {
    store: std::sync::Mutex<HashMap<String, SensorNFTMetadata>>,
}

impl MockBlockfrost {
    pub fn new() -> Self {
        Self { store: std::sync::Mutex::new(HashMap::new()) }
    }
}

impl CardanoAdapter for MockBlockfrost {
    fn submit_mint_tx(&self, metadata: &SensorNFTMetadata) -> Result<String, String> {
        let mut store = self.store.lock().unwrap();
        if store.contains_key(&metadata.sensor_did) {
            return Err(format!("AlreadyRegistered: {}", metadata.sensor_did));
        }
        store.insert(metadata.sensor_did.clone(), metadata.clone());
        Ok(format!("mock_tx_{}", hex::encode(metadata.content_hash())))
    }

    fn query_nft(&self, sensor_did: &str) -> Option<SensorNFTMetadata> {
        self.store.lock().unwrap().get(sensor_did).cloned()
    }
}

// ── Builder helper ────────────────────────────────────────────────────────────

/// Convenience builder for tests.
pub fn make_sensor_nft(did_suffix: &str, pubkey_hex: &str, cid: &str) -> SensorNFTMetadata {
    SensorNFTMetadata {
        nft_id: format!("sensor-{did_suffix}-nft"),
        sensor_did: format!("did:cardano:sensor:{did_suffix}"),
        public_key: pubkey_hex.to_string(),
        latitude: 43.8,
        longitude: -115.9,
        minted_at: Utc::now().timestamp(),
        metadata_cid: cid.to_string(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::{SigningKey, VerifyingKey};
    use rand::rngs::OsRng;

    /// Generate a real 33-byte compressed ECDSA public key hex.
    fn real_pubkey_hex() -> String {
        let sk = SigningKey::random(&mut OsRng);
        let vk = VerifyingKey::from(&sk);
        hex::encode(vk.to_encoded_point(true).as_bytes())
    }

    /// Fake but syntactically valid 66-char hex key for tests that don't
    /// need a real key.
    fn dummy_pubkey_hex() -> String {
        "02".to_string() + &"ab".repeat(32)  // 2 + 64 = 66 chars, compressed prefix 02
    }

    fn dummy_cid() -> &'static str {
        "QmX7f8P3q2K9mN5vL3jK8pR4sT6uV2wX1yZ9aB5cD7eF0"
    }

    // ── Test 1: register sensor → NFT minted successfully ────────────────────

    #[test]
    fn test_register_sensor_mints_nft() {
        let mut registry = NFTRegistry::new();
        let meta = make_sensor_nft("biochar-001", &real_pubkey_hex(), dummy_cid());
        let result = registry.mint(meta.clone());

        assert!(result.is_ok(), "Valid registration should succeed");
        let receipt = result.unwrap();
        assert_eq!(receipt.sensor_did, "did:cardano:sensor:biochar-001");
        assert!(!receipt.tx_id.is_empty(), "tx_id must be populated");
        assert_eq!(receipt.metadata.nft_id, "sensor-biochar-001-nft");
    }

    // ── Test 2: sensor count increments after mint ────────────────────────────

    #[test]
    fn test_sensor_count_after_registration() {
        let mut registry = NFTRegistry::new();
        assert_eq!(registry.sensor_count(), 0);
        registry.mint(make_sensor_nft("a", &real_pubkey_hex(), dummy_cid())).unwrap();
        registry.mint(make_sensor_nft("b", &real_pubkey_hex(), dummy_cid())).unwrap();
        assert_eq!(registry.sensor_count(), 2);
    }

    // ── Test 3: query by DID returns correct metadata ─────────────────────────

    #[test]
    fn test_query_nft_by_did() {
        let mut registry = NFTRegistry::new();
        let pk = real_pubkey_hex();
        let meta = make_sensor_nft("biochar-001", &pk, dummy_cid());
        registry.mint(meta.clone()).unwrap();

        assert!(registry.has_nft_for_did("did:cardano:sensor:biochar-001"));
        let receipt = registry.get_nft("did:cardano:sensor:biochar-001").unwrap();
        assert_eq!(receipt.metadata.public_key, pk);
        assert_eq!(receipt.metadata.latitude, 43.8);
        assert_eq!(receipt.metadata.metadata_cid, dummy_cid());
    }

    // ── Test 4: duplicate registration → fails ────────────────────────────────

    #[test]
    fn test_duplicate_registration_fails() {
        let mut registry = NFTRegistry::new();
        registry.mint(make_sensor_nft("biochar-001", &real_pubkey_hex(), dummy_cid())).unwrap();
        let result = registry.mint(make_sensor_nft("biochar-001", &real_pubkey_hex(), dummy_cid()));

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            RegistrationError::AlreadyRegistered("did:cardano:sensor:biochar-001".to_string())
        );
    }

    // ── Test 5: NFT metadata is immutable (datum hash matches) ───────────────

    #[test]
    fn test_nft_metadata_immutable() {
        let mut registry = NFTRegistry::new();
        registry.mint(make_sensor_nft("biochar-001", &real_pubkey_hex(), dummy_cid())).unwrap();

        let is_intact = registry.verify_immutability("did:cardano:sensor:biochar-001");
        assert_eq!(is_intact, Some(true), "Metadata must pass immutability check");
    }

    // ── Test 6: invalid DID format rejected ──────────────────────────────────

    #[test]
    fn test_invalid_did_format_rejected() {
        let mut registry = NFTRegistry::new();
        let mut bad = make_sensor_nft("biochar-001", &real_pubkey_hex(), dummy_cid());
        bad.sensor_did = "sensor:biochar-001".to_string(); // missing prefix
        let result = registry.mint(bad);
        assert!(matches!(result.unwrap_err(), RegistrationError::InvalidDIDFormat(_)));
    }

    #[test]
    fn test_empty_did_rejected() {
        let mut registry = NFTRegistry::new();
        let mut bad = make_sensor_nft("biochar-001", &real_pubkey_hex(), dummy_cid());
        bad.sensor_did = String::new();
        assert!(matches!(
            registry.mint(bad).unwrap_err(),
            RegistrationError::InvalidDIDFormat(_)
        ));
    }

    // ── Test 7: invalid public key rejected ──────────────────────────────────

    #[test]
    fn test_short_public_key_rejected() {
        let mut registry = NFTRegistry::new();
        let mut bad = make_sensor_nft("biochar-001", "shortkey", dummy_cid());
        assert!(matches!(
            registry.mint(bad).unwrap_err(),
            RegistrationError::InvalidPublicKey(_)
        ));
    }

    #[test]
    fn test_non_hex_public_key_rejected() {
        let mut registry = NFTRegistry::new();
        let bad_key = "02".to_string() + &"ZZ".repeat(32); // non-hex chars
        let mut bad = make_sensor_nft("biochar-001", &bad_key, dummy_cid());
        assert!(matches!(
            registry.mint(bad).unwrap_err(),
            RegistrationError::InvalidPublicKey(_)
        ));
    }

    // ── Test 8: missing IPFS CID rejected ────────────────────────────────────

    #[test]
    fn test_empty_cid_rejected() {
        let mut registry = NFTRegistry::new();
        let mut bad = make_sensor_nft("biochar-001", &real_pubkey_hex(), "");
        assert_eq!(registry.mint(bad).unwrap_err(), RegistrationError::MissingMetadataCID);
    }

    // ── Test 9: unregistered DID returns None ─────────────────────────────────

    #[test]
    fn test_unregistered_did_returns_none() {
        let registry = NFTRegistry::new();
        assert!(!registry.has_nft_for_did("did:cardano:sensor:ghost"));
        assert!(registry.get_nft("did:cardano:sensor:ghost").is_none());
    }

    // ── Test 10: real compressed public key validates correctly ───────────────

    #[test]
    fn test_real_ecdsa_key_validates() {
        let mut registry = NFTRegistry::new();
        let pk = real_pubkey_hex();
        assert_eq!(pk.len(), 66, "Compressed secp256k1 key should be 66 hex chars");
        let result = registry.mint(make_sensor_nft("key-test", &pk, dummy_cid()));
        assert!(result.is_ok(), "Real key should pass validation");
    }

    // ── Test 11: content hash is deterministic ───────────────────────────────

    #[test]
    fn test_content_hash_deterministic() {
        let meta = make_sensor_nft("biochar-001", &dummy_pubkey_hex(), dummy_cid());
        assert_eq!(meta.content_hash(), meta.content_hash(), "Hash must be deterministic");
    }

    // ── Test 12: MockBlockfrost adapter ──────────────────────────────────────

    #[test]
    fn test_mock_blockfrost_mint_and_query() {
        let adapter = MockBlockfrost::new();
        let meta = make_sensor_nft("biochar-001", &real_pubkey_hex(), dummy_cid());

        let tx_id = adapter.submit_mint_tx(&meta);
        assert!(tx_id.is_ok(), "First mint on MockBlockfrost should succeed");

        let queried = adapter.query_nft("did:cardano:sensor:biochar-001");
        assert!(queried.is_some(), "Should be queryable after mint");
        assert_eq!(queried.unwrap().sensor_did, "did:cardano:sensor:biochar-001");
    }

    #[test]
    fn test_mock_blockfrost_rejects_duplicate() {
        let adapter = MockBlockfrost::new();
        let meta = make_sensor_nft("biochar-001", &real_pubkey_hex(), dummy_cid());
        adapter.submit_mint_tx(&meta).unwrap();
        assert!(adapter.submit_mint_tx(&meta).is_err(), "Duplicate on MockBlockfrost must fail");
    }

    // ── Test 13: metadata fields match prompt spec ───────────────────────────

    #[test]
    fn test_metadata_matches_spec_structure() {
        let pk = real_pubkey_hex();
        let meta = SensorNFTMetadata {
            nft_id: "sensor-biochar-001-nft".to_string(),
            sensor_did: "did:cardano:sensor:biochar-001".to_string(),
            public_key: pk.clone(),
            latitude: 43.8,
            longitude: -115.9,
            minted_at: 1704067200,
            metadata_cid: "QmX7f8P3q2K9mN5vL3jK8pR4sT6uV2wX1yZ9aB5cD7eF0".to_string(),
        };

        let json = serde_json::to_value(&meta).unwrap();
        assert_eq!(json["nft_id"], "sensor-biochar-001-nft");
        assert_eq!(json["sensor_did"], "did:cardano:sensor:biochar-001");
        assert_eq!(json["latitude"], 43.8);
        assert_eq!(json["longitude"], -115.9);
        assert_eq!(json["minted_at"], 1704067200i64);
    }
}

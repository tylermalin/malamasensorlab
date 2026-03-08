//! Stage 4 — Prompt 27: Cardano Plutus Contract Simulation
//!
//! Three Plutus contracts for the Mālama Protocol:
//!   1. SensorRegistry.hs  — Mint sensor NFT, store DID + public key on-chain
//!   2. MerkleRootAnchor.hs — Submit Merkle roots with validator quorum proof
//!   3. ReputationTracker.hs — Update validator reputation scores on-chain
//!
//! This Rust module simulates the off-chain submission logic that would call
//! real Cardano node via cardano-serialization-lib / Blockfrost HTTP API.
//!
//! Production note: Replace MockCardanoClient with calls to:
//!   POST https://cardano-mainnet.blockfrost.io/api/v0/tx/submit

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// ── Cardano transaction record ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CardanoTxReceipt {
    pub tx_hash: String,
    pub contract: CardanoContract,
    pub datum_hash: String,
    pub submitted_at: DateTime<Utc>,
    pub slot: u64,
    /// Lovelace fee paid (~0.17 ADA = 170000 lovelace).
    pub fee_lovelace: u64,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CardanoContract {
    SensorRegistry,
    MerkleRootAnchor,
    ReputationTracker,
}

impl std::fmt::Display for CardanoContract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CardanoContract::SensorRegistry => write!(f, "SensorRegistry.hs"),
            CardanoContract::MerkleRootAnchor => write!(f, "MerkleRootAnchor.hs"),
            CardanoContract::ReputationTracker => write!(f, "ReputationTracker.hs"),
        }
    }
}

// ── Contract datums ───────────────────────────────────────────────────────────

/// Datum for SensorRegistry — stored on-chain as NFT metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorRegistryDatum {
    pub sensor_did: String,
    pub public_key_hex: String,
    pub location: (f64, f64),
    pub registered_at_slot: u64,
    pub metadata_cid: String,
}

/// Datum for MerkleRootAnchor — one UTxO per batch submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleRootDatum {
    pub batch_id: String,
    pub merkle_root: String,
    pub validator_signatures: Vec<String>,
    pub quorum_size: usize,
    pub ipfs_cid: String,
    pub anchored_at_slot: u64,
}

/// Datum for ReputationTracker — validator reputation updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationDatum {
    pub validator_id: String,
    pub uptime_100: u64,      // stored as integer: 95 = 95%
    pub accuracy_100: u64,
    pub stake_100: u64,       // 100 = full stake, 0 = slashed
    pub last_updated_slot: u64,
}

// ── Mock Cardano client ────────────────────────────────────────────────────────

pub struct MockCardanoClient {
    /// slot → tx
    pub ledger: HashMap<String, CardanoTxReceipt>,
    pub current_slot: u64,
}

impl MockCardanoClient {
    pub fn new() -> Self {
        Self { ledger: HashMap::new(), current_slot: 10_000_000 }
    }

    fn next_slot(&mut self) -> u64 {
        self.current_slot += 20; // ~20 seconds per slot
        self.current_slot
    }

    fn tx_hash(content: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(content);
        hex::encode(h.finalize())
    }

    fn datum_hash(datum: &serde_json::Value) -> String {
        let bytes = serde_json::to_vec(datum).unwrap_or_default();
        let mut h = Sha256::new();
        h.update(&bytes);
        hex::encode(h.finalize())
    }

    /// Submit to SensorRegistry contract.
    pub fn register_sensor(&mut self, datum: &SensorRegistryDatum) -> Result<CardanoTxReceipt, String> {
        let slot = self.next_slot();
        let bytes = serde_json::to_vec(datum).unwrap_or_default();
        let tx_hash = Self::tx_hash(&bytes);
        let datum_val = serde_json::to_value(datum).unwrap_or_default();

        let receipt = CardanoTxReceipt {
            tx_hash: tx_hash.clone(),
            contract: CardanoContract::SensorRegistry,
            datum_hash: Self::datum_hash(&datum_val),
            submitted_at: Utc::now(),
            slot,
            fee_lovelace: 170_000, // ~0.17 ADA
            confirmed: true,
        };
        self.ledger.insert(tx_hash, receipt.clone());
        Ok(receipt)
    }

    /// Anchor a Merkle root via MerkleRootAnchor contract.
    pub fn anchor_merkle_root(&mut self, datum: &MerkleRootDatum) -> Result<CardanoTxReceipt, String> {
        if datum.quorum_size < 2 {
            return Err("Quorum requires at least 2 signatures".to_string());
        }
        let slot = self.next_slot();
        let bytes = serde_json::to_vec(datum).unwrap_or_default();
        let tx_hash = Self::tx_hash(&bytes);
        let datum_val = serde_json::to_value(datum).unwrap_or_default();

        let receipt = CardanoTxReceipt {
            tx_hash: tx_hash.clone(),
            contract: CardanoContract::MerkleRootAnchor,
            datum_hash: Self::datum_hash(&datum_val),
            submitted_at: Utc::now(),
            slot,
            fee_lovelace: 200_000,
            confirmed: true,
        };
        self.ledger.insert(tx_hash, receipt.clone());
        Ok(receipt)
    }

    /// Update validator reputation via ReputationTracker contract.
    pub fn update_reputation(&mut self, datum: &ReputationDatum) -> Result<CardanoTxReceipt, String> {
        if datum.stake_100 > 100 || datum.uptime_100 > 100 || datum.accuracy_100 > 100 {
            return Err("Percentage values must be 0–100".to_string());
        }
        let slot = self.next_slot();
        let bytes = serde_json::to_vec(datum).unwrap_or_default();
        let tx_hash = Self::tx_hash(&bytes);
        let datum_val = serde_json::to_value(datum).unwrap_or_default();

        let receipt = CardanoTxReceipt {
            tx_hash: tx_hash.clone(),
            contract: CardanoContract::ReputationTracker,
            datum_hash: Self::datum_hash(&datum_val),
            submitted_at: Utc::now(),
            slot,
            fee_lovelace: 150_000,
            confirmed: true,
        };
        self.ledger.insert(tx_hash, receipt.clone());
        Ok(receipt)
    }

    pub fn get_tx(&self, tx_hash: &str) -> Option<&CardanoTxReceipt> {
        self.ledger.get(tx_hash)
    }
}

impl Default for MockCardanoClient { fn default() -> Self { Self::new() } }

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sensor_datum() -> SensorRegistryDatum {
        SensorRegistryDatum {
            sensor_did: "did:cardano:sensor:biochar-001".to_string(),
            public_key_hex: "02a1b2c3".to_string(),
            location: (43.8123, -115.9456),
            registered_at_slot: 10_000_000,
            metadata_cid: "QmXx1234".to_string(),
        }
    }

    fn merkle_datum() -> MerkleRootDatum {
        MerkleRootDatum {
            batch_id: "batch-001".to_string(),
            merkle_root: "a".repeat(64),
            validator_signatures: vec!["sig1".to_string(), "sig2".to_string()],
            quorum_size: 2,
            ipfs_cid: "QmAbcd".to_string(),
            anchored_at_slot: 10_000_020,
        }
    }

    // ── Test 1: sensor registration returns tx hash ───────────────────────────

    #[test]
    fn test_sensor_registration_succeeds() {
        let mut client = MockCardanoClient::new();
        let recv = client.register_sensor(&sensor_datum()).unwrap();
        assert_eq!(recv.tx_hash.len(), 64, "TxHash must be 64-char hex");
        assert_eq!(recv.contract, CardanoContract::SensorRegistry);
        assert!(recv.confirmed);
    }

    // ── Test 2: merkle root anchoring requires quorum ─────────────────────────

    #[test]
    fn test_merkle_root_requires_quorum() {
        let mut client = MockCardanoClient::new();
        let mut datum = merkle_datum();
        datum.quorum_size = 1; // below threshold
        let result = client.anchor_merkle_root(&datum);
        assert!(result.is_err(), "Quorum < 2 must be rejected");
    }

    // ── Test 3: valid merkle root anchored ────────────────────────────────────

    #[test]
    fn test_merkle_root_anchoring_succeeds() {
        let mut client = MockCardanoClient::new();
        let recv = client.anchor_merkle_root(&merkle_datum()).unwrap();
        assert_eq!(recv.contract, CardanoContract::MerkleRootAnchor);
        assert_eq!(recv.fee_lovelace, 200_000);
    }

    // ── Test 4: reputation update stored ─────────────────────────────────────

    #[test]
    fn test_reputation_update_stored() {
        let mut client = MockCardanoClient::new();
        let datum = ReputationDatum {
            validator_id: "v1".to_string(),
            uptime_100: 95,
            accuracy_100: 88,
            stake_100: 100,
            last_updated_slot: 10_000_020,
        };
        let recv = client.update_reputation(&datum).unwrap();
        assert_eq!(recv.contract, CardanoContract::ReputationTracker);
    }

    // ── Test 5: invalid percentage rejected ──────────────────────────────────

    #[test]
    fn test_invalid_percentage_rejected() {
        let mut client = MockCardanoClient::new();
        let datum = ReputationDatum {
            validator_id: "v1".to_string(),
            uptime_100: 150, // invalid
            accuracy_100: 88,
            stake_100: 100,
            last_updated_slot: 0,
        };
        assert!(client.update_reputation(&datum).is_err());
    }

    // ── Test 6: slot increments with each tx ──────────────────────────────────

    #[test]
    fn test_slot_increments() {
        let mut client = MockCardanoClient::new();
        let r1 = client.register_sensor(&sensor_datum()).unwrap();
        let r2 = client.anchor_merkle_root(&merkle_datum()).unwrap();
        assert!(r2.slot > r1.slot, "Later tx must have higher slot");
    }

    // ── Test 7: tx retrievable by hash ────────────────────────────────────────

    #[test]
    fn test_tx_retrievable() {
        let mut client = MockCardanoClient::new();
        let recv = client.register_sensor(&sensor_datum()).unwrap();
        let found = client.get_tx(&recv.tx_hash).unwrap();
        assert_eq!(found.tx_hash, recv.tx_hash);
    }

    // ── Test 8: datum hash is deterministic ──────────────────────────────────

    #[test]
    fn test_datum_hash_deterministic() {
        let mut c1 = MockCardanoClient::new();
        let mut c2 = MockCardanoClient::new();
        let d = sensor_datum();
        let r1 = c1.register_sensor(&d).unwrap();
        let r2 = c2.register_sensor(&d).unwrap();
        assert_eq!(r1.datum_hash, r2.datum_hash);
    }
}

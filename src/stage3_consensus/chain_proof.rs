//! Stage 3 — Prompt 22: Blockchain Proof Recording
//!
//! Records the quorum decision on-chain as an immutable struct:
//!   { merkle_root, validator_sigs[], timestamp, block_height }
//!
//! Every consensus decision becomes an append-only audit record.
//! The `ChainProofStore` simulates the on-chain state (mock for off-chain unit tests).
//!
//! In production: submit via `cardano_nft::submit_blockchain_proof()`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// ── On-chain proof record ─────────────────────────────────────────────────────

/// Immutable on-chain record of a quorum decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlockchainProof {
    /// The batch this proof covers.
    pub batch_id: String,
    /// The Merkle root the validators signed off on.
    pub merkle_root: String,
    /// DER-encoded ECDSA signatures from each approving validator.
    pub validator_signatures: Vec<ValidatorSigRecord>,
    /// Number of valid signatures (redundant for easy on-chain reads).
    pub quorum_size: usize,
    /// RFC-3339 timestamp when quorum was reached.
    pub committed_at: DateTime<Utc>,
    /// Simulated block height at which this was anchored.
    pub block_height: u64,
    /// Chain name (e.g. "cardano", "base", "hedera").
    pub chain: String,
    /// SHA-256 fingerprint of the full proof struct (tamper-evident).
    pub proof_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidatorSigRecord {
    pub validator_id: String,
    pub signature_hex: String,
    pub verifying_key_hex: String,
}

impl BlockchainProof {
    pub fn new(
        batch_id: &str,
        merkle_root: &str,
        sigs: Vec<ValidatorSigRecord>,
        block_height: u64,
        chain: &str,
    ) -> Self {
        let committed_at = Utc::now();
        let quorum_size = sigs.len();

        let mut proto = Self {
            batch_id: batch_id.to_string(),
            merkle_root: merkle_root.to_string(),
            validator_signatures: sigs,
            quorum_size,
            committed_at,
            block_height,
            chain: chain.to_string(),
            proof_hash: String::new(), // computed below
        };
        proto.proof_hash = proto.compute_hash();
        proto
    }

    /// SHA-256 of (batch_id ∥ merkle_root ∥ committed_at ∥ block_height ∥ chain).
    fn compute_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.batch_id.as_bytes());
        h.update(self.merkle_root.as_bytes());
        h.update(self.committed_at.to_rfc3339().as_bytes());
        h.update(self.block_height.to_le_bytes());
        h.update(self.chain.as_bytes());
        for sig in &self.validator_signatures {
            h.update(sig.validator_id.as_bytes());
            h.update(sig.signature_hex.as_bytes());
        }
        hex::encode(h.finalize())
    }

    /// Verify the proof_hash has not been tampered with.
    pub fn is_intact(&self) -> bool {
        self.proof_hash == self.compute_hash()
    }

    /// True if quorum_size >= threshold (2-of-3).
    pub fn meets_quorum(&self, threshold: usize) -> bool {
        self.quorum_size >= threshold
    }
}

// ── On-chain proof store ──────────────────────────────────────────────────────

/// Append-only in-memory store (mock on-chain state).
///
/// In production: wraps a Plutus datum or Blockfrost submission.
pub struct ChainProofStore {
    pub chain: String,
    /// batch_id → proof (immutable once written)
    records: HashMap<String, BlockchainProof>,
    pub next_block_height: u64,
}

impl ChainProofStore {
    pub fn new(chain: &str) -> Self {
        Self { chain: chain.to_string(), records: HashMap::new(), next_block_height: 1 }
    }

    /// Record a new proof on the chain. Returns `Err` if already recorded.
    pub fn record(
        &mut self,
        batch_id: &str,
        merkle_root: &str,
        sigs: Vec<ValidatorSigRecord>,
    ) -> Result<&BlockchainProof, String> {
        if self.records.contains_key(batch_id) {
            return Err(format!("Batch {batch_id} already recorded on {}", self.chain));
        }
        let proof = BlockchainProof::new(
            batch_id,
            merkle_root,
            sigs,
            self.next_block_height,
            &self.chain,
        );
        self.next_block_height += 1;
        self.records.insert(batch_id.to_string(), proof);
        Ok(self.records.get(batch_id).unwrap())
    }

    /// Retrieve an immutable proof by batch_id.
    pub fn get(&self, batch_id: &str) -> Option<&BlockchainProof> {
        self.records.get(batch_id)
    }

    /// Number of proofs recorded.
    pub fn count(&self) -> usize { self.records.len() }

    /// Verify all stored proofs are intact (hash not tampered).
    pub fn verify_all(&self) -> Vec<&str> {
        self.records.iter()
            .filter(|(_, p)| !p.is_intact())
            .map(|(id, _)| id.as_str())
            .collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sig(v: &str) -> ValidatorSigRecord {
        ValidatorSigRecord {
            validator_id: v.to_string(),
            signature_hex: format!("sig_{v}"),
            verifying_key_hex: format!("vk_{v}"),
        }
    }

    const BATCH: &str  = "batch-2025-03-05-1300-biochar-001";
    fn root() -> &'static str { "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }

    // ── Test 1: proof recorded and retrievable ────────────────────────────────

    #[test]
    fn test_proof_recorded_and_retrievable() {
        let mut store = ChainProofStore::new("cardano");
        let sigs = vec![make_sig("v1"), make_sig("v2")];
        let proof = store.record(BATCH, root(), sigs).unwrap();
        assert_eq!(proof.batch_id, BATCH);
        assert_eq!(proof.quorum_size, 2);
        assert_eq!(store.count(), 1);
    }

    // ── Test 2: duplicate record rejected ─────────────────────────────────────

    #[test]
    fn test_duplicate_record_rejected() {
        let mut store = ChainProofStore::new("cardano");
        store.record(BATCH, root(), vec![make_sig("v1")]).unwrap();
        let result = store.record(BATCH, root(), vec![make_sig("v2")]);
        assert!(result.is_err(), "Duplicate batch must be rejected");
    }

    // ── Test 3: proof hash verifies correctly ─────────────────────────────────

    #[test]
    fn test_proof_hash_intact() {
        let proof = BlockchainProof::new(BATCH, root(), vec![make_sig("v1")], 100, "cardano");
        assert!(proof.is_intact(), "Fresh proof must be intact");
    }

    // ── Test 4: tampered proof fails hash check ───────────────────────────────

    #[test]
    fn test_tampered_proof_fails_hash() {
        let mut proof = BlockchainProof::new(BATCH, root(), vec![make_sig("v1")], 100, "cardano");
        proof.merkle_root = "tampered".to_string(); // modify after signing
        assert!(!proof.is_intact(), "Tampered proof must fail hash check");
    }

    // ── Test 5: meets_quorum check ────────────────────────────────────────────

    #[test]
    fn test_meets_quorum() {
        let proof = BlockchainProof::new(BATCH, root(), vec![make_sig("v1"), make_sig("v2")], 1, "cardano");
        assert!(proof.meets_quorum(2));
        assert!(!proof.meets_quorum(3));
    }

    // ── Test 6: block_height increments with each record ─────────────────────

    #[test]
    fn test_block_height_increments() {
        let mut store = ChainProofStore::new("cardano");
        store.record("b1", root(), vec![make_sig("v1")]).unwrap();
        store.record("b2", root(), vec![make_sig("v2")]).unwrap();
        let p1 = store.get("b1").unwrap();
        let p2 = store.get("b2").unwrap();
        assert!(p2.block_height > p1.block_height);
    }

    // ── Test 7: verify_all finds tampered records ─────────────────────────────

    #[test]
    fn test_verify_all_finds_tampered() {
        let mut store = ChainProofStore::new("cardano");
        store.record(BATCH, root(), vec![make_sig("v1")]).unwrap();
        // Tamper directly in the hashmap
        store.records.get_mut(BATCH).unwrap().merkle_root = "evil".to_string();
        let bad = store.verify_all();
        assert_eq!(bad.len(), 1, "One tampered record must be detected");
        assert_eq!(bad[0], BATCH);
    }

    // ── Test 8: proof hash length is 64 chars (SHA-256 hex) ──────────────────

    #[test]
    fn test_proof_hash_length() {
        let proof = BlockchainProof::new(BATCH, root(), vec![], 1, "cardano");
        assert_eq!(proof.proof_hash.len(), 64);
    }

    // ── Test 9: chain name stored correctly ───────────────────────────────────

    #[test]
    fn test_chain_name() {
        let mut store = ChainProofStore::new("hedera");
        let p = store.record(BATCH, root(), vec![make_sig("v1")]).unwrap();
        assert_eq!(p.chain, "hedera");
    }
}

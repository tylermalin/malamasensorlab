use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use crate::stage3_consensus::chain_proof::BlockchainProof;
use crate::stage5_integrity::lsh_engine::LshEngine;
use sha2::{Sha256, Digest};

/// Immutability Verification system for Prompt 33.
/// This connects the "Postcard" (blockchain) and the "Package" (IPFS).
pub struct ImmutabilityVerifier {
    /// Mock IPFS store: CID -> (Data, Expiry)
    pub ipfs_store: HashMap<String, (Vec<f64>, DateTime<Utc>)>,
    /// Mock Blockchain store: MerkleRoot -> BlockchainProof
    pub chain_store: HashMap<String, BlockchainProof>,
}

impl ImmutabilityVerifier {
    pub fn new() -> Self {
        Self {
            ipfs_store: HashMap::new(),
            chain_store: HashMap::new(),
        }
    }

    /// Uploads a batch to IPFS with a TTL for GDPR compliance.
    pub fn upload_batch(&mut self, readings: Vec<f64>, ttl_days: i64) -> String {
        let mut hasher = Sha256::new();
        for r in &readings {
            hasher.update(r.to_be_bytes());
        }
        let cid = format!("Qm{}", hex::encode(hasher.finalize()));
        let expiry = Utc::now() + Duration::days(ttl_days);
        
        self.ipfs_store.insert(cid.clone(), (readings, expiry));
        cid
    }

    /// Anchors the Merkle root on-chain (mock).
    pub fn anchor_root(&mut self, root: &str, proof: BlockchainProof) {
        self.chain_store.insert(root.to_string(), proof);
    }

    /// Full Verification Flow (Prompt 33):
    /// 1. Query blockchain for Merkle root.
    /// 2. Fetch data from IPFS.
    /// 3. Reconstruct Merkle tree (mocked by re-hashing content).
    /// 4. Compare roots.
    pub fn verify_immutability(&self, on_chain_root: &str, ipfs_cid: &str) -> VerificationResult {
        // Step 1: Check if root exists on-chain
        if !self.chain_store.contains_key(on_chain_root) {
            return VerificationResult::MissingOnChain;
        }

        // Step 2: Fetch from IPFS
        let (data, expiry) = match self.ipfs_store.get(ipfs_cid) {
            Some(entry) => entry,
            None => return VerificationResult::DataDeletedOrMissing,
        };

        // Check if data is expired (GDPR)
        if Utc::now() > *expiry {
            return VerificationResult::DataExpired;
        }

        // Step 3 & 4: Reconstruct root and compare
        let mut hasher = Sha256::new();
        for r in data {
            hasher.update(r.to_be_bytes());
        }
        let reconstructed_cid = format!("Qm{}", hex::encode(hasher.finalize()));

        if reconstructed_cid == ipfs_cid {
            VerificationResult::Success {
                fingerprint: LshEngine::compute_lsh_fingerprint(data),
            }
        } else {
            VerificationResult::Tampered
        }
    }

    /// Manually trigger GDPR deletion.
    pub fn purge_expired(&mut self) {
        let now = Utc::now();
        self.ipfs_store.retain(|_, (_, expiry)| *expiry > now);
    }
}

#[derive(Debug, PartialEq)]
pub enum VerificationResult {
    Success { fingerprint: [u8; 32] },
    MissingOnChain,
    DataDeletedOrMissing,
    DataExpired,
    Tampered,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_immutability_flow() {
        let mut verifier = ImmutabilityVerifier::new();
        let readings = vec![23.4, 23.5, 23.6];
        
        // 1. Storage
        let cid = verifier.upload_batch(readings.clone(), 1000);
        
        // 2. Consensus (Mock)
        let proof = BlockchainProof::new(
            "batch_001",
            &cid, // Merkle root
            vec![],
            100,
            "cardano",
        );
        verifier.anchor_root(&cid, proof);

        // 3. Verify
        let result = verifier.verify_immutability(&cid, &cid);
        assert!(matches!(result, VerificationResult::Success { .. }));
    }

    #[test]
    fn test_tamper_detection() {
        let mut verifier = ImmutabilityVerifier::new();
        let readings = vec![10.0, 20.0];
        let cid = verifier.upload_batch(readings, 1000);
        
        let proof = BlockchainProof::new(
            "batch_002",
            &cid,
            vec![],
            101,
            "cardano",
        );
        verifier.anchor_root(&cid, proof);

        // Tamper with data in IPFS
        if let Some((data, _)) = verifier.ipfs_store.get_mut(&cid) {
            data[0] = 99.9;
        }

        let result = verifier.verify_immutability(&cid, &cid);
        assert_eq!(result, VerificationResult::Tampered);
    }

    #[test]
    fn test_gdpr_expiration() {
        let mut verifier = ImmutabilityVerifier::new();
        let readings = vec![1.0, 2.0];
        // Expire immediately
        let cid = verifier.upload_batch(readings, -1);
        
        let proof = BlockchainProof::new(
            "batch_003",
            &cid,
            vec![],
            102,
            "cardano",
        );
        verifier.anchor_root(&cid, proof);

        let result = verifier.verify_immutability(&cid, &cid);
        assert_eq!(result, VerificationResult::DataExpired);
        
        verifier.purge_expired();
        assert_eq!(verifier.ipfs_store.len(), 0);
    }

    #[test]
    fn test_lsh_fingerprint_efficiency() {
        let readings1 = vec![23.4, 23.5, 23.6];
        let readings2 = vec![23.4, 23.5, 23.6, 23.7, 23.8]; // larger batch
        
        let fp1 = LshEngine::compute_lsh_fingerprint(&readings1);
        let fp2 = LshEngine::compute_lsh_fingerprint(&readings2);
        
        assert_eq!(fp1.len(), 32);
        assert_eq!(fp2.len(), 32);
        assert_ne!(fp1, fp2);
    }
}

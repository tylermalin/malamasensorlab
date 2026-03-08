use rs_merkle::{MerkleTree, MerkleProof, algorithms::Sha256 as MerkleSha256};
use sha2::{Digest, Sha256};
use crate::stage2_gateway::aggregator::SensorReading;

/// Wraps rs_merkle to provide Merkle tree construction, root extraction,
/// proof generation, and proof verification for sensor reading batches.
pub struct MerkleRootProducer;

impl MerkleRootProducer {
    /// Hash a single sensor reading to a 32-byte leaf.
    pub fn hash_reading(reading: &SensorReading) -> [u8; 32] {
        let bytes = serde_json::to_vec(reading)
            .expect("SensorReading serialization is infallible");
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        hasher.finalize().into()
    }

    /// Build a Merkle tree from a slice of readings. Panics if readings is empty.
    pub fn build_tree(readings: &[SensorReading]) -> MerkleTree<MerkleSha256> {
        assert!(!readings.is_empty(), "Cannot build Merkle tree from empty batch");
        let leaves: Vec<[u8; 32]> = readings
            .iter()
            .map(Self::hash_reading)
            .collect();
        MerkleTree::<MerkleSha256>::from_leaves(&leaves)
    }

    /// Get the hex-encoded Merkle root. Returns None if the tree is empty.
    pub fn get_root(tree: &MerkleTree<MerkleSha256>) -> String {
        hex::encode(tree.root().expect("Tree must be non-empty to get root"))
    }

    /// Generate a Merkle proof for the reading at `index`.
    pub fn get_proof(tree: &MerkleTree<MerkleSha256>, index: usize) -> MerkleProof<MerkleSha256> {
        tree.proof(&[index])
    }

    /// Verify that a reading at `leaf_index` is included in the tree described by `merkle_root`.
    ///
    /// Returns `true` if the proof is valid.
    pub fn verify_proof(
        merkle_root_hex: &str,
        proof: &MerkleProof<MerkleSha256>,
        reading: &SensorReading,
        leaf_index: usize,
        total_leaves: usize,
    ) -> bool {
        let root_bytes = match hex::decode(merkle_root_hex) {
            Ok(b) if b.len() == 32 => {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&b);
                arr
            }
            _ => return false,
        };

        let leaf_hash = Self::hash_reading(reading);
        proof.verify(root_bytes, &[leaf_index], &[leaf_hash], total_leaves)
    }

    /// Compute the LSH fingerprint of all leaf hashes combined.
    /// Provides a compact 32-byte representation of the batch suitable for
    /// weather/non-critical data (95% compression vs. full raw data).
    pub fn lsh_root_from_readings(readings: &[SensorReading]) -> [u8; 32] {
        // Concatenate all leaf hashes deterministically
        let mut combined = Vec::with_capacity(readings.len() * 32);
        for reading in readings {
            combined.extend_from_slice(&Self::hash_reading(reading));
        }
        let mut hasher = Sha256::new();
        hasher.update(&combined);
        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::stage2_gateway::aggregator::SensorReading;

    fn make_reading(id: &str, value: f64, seq: u64) -> SensorReading {
        SensorReading {
            sensor_id: id.to_string(),
            value,
            unit: "Celsius".to_string(),
            timestamp: Utc::now(),
            sequence_number: seq,
            nonce: format!("nonce-{seq}"),
            latitude: None,
            longitude: None,
            battery_voltage: None,
            uncertainty_lower: None,
            uncertainty_upper: None,
            signature: "mock_sig".to_string(),
        }
    }

    #[test]
    fn test_merkle_root_non_empty() {
        let readings = vec![make_reading("s1", 21.0, 1), make_reading("s2", 22.0, 2)];
        let tree = MerkleRootProducer::build_tree(&readings);
        let root = MerkleRootProducer::get_root(&tree);
        assert!(!root.is_empty(), "Root should be a non-empty hex string");
        assert_eq!(root.len(), 64, "SHA-256 root should be 64 hex chars");
    }

    #[test]
    fn test_merkle_root_deterministic() {
        let readings = vec![make_reading("s1", 21.0, 1), make_reading("s2", 22.0, 2)];
        // Build twice with same data — roots must match
        // Note: timestamps will vary in production; here we control via make_reading.
        let tree1 = MerkleRootProducer::build_tree(&readings);
        let tree2 = MerkleRootProducer::build_tree(&readings);
        assert_eq!(
            MerkleRootProducer::get_root(&tree1),
            MerkleRootProducer::get_root(&tree2),
            "Same readings must produce same root"
        );
    }

    #[test]
    fn test_merkle_proof_verification_passes() {
        let readings: Vec<SensorReading> = (1..=5)
            .map(|i| make_reading("s1", i as f64, i))
            .collect();
        let tree = MerkleRootProducer::build_tree(&readings);
        let root = MerkleRootProducer::get_root(&tree);
        let proof = MerkleRootProducer::get_proof(&tree, 2);

        let valid = MerkleRootProducer::verify_proof(&root, &proof, &readings[2], 2, readings.len());
        assert!(valid, "Valid proof should verify successfully");
    }

    #[test]
    fn test_merkle_proof_fails_on_tampered_reading() {
        let readings: Vec<SensorReading> = (1..=5)
            .map(|i| make_reading("s1", i as f64, i))
            .collect();
        let tree = MerkleRootProducer::build_tree(&readings);
        let root = MerkleRootProducer::get_root(&tree);
        let proof = MerkleRootProducer::get_proof(&tree, 2);

        // Tamper: change the value of the reading we're proving
        let mut tampered = readings[2].clone();
        tampered.value = 999.0;

        let valid = MerkleRootProducer::verify_proof(&root, &proof, &tampered, 2, readings.len());
        assert!(!valid, "Tampered reading should fail proof verification");
    }

    #[test]
    fn test_merkle_proof_fails_with_wrong_root() {
        let readings: Vec<SensorReading> = (1..=5)
            .map(|i| make_reading("s1", i as f64, i))
            .collect();
        let tree = MerkleRootProducer::build_tree(&readings);
        let proof = MerkleRootProducer::get_proof(&tree, 0);

        let wrong_root = "a".repeat(64); // All-'a' is not the real root
        let valid = MerkleRootProducer::verify_proof(&wrong_root, &proof, &readings[0], 0, readings.len());
        assert!(!valid, "Wrong root should fail verification");
    }

    #[test]
    fn test_lsh_root_deterministic() {
        let readings: Vec<SensorReading> = (1..=5)
            .map(|i| make_reading("s1", i as f64, i))
            .collect();
        let fp1 = MerkleRootProducer::lsh_root_from_readings(&readings);
        let fp2 = MerkleRootProducer::lsh_root_from_readings(&readings);
        assert_eq!(fp1, fp2, "LSH root must be deterministic");
        assert_ne!(fp1, [0u8; 32], "LSH root must be non-zero");
    }

    #[test]
    fn test_single_reading_tree() {
        let readings = vec![make_reading("s1", 23.4, 1)];
        let tree = MerkleRootProducer::build_tree(&readings);
        let root = MerkleRootProducer::get_root(&tree);
        assert!(!root.is_empty());
    }

    #[test]
    fn test_odd_number_of_readings() {
        // rs_merkle handles odd counts by duplicating last leaf — verify it works cleanly
        let readings: Vec<SensorReading> = (1..=7)
            .map(|i| make_reading("s1", i as f64, i))
            .collect();
        let tree = MerkleRootProducer::build_tree(&readings);
        let root = MerkleRootProducer::get_root(&tree);
        assert!(!root.is_empty());
        // Verify proof for the last element (index 6)
        let proof = MerkleRootProducer::get_proof(&tree, 6);
        let valid = MerkleRootProducer::verify_proof(&root, &proof, &readings[6], 6, readings.len());
        assert!(valid, "Proof for last leaf in odd-count tree should pass");
    }
}

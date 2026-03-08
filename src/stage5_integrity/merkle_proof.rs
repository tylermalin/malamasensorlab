use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};

/// Merkle Proof for a single reading (Prompt 34).
/// Allows an auditor to verify 23.4°C is in a batch without downloading 32KB of data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    pub reading: f64,
    pub index: usize,
    pub siblings: Vec<[u8; 32]>,
    pub root: [u8; 32],
}

impl MerkleProof {
    /// Verifies that the reading is part of the Merkle tree with the given root.
    pub fn verify(&self) -> bool {
        let mut current_hash = self.hash_reading();
        let mut current_index = self.index;

        for sibling in &self.siblings {
            if current_index % 2 == 0 {
                // Left child
                current_hash = Self::hash_pair(current_hash, *sibling);
            } else {
                // Right child
                current_hash = Self::hash_pair(*sibling, current_hash);
            }
            current_index /= 2;
        }

        current_hash == self.root
    }

    fn hash_reading(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.reading.to_be_bytes());
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    fn hash_pair(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(left);
        hasher.update(right);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Mock generator for testing. In production, this would be computed by the 
    /// Merkle Tree implementation in Stage 2.
    pub fn generate_mock(readings: &[f64], index: usize) -> Option<Self> {
        if index >= readings.len() { return None; }

        let mut current_level: Vec<[u8; 32]> = readings.iter().map(|r| {
            let mut hasher = Sha256::new();
            hasher.update(r.to_be_bytes());
            let mut h = [0u8; 32];
            h.copy_from_slice(&hasher.finalize());
            h
        }).collect();

        // Ensure power of 2 for simplicity in mock
        while !current_level.len().is_power_of_two() {
            current_level.push(*current_level.last().unwrap());
        }

        let mut siblings = Vec::new();
        let mut i = index;
        let mut level = current_level.clone();

        while level.len() > 1 {
            let sibling_index = if i % 2 == 0 { i + 1 } else { i - 1 };
            siblings.push(level[sibling_index]);
            
            let mut next_level = Vec::new();
            for chunk in level.chunks(2) {
                next_level.push(Self::hash_pair(chunk[0], chunk[1]));
            }
            level = next_level;
            i /= 2;
        }

        Some(Self {
            reading: readings[index],
            index,
            siblings,
            root: level[0],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_proof_verification() {
        let readings = vec![23.4, 23.5, 23.6, 23.7];
        let proof = MerkleProof::generate_mock(&readings, 1).unwrap();
        assert!(proof.verify());
    }

    #[test]
    fn test_invalid_reading_fails() {
        let readings = vec![23.4, 23.5];
        let mut proof = MerkleProof::generate_mock(&readings, 0).unwrap();
        proof.reading = 99.9; // Tamper
        assert!(!proof.verify());
    }

    #[test]
    fn test_invalid_sibling_fails() {
        let readings = vec![23.4, 23.5];
        let mut proof = MerkleProof::generate_mock(&readings, 0).unwrap();
        proof.siblings[0][0] ^= 0xFF; // Tamper
        assert!(!proof.verify());
    }
}

use rs_merkle::{MerkleTree, MerkleProof, algorithms::Sha256};
use sha2::{Digest, Sha256 as Sha2};
use crate::stage2_gateway::aggregator::SensorReading;

pub struct MerkleRootProducer {
    pub root: Option<String>,
}

impl MerkleRootProducer {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn build_tree(readings: &[SensorReading]) -> MerkleTree<Sha256> {
        let leaves: Vec<[u8; 32]> = readings
            .iter()
            .map(|r| {
                let bytes = serde_json::to_vec(r).unwrap();
                let mut hasher = Sha2::new();
                hasher.update(&bytes);
                let result = hasher.finalize();
                let mut leaf = [0u8; 32];
                leaf.copy_from_slice(&result);
                leaf
            })
            .collect();

        MerkleTree::<Sha256>::from_leaves(&leaves)
    }

    pub fn get_root(tree: &MerkleTree<Sha256>) -> String {
        hex::encode(tree.root().unwrap())
    }

    pub fn get_proof(tree: &MerkleTree<Sha256>, index: usize) -> MerkleProof<Sha256> {
        tree.proof(&[index])
    }
}

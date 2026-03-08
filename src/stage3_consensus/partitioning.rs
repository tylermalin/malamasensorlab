use sha2::{Sha256, Digest};
use std::collections::HashSet;

pub struct PartitionManager {
    pub all_nodes: Vec<String>,
    pub replication_factor: usize,
}

impl PartitionManager {
    pub fn new(nodes: Vec<String>, replication_factor: usize) -> Self {
        Self {
            all_nodes: nodes,
            replication_factor,
        }
    }

    /// Deterministically assigns a batch to a subset of nodes based on its ID.
    pub fn assign_nodes(&self, batch_id: &str) -> Vec<String> {
        if self.all_nodes.is_empty() {
            return vec![];
        }

        let mut assigned = HashSet::new();
        let mut attempt = 0;

        while assigned.len() < self.replication_factor && assigned.len() < self.all_nodes.len() {
            let mut hasher = Sha256::new();
            hasher.update(batch_id.as_bytes());
            hasher.update(attempt.to_string().as_bytes());
            let hash = hasher.finalize();

            // Convert leading hash bytes to an index
            let index = (u64::from_be_bytes(hash[0..8].try_into().unwrap()) % self.all_nodes.len() as u64) as usize;
            assigned.insert(self.all_nodes[index].clone());
            attempt += 1;
        }

        assigned.into_iter().collect()
    }
}

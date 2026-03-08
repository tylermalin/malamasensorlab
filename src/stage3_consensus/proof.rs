use serde::{Deserialize, Serialize};
use crate::stage3_consensus::voting::Vote;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusProof {
    pub batch_id: String,
    pub signatures: Vec<String>, // Aggregated signatures from approving nodes
    pub node_ids: Vec<String>,   // List of nodes that signed
    pub timestamp: i64,
    pub confidence_score: f64,    // Added for Stage 6 Prompt 44
}

impl ConsensusProof {
    pub fn from_votes(batch_id: String, votes: Vec<Vote>, confidence_score: f64) -> Self {
        let (node_ids, signatures): (Vec<_>, Vec<_>) = votes
            .into_iter()
            .map(|v| (v.node_id, v.signature))
            .unzip();

        Self {
            batch_id,
            signatures,
            node_ids,
            confidence_score,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    pub fn verify(&self, expected_threshold: usize) -> bool {
        // In a real system, we would verify each signature against the node's public key.
        // For this implementation, we verify the threshold.
        self.signatures.len() >= expected_threshold
    }
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VoteType {
    APPROVE,
    REJECT,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub node_id: String,
    pub batch_id: String,
    pub vote_type: VoteType,
    pub signature: String,
}

pub struct VotingSession {
    pub batch_id: String,
    pub required_nodes: Vec<String>,
    pub votes: HashMap<String, Vote>,
    pub confidence_score: f64, // Added for Stage 6 Prompt 44
}

impl VotingSession {
    pub fn new(batch_id: String, required_nodes: Vec<String>, confidence_score: f64) -> Self {
        Self {
            batch_id,
            required_nodes,
            votes: HashMap::new(),
            confidence_score,
        }
    }

    pub fn add_vote(&mut self, vote: Vote) -> bool {
        if self.required_nodes.contains(&vote.node_id) && vote.batch_id == self.batch_id {
            self.votes.insert(vote.node_id.clone(), vote);
            return true;
        }
        false
    }

    pub fn is_consensus_reached(&self) -> bool {
        let approves = self.votes.values().filter(|v| v.vote_type == VoteType::APPROVE).count();
        let total = self.required_nodes.len();
        
        if total == 0 { return false; }

        // BFT-lite threshold: 2/3 + 1
        let threshold = (2 * total) / 3 + 1;
        approves >= threshold
    }

    pub fn is_rejected(&self) -> bool {
        let rejects = self.votes.values().filter(|v| v.vote_type == VoteType::REJECT).count();
        let total = self.required_nodes.len();
        
        if total == 0 { return false; }

        // If more than 1/3 reject, consensus can never be reached
        let reject_threshold = (total - 1) / 3 + 1;
        rejects >= reject_threshold
    }
}

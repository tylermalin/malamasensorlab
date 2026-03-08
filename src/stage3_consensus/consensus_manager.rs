use serde::{Deserialize, Serialize};
use crate::stage3_consensus::voting::{VotingSession, Vote, VoteType};
use crate::stage3_consensus::proof::ConsensusProof;
use crate::stage3_consensus::partitioning::PartitionManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusState {
    PENDING,
    VOTING,
    COMMITTED,
    REJECTED,
}

pub struct ConsensusManager {
    pub state: ConsensusState,
    pub session: Option<VotingSession>,
    pub proof: Option<ConsensusProof>,
    pub partition_manager: PartitionManager,
}

impl ConsensusManager {
    pub fn new(nodes: Vec<String>, replication_factor: usize) -> Self {
        Self {
            state: ConsensusState::PENDING,
            session: None,
            proof: None,
            partition_manager: PartitionManager::new(nodes, replication_factor),
        }
    }

    pub fn start_consensus(&mut self, batch_id: String) {
        let assigned_nodes = self.partition_manager.assign_nodes(&batch_id);
        self.session = Some(VotingSession::new(batch_id, assigned_nodes));
        self.state = ConsensusState::VOTING;
    }

    pub fn handle_vote(&mut self, vote: Vote) {
        if let Some(session) = &mut self.session {
            if session.add_vote(vote) {
                if session.is_consensus_reached() {
                    let approves: Vec<Vote> = session.votes.values()
                        .filter(|v| v.vote_type == VoteType::APPROVE)
                        .cloned()
                        .collect();
                    
                    self.proof = Some(ConsensusProof::from_votes(session.batch_id.clone(), approves));
                    self.state = ConsensusState::COMMITTED;
                } else if session.is_rejected() {
                    self.state = ConsensusState::REJECTED;
                }
            }
        }
    }

    pub fn get_proof(&self) -> Option<ConsensusProof> {
        self.proof.clone()
    }
}

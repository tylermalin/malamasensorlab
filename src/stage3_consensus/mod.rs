pub mod partitioning;
pub mod voting;
pub mod proof;
pub mod consensus_manager;
pub mod quorum;
pub mod reputation;
pub mod graph_partitioning;
pub mod selector;
pub mod retry;
pub mod slashing;
pub mod chain_proof;
pub mod dispute;
pub mod health_monitor;

#[cfg(test)]
mod tests {
    use super::partitioning::*;
    use super::voting::*;
    use super::consensus_manager::*;

    #[test]
    fn test_node_assignment() {
        let nodes = vec!["Node A".to_string(), "Node B".to_string(), "Node C".to_string(), "Node D".to_string()];
        let manager = PartitionManager::new(nodes.clone(), 3);
        
        let batch_ids = vec!["batch_1", "batch_2", "batch_3"];
        for bid in batch_ids {
            let assigned = manager.assign_nodes(bid);
            assert_eq!(assigned.len(), 3);
            for node in &assigned {
                assert!(nodes.contains(node));
            }
        }
    }

    #[test]
    fn test_voting_consensus() {
        let nodes = vec!["N1".to_string(), "N2".to_string(), "N3".to_string()];
        let mut session = VotingSession::new("batch_1".to_string(), nodes);
        
        // Threshold for 3 is (2*3)/3 + 1 = 3
        session.add_vote(Vote {
            node_id: "N1".to_string(),
            batch_id: "batch_1".to_string(),
            vote_type: VoteType::APPROVE,
            signature: "sig1".to_string(),
        });
        assert!(!session.is_consensus_reached());

        session.add_vote(Vote {
            node_id: "N2".to_string(),
            batch_id: "batch_1".to_string(),
            vote_type: VoteType::APPROVE,
            signature: "sig2".to_string(),
        });
        assert!(!session.is_consensus_reached());

        session.add_vote(Vote {
            node_id: "N3".to_string(),
            batch_id: "batch_1".to_string(),
            vote_type: VoteType::APPROVE,
            signature: "sig3".to_string(),
        });
        assert!(session.is_consensus_reached());
    }

    #[test]
    fn test_consensus_manager_lifecycle() {
        let nodes = vec!["N1".to_string(), "N2".to_string(), "N3".to_string()];
        let mut manager = ConsensusManager::new(nodes, 3);
        
        manager.start_consensus("batch_678".to_string());
        assert_eq!(manager.state, ConsensusState::VOTING);

        // Send votes
        for i in 1..=3 {
            manager.handle_vote(Vote {
                node_id: format!("N{}", i),
                batch_id: "batch_678".to_string(),
                vote_type: VoteType::APPROVE,
                signature: format!("sig{}", i),
            });
        }

        assert_eq!(manager.state, ConsensusState::COMMITTED);
        assert!(manager.get_proof().is_some());
    }
}

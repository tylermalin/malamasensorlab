use crate::stage1_birth_identity::did_generator::DidDocument;
use crate::stage3_consensus::proof::ConsensusProof;

pub struct SecurityTester;

impl SecurityTester {
    /// P55: GPS Spoofing Detection
    /// Returns true if spoofing is detected.
    pub fn detect_gps_spoofing(claimed_lat: f64, claimed_lon: f64, signal_strength: f64) -> bool {
        // Mock logic: if signal is too weak for the claimed precision, or coordinates are impossible
        if signal_strength < 10.0 && (claimed_lat.abs() > 90.0 || claimed_lon.abs() > 180.0) {
            return true;
        }
        // Sudden jumps in distance (teleportation)
        false
    }

    /// P55: Replay Attack Prevention
    /// Checks if a batch signature has been seen before.
    pub fn is_replay_attack(batch_id: &str, seen_ids: &[String]) -> bool {
        seen_ids.contains(&batch_id.to_string())
    }

    /// P55: Sybil Attack Defense
    /// Checks if a group of nodes are likely the same entity.
    pub fn detect_sybil(node_ids: &[String], reputation_scores: &[(String, f64)]) -> bool {
        let low_rep_count = node_ids.iter().filter(|id| {
            reputation_scores.iter().find(|(rid, _)| rid == *id)
                .map_or(true, |(_, score)| *score < 0.2)
        }).count();

        // If more than 50% are new/low-rep nodes in a quorum, flag as potential Sybil
        low_rep_count > node_ids.len() / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_protection() {
        let seen = vec!["batch_1".to_string(), "batch_2".to_string()];
        assert!(SecurityTester::is_replay_attack("batch_1", &seen));
        assert!(!SecurityTester::is_replay_attack("batch_3", &seen));
    }

    #[test]
    fn test_gps_spoofing_simple() {
        assert!(SecurityTester::detect_gps_spoofing(100.0, 200.0, 5.0));
        assert!(!SecurityTester::detect_gps_spoofing(45.0, -75.0, 50.0));
    }
}

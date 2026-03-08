use sha2::{Sha256, Digest};

/// Content addressing and reorg protection logic (Prompts 36-37).
pub struct IntegrityCheck;

impl IntegrityCheck {
    /// Prompt 36: Verifies CID matches content.
    /// Re-hashes data and compares to provided CID string (Qm...).
    pub fn validate_cid_match(data: &[f64], cid: &str) -> bool {
        let mut hasher = Sha256::new();
        for r in data {
            let val: f64 = *r;
            hasher.update(val.to_be_bytes());
        }
        let computed_cid = format!("Qm{}", hex::encode(hasher.finalize()));
        computed_cid == cid
    }

    /// Prompt 37: Blockchain Fork Handling.
    /// Returns true if a block is deep enough to be considered final/safe.
    pub fn is_block_safe(current_height: u64, block_height: u64) -> bool {
        if block_height > current_height { return false; }
        let depth = current_height - block_height;
        // Narrative context: depth > 6 = safe
        depth >= 6
    }
    
    /// Prompt 37: Rollback logic (simulated).
    /// If a reorg is detected, we identifying the new common ancestor.
    pub fn detect_reorg(local_history: &[String], network_history: &[String]) -> Option<usize> {
        for (i, (local, network)) in local_history.iter().zip(network_history.iter()).enumerate() {
            if local != network {
                return Some(i); // Reorg starts at index i
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cid_validation() {
        let data = vec![23.4, 23.5];
        let mut hasher = Sha256::new();
        for r in &data {
            let val: f64 = *r;
            hasher.update(val.to_be_bytes());
        }
        let cid = format!("Qm{}", hex::encode(hasher.finalize()));
        
        assert!(IntegrityCheck::validate_cid_match(&data, &cid));
        assert!(!IntegrityCheck::validate_cid_match(&data, "QmTampered"));
    }

    #[test]
    fn test_reorg_protection() {
        let current = 1000;
        assert!(IntegrityCheck::is_block_safe(current, 990)); // Depth 10
        assert!(!IntegrityCheck::is_block_safe(current, 996)); // Depth 4 (too shallow)
    }

    #[test]
    fn test_reorg_detection() {
        let local = vec!["h1".into(), "h2".into(), "h3_bad".into()];
        let net = vec!["h1".into(), "h2".into(), "h3_good".into()];
        
        let split_index = IntegrityCheck::detect_reorg(&local, &net);
        assert_eq!(split_index, Some(2));
    }
}

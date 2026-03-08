use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::stage5_integrity::immutability::{ImmutabilityVerifier, VerificationResult};

/// Full audit trail for a single reading (Prompt 39).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub sensor_did: String,
    pub batch_id: String,
    pub timestamp: DateTime<Utc>,
    pub steps: HashMap<String, StepStatus>,
    pub overall_confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepStatus {
    pub verified: bool,
    pub details: String,
}

/// System for reconstructing audit trails and spot-checking integrity (Prompts 39-40).
pub struct AuditSystem {
    pub reports: Vec<AuditReport>,
    pub alerts: Vec<String>,
    pub quarantined_sensors: Vec<String>,
}

impl AuditSystem {
    pub fn new() -> Self {
        Self {
            reports: Vec::new(),
            alerts: Vec::new(),
            quarantined_sensors: Vec::new(),
        }
    }

    /// Prompt 39: Reconstructs the full chain of proof.
    pub fn generate_report(
        &mut self,
        sensor_did: &str,
        batch_id: &str,
        verifier: &ImmutabilityVerifier,
        root: &str,
        cid: &str,
    ) -> AuditReport {
        let mut steps = HashMap::new();
        
        // Step 1: Blockchain
        let on_chain = verifier.chain_store.contains_key(root);
        steps.insert("Blockchain Proof".to_string(), StepStatus {
            verified: on_chain,
            details: format!("Merkle root {} anchored on Cardano", root),
        });

        // Step 2 & 3: IPFS & Immutability
        let immutability = verifier.verify_immutability(root, cid);
        let (verified, details) = match immutability {
            VerificationResult::Success { .. } => (true, "Data matches on-chain root".to_string()),
            VerificationResult::Tampered => (false, "Data tampered in IPFS!".to_string()),
            _ => (false, "Verification failed".to_string()),
        };
        steps.insert("Immutability".to_string(), StepStatus { verified, details });

        let report = AuditReport {
            sensor_did: sensor_did.to_string(),
            batch_id: batch_id.to_string(),
            timestamp: Utc::now(),
            steps,
            overall_confidence: if verified { 0.95 } else { 0.0 },
        };
        
        self.reports.push(report.clone());
        report
    }

    /// Prompt 40: Periodic spot-checks.
    /// Simulates checking 1% of batches. If failed, alerts and quarantines.
    pub fn perform_spot_check(
        &mut self,
        sensor_did: &str,
        verifier: &ImmutabilityVerifier,
        root: &str,
        cid: &str,
    ) -> bool {
        let result = verifier.verify_immutability(root, cid);
        
        if !matches!(result, VerificationResult::Success { .. }) {
            let msg = format!("TAMPER DETECTED for sensor {} in batch {}. Result: {:?}", sensor_did, root, result);
            self.alerts.push(msg);
            self.quarantined_sensors.push(sensor_did.to_string());
            return false;
        }
        
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage3_consensus::chain_proof::BlockchainProof;

    #[test]
    fn test_audit_report_generation() {
        let mut audit = AuditSystem::new();
        let mut verifier = ImmutabilityVerifier::new();
        
        let cid = verifier.upload_batch(vec![23.4], 1000);
        let proof = BlockchainProof::new("b1", &cid, vec![], 100, "cardano");
        verifier.anchor_root(&cid, proof);

        let report = audit.generate_report("did:1", "b1", &verifier, &cid, &cid);
        assert!(report.steps["Blockchain Proof"].verified);
        assert!(report.steps["Immutability"].verified);
        assert_eq!(report.overall_confidence, 0.95);
    }

    #[test]
    fn test_spot_check_tamper_detection() {
        let mut audit = AuditSystem::new();
        let mut verifier = ImmutabilityVerifier::new();
        
        let cid = verifier.upload_batch(vec![10.0], 1000);
        let proof = BlockchainProof::new("b2", &cid, vec![], 101, "cardano");
        verifier.anchor_root(&cid, proof);

        // Tamper
        if let Some((data, _)) = verifier.ipfs_store.get_mut(&cid) {
            data[0] = 99.9;
        }

        let ok = audit.perform_spot_check("did:evil", &verifier, &cid, &cid);
        assert!(!ok);
        assert!(audit.alerts[0].contains("TAMPER DETECTED"));
        assert_eq!(audit.quarantined_sensors[0], "did:evil");
    }
}

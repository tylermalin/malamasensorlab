pub mod audit;
pub mod proof_generator;
pub mod explorer_logic;

#[cfg(test)]
mod tests {
    use super::audit::*;
    use super::proof_generator::*;
    use crate::stage1_birth_identity::did_generator::generate_sensor_did;
    use crate::stage3_consensus::proof::ConsensusProof;
    use crate::stage4_storage::chain_adapters::AnchorReceipt;

    #[test]
    fn test_full_verification_journey() {
        let did_res = generate_sensor_did("CO2", "Malama", 1.23, 4.56);
        let consensus_proof = ConsensusProof {
            batch_id: "batch_final".into(),
            signatures: vec!["s1".into(), "s2".into()],
            node_ids: vec!["n1".into(), "n2".into()],
            timestamp: 0,
            confidence_score: 1.0,
        };
        let anchors = vec![AnchorReceipt { chain: "Base".into(), tx_id: "tx1".into(), cid: "QmABC".into() }];
        
        let audit = AuditTrail {
            batch_id: "batch_final".into(),
            did_doc: did_res.doc,
            merkle_root: "merkle_root_hash".into(),
            consensus_proof,
            storage_anchors: anchors,
            settlement_receipts: vec![],
            timestamp: 123456789,
        };

        let proof = ProofGenerator::generate(audit, 0xABC);
        assert!(proof.verification_passed);
        assert_eq!(proof.status, "VERIFIED");
    }
}

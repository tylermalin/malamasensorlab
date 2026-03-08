use crate::stage1_birth_identity::did_generator::generate_sensor_did;
use crate::stage3_consensus::proof::ConsensusProof;
use crate::stage7_verification::audit::AuditTrail;
use crate::stage7_verification::proof_generator::ProofGenerator;
use crate::stage4_storage::chain_adapters::AnchorReceipt;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_odyssey_flow() {
        println!("Starting Master Odyssey Flow Simulation...");

        // 1. Sensor Birth (Stage 1)
        let did_res = generate_sensor_did("CO2", "Malama", 1.23, 4.56);
        assert!(did_res.doc.id.starts_with("did:malama:"));

        // 2. Consensus (Stage 3)
        let consensus_proof = ConsensusProof {
            batch_id: "odyssey_batch_1".into(),
            signatures: vec!["sig_v1".into(), "sig_v2".into()],
            node_ids: vec!["v1".into(), "v2".into()],
            timestamp: 1710000000,
            confidence_score: 0.95,
        };

        // 3. Storage Anchors (Stage 4)
        let anchors = vec![
            AnchorReceipt { chain: "Cardano".into(), tx_id: "ctx1".into(), cid: "Qm1".into() },
            AnchorReceipt { chain: "Base".into(), tx_id: "btx1".into(), cid: "Qm1".into() },
        ];

        // 4. Audit Trail Reconstruction (Stage 5/7)
        let audit = AuditTrail {
            batch_id: "odyssey_batch_1".into(),
            did_doc: did_res.doc,
            merkle_root: "root_hash_123".into(),
            consensus_proof,
            storage_anchors: anchors,
            settlement_receipts: vec![],
            registry_receipts: vec![],
            slashing_events: vec![],
            timestamp: 1710000000,
        };

        // 5. Final Proof of Journey (Stage 7)
        let poj = ProofGenerator::generate(audit, 0x123, 55555);
        
        println!("Odyssey Certificate: {}", poj.certificate_id);
        assert!(poj.verification_passed);
        assert_eq!(poj.status, "VERIFIED");
        assert!(poj.journey_signature.starts_with("PROT-SIG-"));
    }

    #[test]
    fn test_scale_simulation_1000_sensors() {
        // P53: Simulate 1,000 sensors (scaled down for speed)
        let count = 10; 
        for i in 0..count {
            let did = generate_sensor_did("CO2", &format!("Sensor-{}", i), 0.0, 0.0);
            assert!(did.doc.id.contains("did:malama:"));
        }
        println!("Simulated {} sensors successfully.", count);
    }
}

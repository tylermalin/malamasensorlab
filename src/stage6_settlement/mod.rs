pub mod token;
pub mod settlement_adapters;
pub mod settlement_manager;

#[cfg(test)]
mod tests {
    use super::token::*;
    use super::settlement_manager::*;
    use super::settlement_adapters::cardano::CardanoSettlementAdapter;
    use crate::stage3_consensus::proof::ConsensusProof;
    use crate::stage4_storage::chain_adapters::AnchorReceipt;

    use std::sync::Arc;

    #[test]
    fn test_token_calculation() {
        let token = CarbonToken::from_reading("batch_1".into(), 450.0, 400.0, 100000.0);
        // reduction = 50. (50 * 100000) / 1000000 = 5.0
        assert_eq!(token.amount, 5.0);
        assert_eq!(token.token_type, TokenType::VCO2);
    }

    #[tokio::test]
    async fn test_settlement_flow() {
        let mut manager = SettlementManager::new();
        manager.add_adapter(Arc::new(CardanoSettlementAdapter));

        let token = CarbonToken::from_reading("batch_1".into(), 450.0, 400.0, 100000.0);
        
        // Use generator for a valid mock document
        let did_res = crate::stage1_birth_identity::did_generator::generate_sensor_did(
            "CO2", "Malama", 0.0, 0.0
        );
        let did_doc = did_res.doc;

        let consensus_proof = ConsensusProof {
            batch_id: "batch_1".into(),
            signatures: vec!["s1".into(), "s2".into()],
            node_ids: vec!["n1".into(), "n2".into()],
            timestamp: 0,
        };
        let anchors = vec![AnchorReceipt { chain: "C".into(), tx_id: "t".into(), cid: "Qm".into() }];

        let results = manager.execute_settlement(&token, &did_doc, "root", &consensus_proof, &anchors).await.unwrap();
        
        assert_eq!(results.len(), 1);
        let receipt = results[0].as_ref().unwrap();
        assert_eq!(receipt.chain, "Cardano");
        assert!(receipt.tx_id.contains("settle_cardano"));
    }
}

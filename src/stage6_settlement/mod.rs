pub mod token;
pub mod settlement_adapters;
pub mod settlement_manager;
pub mod rewards;
pub mod market_settlement;
pub mod registry_report;
pub mod slashing;

#[cfg(test)]
mod tests {
    use super::token::*;
    use super::settlement_manager::*;
    use super::rewards::*;
    use super::market_settlement::*;
    use super::slashing::*;
    use super::settlement_adapters::cardano::CardanoSettlementAdapter;
    use crate::stage3_consensus::proof::ConsensusProof;
    use crate::stage4_storage::chain_adapters::AnchorReceipt;

    use std::sync::Arc;

    #[test]
    fn test_token_calculation() {
        let token = CarbonToken::mint("did:1", "batch_1", 450.0, 400.0, 100000.0, "QmMetadata");
        // reduction = 50. (50 * 100000) / 1_000_000 = 5.0
        assert_eq!(token.amount, 5.0);
        assert_eq!(token.token_type, TokenType::LCO2); // < 10.0 is LCO2
    }

    #[test]
    fn test_reward_payouts() {
        let payouts = RewardManager::calculate_payout("did:1", 5.0, 0.9);
        assert_eq!(payouts.len(), 2);
        assert_eq!(payouts[0].currency, "HBAR");
        assert_eq!(payouts[1].currency, "cUSD");
        assert_eq!(payouts[1].amount, 10.0); // 5 tons * $2
    }

    #[test]
    fn test_market_win() {
        let outcome = MarketSettlement::resolve_bet("bet_1", 10.0, 8.0);
        assert!(outcome.won);
        assert_eq!(outcome.payout, 1.5);
    }

    #[test]
    fn test_slashing() {
        let event = SlashingMechanism::slash_stake("did:bad", 2.0, "Tampering");
        assert_eq!(event.amount_slashed, 200.0);
    }

    #[tokio::test]
    async fn test_settlement_flow_approved() {
        let mut manager = SettlementManager::new();
        manager.add_adapter(Arc::new(CardanoSettlementAdapter));

        let token = CarbonToken::mint("did:1", "batch_1", 450.0, 400.0, 100000.0, "Qm");
        
        let did_res = crate::stage1_birth_identity::did_generator::generate_sensor_did(
            "CO2", "Malama", 0.0, 0.0
        );
        let did_doc = did_res.doc;

        let consensus_proof = ConsensusProof {
            batch_id: "batch_1".into(),
            signatures: vec!["s1".into(), "s2".into()],
            node_ids: vec!["n1".into(), "n2".into()],
            timestamp: 0,
            confidence_score: 0.9, // Passes gate
        };
        let anchors = vec![AnchorReceipt { chain: "C".into(), tx_id: "t".into(), cid: "Qm".into() }];

        let results = manager.execute_settlement(&token, &did_doc, "root", &consensus_proof, &anchors).await.unwrap();
        
        assert_eq!(results.len(), 1);
        let receipt = results[0].as_ref().unwrap();
        assert_eq!(receipt.chain, "Cardano");
    }

    #[tokio::test]
    async fn test_settlement_flow_rejected_confidence() {
        let manager = SettlementManager::new();
        let token = CarbonToken::mint("did:1", "batch_1", 450.0, 440.0, 100000.0, "Qm");
        let did_res = crate::stage1_birth_identity::did_generator::generate_sensor_did("C", "M", 0.0, 0.0);
        
        let consensus_proof = ConsensusProof {
            batch_id: "batch_1".into(),
            signatures: vec!["s1".into(), "s2".into()],
            node_ids: vec!["n1".into(), "n2".into()],
            timestamp: 0,
            confidence_score: 0.7, // Fails gate (< 0.8)
        };
        let anchors = vec![AnchorReceipt { chain: "C".into(), tx_id: "t".into(), cid: "Qm".into() }];

        let result = manager.execute_settlement(&token, &did_res.doc, "root", &consensus_proof, &anchors).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Confidence score"));
    }
}

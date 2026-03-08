use crate::stage6_settlement::token::CarbonToken;
use crate::stage6_settlement::settlement_adapters::{SettlementAdapter, SettlementReceipt};
use crate::stage5_integrity::verifier::IntegrityVerifier;
use crate::stage1_birth_identity::did_generator::DidDocument;
use crate::stage3_consensus::proof::ConsensusProof;
use crate::stage4_storage::chain_adapters::AnchorReceipt;
use std::sync::Arc;

pub struct SettlementManager {
    adapters: Vec<Arc<dyn SettlementAdapter + Send + Sync>>,
}

impl SettlementManager {
    pub fn new() -> Self {
        Self { adapters: Vec::new() }
    }

    pub fn add_adapter(&mut self, adapter: Arc<dyn SettlementAdapter + Send + Sync>) {
        self.adapters.push(adapter);
    }

    /// Orchestrates the final settlement of a verified data journey in parallel.
    pub async fn execute_settlement(
        &self,
        token: &CarbonToken,
        did_doc: &DidDocument,
        merkle_root: &str,
        consensus_proof: &ConsensusProof,
        anchors: &[AnchorReceipt],
    ) -> Result<Vec<Result<SettlementReceipt, String>>, String> {
        // 1. Final Integrity Check before minting
        if !IntegrityVerifier::verify_full_journey(did_doc, merkle_root, consensus_proof, anchors) {
            return Err("Integrity verification failed. Settlement aborted.".to_string());
        }

        // 2. Parallel multi-chain minting
        let mut handles = Vec::new();
        for adapter in &self.adapters {
            let adapter = Arc::clone(adapter);
            let batch_id = token.batch_id.clone();
            let amount = token.amount;
            
            let handle = tokio::spawn(async move {
                adapter.settle(&batch_id, amount)
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(res) => results.push(res),
                Err(e) => results.push(Err(format!("Task panic: {}", e))),
            }
        }

        Ok(results)
    }
}

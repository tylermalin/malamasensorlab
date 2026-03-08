use serde::{Deserialize, Serialize};
use crate::stage1_birth_identity::did_generator::DidDocument;
use crate::stage3_consensus::proof::ConsensusProof;
use crate::stage4_storage::chain_adapters::AnchorReceipt;
use crate::stage6_settlement::settlement_adapters::SettlementReceipt;
use crate::stage6_settlement::registry_report::RegistryReceipt;
use crate::stage6_settlement::slashing::SlashEvent;
use crate::stage5_integrity::verifier::IntegrityVerifier;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTrail {
    pub batch_id: String,
    pub did_doc: DidDocument,
    pub merkle_root: String,
    pub consensus_proof: ConsensusProof,
    pub storage_anchors: Vec<AnchorReceipt>,
    pub settlement_receipts: Vec<SettlementReceipt>,
    pub registry_receipts: Vec<RegistryReceipt>,
    pub slashing_events: Vec<SlashEvent>,
    pub timestamp: i64,
}

impl AuditTrail {
    pub fn verify(&self) -> bool {
        // Orchestrate the full verification using the IntegrityVerifier
        IntegrityVerifier::verify_full_journey(
            &self.did_doc,
            &self.merkle_root,
            &self.consensus_proof,
            &self.storage_anchors,
        )
    }
}

use crate::stage7_verification::audit::AuditTrail;
use crate::stage7_verification::proof_generator::{ProofGenerator, ProofOfJourney};
use serde_json;

pub struct ExplorerApi {
    pub trails: Vec<AuditTrail>,
}

impl ExplorerApi {
    pub fn new(trails: Vec<AuditTrail>) -> Self {
        Self { trails }
    }

    /// P52: Public Audit Query
    /// Returns a JSON string representing the full proof of journey.
    pub fn get_proof_of_journey(&self, batch_id: &str, fingerprint: u64, secret_key: u64) -> Result<String, String> {
        let trail = self.trails.iter()
            .find(|t| t.batch_id == batch_id)
            .ok_or_else(|| "Batch not found".to_string())?;

        let poj = ProofGenerator::generate(trail.clone(), fingerprint, secret_key);
        
        serde_json::to_string_pretty(&poj)
            .map_err(|e| format!("Serialization error: {}", e))
    }

    /// Enterprise buyer view: filter by registry
    pub fn get_registry_credits(&self, registry: &str) -> Vec<AuditTrail> {
        self.trails.iter()
            .filter(|t| t.registry_receipts.iter().any(|r| r.registry_name == registry))
            .cloned()
            .collect()
    }
}

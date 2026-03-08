use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryReceipt {
    pub registry_name: String, // "Verra" or "Gold Standard"
    pub submission_id: String,
    pub status: String,
    pub submitted_at: DateTime<Utc>,
}

/// Mock integration for international carbon registries (Prompt 47).
pub struct RegistryReporter;

impl RegistryReporter {
    pub fn report_to_verra(token_id: &str, _amount: f64) -> RegistryReceipt {
        RegistryReceipt {
            registry_name: "Verra".to_string(),
            submission_id: format!("VERRA-SUB-{}", token_id),
            status: "Pending Verification".to_string(),
            submitted_at: Utc::now(),
        }
    }

    pub fn report_to_gold_standard(token_id: &str, _amount: f64) -> RegistryReceipt {
        RegistryReceipt {
            registry_name: "Gold Standard".to_string(),
            submission_id: format!("GS-SUB-{}", token_id),
            status: "Awaiting Audit".to_string(),
            submitted_at: Utc::now(),
        }
    }
}

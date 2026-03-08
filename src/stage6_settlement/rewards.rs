use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardPayout {
    pub recipient_did: String,
    pub amount: f64,
    pub currency: String, // "HBAR" or "cUSD"
    pub reason: String,
}

/// Manages DePIN reward distribution (Prompt 46).
pub struct RewardManager;

impl RewardManager {
    /// Distributes rewards based on sensor data quality and quantity.
    pub fn calculate_payout(sensor_did: &str, carbon_amount: f64, confidence: f64) -> Vec<RewardPayout> {
        let mut payouts = Vec::new();
        
        // Base reward in HBAR for participating
        payouts.push(RewardPayout {
            recipient_did: sensor_did.to_string(),
            amount: 10.0 * confidence, // Weighted by quality
            currency: "HBAR".to_string(),
            reason: "Data provider reward".to_string(),
        });

        // Carbon bonus in cUSD for successful carbon reduction
        if carbon_amount > 0.0 {
            payouts.push(RewardPayout {
                recipient_did: sensor_did.to_string(),
                amount: carbon_amount * 2.0, // $2 per ton CO2e
                currency: "cUSD".to_string(),
                reason: "Carbon reduction bonus".to_string(),
            });
        }

        payouts
    }
}

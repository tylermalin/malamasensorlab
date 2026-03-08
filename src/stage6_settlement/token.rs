use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TokenType {
    LCO2, // Local Carbon Offset (Small projects, local impact)
    VCO2, // Voluntary Carbon Offset (Large scale, international)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonToken {
    pub token_id: String,
    pub token_type: TokenType,
    pub amount: f64, // In metric tons of CO2 equivalent
    pub batch_id: String,
    pub sensor_did: String,
    pub minted_at: DateTime<Utc>,
    pub maturity_date: DateTime<Utc>, // For forward-financing (Prompt 41)
    pub project_vintage: u32,
    pub metadata_cid: String, // IPFS link to full project audit
}

impl CarbonToken {
    /// Calculates carbon credits based on sensor data (Prompt 41-43).
    pub fn mint(
        sensor_did: &str,
        batch_id: &str,
        baseline_ppm: f64,
        actual_ppm: f64,
        volume_m3: f64,
        metadata_cid: &str,
    ) -> Self {
        let reduction = (baseline_ppm - actual_ppm).max(0.0);
        let amount = (reduction * volume_m3) / 1_000_000.0;
        
        // Narrative logic: large projects (>10 tons) become VCO2, smaller are LCO2
        let token_type = if amount > 10.0 { TokenType::VCO2 } else { TokenType::LCO2 };
        let now = Utc::now();

        Self {
            token_id: format!("CO2-{}-{}", batch_id, now.timestamp()),
            token_type,
            amount,
            batch_id: batch_id.to_string(),
            sensor_did: sensor_did.to_string(),
            minted_at: now,
            maturity_date: now + Duration::days(365), // 1-year forward-financing logic
            project_vintage: now.format("%Y").to_string().parse().unwrap_or(2024),
            metadata_cid: metadata_cid.to_string(),
        }
    }
}

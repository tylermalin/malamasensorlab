use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TokenType {
    LCO2, // Local Carbon Offset
    VCO2, // Voluntary Carbon Offset
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonToken {
    pub token_type: TokenType,
    pub amount: f64, // In metric tons of CO2 equivalent
    pub batch_id: String,
    pub timestamp: i64,
}

impl CarbonToken {
    /// Calculates carbon credits based on sensor data.
    /// Formula: (Baseline - Actual) * Volume / Constant
    /// For this version, we use a simplified model.
    pub fn from_reading(batch_id: String, baseline_ppm: f64, actual_ppm: f64, volume_m3: f64) -> Self {
        let reduction = (baseline_ppm - actual_ppm).max(0.0);
        // Conversion factor: simplified 1 ppm reduction in 1000m3 = 0.001 tons CO2e
        let amount = (reduction * volume_m3) / 1_000_000.0;
        
        let token_type = if amount > 1.0 { TokenType::VCO2 } else { TokenType::LCO2 };

        Self {
            token_type,
            amount,
            batch_id,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

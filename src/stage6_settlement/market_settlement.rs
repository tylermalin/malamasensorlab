use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketOutcome {
    pub bet_id: String,
    pub won: bool,
    pub payout: f64,
}

/// Resolves prediction markets based on environmental data (Prompt 45).
pub struct MarketSettlement;

impl MarketSettlement {
    /// Resolves a bet on whether a target carbon reduction was met.
    pub fn resolve_bet(bet_id: &str, actual_amount: f64, target_amount: f64) -> MarketOutcome {
        let won = actual_amount >= target_amount;
        let payout = if won { 1.5 } else { 0.0 }; // Simple 1.5x multiplier for win

        MarketOutcome {
            bet_id: bet_id.to_string(),
            won,
            payout,
        }
    }
}

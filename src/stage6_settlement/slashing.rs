use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashEvent {
    pub entity_did: String,
    pub amount_slashed: f64,
    pub reason: String,
}

/// Implementation of slashing mechanism for dishonest actors (Prompt 48).
pub struct SlashingMechanism;

impl SlashingMechanism {
    /// Slashes a sensor or validator's stake if they are caught tampering.
    pub fn slash_stake(entity_did: &str, severity: f64, reason: &str) -> SlashEvent {
        // Base slash amount: $100 * severity multiplier
        let slash_amount = 100.0 * severity;

        SlashEvent {
            entity_did: entity_did.to_string(),
            amount_slashed: slash_amount,
            reason: reason.to_string(),
        }
    }
}

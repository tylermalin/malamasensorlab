use serde::{Deserialize, Serialize};

pub mod cardano;
pub mod evm;
pub mod hedera;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementReceipt {
    pub chain: String,
    pub tx_id: String,
    pub token_id: String,
    pub amount: f64,
}

pub trait SettlementAdapter: Send + Sync {
    fn chain_name(&self) -> &str;
    fn settle(&self, batch_id: &str, amount: f64) -> Result<SettlementReceipt, String>;
}

use serde::{Deserialize, Serialize};

pub mod cardano;
pub mod base;
pub mod hedera;
pub mod celo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorReceipt {
    pub chain: String,
    pub tx_id: String,
    pub cid: String,
}

pub trait ChainAdapter {
    fn chain_name(&self) -> &str;
    fn anchor(&self, cid: &str) -> Result<AnchorReceipt, String>;
}

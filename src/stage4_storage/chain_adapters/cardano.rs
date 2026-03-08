use crate::stage4_storage::chain_adapters::{ChainAdapter, AnchorReceipt};

pub struct CardanoAdapter;

impl ChainAdapter for CardanoAdapter {
    fn chain_name(&self) -> &str {
        "Cardano"
    }

    fn anchor(&self, cid: &str) -> Result<AnchorReceipt, String> {
        // Mock transaction ID for Cardano
        let tx_id = format!("tx_cardano_{}", uuid::Uuid::new_v4());
        Ok(AnchorReceipt {
            chain: self.chain_name().to_string(),
            tx_id,
            cid: cid.to_string(),
        })
    }
}

use crate::stage4_storage::chain_adapters::{ChainAdapter, AnchorReceipt};

pub struct HederaAdapter;

impl ChainAdapter for HederaAdapter {
    fn chain_name(&self) -> &str {
        "Hedera"
    }

    fn anchor(&self, cid: &str) -> Result<AnchorReceipt, String> {
        // Mock transaction ID for Hedera
        let tx_id = format!("tx_hedera_{}", uuid::Uuid::new_v4());
        Ok(AnchorReceipt {
            chain: self.chain_name().to_string(),
            tx_id,
            cid: cid.to_string(),
        })
    }
}

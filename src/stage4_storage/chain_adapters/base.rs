use crate::stage4_storage::chain_adapters::{ChainAdapter, AnchorReceipt};

pub struct BaseAdapter;

impl ChainAdapter for BaseAdapter {
    fn chain_name(&self) -> &str {
        "BASE"
    }

    fn anchor(&self, cid: &str) -> Result<AnchorReceipt, String> {
        // Mock transaction ID for BASE
        let tx_id = format!("tx_base_{}", uuid::Uuid::new_v4());
        Ok(AnchorReceipt {
            chain: self.chain_name().to_string(),
            tx_id,
            cid: cid.to_string(),
        })
    }
}

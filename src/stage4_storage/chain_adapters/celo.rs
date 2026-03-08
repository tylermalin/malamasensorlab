use crate::stage4_storage::chain_adapters::{ChainAdapter, AnchorReceipt};

pub struct CeloAdapter;

impl ChainAdapter for CeloAdapter {
    fn chain_name(&self) -> &str {
        "CELO"
    }

    fn anchor(&self, cid: &str) -> Result<AnchorReceipt, String> {
        // Mock transaction ID for Celo
        let tx_id = format!("tx_celo_{}", uuid::Uuid::new_v4());
        Ok(AnchorReceipt {
            chain: self.chain_name().to_string(),
            tx_id,
            cid: cid.to_string(),
        })
    }
}

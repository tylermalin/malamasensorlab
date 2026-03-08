use crate::stage6_settlement::settlement_adapters::{SettlementAdapter, SettlementReceipt};

pub struct EvmSettlementAdapter {
    pub chain: String,
}

impl SettlementAdapter for EvmSettlementAdapter {
    fn chain_name(&self) -> &str {
        &self.chain
    }

    fn settle(&self, batch_id: &str, amount: f64) -> Result<SettlementReceipt, String> {
        let tx_id = format!("0x{}", hex::encode(uuid::Uuid::new_v4().as_bytes()));
        let end = batch_id.len().min(8);
        let token_id = format!("0x_vco2_contract_{}", &batch_id[..end]);
        
        Ok(SettlementReceipt {
            chain: self.chain_name().to_string(),
            tx_id,
            token_id,
            amount,
        })
    }
}

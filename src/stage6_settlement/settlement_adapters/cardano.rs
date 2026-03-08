use crate::stage6_settlement::settlement_adapters::{SettlementAdapter, SettlementReceipt};

pub struct CardanoSettlementAdapter;

impl SettlementAdapter for CardanoSettlementAdapter {
    fn chain_name(&self) -> &str {
        "Cardano"
    }

    fn settle(&self, batch_id: &str, amount: f64) -> Result<SettlementReceipt, String> {
        let tx_id = format!("settle_cardano_{}", uuid::Uuid::new_v4());
        let end = batch_id.len().min(8);
        let token_id = format!("asset1_lco2_{}", &batch_id[..end]);
        
        Ok(SettlementReceipt {
            chain: self.chain_name().to_string(),
            tx_id,
            token_id,
            amount,
        })
    }
}

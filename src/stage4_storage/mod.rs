pub mod ipfs_adapter;
pub mod chain_adapters;
pub mod storage_manager;

#[cfg(test)]
mod tests {
    use super::storage_manager::StorageManager;
    use super::chain_adapters::cardano::CardanoAdapter;
    use super::chain_adapters::base::BaseAdapter;
    use serde_json::json;

    #[tokio::test]
    async fn test_storage_orchestration() {
        let mut manager = StorageManager::new("https://ipfs.infura.io:5001");
        manager.add_adapter(Box::new(CardanoAdapter));
        manager.add_adapter(Box::new(BaseAdapter));

        let content = json!({
            "batch_id": "test_batch_123",
            "data": "verified sensor reading"
        });

        let receipts = manager.store_and_anchor(&content).await.unwrap();
        
        // Should have 2 receipts (Cardano and Base)
        assert_eq!(receipts.len(), 2);
        assert!(receipts[0].cid.starts_with("Qm"));
        assert!(receipts[1].cid.starts_with("Qm"));
        assert!(receipts[0].tx_id.contains("cardano"));
        assert!(receipts[1].tx_id.contains("base"));
    }
}

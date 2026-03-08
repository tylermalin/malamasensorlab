use crate::stage4_storage::ipfs_adapter::IpfsAdapter;
use crate::stage4_storage::chain_adapters::{ChainAdapter, AnchorReceipt};
use serde::Serialize;
use std::error::Error;

pub struct StorageManager {
    ipfs: IpfsAdapter,
    adapters: Vec<Box<dyn ChainAdapter>>,
}

impl StorageManager {
    pub fn new(ipfs_gateway: &str) -> Self {
        Self {
            ipfs: IpfsAdapter::new(ipfs_gateway),
            adapters: Vec::new(),
        }
    }

    pub fn add_adapter(&mut self, adapter: Box<dyn ChainAdapter>) {
        self.adapters.push(adapter);
    }

    /// Primary orchestration flow: Off-chain IPFS upload followed by Multi-chain Anchoring.
    pub async fn store_and_anchor<T: Serialize>(&self, content: &T) -> Result<Vec<AnchorReceipt>, Box<dyn Error>> {
        // 1. Store off-chain in IPFS
        let cid = self.ipfs.upload(content).await?;
        
        // 2. Anchor the CID on all registered blockchains
        let mut receipts = Vec::new();
        for adapter in &self.adapters {
            if let Ok(receipt) = adapter.anchor(&cid) {
                receipts.push(receipt);
            }
        }
        
        Ok(receipts)
    }
}

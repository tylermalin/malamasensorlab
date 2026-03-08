use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::error::Error;

pub struct IpfsAdapter {
    gateway_url: String,
    client: Client,
}

impl IpfsAdapter {
    pub fn new(gateway_url: &str) -> Self {
        Self {
            gateway_url: gateway_url.to_string(),
            client: Client::new(),
        }
    }

    /// Uploads content to IPFS. In a real-world scenario, this would use a pinning service API.
    /// For this implementation, we simulate the CID generation.
    pub async fn upload<T: Serialize>(&self, content: &T) -> Result<String, Box<dyn Error>> {
        let _json = serde_json::to_string(content)?;
        
        // Simulation of IPFS upload: 
        // We'd typically POST to a service like Pinata or a local IPFS node.
        // Instead, we deterministicly generate a 'CID' based on the content hash.
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(serde_json::to_vec(content)?);
        let result = hasher.finalize();
        
        // Return a mock CID (base58 encoded hash)
        let cid = format!("Qm{}", bs58::encode(result).into_string());
        Ok(cid)
    }
}

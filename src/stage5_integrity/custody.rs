use serde::{Deserialize, Serialize};
use crate::stage1_birth_identity::did_generator::DidDocument;
use k256::ecdsa::SigningKey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyLink {
    pub handler_did: String,
    pub input_hash: String,
    pub output_hash: String,
    pub timestamp: i64,
    pub signature: String,
}

pub struct ChainOfCustody {
    pub links: Vec<CustodyLink>,
}

impl ChainOfCustody {
    pub fn new() -> Self {
        Self { links: Vec::new() }
    }

    pub fn add_link(
        &mut self,
        handler_did: String,
        input_hash: String,
        output_hash: String,
        _private_key: &SigningKey,
    ) {
        let timestamp = chrono::Utc::now().timestamp();
        let _payload = format!("{}{}{}{}", handler_did, input_hash, output_hash, timestamp);
        
        // Reuse signing logic (mocked or actual signature)
        // For simplicity, we'll store a mock signature here or use the sign_challenge pattern
        let signature = hex::encode(format!("sig_{}", uuid::Uuid::new_v4())); 

        self.links.push(CustodyLink {
            handler_did,
            input_hash,
            output_hash,
            timestamp,
            signature,
        });
    }

    pub fn verify_chain(&self) -> bool {
        for i in 1..self.links.len() {
            if self.links[i].input_hash != self.links[i-1].output_hash {
                return false;
            }
        }
        true
    }
}

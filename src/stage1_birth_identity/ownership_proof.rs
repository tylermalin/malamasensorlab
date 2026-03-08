use k256::ecdsa::{SigningKey, VerifyingKey, Signature, signature::Signer, signature::Verifier};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Challenge {
    pub nonce: String,
    pub timestamp: DateTime<Utc>,
}

pub fn create_challenge() -> Challenge {
    let nonce: String = (0..16)
        .map(|_| rand::random::<u8>())
        .map(|b| format!("{:02x}", b))
        .collect();
    
    Challenge {
        nonce,
        timestamp: Utc::now(),
    }
}

pub fn sign_challenge(challenge: &Challenge, private_key: &SigningKey) -> String {
    let message = format!("{}{}", challenge.nonce, challenge.timestamp.to_rfc3339());
    let signature: Signature = private_key.sign(message.as_bytes());
    hex::encode(signature.to_bytes())
}

pub fn verify_signature(challenge: &Challenge, signature_hex: &str, public_key: &VerifyingKey) -> bool {
    let message = format!("{}{}", challenge.nonce, challenge.timestamp.to_rfc3339());
    let signature_bytes = match hex::decode(signature_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    
    let signature = match Signature::from_slice(&signature_bytes) {
        Ok(s) => s,
        Err(_) => return false,
    };
    
    public_key.verify(message.as_bytes(), &signature).is_ok()
}

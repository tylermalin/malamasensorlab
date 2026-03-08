pub mod did_generator;
pub mod ownership_proof;
pub mod sensor_state;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub sensor_id: String,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub signature: String, // Signature of (sensor_id, value, timestamp)
}

#[cfg(test)]
mod tests {
    use super::did_generator::*;
    use super::ownership_proof::*;
    use super::sensor_state::*;
    use super::sensor_state::*;
    use std::collections::HashSet;
    use k256::ecdsa::signature::Signer;
    use chrono::Utc;

    #[test]
    fn test_did_generation() {
        let result = generate_sensor_did("temperature", "Tropic Square", 43.8, -115.9);
        assert!(result.did.starts_with("did:cardano:sensor:"));
        assert_eq!(result.doc.metadata.sensor_type, "temperature");
    }

    #[test]
    fn test_ownership_proof() {
        let result = generate_sensor_did("temperature", "Tropic Square", 43.8, -115.9);
        let challenge = create_challenge();
        let signature = sign_challenge(&challenge, &result.private_key);
        let verifying_key = k256::ecdsa::VerifyingKey::from(&result.private_key);
        assert!(verify_signature(&challenge, &signature, &verifying_key));
    }

    #[test]
    fn test_did_uniqueness() {
        let mut dids = HashSet::new();
        for _ in 0..100 {
            let result = generate_sensor_did("temperature", "Tropic Square", 43.8, -115.9);
            assert!(dids.insert(result.did));
        }
        assert_eq!(dids.len(), 100);
    }

    #[test]
    fn test_state_transitions() {
        let result = generate_sensor_did("temperature", "Tropic Square", 43.8, -115.9);
        let mut sensor = Sensor::new(result.did);
        let verifying_key = k256::ecdsa::VerifyingKey::from(&result.private_key);
        
        // Generate a valid signature for registration
        let now = Utc::now();
        let message = format!("{:?}{:?}{}", SensorState::UNREGISTERED, SensorState::REGISTERED, now.to_rfc3339());
        let sig: k256::ecdsa::Signature = result.private_key.sign(message.as_bytes());
        let sig_hex = hex::encode(sig.to_bytes());
        
        let res = sensor.register(verifying_key, now, sig_hex);
        assert!(res.is_ok());
        assert_eq!(sensor.state(), SensorState::REGISTERED);
    }
}

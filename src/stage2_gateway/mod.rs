pub mod aggregator;
pub mod merkle_tree;
pub mod wal;
pub mod node;

#[cfg(test)]
mod tests {
    use super::aggregator::*;
    use super::merkle_tree::*;
    use super::node::*;
    use chrono::Utc;

    #[test]
    fn test_batch_aggregation() {
        let mut aggregator = BatchAggregator::new(60);
        let reading = SensorReading {
            sensor_id: "did:cardano:sensor:123".to_string(),
            value: 22.5,
            timestamp: Utc::now(),
            signature: "sig".to_string(),
        };
        aggregator.add_reading(reading);
        assert_eq!(aggregator.current_batch.len(), 1);
    }

    #[test]
    fn test_merkle_root() {
        let readings = vec![
            SensorReading {
                sensor_id: "s1".to_string(),
                value: 21.0,
                timestamp: Utc::now(),
                signature: "sig1".to_string(),
            },
            SensorReading {
                sensor_id: "s2".to_string(),
                value: 22.0,
                timestamp: Utc::now(),
                signature: "sig2".to_string(),
            },
        ];

        let tree = MerkleRootProducer::build_tree(&readings);
        let root = MerkleRootProducer::get_root(&tree);
        assert!(!root.is_empty());
    }

    #[test]
    fn test_gateway_cycle() {
        use k256::ecdsa::{SigningKey, signature::Signer};
        use rand::rngs::OsRng;
        
        let mut gateway = GatewayNode::new(0, "/tmp/test_wal.log");
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = k256::ecdsa::VerifyingKey::from(&signing_key);
        let sensor_id = "did:malama:sensor:1".to_string();
        
        gateway.register_sensor(sensor_id.clone(), verifying_key);

        let timestamp = Utc::now();
        let value = 21.0;
        let message = format!("{}{}{}", sensor_id, value, timestamp.to_rfc3339());
        let sig: k256::ecdsa::Signature = signing_key.sign(message.as_bytes());
        let sig_hex = hex::encode(sig.to_bytes());

        let reading = SensorReading {
            sensor_id,
            value,
            timestamp,
            signature: sig_hex,
        };
        
        let res = gateway.receive_reading(reading);
        assert!(res.is_ok(), "Failed to receive reading: {:?}", res.err());
        
        let batch = gateway.process_cycle();
        assert!(batch.is_some());
        assert_eq!(gateway.state, GatewayState::BROADCASTING);
        assert!(gateway.current_merkle_root.is_some());

        gateway.confirm_broadcast();
        assert_eq!(gateway.state, GatewayState::COLLECTING);
    }
}

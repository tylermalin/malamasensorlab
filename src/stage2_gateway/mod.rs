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
    use k256::ecdsa::{SigningKey, signature::Signer};
    use rand::rngs::OsRng;

    // ──────────────────────────────────────────────
    // Helpers
    // ──────────────────────────────────────────────

    fn make_reading(id: &str, value: f64, seq: u64) -> SensorReading {
        SensorReading {
            sensor_id: id.to_string(),
            value,
            unit: "Celsius".to_string(),
            timestamp: Utc::now(),
            sequence_number: seq,
            nonce: format!("nonce-{seq}"),
            latitude: Some(43.8),
            longitude: Some(-115.9),
            battery_voltage: Some(4.2),
            uncertainty_lower: Some(value - 0.3),
            uncertainty_upper: Some(value + 0.3),
            signature: "mock_sig".to_string(),
        }
    }

    fn make_signed_reading(
        signing_key: &SigningKey,
        sensor_id: &str,
        value: f64,
        seq: u64,
    ) -> SensorReading {
        let timestamp = Utc::now();
        let nonce = format!("nonce-{seq}");
        let message = format!(
            "{}{}{}{}", sensor_id, value, timestamp.to_rfc3339(), nonce
        );
        let sig: k256::ecdsa::Signature = signing_key.sign(message.as_bytes());
        SensorReading {
            sensor_id: sensor_id.to_string(),
            value,
            unit: "Celsius".to_string(),
            timestamp,
            sequence_number: seq,
            nonce,
            latitude: None,
            longitude: None,
            battery_voltage: None,
            uncertainty_lower: None,
            uncertainty_upper: None,
            signature: hex::encode(sig.to_bytes()),
        }
    }

    // ──────────────────────────────────────────────
    // Aggregator tests
    // ──────────────────────────────────────────────

    #[test]
    fn test_batch_aggregation_basic() {
        let mut aggregator = BatchAggregator::new(3600, 100);
        let reading = make_reading("did:cardano:sensor:123", 22.5, 1);
        let accepted = aggregator.add_reading(reading);
        assert!(accepted);
        assert_eq!(aggregator.current_batch.len(), 1);
    }

    #[test]
    fn test_volume_threshold_triggers_seal() {
        let mut agg = BatchAggregator::new(3600, 3);
        for i in 0..3 {
            agg.add_reading(make_reading("s1", i as f64, i));
        }
        assert_eq!(agg.should_seal(), Some(SealReason::VolumeThreshold));
        let batch = agg.seal_batch().expect("Should produce a batch");
        assert_eq!(batch.readings.len(), 3);
        assert!(!batch.lsh_fingerprint.is_empty());
    }

    #[test]
    fn test_deduplication_rejects_exact_duplicate() {
        let mut agg = BatchAggregator::new(3600, 100);
        let r = make_reading("s1", 21.0, 1);
        assert!(agg.add_reading(r.clone()));
        assert!(!agg.add_reading(r), "Duplicate reading must be rejected");
        assert_eq!(agg.current_batch.len(), 1);
    }

    #[test]
    fn test_force_seal_on_small_batch() {
        let mut agg = BatchAggregator::new(3600, 100);
        agg.add_reading(make_reading("s1", 21.0, 1));
        let batch = agg.force_seal().expect("Force seal must succeed with 1 reading");
        assert_eq!(batch.readings.len(), 1);
        assert!(agg.current_batch.is_empty());
    }

    // ──────────────────────────────────────────────
    // Merkle tree tests
    // ──────────────────────────────────────────────

    #[test]
    fn test_merkle_root_produced() {
        let readings: Vec<SensorReading> = (1..=4)
            .map(|i| make_reading("s1", i as f64, i))
            .collect();
        let tree = MerkleRootProducer::build_tree(&readings);
        let root = MerkleRootProducer::get_root(&tree);
        assert!(!root.is_empty());
        assert_eq!(root.len(), 64);
    }

    #[test]
    fn test_merkle_proof_valid() {
        let readings: Vec<SensorReading> = (1..=5)
            .map(|i| make_reading("s1", i as f64, i))
            .collect();
        let tree = MerkleRootProducer::build_tree(&readings);
        let root = MerkleRootProducer::get_root(&tree);
        let proof = MerkleRootProducer::get_proof(&tree, 2);
        assert!(
            MerkleRootProducer::verify_proof(&root, &proof, &readings[2], 2, readings.len()),
            "Valid proof should verify"
        );
    }

    #[test]
    fn test_merkle_proof_fails_on_tampered_value() {
        let readings: Vec<SensorReading> = (1..=5)
            .map(|i| make_reading("s1", i as f64, i))
            .collect();
        let tree = MerkleRootProducer::build_tree(&readings);
        let root = MerkleRootProducer::get_root(&tree);
        let proof = MerkleRootProducer::get_proof(&tree, 1);
        let mut tampered = readings[1].clone();
        tampered.value = 9999.0;
        assert!(
            !MerkleRootProducer::verify_proof(&root, &proof, &tampered, 1, readings.len()),
            "Tampered reading must fail proof"
        );
    }

    // ──────────────────────────────────────────────
    // Gateway node tests
    // ──────────────────────────────────────────────

    #[test]
    fn test_gateway_cycle_with_signature_verification() {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = k256::ecdsa::VerifyingKey::from(&signing_key);
        let sensor_id = "did:malama:sensor:1".to_string();

        let mut gateway = GatewayNode::new(0, 1, "/tmp/test_wal_cycle.log");
        gateway.register_sensor(sensor_id.clone(), verifying_key);

        let reading = make_signed_reading(&signing_key, &sensor_id, 21.0, 1);
        assert!(gateway.receive_reading(reading).is_ok());

        let batch = gateway.process_cycle();
        assert!(batch.is_some(), "Cycle should produce a batch");
        assert_eq!(gateway.state, GatewayState::BROADCASTING);
        assert!(gateway.current_merkle_root.is_some());

        gateway.confirm_broadcast();
        assert_eq!(gateway.state, GatewayState::COLLECTING);
    }

    #[test]
    fn test_gateway_rejects_unknown_sensor() {
        let mut gateway = GatewayNode::new(3600, 100, "/tmp/test_wal_unknown.log");
        let reading = make_reading("did:unknown:sensor:x", 21.0, 1);
        let result = gateway.receive_reading(reading);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown sensor"));
    }

    #[test]
    fn test_gateway_rejects_wrong_signature() {
        let key1 = SigningKey::random(&mut OsRng);
        let key2 = SigningKey::random(&mut OsRng); // different key
        let verifying_key1 = k256::ecdsa::VerifyingKey::from(&key1);
        let sensor_id = "did:malama:sensor:abc".to_string();

        let mut gateway = GatewayNode::new(3600, 100, "/tmp/test_wal_wrong_sig.log");
        gateway.register_sensor(sensor_id.clone(), verifying_key1);

        // Sign with key2 but register key1 → should fail
        let reading = make_signed_reading(&key2, &sensor_id, 21.0, 1);
        let result = gateway.receive_reading(reading);
        assert!(result.is_err(), "Wrong key signature must be rejected");
    }

    #[test]
    fn test_gateway_verify_inclusion_for_last_batch() {
        let mut gateway = GatewayNode::new(3600, 100, "/tmp/test_wal_inclusion.log");

        let readings: Vec<SensorReading> = (1..=3)
            .map(|i| make_reading("s1", i as f64, i))
            .collect();
        for r in &readings {
            gateway.receive_reading_unchecked(r.clone()).unwrap();
        }

        gateway.aggregator.window_duration_secs = 0; // force timer expiry
        let batch = gateway.process_cycle();
        assert!(batch.is_some());

        // Verify that reading at index 1 is provably in the batch
        let result = gateway.verify_inclusion(&readings[1], 1);
        assert!(result.is_ok());
        assert!(result.unwrap(), "Reading should be provably in batch");
    }

    #[test]
    fn test_gateway_force_seal() {
        let mut gateway = GatewayNode::new(3600, 100, "/tmp/test_wal_force.log");
        gateway.receive_reading_unchecked(make_reading("s1", 21.0, 1)).unwrap();
        let batch = gateway.force_seal();
        assert!(batch.is_some());
        assert_eq!(gateway.state, GatewayState::BROADCASTING);
        assert!(batch.unwrap().merkle_root.is_some());
    }

    #[test]
    fn test_gateway_deregister_sensor() {
        let key = SigningKey::random(&mut OsRng);
        let vk = k256::ecdsa::VerifyingKey::from(&key);
        let mut gateway = GatewayNode::new(3600, 100, "/tmp/test_wal_dereg.log");
        gateway.register_sensor("did:malama:sensor:q".to_string(), vk);
        assert_eq!(gateway.registered_sensor_count(), 1);
        assert!(gateway.deregister_sensor("did:malama:sensor:q"));
        assert_eq!(gateway.registered_sensor_count(), 0);

        // After deregistration, readings from this sensor must be rejected
        let reading = make_signed_reading(&key, "did:malama:sensor:q", 21.0, 1);
        assert!(gateway.receive_reading(reading).is_err());
    }
}

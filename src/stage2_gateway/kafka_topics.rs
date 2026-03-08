//! Stage 2 — Prompt 12: Kafka Topic Architecture
//!
//! Defines the 5 Kafka topics used by the Mālama Protocol Gateway.
//! Each topic is partitioned by `sensorDID` for per-sensor ordering guarantees.
//!
//! Topics:
//! 1. `raw-sensor-readings`   — raw signed readings from sensors
//! 2. `validated-readings`    — readings that passed signature + range checks
//! 3. `batch-pending`         — sealed batches awaiting blockchain submission
//! 4. `blockchain-confirmed`  — tx IDs returned by chain adapters
//! 5. `alerts`                — validation failures, offline sensors, anomalies

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

// ── Topic definitions ─────────────────────────────────────────────────────────

pub const TOPIC_RAW_READINGS:       &str = "raw-sensor-readings";
pub const TOPIC_VALIDATED_READINGS: &str = "validated-readings";
pub const TOPIC_BATCH_PENDING:      &str = "batch-pending";
pub const TOPIC_BLOCKCHAIN_CONFIRMED: &str = "blockchain-confirmed";
pub const TOPIC_ALERTS:             &str = "alerts";

/// All 5 gateway topics.
pub const ALL_TOPICS: &[&str] = &[
    TOPIC_RAW_READINGS,
    TOPIC_VALIDATED_READINGS,
    TOPIC_BATCH_PENDING,
    TOPIC_BLOCKCHAIN_CONFIRMED,
    TOPIC_ALERTS,
];

// ── Partition routing ─────────────────────────────────────────────────────────

/// Compute the Kafka partition for a `sensorDID`.
///
/// Uses SHA-256 of the DID modulo `num_partitions` — same DID always maps to
/// the same partition, guaranteeing per-sensor message ordering.
pub fn partition_for_did(sensor_did: &str, num_partitions: u32) -> u32 {
    let mut h = Sha256::new();
    h.update(sensor_did.as_bytes());
    let bytes: [u8; 32] = h.finalize().into();
    // Use first 4 bytes as u32
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) % num_partitions
}

// ── Message envelope ──────────────────────────────────────────────────────────

/// Generic Kafka message envelope wrapping any payload `T`.
#[derive(Debug, Serialize, Deserialize)]
pub struct KafkaMessage<T: Serialize> {
    pub topic: String,
    pub partition: u32,
    /// Kafka partition key — always `sensorDID` for ordering.
    pub partition_key: String,
    pub offset: u64,
    pub timestamp: DateTime<Utc>,
    pub payload: T,
}

impl<T: Serialize> KafkaMessage<T> {
    pub fn new(topic: &str, sensor_did: &str, partition_key: &str, num_partitions: u32, payload: T) -> Self {
        Self {
            topic: topic.to_string(),
            partition: partition_for_did(sensor_did, num_partitions),
            partition_key: partition_key.to_string(),
            offset: 0, // Set by broker
            timestamp: Utc::now(),
            payload,
        }
    }
}

// ── Topic-specific payloads ───────────────────────────────────────────────────

/// Payload for `raw-sensor-readings`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawReadingPayload {
    pub sensor_did: String,
    pub value: f64,
    pub unit: String,
    pub timestamp: DateTime<Utc>,
    pub sequence_number: u64,
    pub nonce: String,
    pub signature: String,
}

/// Payload for `validated-readings`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedReadingPayload {
    pub sensor_did: String,
    pub value: f64,
    pub confidence_score: f64,
    pub signature_verified: bool,
    pub timestamp: DateTime<Utc>,
}

/// Payload for `batch-pending`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPendingPayload {
    pub batch_id: String,
    pub merkle_root: String,
    pub reading_count: usize,
    pub sensor_dids: Vec<String>,
    pub sealed_at: DateTime<Utc>,
    pub target_chains: Vec<String>, // e.g. ["cardano", "base", "hedera"]
}

/// Payload for `blockchain-confirmed`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainConfirmedPayload {
    pub batch_id: String,
    pub chain: String,
    pub tx_id: String,
    pub block_height: u64,
    pub confirmed_at: DateTime<Utc>,
}

/// Severity levels for alerts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity { Info, Warning, Critical }

/// Payload for `alerts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertPayload {
    pub sensor_did: String,
    pub severity: AlertSeverity,
    pub kind: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

// ── In-memory mock broker ─────────────────────────────────────────────────────

/// Simple in-memory Kafka broker mock for testing.
pub struct MockKafkaBroker {
    pub num_partitions: u32,
    /// topic → Vec<raw JSON bytes>
    messages: std::collections::HashMap<String, Vec<Vec<u8>>>,
}

impl MockKafkaBroker {
    pub fn new(num_partitions: u32) -> Self {
        let mut messages = std::collections::HashMap::new();
        for t in ALL_TOPICS { messages.insert(t.to_string(), Vec::new()); }
        Self { num_partitions, messages }
    }

    /// Publish a message to a topic.
    pub fn publish<T: Serialize>(&mut self, topic: &str, sensor_did: &str, payload: T) -> u64 {
        let msg = KafkaMessage::new(topic, sensor_did, sensor_did, self.num_partitions, payload);
        let bytes = serde_json::to_vec(&msg).unwrap();
        let queue = self.messages.entry(topic.to_string()).or_default();
        queue.push(bytes);
        (queue.len() - 1) as u64 // offset
    }

    /// Count of messages in a topic.
    pub fn len(&self, topic: &str) -> usize {
        self.messages.get(topic).map(|v| v.len()).unwrap_or(0)
    }

    /// True if a topic has no messages.
    pub fn is_empty(&self, topic: &str) -> bool { self.len(topic) == 0 }

    /// Drain all messages from a topic (consumer poll simulation).
    pub fn poll(&mut self, topic: &str) -> Vec<Vec<u8>> {
        self.messages.get_mut(topic).map(|v| v.drain(..).collect()).unwrap_or_default()
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // ── Test 1: all 5 topics defined ──────────────────────────────────────────

    #[test]
    fn test_all_five_topics_defined() {
        assert_eq!(ALL_TOPICS.len(), 5);
        assert!(ALL_TOPICS.contains(&TOPIC_RAW_READINGS));
        assert!(ALL_TOPICS.contains(&TOPIC_VALIDATED_READINGS));
        assert!(ALL_TOPICS.contains(&TOPIC_BATCH_PENDING));
        assert!(ALL_TOPICS.contains(&TOPIC_BLOCKCHAIN_CONFIRMED));
        assert!(ALL_TOPICS.contains(&TOPIC_ALERTS));
    }

    // ── Test 2: same DID always routes to same partition ─────────────────────

    #[test]
    fn test_partition_is_deterministic_for_did() {
        let did = "did:cardano:sensor:biochar-001";
        let p1 = partition_for_did(did, 12);
        let p2 = partition_for_did(did, 12);
        assert_eq!(p1, p2, "Same DID must always map to same partition");
    }

    // ── Test 3: different DIDs may map to different partitions ────────────────

    #[test]
    fn test_different_dids_can_use_different_partitions() {
        let partitions: std::collections::HashSet<u32> = (0..20)
            .map(|i| partition_for_did(&format!("did:cardano:sensor:{i}"), 12))
            .collect();
        assert!(partitions.len() > 1, "Different DIDs should use different partitions");
    }

    // ── Test 4: partition within range ────────────────────────────────────────

    #[test]
    fn test_partition_within_range() {
        for i in 0..50 {
            let p = partition_for_did(&format!("did:cardano:sensor:s{i}"), 12);
            assert!(p < 12, "Partition {p} must be < 12");
        }
    }

    // ── Test 5: publish to raw-sensor-readings ────────────────────────────────

    #[test]
    fn test_publish_raw_reading() {
        let mut broker = MockKafkaBroker::new(12);
        let payload = RawReadingPayload {
            sensor_did: "did:cardano:sensor:biochar-001".to_string(),
            value: 23.4, unit: "Celsius".to_string(),
            timestamp: Utc::now(), sequence_number: 1,
            nonce: "abc".to_string(), signature: "sig".to_string(),
        };
        let off = broker.publish(TOPIC_RAW_READINGS, "did:cardano:sensor:biochar-001", payload);
        assert_eq!(off, 0);
        assert_eq!(broker.len(TOPIC_RAW_READINGS), 1);
    }

    // ── Test 6: publish to all 5 topics ──────────────────────────────────────

    #[test]
    fn test_publish_to_all_topics() {
        let mut broker = MockKafkaBroker::new(12);
        let did = "did:cardano:sensor:biochar-001";

        broker.publish(TOPIC_RAW_READINGS, did,
            RawReadingPayload { sensor_did: did.to_string(), value: 1.0,
                unit: "C".to_string(), timestamp: Utc::now(), sequence_number: 1,
                nonce: "n".to_string(), signature: "s".to_string() });
        broker.publish(TOPIC_VALIDATED_READINGS, did,
            ValidatedReadingPayload { sensor_did: did.to_string(), value: 1.0,
                confidence_score: 0.95, signature_verified: true, timestamp: Utc::now() });
        broker.publish(TOPIC_BATCH_PENDING, did,
            BatchPendingPayload { batch_id: "b1".to_string(), merkle_root: "r".to_string(),
                reading_count: 1, sensor_dids: vec![did.to_string()],
                sealed_at: Utc::now(), target_chains: vec!["cardano".to_string()] });
        broker.publish(TOPIC_BLOCKCHAIN_CONFIRMED, did,
            BlockchainConfirmedPayload { batch_id: "b1".to_string(), chain: "cardano".to_string(),
                tx_id: "tx1".to_string(), block_height: 100, confirmed_at: Utc::now() });
        broker.publish(TOPIC_ALERTS, did,
            AlertPayload { sensor_did: did.to_string(), severity: AlertSeverity::Warning,
                kind: "offline".to_string(), message: "gone offline".to_string(),
                timestamp: Utc::now() });

        for topic in ALL_TOPICS {
            assert_eq!(broker.len(topic), 1, "Topic {topic} must have 1 message");
        }
    }

    // ── Test 7: poll drains topic ─────────────────────────────────────────────

    #[test]
    fn test_poll_drains_messages() {
        let mut broker = MockKafkaBroker::new(12);
        let did = "did:cardano:sensor:biochar-001";
        broker.publish(TOPIC_ALERTS, did, AlertPayload {
            sensor_did: did.to_string(), severity: AlertSeverity::Critical,
            kind: "tamper".to_string(), message: "tampered".to_string(),
            timestamp: Utc::now(),
        });
        assert_eq!(broker.len(TOPIC_ALERTS), 1);
        let drained = broker.poll(TOPIC_ALERTS);
        assert_eq!(drained.len(), 1);
        assert_eq!(broker.len(TOPIC_ALERTS), 0, "Poll must drain the topic");
    }

    // ── Test 8: KafkaMessage sets correct partition ───────────────────────────

    #[test]
    fn test_kafka_message_partition_set() {
        let did = "did:cardano:sensor:biochar-001";
        let msg = KafkaMessage::new(TOPIC_RAW_READINGS, did, did, 12, "payload".to_string());
        assert_eq!(msg.partition, partition_for_did(did, 12));
        assert_eq!(msg.partition_key, did);
    }
}

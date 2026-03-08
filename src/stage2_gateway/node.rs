use std::collections::HashMap;
use k256::ecdsa::{VerifyingKey, signature::Verifier};
use k256::ecdsa::Signature;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use crate::stage2_gateway::aggregator::{BatchAggregator, DataBatch, SensorReading};
use crate::stage2_gateway::merkle_tree::MerkleRootProducer;
use crate::stage2_gateway::wal::WriteAheadLog;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GatewayState {
    COLLECTING,
    SEALING,
    BROADCASTING,
    CONFIRMED,
}

pub struct GatewayNode {
    pub state: GatewayState,
    pub aggregator: BatchAggregator,
    pub wal: WriteAheadLog,
    pub current_merkle_root: Option<String>,
    pub sensor_registry: HashMap<String, VerifyingKey>,
}

impl GatewayNode {
    pub fn new(window_secs: i64, wal_path: &str) -> Self {
        Self {
            state: GatewayState::COLLECTING,
            aggregator: BatchAggregator::new(window_secs),
            wal: WriteAheadLog::new(wal_path),
            current_merkle_root: None,
            sensor_registry: HashMap::new(),
        }
    }

    pub fn register_sensor(&mut self, did: String, key: VerifyingKey) {
        self.sensor_registry.insert(did, key);
    }

    fn verify_reading(&self, reading: &SensorReading) -> bool {
        let key = match self.sensor_registry.get(&reading.sensor_id) {
            Some(k) => k,
            None => return false, // Unknown sensor
        };

        let message = format!("{}{}{}", reading.sensor_id, reading.value, reading.timestamp.to_rfc3339());
        if let Ok(sig) = Signature::from_str(&reading.signature) {
            return key.verify(message.as_bytes(), &sig).is_ok();
        }
        false
    }

    pub fn receive_reading(&mut self, reading: SensorReading) -> Result<(), String> {
        if self.state != GatewayState::COLLECTING {
            return Err("Gateway not in collecting state".to_string());
        }

        if !self.verify_reading(&reading) {
            return Err("Invalid reading signature or unknown sensor".to_string());
        }

        self.aggregator.add_reading(reading);
        Ok(())
    }

    pub fn process_cycle(&mut self) -> Option<DataBatch> {
        if self.state == GatewayState::COLLECTING && self.aggregator.should_seal() {
            self.state = GatewayState::SEALING;
            if let Some(batch) = self.aggregator.seal_batch() {
                // 1. Write to WAL for ACID guarantee
                if let Err(e) = self.wal.write_batch(&batch) {
                    eprintln!("WAL write failed: {}", e);
                    // In a production system, we'd handle this more gracefully
                }
                
                // 2. Build Merkle Tree
                let tree = MerkleRootProducer::build_tree(&batch.readings);
                self.current_merkle_root = Some(MerkleRootProducer::get_root(&tree));
                
                self.state = GatewayState::BROADCASTING;
                return Some(batch);
            }
            self.state = GatewayState::COLLECTING;
        }
        None
    }

    pub fn confirm_broadcast(&mut self) {
        if self.state == GatewayState::BROADCASTING {
            self.state = GatewayState::CONFIRMED;
            let _ = self.wal.clear(); 
            self.state = GatewayState::COLLECTING;
        }
    }
}

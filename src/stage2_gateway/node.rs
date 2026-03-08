use std::collections::HashMap;
use k256::ecdsa::{VerifyingKey, signature::Verifier};
use k256::ecdsa::Signature;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use crate::stage2_gateway::aggregator::{BatchAggregator, DataBatch, SensorReading};
use crate::stage2_gateway::merkle_tree::MerkleRootProducer;
use crate::stage2_gateway::wal::WriteAheadLog;

/// Lifecycle states of a GatewayNode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GatewayState {
    /// Accepting signed readings from registered sensors.
    COLLECTING,
    /// Sealing the current batch into a Merkle tree.
    SEALING,
    /// Merkle root computed; waiting for multi-chain broadcast confirmation.
    BROADCASTING,
    /// Broadcast confirmed; ready to resume collecting.
    CONFIRMED,
}

/// A gateway node that:
/// - Maintains a registry of authorized sensor public keys
/// - Verifies every incoming reading's ECDSA signature
/// - Aggregates readings into time/volume-based batches
/// - Builds Merkle trees and computes roots
/// - Persists batches to a Write-Ahead Log for crash recovery
pub struct GatewayNode {
    pub state: GatewayState,
    pub aggregator: BatchAggregator,
    pub wal: WriteAheadLog,
    pub current_merkle_root: Option<String>,
    pub last_sealed_batch: Option<DataBatch>,
    /// sensor DID → ECDSA verifying key
    pub sensor_registry: HashMap<String, VerifyingKey>,
}

impl GatewayNode {
    /// Create a gateway with the given batch window (seconds) and volume threshold.
    pub fn new(window_secs: i64, volume_threshold: usize, wal_path: &str) -> Self {
        Self {
            state: GatewayState::COLLECTING,
            aggregator: BatchAggregator::new(window_secs, volume_threshold),
            wal: WriteAheadLog::new(wal_path),
            current_merkle_root: None,
            last_sealed_batch: None,
            sensor_registry: HashMap::new(),
        }
    }

    /// Register a sensor's verifying key by its DID.
    pub fn register_sensor(&mut self, did: String, key: VerifyingKey) {
        self.sensor_registry.insert(did, key);
    }

    /// Deregister a sensor (e.g. quarantined or retired).
    pub fn deregister_sensor(&mut self, did: &str) -> bool {
        self.sensor_registry.remove(did).is_some()
    }

    /// Number of registered sensors.
    pub fn registered_sensor_count(&self) -> usize {
        self.sensor_registry.len()
    }

    /// Verify reading signature against the registered key for that sensor.
    fn verify_reading(&self, reading: &SensorReading) -> Result<(), String> {
        let key = self
            .sensor_registry
            .get(&reading.sensor_id)
            .ok_or_else(|| format!("Unknown sensor: {}", reading.sensor_id))?;

        // Message format must match Stage 1 signing convention:
        // sensor_id || value || rfc3339_timestamp || nonce
        let message = format!(
            "{}{}{}{}",
            reading.sensor_id,
            reading.value,
            reading.timestamp.to_rfc3339(),
            reading.nonce
        );

        let sig = Signature::from_str(&reading.signature)
            .map_err(|e| format!("Invalid signature format: {e}"))?;

        key.verify(message.as_bytes(), &sig)
            .map_err(|_| "Signature verification failed".to_string())
    }

    /// Accept a signed reading from a registered sensor.
    /// Returns an error if:
    /// - Gateway is not in COLLECTING state
    /// - Sensor is not registered
    /// - Signature is invalid
    /// - Reading is a duplicate (same content hash seen before in this window)
    pub fn receive_reading(&mut self, reading: SensorReading) -> Result<(), String> {
        if self.state != GatewayState::COLLECTING {
            return Err(format!("Gateway state is {:?}, not COLLECTING", self.state));
        }

        self.verify_reading(&reading)?;

        if !self.aggregator.add_reading(reading) {
            return Err("Duplicate reading rejected".to_string());
        }

        Ok(())
    }

    /// Accept a reading without signature verification (for testing / mock sensors).
    #[cfg(test)]
    pub fn receive_reading_unchecked(&mut self, reading: SensorReading) -> Result<(), String> {
        if self.state != GatewayState::COLLECTING {
            return Err(format!("Gateway state is {:?}, not COLLECTING", self.state));
        }
        if !self.aggregator.add_reading(reading) {
            return Err("Duplicate reading rejected".to_string());
        }
        Ok(())
    }

    /// Run a cycle: check sealing condition, build Merkle tree, persist to WAL.
    /// Returns the sealed batch if one was produced, otherwise None.
    pub fn process_cycle(&mut self) -> Option<DataBatch> {
        if self.state != GatewayState::COLLECTING {
            return None;
        }

        if self.aggregator.should_seal().is_none() {
            return None;
        }

        self.state = GatewayState::SEALING;

        if let Some(mut batch) = self.aggregator.seal_batch() {
            // Persist to WAL before doing anything else (ACID guarantee)
            if let Err(e) = self.wal.write_batch(&batch) {
                eprintln!("[GatewayNode] WAL write failed: {e}");
                // In production: alert ops team and halt — don't broadcast unrecorded data
            }

            // Build Merkle tree and attach root to batch
            let tree = MerkleRootProducer::build_tree(&batch.readings);
            let root = MerkleRootProducer::get_root(&tree);
            batch.merkle_root = Some(root.clone());

            self.current_merkle_root = Some(root);
            self.last_sealed_batch = Some(batch.clone());
            self.state = GatewayState::BROADCASTING;

            return Some(batch);
        }

        self.state = GatewayState::COLLECTING;
        None
    }

    /// Trigger force-seal (e.g. sensor going offline, manual flush).
    pub fn force_seal(&mut self) -> Option<DataBatch> {
        if self.state != GatewayState::COLLECTING {
            return None;
        }
        self.state = GatewayState::SEALING;

        if let Some(mut batch) = self.aggregator.force_seal() {
            if let Err(e) = self.wal.write_batch(&batch) {
                eprintln!("[GatewayNode] WAL write failed on force seal: {e}");
            }
            let tree = MerkleRootProducer::build_tree(&batch.readings);
            let root = MerkleRootProducer::get_root(&tree);
            batch.merkle_root = Some(root.clone());
            self.current_merkle_root = Some(root);
            self.last_sealed_batch = Some(batch.clone());
            self.state = GatewayState::BROADCASTING;
            return Some(batch);
        }

        self.state = GatewayState::COLLECTING;
        None
    }

    /// Confirm that the batch was broadcast to the blockchain.
    /// Clears the WAL and resets state to COLLECTING.
    pub fn confirm_broadcast(&mut self) {
        if self.state == GatewayState::BROADCASTING {
            self.state = GatewayState::CONFIRMED;
            if let Err(e) = self.wal.clear() {
                eprintln!("[GatewayNode] WAL clear failed: {e}");
            }
            self.state = GatewayState::COLLECTING;
        }
    }

    /// Verify a specific reading's inclusion in the last sealed batch via Merkle proof.
    pub fn verify_inclusion(
        &self,
        reading: &SensorReading,
        leaf_index: usize,
    ) -> Result<bool, String> {
        let batch = self
            .last_sealed_batch
            .as_ref()
            .ok_or("No sealed batch available")?;

        let root = batch
            .merkle_root
            .as_ref()
            .ok_or("Batch has no Merkle root")?;

        let tree = MerkleRootProducer::build_tree(&batch.readings);
        let proof = MerkleRootProducer::get_proof(&tree, leaf_index);
        let total = batch.readings.len();

        Ok(MerkleRootProducer::verify_proof(root, &proof, reading, leaf_index, total))
    }
}

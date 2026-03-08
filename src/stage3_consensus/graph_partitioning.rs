//! Stage 3 — Prompt 18: Graph Partitioning & Adaptive Packaging
//!
//! Enhanced partitioning that routes sensors to validator nodes based on:
//!   - Geographic proximity (minimize latency)
//!   - Load balancing (spread sensor count evenly)
//!   - Sensor affinity (same sensor always goes to same primary node)
//!
//! Each `ValidatorNode` has a region, current load, and capacity.
//! The `GraphPartitioner` builds an assignment graph and rebalances on demand.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

// ── Geo regions ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GeoRegion {
    NorthAmerica,
    Europe,
    AsiaPacific,
    SouthAmerica,
    Africa,
    Global,
}

// ── Validator node ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorNode {
    pub node_id: String,
    pub region: GeoRegion,
    pub capacity: usize,
    pub current_load: usize,
}

impl ValidatorNode {
    pub fn new(node_id: &str, region: GeoRegion, capacity: usize) -> Self {
        Self { node_id: node_id.to_string(), region, capacity, current_load: 0 }
    }

    pub fn available_capacity(&self) -> usize {
        self.capacity.saturating_sub(self.current_load)
    }

    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 { return 1.0; }
        self.current_load as f64 / self.capacity as f64
    }

    pub fn is_overloaded(&self) -> bool { self.current_load >= self.capacity }
}

// ── Graph partitioner ─────────────────────────────────────────────────────────

/// Adaptive partitioner: assigns sensor batches to validator nodes.
pub struct GraphPartitioner {
    pub nodes: Vec<ValidatorNode>,
    /// sensor_did → preferred node_id (affinity cache)
    affinity: HashMap<String, String>,
}

impl GraphPartitioner {
    pub fn new(nodes: Vec<ValidatorNode>) -> Self {
        Self { nodes, affinity: HashMap::new() }
    }

    /// Assign a sensor's batch to the best validator node.
    ///
    /// Priority:
    /// 1. Existing affinity (same sensor always same primary validator)
    /// 2. Preferred region match with spare capacity
    /// 3. Lowest utilization globally
    pub fn assign(
        &mut self,
        sensor_did: &str,
        preferred_region: &GeoRegion,
    ) -> Option<String> {
        // Check affinity
        if let Some(node_id) = self.affinity.get(sensor_did) {
            if let Some(n) = self.nodes.iter().find(|n| &n.node_id == node_id) {
                if !n.is_overloaded() {
                    let nid = node_id.clone();
                    self.increment_load(&nid);
                    return Some(nid);
                }
            }
        }

        // Best node: preferred region first, then global fallback
        let best = self.nodes.iter()
            .filter(|n| !n.is_overloaded())
            .min_by(|a, b| {
                let a_score = if &a.region == preferred_region { 0 } else { 1 };
                let b_score = if &b.region == preferred_region { 0 } else { 1 };
                a_score.cmp(&b_score)
                    .then(a.utilization().partial_cmp(&b.utilization()).unwrap())
            })
            .map(|n| n.node_id.clone());

        if let Some(ref node_id) = best {
            self.affinity.insert(sensor_did.to_string(), node_id.clone());
            self.increment_load(node_id);
        }
        best
    }

    fn increment_load(&mut self, node_id: &str) {
        if let Some(n) = self.nodes.iter_mut().find(|n| n.node_id == node_id) {
            n.current_load += 1;
        }
    }

    /// Release a sensor's slot after batch is committed.
    pub fn release(&mut self, sensor_did: &str) {
        if let Some(node_id) = self.affinity.get(sensor_did) {
            let nid = node_id.clone();
            if let Some(n) = self.nodes.iter_mut().find(|n| n.node_id == nid) {
                n.current_load = n.current_load.saturating_sub(1);
            }
        }
    }

    /// Rebalance: move sensors from overloaded nodes to underloaded nodes.
    /// Returns the number of affinity reassignments made.
    pub fn rebalance(&mut self) -> usize {
        let overloaded: Vec<String> = self.nodes.iter()
            .filter(|n| n.utilization() > 0.85)
            .map(|n| n.node_id.clone())
            .collect();

        if overloaded.is_empty() { return 0; }

        let mut reassigned = 0;
        let sensor_dids: Vec<String> = self.affinity.keys().cloned().collect();

        for did in sensor_dids {
            if let Some(current) = self.affinity.get(&did) {
                if overloaded.contains(current) {
                    // Find least-loaded alternative
                    let current_clone = current.clone();
                    let alt = self.nodes.iter()
                        .filter(|n| n.node_id != current_clone && !n.is_overloaded())
                        .min_by(|a, b| a.utilization().partial_cmp(&b.utilization()).unwrap())
                        .map(|n| n.node_id.clone());

                    if let Some(new_node) = alt {
                        self.affinity.insert(did, new_node);
                        reassigned += 1;
                    }
                }
            }
        }
        reassigned
    }

    /// Load report: node_id → (current_load, capacity, utilization%)
    pub fn load_report(&self) -> Vec<(&str, usize, usize, f64)> {
        self.nodes.iter()
            .map(|n| (n.node_id.as_str(), n.current_load, n.capacity, n.utilization()))
            .collect()
    }

    /// Deterministic backup assignment (no affinity, no state mutation).
    /// Used by retry logic when primary fails.
    pub fn deterministic_assign(nodes: &[ValidatorNode], sensor_did: &str) -> Option<String> {
        let mut h = Sha256::new();
        h.update(sensor_did.as_bytes());
        let bytes: [u8; 32] = h.finalize().into();
        let idx = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
            as usize % nodes.len().max(1);
        nodes.get(idx).map(|n| n.node_id.clone())
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_nodes() -> Vec<ValidatorNode> {
        vec![
            ValidatorNode::new("v1-us",  GeoRegion::NorthAmerica, 100),
            ValidatorNode::new("v2-eu",  GeoRegion::Europe,       100),
            ValidatorNode::new("v3-ap",  GeoRegion::AsiaPacific,  100),
        ]
    }

    // ── Test 1: sensor assigned to preferred region ───────────────────────────

    #[test]
    fn test_assigns_preferred_region() {
        let mut p = GraphPartitioner::new(make_nodes());
        let assigned = p.assign("did:sensor:s1", &GeoRegion::Europe);
        assert_eq!(assigned.unwrap(), "v2-eu", "EU sensor should prefer EU node");
    }

    // ── Test 2: affinity respected on second call ─────────────────────────────

    #[test]
    fn test_affinity_respected() {
        let mut p = GraphPartitioner::new(make_nodes());
        let first  = p.assign("did:sensor:s1", &GeoRegion::NorthAmerica).unwrap();
        let second = p.assign("did:sensor:s1", &GeoRegion::NorthAmerica).unwrap();
        assert_eq!(first, second, "Same sensor must return same node (affinity)");
    }

    // ── Test 3: overloaded node skipped ──────────────────────────────────────

    #[test]
    fn test_overloaded_node_skipped() {
        let mut nodes = make_nodes();
        nodes[0].current_load = 100; // v1-us full
        let mut p = GraphPartitioner::new(nodes);
        let assigned = p.assign("did:sensor:new", &GeoRegion::NorthAmerica).unwrap();
        assert_ne!(assigned, "v1-us", "Overloaded node must be skipped");
    }

    // ── Test 4: no available nodes returns None ───────────────────────────────

    #[test]
    fn test_all_nodes_overloaded_returns_none() {
        let mut nodes = make_nodes();
        for n in &mut nodes { n.current_load = n.capacity; }
        let mut p = GraphPartitioner::new(nodes);
        assert!(p.assign("did:sensor:x", &GeoRegion::Global).is_none());
    }

    // ── Test 5: release decrements load ──────────────────────────────────────

    #[test]
    fn test_release_decrements_load() {
        let mut p = GraphPartitioner::new(make_nodes());
        p.assign("did:sensor:s1", &GeoRegion::NorthAmerica);
        let before = p.nodes[0].current_load;
        p.release("did:sensor:s1");
        assert!(p.nodes[0].current_load < before || before == 0);
    }

    // ── Test 6: rebalance moves sensors from overloaded node ─────────────────

    #[test]
    fn test_rebalance_moves_from_overloaded() {
        let mut p = GraphPartitioner::new(make_nodes());
        // Overload v1-us manually
        p.nodes[0].current_load = 90; // 90% utilization → triggers rebalance
        // Pin s1 to v1-us
        p.affinity.insert("did:sensor:s1".to_string(), "v1-us".to_string());
        let moved = p.rebalance();
        assert!(moved >= 1, "At least one sensor must be reassigned");
        assert_ne!(p.affinity.get("did:sensor:s1").unwrap(), "v1-us",
            "Sensor must have been reassigned away from overloaded node");
    }

    // ── Test 7: deterministic_assign is consistent ────────────────────────────

    #[test]
    fn test_deterministic_assign_consistent() {
        let nodes = make_nodes();
        let a1 = GraphPartitioner::deterministic_assign(&nodes, "did:sensor:s1");
        let a2 = GraphPartitioner::deterministic_assign(&nodes, "did:sensor:s1");
        assert_eq!(a1, a2, "Deterministic assignment must be consistent");
    }

    // ── Test 8: utilization calculation correct ───────────────────────────────

    #[test]
    fn test_utilization_calc() {
        let mut n = ValidatorNode::new("v1", GeoRegion::Global, 100);
        n.current_load = 75;
        assert!((n.utilization() - 0.75).abs() < 1e-9);
        assert!(!n.is_overloaded());
        n.current_load = 100;
        assert!(n.is_overloaded());
    }

    // ── Test 9: load is spread across multiple sensors ────────────────────────

    #[test]
    fn test_load_distributed_across_sensors() {
        let mut p = GraphPartitioner::new(make_nodes());
        // Assign 30 sensors from 3 regions
        for i in 0..10 {
            p.assign(&format!("s{i}-na"), &GeoRegion::NorthAmerica);
            p.assign(&format!("s{i}-eu"), &GeoRegion::Europe);
            p.assign(&format!("s{i}-ap"), &GeoRegion::AsiaPacific);
        }
        // Each node should have load > 0
        for n in &p.nodes {
            assert!(n.current_load > 0, "Node {} has no load", n.node_id);
        }
    }
}

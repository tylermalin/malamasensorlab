use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// IPFS Pin Types (Prompt 35).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinSource {
    Pinata,
    LocalNode,
    Filecoin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinStatus {
    pub source: PinSource,
    pub is_active: bool,
    pub last_verified: DateTime<Utc>,
}

/// Manages multi-node IPFS pinning and redundancy (Prompt 35).
pub struct IpfsPinManager {
    /// CID -> Vec of PinStatus
    pub pins: HashMap<String, Vec<PinStatus>>,
    /// Alert log: (Timestamp, Message)
    pub alerts: Vec<(DateTime<Utc>, String)>,
}

impl IpfsPinManager {
    pub fn new() -> Self {
        Self {
            pins: HashMap::new(),
            alerts: Vec::new(),
        }
    }

    /// Pins a CID to multiple sources for redundancy.
    pub fn pin_triple_redundancy(&mut self, cid: &str) {
        let now = Utc::now();
        let statuses = vec![
            PinStatus { source: PinSource::Pinata, is_active: true, last_verified: now },
            PinStatus { source: PinSource::LocalNode, is_active: true, last_verified: now },
            PinStatus { source: PinSource::Filecoin, is_active: true, last_verified: now },
        ];
        self.pins.insert(cid.to_string(), statuses);
    }

    /// Simulates a periodic retrieval test.
    /// If a pin is found to be offline, an alert is generated.
    pub fn perform_health_check(&mut self) -> bool {
        let mut all_ok = true;
        let now = Utc::now();
        
        for (cid, statuses) in self.pins.iter_mut() {
            for status in statuses.iter_mut() {
                // In a real system, we would attempt to fetch the data here.
                // For the simulation, we keep it active unless manually failed in tests.
                if !status.is_active {
                    self.alerts.push((now, format!("ALERT: CID {} pin offline on {:?}", cid, status.source)));
                    all_ok = false;
                }
                status.last_verified = now;
            }
        }
        
        all_ok
    }

    /// Simulates a node failure for testing.
    pub fn simulate_failure(&mut self, cid: &str, source: PinSource) {
        if let Some(statuses) = self.pins.get_mut(cid) {
            if let Some(status) = statuses.iter_mut().find(|s| s.source == source) {
                status.is_active = false;
            }
        }
    }

    /// Returns the number of active pins for a given CID.
    pub fn active_redundancy_count(&self, cid: &str) -> usize {
        self.pins.get(cid)
            .map(|statuses| statuses.iter().filter(|s| s.is_active).count())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triple_redundancy_pinning() {
        let mut manager = IpfsPinManager::new();
        let cid = "QmRedundantData";
        manager.pin_triple_redundancy(cid);
        
        assert_eq!(manager.active_redundancy_count(cid), 3);
        assert!(manager.perform_health_check());
    }

    #[test]
    fn test_pin_failure_alerting() {
        let mut manager = IpfsPinManager::new();
        let cid = "QmCriticalData";
        manager.pin_triple_redundancy(cid);
        
        manager.simulate_failure(cid, PinSource::Pinata);
        assert_eq!(manager.active_redundancy_count(cid), 2);
        
        let all_ok = manager.perform_health_check();
        assert!(!all_ok);
        assert_eq!(manager.alerts.len(), 1);
        assert!(manager.alerts[0].1.contains("Pinata"));
    }
}

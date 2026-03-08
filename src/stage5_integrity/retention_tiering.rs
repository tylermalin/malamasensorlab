use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Storage Tiers (Prompt 38).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageTier {
    Hot,   // Last 30 days (Fast, expensive)
    Warm,  // 30 days - 3 years (Slower, cheaper)
    Cold,  // 3+ years (Filecoin/Archive)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRecord {
    pub cid: String,
    pub created_at: DateTime<Utc>,
    pub tier: StorageTier,
}

/// Manages data retention and tier migration (Prompt 38).
pub struct RetentionManager {
    /// CID -> DataRecord
    pub inventory: HashMap<String, DataRecord>,
}

impl RetentionManager {
    pub fn new() -> Self {
        Self {
            inventory: HashMap::new(),
        }
    }

    /// Admits new data into Hot storage.
    pub fn add_record(&mut self, cid: &str) {
        let record = DataRecord {
            cid: cid.to_string(),
            created_at: Utc::now(),
            tier: StorageTier::Hot,
        };
        self.inventory.insert(cid.to_string(), record);
    }

    /// Automatically migrates data based on age (Prompt 38 logic).
    pub fn update_tiers(&mut self) -> u32 {
        let mut migrations = 0;
        let now = Utc::now();
        
        let hot_cutoff = now - Duration::days(30);
        let warm_cutoff = now - Duration::days(1095); // 3 years

        for record in self.inventory.values_mut() {
            let old_tier = record.tier;
            
            if record.created_at < warm_cutoff {
                record.tier = StorageTier::Cold;
            } else if record.created_at < hot_cutoff {
                record.tier = StorageTier::Warm;
            } else {
                record.tier = StorageTier::Hot;
            }
            
            if old_tier != record.tier {
                migrations += 1;
            }
        }
        
        migrations
    }

    /// Simulates manual age setting for testing.
    pub fn simulate_aging(&mut self, cid: &str, age_days: i64) {
        if let Some(record) = self.inventory.get_mut(cid) {
            record.created_at = Utc::now() - Duration::days(age_days);
        }
    }

    pub fn get_tier(&self, cid: &str) -> Option<StorageTier> {
        self.inventory.get(cid).map(|r| r.tier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_migration() {
        let mut manager = RetentionManager::new();
        
        manager.add_record("QmHot");
        manager.add_record("QmWarmCandidate");
        manager.add_record("QmColdCandidate");
        
        // Simulate aging
        manager.simulate_aging("QmWarmCandidate", 31);
        manager.simulate_aging("QmColdCandidate", 1100);
        
        let migrations = manager.update_tiers();
        assert_eq!(migrations, 2);
        
        assert_eq!(manager.get_tier("QmHot"), Some(StorageTier::Hot));
        assert_eq!(manager.get_tier("QmWarmCandidate"), Some(StorageTier::Warm));
        assert_eq!(manager.get_tier("QmColdCandidate"), Some(StorageTier::Cold));
    }
}

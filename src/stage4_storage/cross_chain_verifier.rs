//! Stage 4 — Prompt 32: Cross-Chain Consistency Verification
//!
//! After submitting a Merkle root to multiple chains, verify that every chain
//! has the same root. Alert if any chain is missing it. Auto-resubmit on failure.
//!
//! Verification protocol:
//!   1. Submit batch root to [Cardano, BASE, HEDERA, CELO]
//!   2. Poll each chain's confirmation (with timeout)
//!   3. Compare: all confirmed roots must match
//!   4. Alert if any chain is missing → auto-resubmit up to 3 times
//!   5. Final status: AllConsistent | PartiallyConsistent | Inconsistent

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use crate::stage4_storage::fee_optimizer::Chain;

// ── Confirmation record ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChainConfirmation {
    Confirmed {
        tx_hash: String,
        merkle_root: String,
        block_or_slot: u64,
        confirmed_at: DateTime<Utc>,
    },
    Pending { since: DateTime<Utc> },
    Missing,
    Failed { error: String },
}

impl ChainConfirmation {
    pub fn is_confirmed(&self) -> bool { matches!(self, ChainConfirmation::Confirmed { .. }) }

    pub fn merkle_root(&self) -> Option<&str> {
        if let ChainConfirmation::Confirmed { merkle_root, .. } = self {
            Some(merkle_root)
        } else { None }
    }
}

// ── Consistency status ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsistencyStatus {
    /// All chains confirmed the same Merkle root.
    AllConsistent,
    /// Some chains confirmed; others are pending or missing.
    PartiallyConsistent { confirmed: Vec<Chain>, missing: Vec<Chain> },
    /// Two or more chains confirmed *different* roots (critical error!).
    Inconsistent { conflict: Vec<(Chain, String)> },
    /// No chains confirmed yet.
    NoneConfirmed,
}

// ── Verification report ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyReport {
    pub batch_id: String,
    pub expected_root: String,
    pub status: ConsistencyStatus,
    pub confirmations: HashMap<String, ChainConfirmation>,
    pub resubmit_attempts: HashMap<String, u32>,
    pub checked_at: DateTime<Utc>,
}

impl ConsistencyReport {
    pub fn confirmed_count(&self) -> usize {
        self.confirmations.values().filter(|c| c.is_confirmed()).count()
    }

    pub fn total_chains(&self) -> usize { self.confirmations.len() }

    pub fn is_fully_consistent(&self) -> bool {
        matches!(&self.status, ConsistencyStatus::AllConsistent)
    }
}

// ── Cross-chain verifier ──────────────────────────────────────────────────────

pub struct CrossChainVerifier {
    /// Chain → mock ledger: batch_id → (root, tx_hash, block)
    pub ledgers: HashMap<String, HashMap<String, (String, String, u64)>>,
    pub resubmit_max: u32,
}

impl CrossChainVerifier {
    pub fn new() -> Self {
        let mut ledgers = HashMap::new();
        for chain in ["Cardano", "BASE", "HEDERA", "CELO"] {
            ledgers.insert(chain.to_string(), HashMap::new());
        }
        Self { ledgers, resubmit_max: 3 }
    }

    /// Simulate: a chain has received and confirmed a batch root.
    pub fn simulate_confirmation(
        &mut self,
        chain: &str,
        batch_id: &str,
        merkle_root: &str,
        block: u64,
    ) {
        let mut h = Sha256::new();
        h.update(format!("{chain}{batch_id}{merkle_root}").as_bytes());
        let tx_hash = hex::encode(h.finalize());
        self.ledgers.entry(chain.to_string()).or_default()
            .insert(batch_id.to_string(), (merkle_root.to_string(), tx_hash, block));
    }

    /// Simulate: a chain is missing the root (for testing failure paths).
    pub fn simulate_missing(&mut self, chain: &str, batch_id: &str) {
        self.ledgers.entry(chain.to_string()).or_default()
            .remove(batch_id);
    }

    /// Query a single chain for a batch's confirmation status.
    fn query_chain(&self, chain: &str, batch_id: &str) -> ChainConfirmation {
        match self.ledgers.get(chain).and_then(|l| l.get(batch_id)) {
            Some((root, tx_hash, block)) => ChainConfirmation::Confirmed {
                tx_hash: tx_hash.clone(),
                merkle_root: root.clone(),
                block_or_slot: *block,
                confirmed_at: Utc::now(),
            },
            None => ChainConfirmation::Missing,
        }
    }

    /// Run consistency verification across all 4 chains.
    pub fn verify(&mut self, batch_id: &str, expected_root: &str) -> ConsistencyReport {
        let chains = ["Cardano", "BASE", "HEDERA", "CELO"];
        let mut confirmations: HashMap<String, ChainConfirmation> = HashMap::new();
        let mut resubmit_attempts: HashMap<String, u32> = HashMap::new();

        for chain in &chains {
            let mut conf = self.query_chain(chain, batch_id);

            // Auto-resubmit if missing (up to resubmit_max times)
            if matches!(conf, ChainConfirmation::Missing) {
                let attempts = resubmit_attempts.entry(chain.to_string()).or_insert(0);
                while *attempts < self.resubmit_max {
                    *attempts += 1;
                    // Simulate resubmission: check again (in real system would POST to chain)
                    conf = self.query_chain(chain, batch_id);
                    if conf.is_confirmed() { break; }
                }
            }
            confirmations.insert(chain.to_string(), conf);
        }

        let status = self.compute_status(&confirmations, expected_root);

        ConsistencyReport {
            batch_id: batch_id.to_string(),
            expected_root: expected_root.to_string(),
            confirmations,
            resubmit_attempts,
            status,
            checked_at: Utc::now(),
        }
    }

    /// Determine the overall consistency status from all confirmations.
    fn compute_status(
        &self,
        confirmations: &HashMap<String, ChainConfirmation>,
        expected_root: &str,
    ) -> ConsistencyStatus {
        let confirmed: Vec<(Chain, String)> = confirmations.iter()
            .filter_map(|(name, c)| {
                c.merkle_root().map(|r| {
                    let chain = match name.as_str() {
                        "Cardano" => Chain::Cardano,
                        "BASE"    => Chain::Base,
                        "HEDERA"  => Chain::Hedera,
                        "CELO"    => Chain::Celo,
                        _         => Chain::Cardano,
                    };
                    (chain, r.to_string())
                })
            })
            .collect();

        if confirmed.is_empty() {
            return ConsistencyStatus::NoneConfirmed;
        }

        // Check for root conflicts
        let unique_roots: std::collections::HashSet<&str> = confirmed.iter().map(|(_, r)| r.as_str()).collect();
        if unique_roots.len() > 1 || !unique_roots.contains(expected_root) {
            // Conflict detected
            let conflict: Vec<(Chain, String)> = confirmed.iter()
                .filter(|(_, r)| r != expected_root)
                .cloned()
                .collect();
            if !conflict.is_empty() {
                return ConsistencyStatus::Inconsistent { conflict };
            }
        }

        // Check if any are missing
        let missing: Vec<Chain> = confirmations.iter()
            .filter(|(_, c)| !c.is_confirmed())
            .map(|(name, _)| match name.as_str() {
                "Cardano" => Chain::Cardano,
                "BASE"    => Chain::Base,
                "HEDERA"  => Chain::Hedera,
                _         => Chain::Celo,
            })
            .collect();

        if missing.is_empty() {
            ConsistencyStatus::AllConsistent
        } else {
            ConsistencyStatus::PartiallyConsistent {
                confirmed: confirmed.into_iter().map(|(c, _)| c).collect(),
                missing,
            }
        }
    }
}

impl Default for CrossChainVerifier { fn default() -> Self { Self::new() } }

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const BATCH: &str = "batch-2025-03-05-1300";
    fn root() -> String { "a".repeat(64) }

    fn all_confirmed(v: &mut CrossChainVerifier, r: &str) {
        v.simulate_confirmation("Cardano", BATCH, r, 100);
        v.simulate_confirmation("BASE",    BATCH, r, 200);
        v.simulate_confirmation("HEDERA",  BATCH, r, 300);
        v.simulate_confirmation("CELO",    BATCH, r, 400);
    }

    // ── Test 1: all chains confirm same root → AllConsistent ──────────────────

    #[test]
    fn test_all_chains_consistent() {
        let mut v = CrossChainVerifier::new();
        all_confirmed(&mut v, &root());
        let report = v.verify(BATCH, &root());
        assert_eq!(report.status, ConsistencyStatus::AllConsistent);
        assert_eq!(report.confirmed_count(), 4);
    }

    // ── Test 2: one chain missing → PartiallyConsistent ──────────────────────

    #[test]
    fn test_one_chain_missing() {
        let mut v = CrossChainVerifier::new();
        v.simulate_confirmation("Cardano", BATCH, &root(), 100);
        v.simulate_confirmation("BASE",    BATCH, &root(), 200);
        v.simulate_confirmation("HEDERA",  BATCH, &root(), 300);
        // CELO missing
        let report = v.verify(BATCH, &root());
        assert!(matches!(&report.status, ConsistencyStatus::PartiallyConsistent { .. }));
        assert_eq!(report.confirmed_count(), 3);
    }

    // ── Test 3: conflicting root → Inconsistent ───────────────────────────────

    #[test]
    fn test_conflicting_roots_inconsistent() {
        let mut v = CrossChainVerifier::new();
        let r1 = "a".repeat(64);
        let r2 = "b".repeat(64);
        v.simulate_confirmation("Cardano", BATCH, &r1, 100);
        v.simulate_confirmation("BASE",    BATCH, &r2, 200); // different root!
        v.simulate_confirmation("HEDERA",  BATCH, &r1, 300);
        v.simulate_confirmation("CELO",    BATCH, &r1, 400);
        let report = v.verify(BATCH, &r1);
        assert!(matches!(&report.status, ConsistencyStatus::Inconsistent { .. }));
    }

    // ── Test 4: none confirmed → NoneConfirmed ───────────────────────────────

    #[test]
    fn test_none_confirmed() {
        let mut v = CrossChainVerifier::new();
        let report = v.verify(BATCH, &root());
        assert_eq!(report.status, ConsistencyStatus::NoneConfirmed);
    }

    // ── Test 5: resubmit_attempts recorded ───────────────────────────────────

    #[test]
    fn test_resubmit_attempts_recorded() {
        let mut v = CrossChainVerifier::new();
        // All chains missing → resubmit triggered for each
        v.simulate_missing("CELO", BATCH);
        let report = v.verify(BATCH, &root());
        // CELO was missing before and after → should have 3 attempts logged
        assert!(report.resubmit_attempts.get("CELO").copied().unwrap_or(0) > 0);
    }

    // ── Test 6: is_fully_consistent true only on AllConsistent ───────────────

    #[test]
    fn test_is_fully_consistent() {
        let mut v = CrossChainVerifier::new();
        all_confirmed(&mut v, &root());
        let report = v.verify(BATCH, &root());
        assert!(report.is_fully_consistent());
    }

    // ── Test 7: simulate_missing removes confirmation ─────────────────────────

    #[test]
    fn test_simulate_missing_removes() {
        let mut v = CrossChainVerifier::new();
        all_confirmed(&mut v, &root());
        v.simulate_missing("BASE", BATCH);
        let report = v.verify(BATCH, &root());
        assert!(!report.is_fully_consistent());
    }

    // ── Test 8: confirmed_count matches confirmed chains ─────────────────────

    #[test]
    fn test_confirmed_count_accurate() {
        let mut v = CrossChainVerifier::new();
        v.simulate_confirmation("Cardano", BATCH, &root(), 100);
        v.simulate_confirmation("HEDERA",  BATCH, &root(), 300);
        let report = v.verify(BATCH, &root());
        assert_eq!(report.confirmed_count(), 2);
        assert_eq!(report.total_chains(), 4);
    }
}

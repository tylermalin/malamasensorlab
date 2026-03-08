//! Stage 3 — Prompt 19: Validator Selection & Load Balancing
//!
//! Routes each batch to the fastest available validator.
//! Failover: if primary is offline, automatically selects next best.
//!
//! Selection criteria (in order):
//!   1. Health (must be online)
//!   2. Latency (lower is better)
//!   3. Load (fewer pending batches is better)
//!   4. Reputation score (higher is better)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

// ── Validator health status ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidatorHealth {
    Online,
    Degraded { reason: String },
    Offline,
}

impl ValidatorHealth {
    pub fn is_available(&self) -> bool {
        matches!(self, ValidatorHealth::Online | ValidatorHealth::Degraded { .. })
    }
}

// ── Validator candidate ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorCandidate {
    pub validator_id: String,
    pub health: ValidatorHealth,
    /// Average response time in milliseconds (lower = better).
    pub latency_ms: u64,
    /// Number of batches currently pending review.
    pub pending_batches: usize,
    /// Reputation 0.0–1.0 from reputation ledger.
    pub reputation_score: f64,
    pub last_seen: DateTime<Utc>,
}

impl ValidatorCandidate {
    /// Composite selection score — lower is better (like a cost function).
    /// Score = (latency_ms / 1000) + pending_batches * 0.5 + (1 - reputation_score) * 10
    pub fn selection_score(&self) -> f64 {
        self.latency_ms as f64 / 1000.0
            + self.pending_batches as f64 * 0.5
            + (1.0 - self.reputation_score.clamp(0.0, 1.0)) * 10.0
    }
}

// ── Validator selector ────────────────────────────────────────────────────────

pub struct ValidatorSelector {
    candidates: HashMap<String, ValidatorCandidate>,
}

impl ValidatorSelector {
    pub fn new() -> Self { Self { candidates: HashMap::new() } }

    /// Register or update a validator candidate.
    pub fn upsert(&mut self, candidate: ValidatorCandidate) {
        self.candidates.insert(candidate.validator_id.clone(), candidate);
    }

    /// Mark a validator offline.
    pub fn mark_offline(&mut self, validator_id: &str) {
        if let Some(c) = self.candidates.get_mut(validator_id) {
            c.health = ValidatorHealth::Offline;
        }
    }

    /// Mark a validator online.
    pub fn mark_online(&mut self, validator_id: &str) {
        if let Some(c) = self.candidates.get_mut(validator_id) {
            c.health = ValidatorHealth::Online;
            c.last_seen = Utc::now();
        }
    }

    /// Update latency measurement for a validator.
    pub fn update_latency(&mut self, validator_id: &str, latency_ms: u64) {
        if let Some(c) = self.candidates.get_mut(validator_id) {
            c.latency_ms = latency_ms;
        }
    }

    /// Select the best available validator (primary).
    /// Returns None if no validators are online.
    pub fn select_primary(&self) -> Option<&ValidatorCandidate> {
        self.candidates.values()
            .filter(|c| c.health.is_available())
            .min_by(|a, b| a.selection_score().partial_cmp(&b.selection_score()).unwrap())
    }

    /// Select the best N validators, excluding `excluded`.
    /// Used for failover: pass the primary's ID as excluded.
    pub fn select_failover(&self, excluded: &str, n: usize) -> Vec<&ValidatorCandidate> {
        let mut available: Vec<&ValidatorCandidate> = self.candidates.values()
            .filter(|c| c.health.is_available() && c.validator_id != excluded)
            .collect();
        available.sort_by(|a, b| a.selection_score().partial_cmp(&b.selection_score()).unwrap());
        available.into_iter().take(n).collect()
    }

    /// Route: returns [primary, ...failovers] ordered by selection score.
    /// Always returns at least 1 if any validator is online.
    pub fn route(&self, failover_count: usize) -> Vec<&ValidatorCandidate> {
        let mut available: Vec<&ValidatorCandidate> = self.candidates.values()
            .filter(|c| c.health.is_available())
            .collect();
        available.sort_by(|a, b| a.selection_score().partial_cmp(&b.selection_score()).unwrap());
        available.into_iter().take(1 + failover_count).collect()
    }

    pub fn online_count(&self) -> usize {
        self.candidates.values().filter(|c| c.health.is_available()).count()
    }
}

impl Default for ValidatorSelector { fn default() -> Self { Self::new() } }

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn candidate(id: &str, latency: u64, pending: usize, rep: f64, health: ValidatorHealth) -> ValidatorCandidate {
        ValidatorCandidate {
            validator_id: id.to_string(),
            health,
            latency_ms: latency,
            pending_batches: pending,
            reputation_score: rep,
            last_seen: Utc::now(),
        }
    }

    fn make_selector() -> ValidatorSelector {
        let mut sel = ValidatorSelector::new();
        sel.upsert(candidate("v1", 50,  2, 0.95, ValidatorHealth::Online));
        sel.upsert(candidate("v2", 120, 1, 0.90, ValidatorHealth::Online));
        sel.upsert(candidate("v3", 80,  5, 0.85, ValidatorHealth::Online));
        sel
    }

    // ── Test 1: fastest validator selected as primary ─────────────────────────

    #[test]
    fn test_fastest_selected_as_primary() {
        let sel = make_selector();
        let primary = sel.select_primary().unwrap();
        // v1: score = 50/1000 + 2*0.5 + (1-0.95)*10 = 0.05 + 1.0 + 0.5 = 1.55
        // v2: score = 0.12 + 0.5 + 1.0 = 1.62
        // v3: score = 0.08 + 2.5 + 1.5 = 4.08
        assert_eq!(primary.validator_id, "v1", "v1 must have lowest score");
    }

    // ── Test 2: offline validator excluded from selection ─────────────────────

    #[test]
    fn test_offline_validator_excluded() {
        let mut sel = make_selector();
        sel.mark_offline("v1");
        let primary = sel.select_primary().unwrap();
        assert_ne!(primary.validator_id, "v1", "Offline v1 must not be selected");
    }

    // ── Test 3: failover excludes primary ─────────────────────────────────────

    #[test]
    fn test_failover_excludes_primary() {
        let sel = make_selector();
        let failovers = sel.select_failover("v1", 2);
        assert!(!failovers.iter().any(|c| c.validator_id == "v1"));
        assert_eq!(failovers.len(), 2);
    }

    // ── Test 4: mark_online restores validator ────────────────────────────────

    #[test]
    fn test_mark_online_restores() {
        let mut sel = make_selector();
        sel.mark_offline("v1");
        assert_eq!(sel.online_count(), 2);
        sel.mark_online("v1");
        assert_eq!(sel.online_count(), 3);
    }

    // ── Test 5: all offline returns None ─────────────────────────────────────

    #[test]
    fn test_all_offline_returns_none() {
        let mut sel = make_selector();
        sel.mark_offline("v1");
        sel.mark_offline("v2");
        sel.mark_offline("v3");
        assert!(sel.select_primary().is_none());
    }

    // ── Test 6: route returns primary + failovers ─────────────────────────────

    #[test]
    fn test_route_returns_primary_and_failovers() {
        let sel = make_selector();
        let route = sel.route(2);
        assert_eq!(route.len(), 3, "1 primary + 2 failovers");
        assert_eq!(route[0].validator_id, "v1", "First must be primary");
    }

    // ── Test 7: high latency increases score ──────────────────────────────────

    #[test]
    fn test_high_latency_increases_score() {
        let low  = candidate("low",  50,  0, 1.0, ValidatorHealth::Online);
        let high = candidate("high", 5000, 0, 1.0, ValidatorHealth::Online);
        assert!(high.selection_score() > low.selection_score());
    }

    // ── Test 8: update_latency changes score ──────────────────────────────────

    #[test]
    fn test_update_latency() {
        let mut sel = make_selector();
        sel.update_latency("v2", 10);
        let primary = sel.select_primary().unwrap();
        assert_eq!(primary.validator_id, "v2", "After latency update v2 should win");
    }
}

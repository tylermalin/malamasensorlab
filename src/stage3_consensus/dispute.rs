//! Stage 3 — Prompt 23: Dispute Resolution
//!
//! When 2-of-3 validators approve but 1 rejects, the disagreement is logged,
//! investigated, and either closed or escalated to manual review.
//!
//! Resolution pipeline:
//!   1. Detect: quorum reached BUT at least one dissenter
//!   2. Log: create DisputeRecord with full context
//!   3. Auto-investigate: re-check signatures, replay batch validation
//!   4. Verdict: AutoResolved (batch fine) | Escalated (manual review needed)
//!   5. Audit trail: all disputes immutably recorded

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// ── Dispute status ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisputeStatus {
    /// Freshly opened — awaiting investigation.
    Open,
    /// Auto-investigation completed, batch confirmed valid. Dissenter flagged.
    AutoResolved { finding: String },
    /// Requires human review — batch held pending decision.
    Escalated { reason: String },
    /// Human reviewer closed the dispute.
    ClosedByReviewer { decision: String, reviewer: String },
    /// Dissenting validator found to be acting dishonestly — slashing triggered.
    ValidatorPenalized { validator_id: String, penalty: String },
}

// ── Dispute record ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeRecord {
    pub dispute_id: String,
    pub batch_id: String,
    pub merkle_root: String,
    /// Validators who approved.
    pub approvers: Vec<String>,
    /// Validators who rejected.
    pub dissenters: Vec<String>,
    pub status: DisputeStatus,
    pub opened_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Audit trail of status changes.
    pub history: Vec<(DisputeStatus, DateTime<Utc>)>,
}

impl DisputeRecord {
    pub fn new(batch_id: &str, merkle_root: &str, approvers: Vec<String>, dissenters: Vec<String>) -> Self {
        let now = Utc::now();
        let dispute_id = format!("dispute-{batch_id}");
        Self {
            dispute_id,
            batch_id: batch_id.to_string(),
            merkle_root: merkle_root.to_string(),
            approvers,
            dissenters,
            status: DisputeStatus::Open,
            opened_at: now,
            updated_at: now,
            history: Vec::new(),
        }
    }

    /// Transition to a new status, appending to history.
    pub fn transition(&mut self, new_status: DisputeStatus) {
        let now = Utc::now();
        let old = std::mem::replace(&mut self.status, new_status);
        self.history.push((old, now));
        self.updated_at = now;
    }

    pub fn is_resolved(&self) -> bool {
        matches!(
            &self.status,
            DisputeStatus::AutoResolved { .. }
            | DisputeStatus::ClosedByReviewer { .. }
            | DisputeStatus::ValidatorPenalized { .. }
        )
    }

    pub fn dissenter_count(&self) -> usize { self.dissenters.len() }
}

// ── Auto-investigation ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvestigationResult {
    BatchValid   { finding: String },
    BatchSuspect { reason: String },
}

/// Auto-investigate a dispute.
///
/// Rules:
/// - If only 1 dissenter and they have a known history of lone dissents → AutoResolve
/// - If the dissenter's signature is invalid → ValidatorPenalized
/// - If merkle_root doesn't match expected → Escalate
/// - Default: Escalate for manual review
pub fn auto_investigate(
    dispute: &mut DisputeRecord,
    dissenter_has_bad_history: bool,
    dissenter_signature_valid: bool,
    merkle_root_matches: bool,
) -> InvestigationResult {
    if !dissenter_signature_valid {
        let penalty = "Submitted invalid/forged signature during dispute".to_string();
        let vid = dispute.dissenters.first().cloned().unwrap_or_default();
        dispute.transition(DisputeStatus::ValidatorPenalized {
            validator_id: vid,
            penalty: penalty.clone(),
        });
        return InvestigationResult::BatchValid { finding: "Dissenter signature was invalid — auto-resolved".to_string() };
    }

    if !merkle_root_matches {
        dispute.transition(DisputeStatus::Escalated {
            reason: "Merkle root mismatch — batch may be corrupted".to_string(),
        });
        return InvestigationResult::BatchSuspect {
            reason: "Merkle root does not match batch".to_string(),
        };
    }

    if dissenter_has_bad_history {
        dispute.transition(DisputeStatus::AutoResolved {
            finding: "Dissenter has prior lone-dissent history — batch auto-approved".to_string(),
        });
        return InvestigationResult::BatchValid {
            finding: "Known dissenter pattern — resolved automatically".to_string(),
        };
    }

    // Default: escalate
    dispute.transition(DisputeStatus::Escalated {
        reason: "No automatic resolution identified — escalating to manual review".to_string(),
    });
    InvestigationResult::BatchSuspect {
        reason: "Requires manual review".to_string(),
    }
}

// ── Dispute registry ─────────────────────────────────────────────────────────

pub struct DisputeRegistry {
    disputes: HashMap<String, DisputeRecord>,
}

impl DisputeRegistry {
    pub fn new() -> Self { Self { disputes: HashMap::new() } }

    /// Open a new dispute for a batch.
    pub fn open(&mut self, batch_id: &str, merkle_root: &str, approvers: Vec<String>, dissenters: Vec<String>) -> &DisputeRecord {
        let record = DisputeRecord::new(batch_id, merkle_root, approvers, dissenters);
        let id = record.dispute_id.clone();
        self.disputes.insert(id, record);
        self.disputes.get(&format!("dispute-{batch_id}")).unwrap()
    }

    pub fn get(&self, batch_id: &str) -> Option<&DisputeRecord> {
        self.disputes.get(&format!("dispute-{batch_id}"))
    }

    pub fn get_mut(&mut self, batch_id: &str) -> Option<&mut DisputeRecord> {
        self.disputes.get_mut(&format!("dispute-{batch_id}"))
    }

    /// All open disputes.
    pub fn open_disputes(&self) -> Vec<&DisputeRecord> {
        self.disputes.values().filter(|d| !d.is_resolved()).collect()
    }

    /// All escalated disputes.
    pub fn escalated(&self) -> Vec<&DisputeRecord> {
        self.disputes.values()
            .filter(|d| matches!(&d.status, DisputeStatus::Escalated { .. }))
            .collect()
    }

    pub fn total(&self) -> usize { self.disputes.len() }
}

impl Default for DisputeRegistry { fn default() -> Self { Self::new() } }

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const BATCH: &str = "batch-2025-03-05-1300";
    fn root() -> String { "a".repeat(64) }

    // ── Test 1: dispute opened with status Open ───────────────────────────────

    #[test]
    fn test_dispute_opens_as_open() {
        let record = DisputeRecord::new(BATCH, &root(),
            vec!["v1".to_string(), "v2".to_string()],
            vec!["v3".to_string()]);
        assert_eq!(record.status, DisputeStatus::Open);
        assert!(!record.is_resolved());
        assert_eq!(record.dissenter_count(), 1);
    }

    // ── Test 2: auto-resolve when dissenter signature invalid ─────────────────

    #[test]
    fn test_auto_resolve_invalid_signature() {
        let mut record = DisputeRecord::new(BATCH, &root(),
            vec!["v1".to_string(), "v2".to_string()],
            vec!["v3".to_string()]);
        let result = auto_investigate(&mut record,
            false, // no bad history
            false, // invalid sig → penalize
            true);
        assert!(matches!(result, InvestigationResult::BatchValid { .. }));
        assert!(matches!(&record.status, DisputeStatus::ValidatorPenalized { .. }));
    }

    // ── Test 3: escalate when merkle root mismatch ────────────────────────────

    #[test]
    fn test_escalate_on_merkle_mismatch() {
        let mut record = DisputeRecord::new(BATCH, &root(),
            vec!["v1".to_string(), "v2".to_string()],
            vec!["v3".to_string()]);
        let result = auto_investigate(&mut record,
            false,
            true,
            false); // merkle mismatch
        assert!(matches!(result, InvestigationResult::BatchSuspect { .. }));
        assert!(matches!(&record.status, DisputeStatus::Escalated { .. }));
    }

    // ── Test 4: auto-resolve known bad-history dissenter ─────────────────────

    #[test]
    fn test_auto_resolve_bad_history() {
        let mut record = DisputeRecord::new(BATCH, &root(),
            vec!["v1".to_string(), "v2".to_string()],
            vec!["v3".to_string()]);
        let result = auto_investigate(&mut record,
            true, // bad history → auto-resolve
            true,
            true);
        assert!(matches!(result, InvestigationResult::BatchValid { .. }));
        assert!(matches!(&record.status, DisputeStatus::AutoResolved { .. }));
    }

    // ── Test 5: default path escalates ───────────────────────────────────────

    #[test]
    fn test_default_escalates() {
        let mut record = DisputeRecord::new(BATCH, &root(),
            vec!["v1".to_string(), "v2".to_string()],
            vec!["v3".to_string()]);
        let result = auto_investigate(&mut record,
            false, // no bad history
            true,  // valid sig
            true); // merkle ok → default: escalate
        assert!(matches!(result, InvestigationResult::BatchSuspect { .. }));
        assert!(matches!(&record.status, DisputeStatus::Escalated { .. }));
    }

    // ── Test 6: history records all transitions ───────────────────────────────

    #[test]
    fn test_history_records_transitions() {
        let mut record = DisputeRecord::new(BATCH, &root(), vec![], vec!["v3".to_string()]);
        record.transition(DisputeStatus::Escalated { reason: "test".to_string() });
        record.transition(DisputeStatus::ClosedByReviewer {
            decision: "approved".to_string(), reviewer: "alice".to_string()
        });
        assert_eq!(record.history.len(), 2);
        assert!(record.is_resolved());
    }

    // ── Test 7: registry tracks open and escalated ────────────────────────────

    #[test]
    fn test_registry_open_and_escalated() {
        let mut reg = DisputeRegistry::new();
        reg.open(BATCH, &root(), vec!["v1".to_string(), "v2".to_string()], vec!["v3".to_string()]);
        assert_eq!(reg.open_disputes().len(), 1);

        // Escalate
        let dispute = reg.get_mut(BATCH).unwrap();
        dispute.transition(DisputeStatus::Escalated { reason: "manual review".to_string() });
        assert_eq!(reg.escalated().len(), 1);
    }

    // ── Test 8: resolved dispute removed from open ────────────────────────────

    #[test]
    fn test_resolved_removed_from_open() {
        let mut reg = DisputeRegistry::new();
        reg.open("b1", &root(), vec![], vec!["v3".to_string()]);
        reg.get_mut("b1").unwrap().transition(DisputeStatus::AutoResolved {
            finding: "ok".to_string()
        });
        assert_eq!(reg.open_disputes().len(), 0);
    }

    // ── Test 9: multiple disputes tracked independently ───────────────────────

    #[test]
    fn test_multiple_disputes_independent() {
        let mut reg = DisputeRegistry::new();
        reg.open("b1", &root(), vec!["v1".to_string(), "v2".to_string()], vec!["v3".to_string()]);
        reg.open("b2", &root(), vec!["v1".to_string(), "v3".to_string()], vec!["v2".to_string()]);
        assert_eq!(reg.total(), 2);
        assert_eq!(reg.open_disputes().len(), 2);
    }
}

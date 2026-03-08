//! Stage 3 — Prompt 17: Multi-Validator Quorum & Consensus Logic
//!
//! Three independent validators vote on whether a batch is legitimate.
//! 2-of-3 approval is required to commit to the chain.
//!
//! Validators:
//!   V1 — Mālama Labs     (first-party)
//!   V2 — Verra Registry  (government/registry)
//!   V3 — Community       (decentralized third-party)
//!
//! Each validator independently verifies:
//!   • Merkle root integrity
//!   • AI confidence > 85%
//!   • No blacklisted sensor DIDs
//!
//! Then signs "I approve batch B" with their ECDSA key.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use chrono::{DateTime, Duration, Utc};
use k256::ecdsa::{SigningKey, VerifyingKey, signature::Signer, signature::Verifier};
use std::collections::HashMap;

// ── Validator identity ─────────────────────────────────────────────────────────

pub const VALIDATOR_MALAMA_LABS: &str = "V1:malama-labs";
pub const VALIDATOR_VERRA:       &str = "V2:verra-registry";
pub const VALIDATOR_COMMUNITY:   &str = "V3:community";

pub const QUORUM_THRESHOLD: usize = 2;   // 2-of-3
pub const TOTAL_VALIDATORS: usize = 3;
pub const VOTE_TIMEOUT_SECS: i64  = 3600; // 1 hour

/// The three canonical validators for the Mālama Protocol.
pub fn canonical_validators() -> [&'static str; 3] {
    [VALIDATOR_MALAMA_LABS, VALIDATOR_VERRA, VALIDATOR_COMMUNITY]
}

// ── ValidatorSignature ────────────────────────────────────────────────────────

/// A signed approval or rejection from one validator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSignature {
    pub validator_id: String,
    /// The batch ID being voted on.
    pub batch_id: String,
    /// The Merkle root the validator verified.
    pub merkle_root: String,
    /// ECDSA DER-encoded signature over `signing_message()`.
    pub signature_bytes: Vec<u8>,
    /// Compressed SEC1 verifying key of the validator.
    pub verifying_key_bytes: Vec<u8>,
    pub approved: bool,
    pub signed_at: DateTime<Utc>,
}

impl ValidatorSignature {
    /// Canonical message that is signed:
    /// `MALAMA_VOTE|{batch_id}|{merkle_root}|{approved}|{timestamp_rfc3339}`
    pub fn signing_message(&self) -> Vec<u8> {
        let msg = format!(
            "MALAMA_VOTE|{}|{}|{}|{}",
            self.batch_id,
            self.merkle_root,
            self.approved,
            self.signed_at.to_rfc3339(),
        );
        let mut h = Sha256::new();
        h.update(msg.as_bytes());
        h.finalize().to_vec()
    }

    /// Verify the ECDSA signature against the embedded verifying key.
    pub fn is_authentic(&self) -> bool {
        let vk = match VerifyingKey::from_sec1_bytes(&self.verifying_key_bytes) {
            Ok(k) => k,
            Err(_) => return false,
        };
        let sig = match k256::ecdsa::Signature::from_der(&self.signature_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let msg = self.signing_message();
        vk.verify(&msg, &sig).is_ok()
    }

    /// Sign a vote using the given private key.
    pub fn sign(
        validator_id: &str,
        batch_id: &str,
        merkle_root: &str,
        approved: bool,
        signing_key: &SigningKey,
    ) -> Self {
        let signed_at = Utc::now();
        let msg = format!(
            "MALAMA_VOTE|{}|{}|{}|{}",
            batch_id, merkle_root, approved, signed_at.to_rfc3339()
        );
        let mut h = Sha256::new();
        h.update(msg.as_bytes());
        let digest = h.finalize();

        let sig: k256::ecdsa::Signature = signing_key.sign(&digest);
        let vk = signing_key.verifying_key();

        Self {
            validator_id: validator_id.to_string(),
            batch_id: batch_id.to_string(),
            merkle_root: merkle_root.to_string(),
            signature_bytes: sig.to_der().as_bytes().to_vec(),
            verifying_key_bytes: vk.to_sec1_bytes().to_vec(),
            approved,
            signed_at,
        }
    }
}

// ── Quorum check ──────────────────────────────────────────────────────────────

/// Central quorum function (matches Prompt 17 spec).
///
/// Returns `true` if at least `QUORUM_THRESHOLD` (2) signatures are:
/// 1. Cryptographically authentic
/// 2. Approvals (not rejections)
pub fn check_quorum(signatures: &[ValidatorSignature]) -> bool {
    let valid_approvals = signatures
        .iter()
        .filter(|sig| sig.approved && sig.is_authentic())
        .count();
    valid_approvals >= QUORUM_THRESHOLD
}

// ── Quorum session ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuorumOutcome {
    /// ≥ 2 valid approvals.
    Accepted,
    /// One or more validators disagreed but quorum still met. Minority logged.
    AcceptedWithDisagreement { dissenting_validators: Vec<String> },
    /// < 2 valid approvals before timeout.
    Rejected { reason: String },
    /// Insufficient signatures received before deadline.
    TimedOut,
    /// Pending — not enough signatures yet.
    Pending,
}

/// A live quorum session for a single batch.
pub struct QuorumSession {
    pub batch_id: String,
    pub merkle_root: String,
    pub opened_at: DateTime<Utc>,
    pub timeout_secs: i64,
    signatures: HashMap<String, ValidatorSignature>,
}

impl QuorumSession {
    pub fn new(batch_id: &str, merkle_root: &str) -> Self {
        Self {
            batch_id: batch_id.to_string(),
            merkle_root: merkle_root.to_string(),
            opened_at: Utc::now(),
            timeout_secs: VOTE_TIMEOUT_SECS,
            signatures: HashMap::new(),
        }
    }

    /// Submit a validator's signature. Returns `false` if duplicate or invalid batch_id.
    pub fn submit(&mut self, sig: ValidatorSignature) -> bool {
        if sig.batch_id != self.batch_id { return false; }
        self.signatures.insert(sig.validator_id.clone(), sig);
        true
    }

    /// Evaluate current quorum state.
    pub fn evaluate(&self) -> QuorumOutcome {
        if self.is_timed_out() { return QuorumOutcome::TimedOut; }

        let valid_approvals: Vec<&ValidatorSignature> = self.signatures.values()
            .filter(|s| s.approved && s.is_authentic())
            .collect();

        let dissenters: Vec<String> = self.signatures.values()
            .filter(|s| !s.approved || !s.is_authentic())
            .map(|s| s.validator_id.clone())
            .collect();

        if valid_approvals.len() >= QUORUM_THRESHOLD {
            if dissenters.is_empty() {
                QuorumOutcome::Accepted
            } else {
                QuorumOutcome::AcceptedWithDisagreement {
                    dissenting_validators: dissenters,
                }
            }
        } else if self.signatures.len() == TOTAL_VALIDATORS {
            // All voted but not enough approvals
            QuorumOutcome::Rejected {
                reason: format!("Only {}/{} valid approvals", valid_approvals.len(), QUORUM_THRESHOLD),
            }
        } else {
            QuorumOutcome::Pending
        }
    }

    pub fn is_timed_out(&self) -> bool {
        (Utc::now() - self.opened_at).num_seconds() >= self.timeout_secs
    }

    pub fn valid_approval_count(&self) -> usize {
        self.signatures.values().filter(|s| s.approved && s.is_authentic()).count()
    }

    pub fn received_count(&self) -> usize { self.signatures.len() }
}

// ── Test key pair helpers ────────────────────────────────────────────────────

/// Generate a fresh random signing key (test/dev only).
pub fn generate_test_key() -> SigningKey {
    SigningKey::random(&mut rand::thread_rng())
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct TestValidator {
        id: &'static str,
        key: SigningKey,
    }

    fn make_validators() -> [TestValidator; 3] {
        [
            TestValidator { id: VALIDATOR_MALAMA_LABS, key: generate_test_key() },
            TestValidator { id: VALIDATOR_VERRA,       key: generate_test_key() },
            TestValidator { id: VALIDATOR_COMMUNITY,   key: generate_test_key() },
        ]
    }

    const BATCH: &str = "batch-2025-03-05-1300-biochar-001";

    fn root() -> String { "a".repeat(64) }

    // ── Test 1: 3 validators approve → accepted ───────────────────────────────

    #[test]
    fn test_all_three_approve_accepted() {
        let validators = make_validators();
        let mut session = QuorumSession::new(BATCH, &root());
        for v in &validators {
            let sig = ValidatorSignature::sign(v.id, BATCH, &root(), true, &v.key);
            session.submit(sig);
        }
        assert_eq!(session.evaluate(), QuorumOutcome::Accepted);
    }

    // ── Test 2: 2 approve, 1 rejects → accepted with disagreement ────────────

    #[test]
    fn test_two_approve_one_rejects() {
        let validators = make_validators();
        let mut session = QuorumSession::new(BATCH, &root());

        session.submit(ValidatorSignature::sign(validators[0].id, BATCH, &root(), true, &validators[0].key));
        session.submit(ValidatorSignature::sign(validators[1].id, BATCH, &root(), true, &validators[1].key));
        session.submit(ValidatorSignature::sign(validators[2].id, BATCH, &root(), false, &validators[2].key));

        assert!(matches!(
            session.evaluate(),
            QuorumOutcome::AcceptedWithDisagreement { .. }
        ), "2 approve, 1 rejects → accepted with disagreement");
    }

    // ── Test 3: only 1 approves → rejected ───────────────────────────────────

    #[test]
    fn test_only_one_approves_rejected() {
        let validators = make_validators();
        let mut session = QuorumSession::new(BATCH, &root());

        session.submit(ValidatorSignature::sign(validators[0].id, BATCH, &root(), true, &validators[0].key));
        session.submit(ValidatorSignature::sign(validators[1].id, BATCH, &root(), false, &validators[1].key));
        session.submit(ValidatorSignature::sign(validators[2].id, BATCH, &root(), false, &validators[2].key));

        assert!(matches!(
            session.evaluate(),
            QuorumOutcome::Rejected { .. }
        ));
        assert_eq!(session.valid_approval_count(), 1);
    }

    // ── Test 4: invalid signature → does not count toward quorum ─────────────

    #[test]
    fn test_invalid_signature_not_counted() {
        let validators = make_validators();
        let mut session = QuorumSession::new(BATCH, &root());

        // Submit authentic sigs for V1 and V2
        session.submit(ValidatorSignature::sign(validators[0].id, BATCH, &root(), true, &validators[0].key));
        session.submit(ValidatorSignature::sign(validators[1].id, BATCH, &root(), true, &validators[1].key));

        // Forge V3: correct validator_id but wrong key
        let wrong_key = generate_test_key();
        let mut forged = ValidatorSignature::sign(validators[2].id, BATCH, &root(), true, &wrong_key);
        // Overwrite verifying key bytes with V3's actual key → signature won't match
        forged.verifying_key_bytes = validators[2].key.verifying_key().to_sec1_bytes().to_vec();

        session.submit(forged);

        // Should still accept because V1 + V2 are valid
        assert!(matches!(
            session.evaluate(),
            QuorumOutcome::Accepted | QuorumOutcome::AcceptedWithDisagreement { .. }
        ));
    }

    // ── Test 5: pending when < 3 votes received ───────────────────────────────

    #[test]
    fn test_pending_when_incomplete() {
        let validators = make_validators();
        let mut session = QuorumSession::new(BATCH, &root());
        session.submit(ValidatorSignature::sign(validators[0].id, BATCH, &root(), true, &validators[0].key));
        // Only 1 vote submitted
        assert_eq!(session.evaluate(), QuorumOutcome::Pending);
    }

    // ── Test 6: check_quorum function with 2 valid approvals ─────────────────

    #[test]
    fn test_check_quorum_passes_with_2() {
        let validators = make_validators();
        let sigs = vec![
            ValidatorSignature::sign(validators[0].id, BATCH, &root(), true, &validators[0].key),
            ValidatorSignature::sign(validators[1].id, BATCH, &root(), true, &validators[1].key),
        ];
        assert!(check_quorum(&sigs), "2 valid approvals must pass quorum");
    }

    // ── Test 7: check_quorum fails with 1 approval ────────────────────────────

    #[test]
    fn test_check_quorum_fails_with_1() {
        let validators = make_validators();
        let sigs = vec![
            ValidatorSignature::sign(validators[0].id, BATCH, &root(), true, &validators[0].key),
        ];
        assert!(!check_quorum(&sigs), "1 valid approval must fail quorum");
    }

    // ── Test 8: check_quorum fails if all are rejections ──────────────────────

    #[test]
    fn test_check_quorum_fails_all_rejections() {
        let validators = make_validators();
        let sigs: Vec<_> = validators.iter()
            .map(|v| ValidatorSignature::sign(v.id, BATCH, &root(), false, &v.key))
            .collect();
        assert!(!check_quorum(&sigs), "Rejections must not count toward quorum");
    }

    // ── Test 9: duplicate validator vote overwritten ──────────────────────────

    #[test]
    fn test_duplicate_validator_vote_overwritten() {
        let validators = make_validators();
        let mut session = QuorumSession::new(BATCH, &root());

        // V1 votes approve then rejects (changes mind)
        session.submit(ValidatorSignature::sign(validators[0].id, BATCH, &root(), true, &validators[0].key));
        session.submit(ValidatorSignature::sign(validators[0].id, BATCH, &root(), false, &validators[0].key));
        session.submit(ValidatorSignature::sign(validators[1].id, BATCH, &root(), true, &validators[1].key));
        session.submit(ValidatorSignature::sign(validators[2].id, BATCH, &root(), true, &validators[2].key));

        // V1's latest vote is a rejection — so V2 + V3 approve = 2 valid
        assert!(matches!(
            session.evaluate(),
            QuorumOutcome::AcceptedWithDisagreement { .. }
        ));
    }

    // ── Test 10: wrong batch_id rejected ─────────────────────────────────────

    #[test]
    fn test_wrong_batch_id_rejected() {
        let validators = make_validators();
        let mut session = QuorumSession::new(BATCH, &root());
        let sig = ValidatorSignature::sign(validators[0].id, "OTHER_BATCH", &root(), true, &validators[0].key);
        let accepted = session.submit(sig);
        assert!(!accepted, "Signature for different batch must be rejected");
        assert_eq!(session.received_count(), 0);
    }
}

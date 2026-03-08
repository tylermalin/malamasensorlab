//! Stage 1 — DID Lifecycle State Machine (Prompt 8)
//!
//! Full 6-state lifecycle for a sensor DID:
//!
//! ```text
//! UNREGISTERED ──► REGISTERED ──► ACTIVE ──────────► RETIRED
//!                      │              │
//!                      │           OFFLINE
//!                      │           /    \
//!                      └──────────  QUARANTINED ──► RETIRED
//! ```
//!
//! Every transition is:
//! - Guarded (only legal transitions allowed)
//! - Signed by an admin ECDSA key
//! - Logged to an append-only, on-chain-anchored event history
//!
//! Timeouts:
//! - REGISTERED but not activated within `activation_timeout_secs` → auto-RETIRED
//! - OFFLINE for longer than `offline_timeout_secs` → eligible for QUARANTINED
//! - QUARANTINED for longer than `quarantine_timeout_secs` → eligible for RETIRED
//!
//! # Narrative
//! "A sensor's life is a story: born, registered, active, sometimes offline,
//!  sometimes under investigation, and eventually retired — every chapter signed
//!  and immutably recorded."

use k256::ecdsa::{SigningKey, VerifyingKey, Signature, signature::{Signer, Verifier}};
use std::str::FromStr;
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};

// ── States ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SensorState {
    /// DID generated but not yet anchored on-chain.
    UNREGISTERED,
    /// NFT minted on Cardano; waiting for physical installation confirmation.
    REGISTERED,
    /// Sensor is live and sending valid readings.
    ACTIVE,
    /// Sensor missed its heartbeat window (expected < 5 min gap, saw > 1 hour).
    OFFLINE,
    /// Tampering or multiple signature failures detected — readings suspended.
    QUARANTINED,
    /// End of life; no further transitions allowed (terminal state).
    RETIRED,
}

impl SensorState {
    /// Human-readable description of the state.
    pub fn description(&self) -> &'static str {
        match self {
            SensorState::UNREGISTERED  => "DID generated, not yet on-chain",
            SensorState::REGISTERED    => "NFT minted, awaiting field activation",
            SensorState::ACTIVE        => "Live — sending valid readings",
            SensorState::OFFLINE       => "Heartbeat missed — temporarily offline",
            SensorState::QUARANTINED   => "Tamper/failure detected — readings suspended",
            SensorState::RETIRED       => "End of life — terminal state",
        }
    }

    /// True if this is a terminal state (no further transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(self, SensorState::RETIRED)
    }
}

// ── Legal transition graph ────────────────────────────────────────────────────

/// Returns true if transitioning from `from` → `to` is permitted.
///
/// This mirrors the Plutus validator's `assertValidTransition`.
pub fn is_legal_transition(from: SensorState, to: SensorState) -> bool {
    use SensorState::*;
    matches!(
        (from, to),
        (UNREGISTERED,  REGISTERED)   |  // NFT minted
        (REGISTERED,    ACTIVE)        |  // Sensor powered on and confirmed
        (REGISTERED,    RETIRED)       |  // Never activated — retired early
        (ACTIVE,        OFFLINE)       |  // Heartbeat missed
        (ACTIVE,        QUARANTINED)   |  // Tampering detected while active
        (ACTIVE,        RETIRED)       |  // Planned retirement
        (OFFLINE,       ACTIVE)        |  // Reconnected
        (OFFLINE,       QUARANTINED)   |  // Offline too long or tampering while offline
        (OFFLINE,       RETIRED)       |  // Abandoned
        (QUARANTINED,   ACTIVE)        |  // Investigation cleared, sensor good
        (QUARANTINED,   RETIRED)          // Unrecoverable tampering
    )
}

// ── Transition reasons ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransitionReason {
    NFTMinted,
    FieldActivationConfirmed,
    HeartbeatMissed { last_seen_secs_ago: u64 },
    TamperingDetected { evidence_cid: Option<String> },
    ConnectionRestored,
    InvestigationCleared,
    PlannedRetirement,
    AbandonedOffline,
    ActivationTimeout,
    QuarantineTimeout,
    ManualAdmin { note: String },
}

// ── State transition event ────────────────────────────────────────────────────

/// One entry in the immutable transition log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: SensorState,
    pub to: SensorState,
    pub reason: TransitionReason,
    pub timestamp: DateTime<Utc>,
    /// Hex-encoded ECDSA signature by the admin key over the canonical message.
    pub admin_signature: String,
    /// Tx ID when this event is anchored on-chain (set after Cardano submission).
    pub on_chain_tx_id: Option<String>,
}

// ── Signing helpers ───────────────────────────────────────────────────────────

/// Canonical signing message for a state transition.
///
/// Format: `"<sensor_did>|<FROM>|<TO>|<rfc3339_timestamp>"`
pub fn transition_signing_message(
    sensor_did: &str,
    from: SensorState,
    to: SensorState,
    timestamp: &DateTime<Utc>,
) -> String {
    format!("{sensor_did}|{from:?}|{to:?}|{}", timestamp.to_rfc3339())
}

pub fn sign_transition(
    sensor_did: &str,
    from: SensorState,
    to: SensorState,
    timestamp: &DateTime<Utc>,
    admin_key: &SigningKey,
) -> String {
    let msg = transition_signing_message(sensor_did, from, to, timestamp);
    let sig: Signature = admin_key.sign(msg.as_bytes());
    hex::encode(sig.to_bytes())
}

// ── Sensor DID lifecycle ──────────────────────────────────────────────────────

/// Full lifecycle tracker for a single sensor DID.
pub struct SensorLifecycle {
    pub did: String,
    state: SensorState,
    /// ECDSA key that must sign every transition (admin / ops key).
    pub admin_key: VerifyingKey,
    /// Append-only transition history.
    pub history: Vec<StateTransition>,
    /// Timeout configuration.
    pub activation_timeout_secs: i64,
    pub offline_timeout_secs: i64,
    pub quarantine_timeout_secs: i64,
}

impl SensorLifecycle {
    pub fn new(did: impl Into<String>, admin_key: VerifyingKey) -> Self {
        Self {
            did: did.into(),
            state: SensorState::UNREGISTERED,
            admin_key,
            history: Vec::new(),
            activation_timeout_secs: 30 * 24 * 3600, // 30 days default
            offline_timeout_secs: 7 * 24 * 3600,     // 7 days
            quarantine_timeout_secs: 14 * 24 * 3600, // 14 days
        }
    }

    pub fn state(&self) -> SensorState { self.state }

    /// Attempt a signed state transition.
    ///
    /// Validates:
    /// 1. Transition is legal (graph guard)
    /// 2. Target is not a repeated terminal state
    /// 3. Admin ECDSA signature is valid
    pub fn transition(
        &mut self,
        to: SensorState,
        reason: TransitionReason,
        timestamp: DateTime<Utc>,
        admin_sig_hex: &str,
    ) -> Result<(), String> {
        let from = self.state;

        // Guard 1: legal transition
        if !is_legal_transition(from, to) {
            return Err(format!("Illegal transition: {from:?} → {to:?}"));
        }

        // Guard 2: terminal state check
        if from.is_terminal() {
            return Err(format!("Sensor is RETIRED — no further transitions allowed"));
        }

        // Guard 3: admin signature
        let msg = transition_signing_message(&self.did, from, to, &timestamp);
        let sig = Signature::from_str(admin_sig_hex)
            .map_err(|e| format!("Bad signature format: {e}"))?;
        self.admin_key
            .verify(msg.as_bytes(), &sig)
            .map_err(|_| "Admin signature verification failed".to_string())?;

        self.state = to;
        self.history.push(StateTransition {
            from,
            to,
            reason,
            timestamp,
            admin_signature: admin_sig_hex.to_string(),
            on_chain_tx_id: None,
        });
        Ok(())
    }

    // ── Convenience wrappers ─────────────────────────────────────────────────

    pub fn register(
        &mut self,
        timestamp: DateTime<Utc>,
        admin_key: &SigningKey,
    ) -> Result<(), String> {
        let sig = sign_transition(&self.did, self.state, SensorState::REGISTERED, &timestamp, admin_key);
        self.transition(SensorState::REGISTERED, TransitionReason::NFTMinted, timestamp, &sig)
    }

    pub fn activate(
        &mut self,
        timestamp: DateTime<Utc>,
        admin_key: &SigningKey,
    ) -> Result<(), String> {
        let sig = sign_transition(&self.did, self.state, SensorState::ACTIVE, &timestamp, admin_key);
        self.transition(SensorState::ACTIVE, TransitionReason::FieldActivationConfirmed, timestamp, &sig)
    }

    pub fn go_offline(
        &mut self,
        last_seen_secs_ago: u64,
        timestamp: DateTime<Utc>,
        admin_key: &SigningKey,
    ) -> Result<(), String> {
        let sig = sign_transition(&self.did, self.state, SensorState::OFFLINE, &timestamp, admin_key);
        self.transition(SensorState::OFFLINE, TransitionReason::HeartbeatMissed { last_seen_secs_ago }, timestamp, &sig)
    }

    pub fn quarantine(
        &mut self,
        evidence_cid: Option<String>,
        timestamp: DateTime<Utc>,
        admin_key: &SigningKey,
    ) -> Result<(), String> {
        let sig = sign_transition(&self.did, self.state, SensorState::QUARANTINED, &timestamp, admin_key);
        self.transition(SensorState::QUARANTINED, TransitionReason::TamperingDetected { evidence_cid }, timestamp, &sig)
    }

    pub fn restore(
        &mut self,
        timestamp: DateTime<Utc>,
        admin_key: &SigningKey,
    ) -> Result<(), String> {
        let sig = sign_transition(&self.did, self.state, SensorState::ACTIVE, &timestamp, admin_key);
        self.transition(SensorState::ACTIVE, TransitionReason::ConnectionRestored, timestamp, &sig)
    }

    pub fn retire(
        &mut self,
        timestamp: DateTime<Utc>,
        admin_key: &SigningKey,
    ) -> Result<(), String> {
        let sig = sign_transition(&self.did, self.state, SensorState::RETIRED, &timestamp, admin_key);
        self.transition(SensorState::RETIRED, TransitionReason::PlannedRetirement, timestamp, &sig)
    }

    // ── Timeout checks ───────────────────────────────────────────────────────

    /// Returns true if the sensor entered REGISTERED more than `activation_timeout_secs` ago
    /// without ever reaching ACTIVE.
    pub fn activation_timed_out(&self, now: DateTime<Utc>) -> bool {
        if self.state != SensorState::REGISTERED { return false; }
        if let Some(reg) = self.history.iter().find(|t| t.to == SensorState::REGISTERED) {
            return (now - reg.timestamp).num_seconds() > self.activation_timeout_secs;
        }
        false
    }

    /// Returns true if the sensor has been OFFLINE longer than `offline_timeout_secs`.
    pub fn offline_timed_out(&self, now: DateTime<Utc>) -> bool {
        if self.state != SensorState::OFFLINE { return false; }
        if let Some(ev) = self.history.iter().rev().find(|t| t.to == SensorState::OFFLINE) {
            return (now - ev.timestamp).num_seconds() > self.offline_timeout_secs;
        }
        false
    }

    /// Returns true if the sensor has been QUARANTINED longer than `quarantine_timeout_secs`.
    pub fn quarantine_timed_out(&self, now: DateTime<Utc>) -> bool {
        if self.state != SensorState::QUARANTINED { return false; }
        if let Some(ev) = self.history.iter().rev().find(|t| t.to == SensorState::QUARANTINED) {
            return (now - ev.timestamp).num_seconds() > self.quarantine_timeout_secs;
        }
        false
    }

    /// Audit helper: return all transitions in the history for a specific target state.
    pub fn transitions_to(&self, state: SensorState) -> Vec<&StateTransition> {
        self.history.iter().filter(|t| t.to == state).collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::SigningKey;
    use rand::rngs::OsRng;
    use chrono::Utc;

    fn setup() -> (SensorLifecycle, SigningKey) {
        let admin_sk = SigningKey::random(&mut OsRng);
        let admin_vk = VerifyingKey::from(&admin_sk);
        let sensor = SensorLifecycle::new("did:cardano:sensor:biochar-001", admin_vk);
        (sensor, admin_sk)
    }

    // ── Test 1: initial state is UNREGISTERED ─────────────────────────────────

    #[test]
    fn test_initial_state_unregistered() {
        let (sensor, _) = setup();
        assert_eq!(sensor.state(), SensorState::UNREGISTERED);
    }

    // ── Test 2: full happy path UNREGISTERED → REGISTERED → ACTIVE ────────────

    #[test]
    fn test_happy_path_registration_and_activation() {
        let (mut sensor, sk) = setup();
        let now = Utc::now();
        sensor.register(now, &sk).expect("Register must succeed");
        assert_eq!(sensor.state(), SensorState::REGISTERED);
        sensor.activate(now, &sk).expect("Activate must succeed");
        assert_eq!(sensor.state(), SensorState::ACTIVE);
    }

    // ── Test 3: ACTIVE → OFFLINE → ACTIVE (reconnect) ────────────────────────

    #[test]
    fn test_offline_and_reconnect() {
        let (mut sensor, sk) = setup();
        let now = Utc::now();
        sensor.register(now, &sk).unwrap();
        sensor.activate(now, &sk).unwrap();
        sensor.go_offline(3700, now, &sk).unwrap();
        assert_eq!(sensor.state(), SensorState::OFFLINE);
        sensor.restore(now, &sk).unwrap();
        assert_eq!(sensor.state(), SensorState::ACTIVE);
    }

    // ── Test 4: ACTIVE → QUARANTINED → RETIRED ───────────────────────────────

    #[test]
    fn test_quarantine_and_retire() {
        let (mut sensor, sk) = setup();
        let now = Utc::now();
        sensor.register(now, &sk).unwrap();
        sensor.activate(now, &sk).unwrap();
        sensor.quarantine(Some("QmEvidenceCID".to_string()), now, &sk).unwrap();
        assert_eq!(sensor.state(), SensorState::QUARANTINED);
        sensor.retire(now, &sk).unwrap();
        assert_eq!(sensor.state(), SensorState::RETIRED);
    }

    // ── Test 5: illegal transition rejected ──────────────────────────────────

    #[test]
    fn test_illegal_transition_rejected() {
        let (mut sensor, sk) = setup();
        let now = Utc::now();
        // Cannot go UNREGISTERED → ACTIVE directly
        let sig = sign_transition(
            &sensor.did, SensorState::UNREGISTERED, SensorState::ACTIVE, &now, &sk
        );
        let result = sensor.transition(SensorState::ACTIVE, TransitionReason::FieldActivationConfirmed, now, &sig);
        assert!(result.is_err(), "Illegal transition must fail");
        assert!(result.unwrap_err().contains("Illegal transition"));
    }

    // ── Test 6: wrong admin key rejected ──────────────────────────────────────

    #[test]
    fn test_wrong_admin_key_rejected() {
        let (mut sensor, _) = setup();
        let wrong_key = SigningKey::random(&mut OsRng);
        let now = Utc::now();
        // Sign with a different key
        let sig = sign_transition(
            &sensor.did, SensorState::UNREGISTERED, SensorState::REGISTERED, &now, &wrong_key
        );
        let result = sensor.transition(
            SensorState::REGISTERED, TransitionReason::NFTMinted, now, &sig
        );
        assert!(result.is_err(), "Wrong admin key must be rejected");
    }

    // ── Test 7: RETIRED state is terminal ────────────────────────────────────

    #[test]
    fn test_retired_is_terminal() {
        let (mut sensor, sk) = setup();
        let now = Utc::now();
        sensor.register(now, &sk).unwrap();
        sensor.retire(now, &sk).unwrap();
        assert_eq!(sensor.state(), SensorState::RETIRED);
        // Attempt any further transition
        let sig = sign_transition(&sensor.did, SensorState::RETIRED, SensorState::ACTIVE, &now, &sk);
        let result = sensor.transition(SensorState::ACTIVE, TransitionReason::ConnectionRestored, now, &sig);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("RETIRED"));
    }

    // ── Test 8: history is append-only and complete ───────────────────────────

    #[test]
    fn test_history_records_all_transitions() {
        let (mut sensor, sk) = setup();
        let now = Utc::now();
        sensor.register(now, &sk).unwrap();
        sensor.activate(now, &sk).unwrap();
        sensor.go_offline(7200, now, &sk).unwrap();
        sensor.restore(now, &sk).unwrap();
        sensor.retire(now, &sk).unwrap();
        assert_eq!(sensor.history.len(), 5, "All 5 transitions must be logged");
        assert_eq!(sensor.history[0].from, SensorState::UNREGISTERED);
        assert_eq!(sensor.history[4].to, SensorState::RETIRED);
    }

    // ── Test 9: is_legal_transition covers all edges ──────────────────────────

    #[test]
    fn test_legal_transition_graph() {
        use SensorState::*;
        let legal = [
            (UNREGISTERED, REGISTERED), (REGISTERED, ACTIVE), (REGISTERED, RETIRED),
            (ACTIVE, OFFLINE), (ACTIVE, QUARANTINED), (ACTIVE, RETIRED),
            (OFFLINE, ACTIVE), (OFFLINE, QUARANTINED), (OFFLINE, RETIRED),
            (QUARANTINED, ACTIVE), (QUARANTINED, RETIRED),
        ];
        for (f, t) in &legal {
            assert!(is_legal_transition(*f, *t), "{f:?} → {t:?} should be legal");
        }
        let illegal = [
            (UNREGISTERED, ACTIVE), (UNREGISTERED, QUARANTINED),
            (QUARANTINED, OFFLINE), (RETIRED, ACTIVE),
        ];
        for (f, t) in &illegal {
            assert!(!is_legal_transition(*f, *t), "{f:?} → {t:?} should be illegal");
        }
    }

    // ── Test 10: activation timeout detection ─────────────────────────────────

    #[test]
    fn test_activation_timeout_detected() {
        let (mut sensor, sk) = setup();
        // Set a very short timeout
        sensor.activation_timeout_secs = 1;
        let past = Utc::now() - Duration::seconds(10);
        sensor.register(past, &sk).unwrap();
        assert!(sensor.activation_timed_out(Utc::now()), "Should detect activation timeout");
    }

    #[test]
    fn test_activation_no_timeout_when_short() {
        let (mut sensor, sk) = setup();
        sensor.activation_timeout_secs = 3600;
        sensor.register(Utc::now(), &sk).unwrap();
        assert!(!sensor.activation_timed_out(Utc::now()), "Should not timeout immediately");
    }

    // ── Test 11: offline timeout detection ────────────────────────────────────

    #[test]
    fn test_offline_timeout_detected() {
        let (mut sensor, sk) = setup();
        sensor.offline_timeout_secs = 1;
        let now = Utc::now();
        sensor.register(now, &sk).unwrap();
        sensor.activate(now, &sk).unwrap();
        let past = now - Duration::seconds(10);
        sensor.go_offline(3600, past, &sk).unwrap();
        assert!(sensor.offline_timed_out(Utc::now()), "Should detect offline timeout");
    }

    // ── Test 12: quarantine timeout detection ─────────────────────────────────

    #[test]
    fn test_quarantine_timeout_detected() {
        let (mut sensor, sk) = setup();
        sensor.quarantine_timeout_secs = 1;
        let now = Utc::now();
        sensor.register(now, &sk).unwrap();
        sensor.activate(now, &sk).unwrap();
        let past = now - Duration::seconds(10);
        sensor.quarantine(None, past, &sk).unwrap();
        assert!(sensor.quarantine_timed_out(Utc::now()), "Should detect quarantine timeout");
    }

    // ── Test 13: transitions_to() helper ──────────────────────────────────────

    #[test]
    fn test_transitions_to_helper() {
        let (mut sensor, sk) = setup();
        let now = Utc::now();
        sensor.register(now, &sk).unwrap();
        sensor.activate(now, &sk).unwrap();
        sensor.go_offline(3600, now, &sk).unwrap();
        sensor.restore(now, &sk).unwrap();  // back to ACTIVE
        let active_transitions = sensor.transitions_to(SensorState::ACTIVE);
        assert_eq!(active_transitions.len(), 2, "ACTIVE reached twice: activate + restore");
    }

    // ── Test 14: QUARANTINED → ACTIVE (cleared investigation) ─────────────────

    #[test]
    fn test_quarantine_cleared_back_to_active() {
        let (mut sensor, sk) = setup();
        let now = Utc::now();
        sensor.register(now, &sk).unwrap();
        sensor.activate(now, &sk).unwrap();
        sensor.quarantine(None, now, &sk).unwrap();
        // Investigation complete — sensor cleared
        let sig = sign_transition(&sensor.did, SensorState::QUARANTINED, SensorState::ACTIVE, &now, &sk);
        sensor.transition(SensorState::ACTIVE, TransitionReason::InvestigationCleared, now, &sig).unwrap();
        assert_eq!(sensor.state(), SensorState::ACTIVE);
    }

    // ── Test 15: SensorState descriptions non-empty ───────────────────────────

    #[test]
    fn test_state_descriptions() {
        for state in [
            SensorState::UNREGISTERED, SensorState::REGISTERED, SensorState::ACTIVE,
            SensorState::OFFLINE, SensorState::QUARANTINED, SensorState::RETIRED,
        ] {
            assert!(!state.description().is_empty());
        }
    }
}

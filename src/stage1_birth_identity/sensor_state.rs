use k256::ecdsa::{VerifyingKey, signature::Verifier};
use k256::ecdsa::Signature;
use std::str::FromStr;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensorState {
    UNREGISTERED,
    REGISTERED,
    ACTIVE,
    OFFLINE,
    QUARANTINED,
    RETIRED,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: SensorState,
    pub to: SensorState,
    pub timestamp: DateTime<Utc>,
    pub signature: String, // Signature of (from, to, timestamp)
}

pub struct Sensor {
    pub did: String,
    state: SensorState,
    pub history: Vec<StateTransition>,
    pub verifying_key: Option<VerifyingKey>,
}

impl Sensor {
    pub fn new(did: String) -> Self {
        Self {
            did,
            state: SensorState::UNREGISTERED,
            history: Vec::new(),
            verifying_key: None,
        }
    }

    pub fn with_key(did: String, key: VerifyingKey) -> Self {
        Self {
            did,
            state: SensorState::UNREGISTERED,
            history: Vec::new(),
            verifying_key: Some(key),
        }
    }

    pub fn state(&self) -> SensorState {
        self.state
    }

    fn verify_transition_sig(&self, from: SensorState, to: SensorState, timestamp: DateTime<Utc>, signature_hex: &str) -> bool {
        let key = match &self.verifying_key {
            Some(k) => k,
            None => return true, // If no key set yet (during registration), allow or handle differently
        };

        let message = format!("{:?}{:?}{}", from, to, timestamp.to_rfc3339());
        if let Ok(sig) = Signature::from_str(signature_hex) {
            return key.verify(message.as_bytes(), &sig).is_ok();
        }
        false
    }

    pub fn transition_to(&mut self, new_state: SensorState, timestamp: DateTime<Utc>, signature: String) -> Result<(), String> {
        if !self.verify_transition_sig(self.state, new_state, timestamp, &signature) {
            return Err("Invalid transition signature".to_string());
        }

        let transition = StateTransition {
            from: self.state,
            to: new_state,
            timestamp,
            signature,
        };
        self.state = new_state;
        self.history.push(transition);
        Ok(())
    }

    pub fn register(&mut self, key: VerifyingKey, timestamp: DateTime<Utc>, signature: String) -> Result<(), String> {
        if self.state == SensorState::UNREGISTERED {
            self.verifying_key = Some(key);
            self.transition_to(SensorState::REGISTERED, timestamp, signature)
        } else {
            Err("Sensor already registered".to_string())
        }
    }

    pub fn activate(&mut self, timestamp: DateTime<Utc>, signature: String) -> Result<(), String> {
        if self.state == SensorState::REGISTERED || self.state == SensorState::OFFLINE {
            self.transition_to(SensorState::ACTIVE, timestamp, signature)
        } else {
            Err("Sensor not in a state that can be activated".to_string())
        }
    }
}

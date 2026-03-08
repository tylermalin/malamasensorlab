use serde::{Deserialize, Serialize};
use crate::stage7_verification::audit::AuditTrail;
// Fingerprint is passed as u64

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOfJourney {
    pub audit_trail: AuditTrail,
    pub fingerprint: u64,
    pub status: String,
    pub verification_passed: bool,
}

pub struct ProofGenerator;

impl ProofGenerator {
    pub fn generate(audit_trail: AuditTrail, fingerprint: u64) -> ProofOfJourney {
        let verification_passed = audit_trail.verify();
        let status = if verification_passed {
            "VERIFIED".to_string()
        } else {
            "TAMPERED".to_string()
        };

        ProofOfJourney {
            audit_trail,
            fingerprint,
            status,
            verification_passed,
        }
    }
}

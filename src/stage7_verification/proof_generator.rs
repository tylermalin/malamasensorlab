use serde::{Deserialize, Serialize};
use crate::stage7_verification::audit::AuditTrail;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOfJourney {
    pub audit_trail: AuditTrail,
    pub journey_signature: String, // Final protocol signature
    pub fingerprint: u64,
    pub status: String,
    pub verification_passed: bool,
    pub certificate_id: String, // P49: Verifiable Certificate ID
}

pub struct ProofGenerator;

impl ProofGenerator {
    /// Generates a final signed Proof of Journey (PoJ).
    pub fn generate(audit_trail: AuditTrail, fingerprint: u64, secret_key: u64) -> ProofOfJourney {
        let verification_passed = audit_trail.verify();
        let status = if verification_passed {
            "VERIFIED".to_string()
        } else {
            "TAMPERED".to_string()
        };

        let certificate_id = format!("MALAMA-CERT-{}-{}", audit_trail.batch_id, audit_trail.timestamp);
        let journey_signature = format!("PROT-SIG-{:x}-{}", secret_key, audit_trail.batch_id);

        ProofOfJourney {
            audit_trail,
            journey_signature,
            fingerprint,
            status,
            verification_passed,
            certificate_id,
        }
    }
}

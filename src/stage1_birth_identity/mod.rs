pub mod did_generator;
pub mod ownership_proof;
pub mod sensor_state;
pub mod provenance;
pub mod reading_signer;
pub mod reputation;
pub mod cardano_nft;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// A signed sensor reading (Stage 1 variant — minimal fields for the birth stage).
/// The full multi-field version is defined in stage2_gateway::aggregator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub sensor_id: String,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    /// Hex-encoded ECDSA signature over `"<sensor_id><value><timestamp>"`.
    pub signature: String,
}

#[cfg(test)]
mod tests {
    use super::did_generator::*;
    use super::ownership_proof::*;
    use super::sensor_state::*;
    use super::provenance::*;
    use std::collections::HashSet;
    use k256::ecdsa::{SigningKey, signature::Signer};
    use rand::rngs::OsRng;
    use chrono::Utc;

    // ── DID tests ────────────────────────────────────────────────────────────

    #[test]
    fn test_did_generation() {
        let result = generate_sensor_did("temperature", "Tropic Square", 43.8, -115.9);
        assert!(result.did.starts_with("did:cardano:sensor:"));
        assert_eq!(result.doc.metadata.sensor_type, "temperature");
    }

    #[test]
    fn test_ownership_proof() {
        let result = generate_sensor_did("temperature", "Tropic Square", 43.8, -115.9);
        let challenge = create_challenge();
        let signature = sign_challenge(&challenge, &result.private_key);
        let verifying_key = k256::ecdsa::VerifyingKey::from(&result.private_key);
        assert!(verify_signature(&challenge, &signature, &verifying_key));
    }

    #[test]
    fn test_did_uniqueness() {
        let mut dids = HashSet::new();
        for _ in 0..100 {
            let result = generate_sensor_did("temperature", "Tropic Square", 43.8, -115.9);
            assert!(dids.insert(result.did));
        }
        assert_eq!(dids.len(), 100);
    }

    #[test]
    fn test_state_transitions() {
        let result = generate_sensor_did("temperature", "Tropic Square", 43.8, -115.9);
        let mut sensor = Sensor::new(result.did);
        let verifying_key = k256::ecdsa::VerifyingKey::from(&result.private_key);

        let now = Utc::now();
        let message = format!("{:?}{:?}{}", SensorState::UNREGISTERED, SensorState::REGISTERED, now.to_rfc3339());
        let sig: k256::ecdsa::Signature = result.private_key.sign(message.as_bytes());
        let sig_hex = hex::encode(sig.to_bytes());

        let res = sensor.register(verifying_key, now, sig_hex);
        assert!(res.is_ok());
        assert_eq!(sensor.state(), SensorState::REGISTERED);
    }

    // ── Provenance tests ─────────────────────────────────────────────────────

    fn make_provenance_record(key: &SigningKey) -> ProvenanceRecord {
        let manufacturer = ManufacturerInfo {
            name: "Tropic Square".to_string(),
            manufacturing_date: "2024-12-15".to_string(),
            manufacturing_location: "Prague, Czech Republic".to_string(),
            serial_number: "TSQ-20241215-001".to_string(),
            initial_calibration: CalibrationRecord {
                date: "2024-12-20".to_string(),
                reference_lab: "Calibration Lab, TU Prague".to_string(),
                accuracy: "±0.3°C".to_string(),
                tracked_to: "NIST Standard".to_string(),
            },
        };
        let deployment = DeploymentInfo {
            installed_at: Utc::now(),
            location: DeploymentLocation {
                address: "Idaho City Biochar Farm, Boise County, ID".to_string(),
                coordinates: Coordinates { latitude: 43.8, longitude: -115.9 },
            },
            deployed_by: "Jeffrey Wise (Mālama COO)".to_string(),
            expected_lifespan_months: 36,
        };

        let make_entry = |ts: &str, custodian: &str, action: &str, coords: Option<Coordinates>| {
            CustodyEntry {
                timestamp: ts.to_string(),
                custodian: custodian.to_string(),
                action: action.to_string(),
                coordinates: coords,
                custodian_signature: sign_custody_entry(ts, custodian, action, key),
            }
        };

        ProvenanceBuilder::new("did:cardano:sensor:biochar-001", manufacturer, deployment)
            .add_custody_entry(make_entry("2024-12-15", "Tropic Square", "manufactured", None))
            .add_custody_entry(make_entry("2024-12-20", "TU Prague Lab", "calibrated", None))
            .add_custody_entry(make_entry("2025-01-15", "Mālama Labs", "received", None))
            .add_custody_entry(make_entry(
                "2025-02-01",
                "Idaho Farm",
                "installed",
                Some(Coordinates { latitude: 43.8001, longitude: -115.9002 }),
            ))
            .build()
    }

    #[test]
    fn test_provenance_record_created() {
        let key = SigningKey::random(&mut OsRng);
        let record = make_provenance_record(&key);
        assert_eq!(record.sensor_did, "did:cardano:sensor:biochar-001");
        assert_eq!(record.chain_of_custody.len(), 4);
        assert_eq!(record.created_hash.len(), 64);
    }

    #[test]
    fn test_provenance_integrity_passes() {
        let key = SigningKey::random(&mut OsRng);
        let record = make_provenance_record(&key);
        assert!(record.verify_integrity(), "Fresh record must pass integrity");
    }

    #[test]
    fn test_provenance_immutability_fails_on_modification() {
        let key = SigningKey::random(&mut OsRng);
        let mut record = make_provenance_record(&key);
        record.manufacturer_info.name = "Fake Factory".to_string();
        assert!(!record.verify_integrity(), "Tampered record must fail integrity");
    }

    #[test]
    fn test_custody_signature_verification() {
        let key = SigningKey::random(&mut OsRng);
        let record = make_provenance_record(&key);
        let vk = k256::ecdsa::VerifyingKey::from(&key);
        let vk_hex = hex::encode(vk.to_encoded_point(true).as_bytes());
        for entry in &record.chain_of_custody {
            let ok = ProvenanceRecord::verify_custody_signature(entry, &vk_hex).unwrap();
            assert!(ok, "Signature for '{}' must verify", entry.action);
        }
    }

    #[test]
    fn test_location_audit_within_100m() {
        let key = SigningKey::random(&mut OsRng);
        let record = make_provenance_record(&key);
        let result = record.audit_installation_location(100.0).unwrap();
        assert!(result, "GPS within 100m should pass audit");
    }
}

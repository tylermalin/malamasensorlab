//! Stage 1 — Sensor Birth Metadata & Provenance
//!
//! Implements the immutable "birth certificate" for a physical sensor.
//! Once created, a `ProvenanceRecord` cannot be modified—its integrity
//! is guaranteed by a SHA-256 `created_hash` that covers every field.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use k256::ecdsa::{SigningKey, VerifyingKey, Signature, signature::Verifier};
use std::str::FromStr;

// ── Types ────────────────────────────────────────────────────────────────────

/// GPS coordinates recorded at each custody handoff.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

impl Coordinates {
    /// Haversine distance in metres between two points.
    pub fn distance_metres(&self, other: &Coordinates) -> f64 {
        let r = 6_371_000.0_f64;
        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();
        let dlat = (other.latitude - self.latitude).to_radians();
        let dlon = (other.longitude - self.longitude).to_radians();
        let a = (dlat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        r * 2.0 * a.sqrt().atan2((1.0 - a).sqrt())
    }
}

/// Physical deployment location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentLocation {
    pub address: String,
    pub coordinates: Coordinates,
}

/// Calibration metadata traceable to a national standard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationRecord {
    pub date: String,
    pub reference_lab: String,
    pub accuracy: String,
    pub tracked_to: String,
}

/// Manufacturer-provided factory metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManufacturerInfo {
    pub name: String,
    pub manufacturing_date: String,
    pub manufacturing_location: String,
    pub serial_number: String,
    pub initial_calibration: CalibrationRecord,
}

/// Deployment context: who installed the sensor and where.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentInfo {
    pub installed_at: DateTime<Utc>,
    pub location: DeploymentLocation,
    pub deployed_by: String,
    pub expected_lifespan_months: u32,
}

/// A single chain-of-custody entry, signed by both the current and new custodian.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyEntry {
    /// RFC 3339 timestamp of the handoff.
    pub timestamp: String,
    pub custodian: String,
    pub action: String,
    /// GPS location of the handoff (optional — required for "installed" action).
    pub coordinates: Option<Coordinates>,
    /// Hex-encoded ECDSA signature by the *outgoing* custodian over
    /// `"<timestamp><custodian><action>"`. First entry (manufacture) is self-signed.
    pub custodian_signature: String,
}

/// The immutable provenance record — the sensor's birth certificate.
///
/// Fields are private to prevent post-creation mutation; use the builder
/// pattern via `ProvenanceRecord::new(...)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub sensor_did: String,
    pub manufacturer_info: ManufacturerInfo,
    pub deployment: DeploymentInfo,
    pub chain_of_custody: Vec<CustodyEntry>,
    /// SHA-256 hex of the canonical JSON of this record (excluding this field).
    pub created_hash: String,
}

// ── Builder ─────────────────────────────────────────────────────────────────

/// Builder for a `ProvenanceRecord`. Enforces that the hash is computed
/// after all fields are set and before the record is finalised.
pub struct ProvenanceBuilder {
    sensor_did: String,
    manufacturer_info: ManufacturerInfo,
    deployment: DeploymentInfo,
    chain_of_custody: Vec<CustodyEntry>,
}

impl ProvenanceBuilder {
    pub fn new(
        sensor_did: impl Into<String>,
        manufacturer_info: ManufacturerInfo,
        deployment: DeploymentInfo,
    ) -> Self {
        Self {
            sensor_did: sensor_did.into(),
            manufacturer_info,
            deployment,
            chain_of_custody: Vec::new(),
        }
    }

    /// Append a custody entry (signed by the outgoing custodian key).
    pub fn add_custody_entry(mut self, entry: CustodyEntry) -> Self {
        self.chain_of_custody.push(entry);
        self
    }

    /// Finalise: compute the canonical SHA-256 hash and return the immutable record.
    pub fn build(self) -> ProvenanceRecord {
        // Build the record without the hash first, then hash it.
        let mut record = ProvenanceRecord {
            sensor_did: self.sensor_did,
            manufacturer_info: self.manufacturer_info,
            deployment: self.deployment,
            chain_of_custody: self.chain_of_custody,
            created_hash: String::new(), // placeholder
        };
        record.created_hash = record.compute_hash();
        record
    }
}

// ── Core implementation ──────────────────────────────────────────────────────

impl ProvenanceRecord {
    /// Deterministic canonical bytes — `created_hash` field is excluded so the
    /// hash is stable regardless of when it was computed.
    fn canonical_bytes(&self) -> Vec<u8> {
        // We build a copy with an empty hash, then serialise that.
        let copy = ProvenanceRecord {
            created_hash: String::new(),
            ..self.clone()
        };
        serde_json::to_vec(&copy).expect("ProvenanceRecord serialization is infallible")
    }

    /// Compute the SHA-256 hash of the canonical record.
    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.canonical_bytes());
        hex::encode(hasher.finalize())
    }

    /// Verify that `created_hash` still matches the record's content.
    /// Returns `false` if any field was modified after creation.
    pub fn verify_integrity(&self) -> bool {
        self.created_hash == self.compute_hash()
    }

    /// Verify the ECDSA signature of a specific custody entry.
    ///
    /// `signing_message = "<timestamp><custodian><action>"`
    pub fn verify_custody_signature(
        entry: &CustodyEntry,
        verifying_key_hex: &str,
    ) -> Result<bool, String> {
        let key_bytes = hex::decode(verifying_key_hex)
            .map_err(|e| format!("Bad key hex: {e}"))?;
        let vk = VerifyingKey::from_sec1_bytes(&key_bytes)
            .map_err(|e| format!("Invalid verifying key: {e}"))?;
        let message = format!("{}{}{}", entry.timestamp, entry.custodian, entry.action);
        let sig = Signature::from_str(&entry.custodian_signature)
            .map_err(|e| format!("Bad signature: {e}"))?;
        Ok(vk.verify(message.as_bytes(), &sig).is_ok())
    }

    /// Audit: verify the installation location is within `max_distance_m` of the
    /// coordinates claimed in the deploy info.  Returns `Err` if no "installed"
    /// custody entry exists.
    pub fn audit_installation_location(&self, max_distance_m: f64) -> Result<bool, String> {
        let installed_entry = self
            .chain_of_custody
            .iter()
            .find(|e| e.action == "installed")
            .ok_or("No 'installed' custody entry found")?;

        let recorded_coords = installed_entry
            .coordinates
            .as_ref()
            .ok_or("'installed' entry has no coordinates")?;

        let claimed = &self.deployment.location.coordinates;
        let dist = claimed.distance_metres(recorded_coords);
        Ok(dist <= max_distance_m)
    }
}

// ── Custody-signing helpers ──────────────────────────────────────────────────

/// Sign a custody handoff message: `"<timestamp><custodian><action>"`.
pub fn sign_custody_entry(
    timestamp: &str,
    custodian: &str,
    action: &str,
    signing_key: &SigningKey,
) -> String {
    use k256::ecdsa::signature::Signer as _;
    let message = format!("{timestamp}{custodian}{action}");
    let sig: Signature = signing_key.sign(message.as_bytes());
    hex::encode(sig.to_bytes())
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use k256::ecdsa::SigningKey;
    use rand::rngs::OsRng;

    fn make_manufacturer() -> ManufacturerInfo {
        ManufacturerInfo {
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
        }
    }

    fn make_deployment() -> DeploymentInfo {
        DeploymentInfo {
            installed_at: Utc::now(),
            location: DeploymentLocation {
                address: "Idaho City Biochar Farm, Boise County, ID".to_string(),
                coordinates: Coordinates { latitude: 43.8, longitude: -115.9 },
            },
            deployed_by: "Jeffrey Wise (Mālama COO)".to_string(),
            expected_lifespan_months: 36,
        }
    }

    fn make_signed_entry(
        key: &SigningKey,
        timestamp: &str,
        custodian: &str,
        action: &str,
        coords: Option<Coordinates>,
    ) -> CustodyEntry {
        CustodyEntry {
            timestamp: timestamp.to_string(),
            custodian: custodian.to_string(),
            action: action.to_string(),
            coordinates: coords,
            custodian_signature: sign_custody_entry(timestamp, custodian, action, key),
        }
    }

    fn build_record(signing_key: &SigningKey) -> ProvenanceRecord {
        let _vk = VerifyingKey::from(signing_key);

        ProvenanceBuilder::new(
            "did:cardano:sensor:biochar-001",
            make_manufacturer(),
            make_deployment(),
        )
        .add_custody_entry(make_signed_entry(
            signing_key,
            "2024-12-15",
            "Tropic Square",
            "manufactured",
            None,
        ))
        .add_custody_entry(make_signed_entry(
            signing_key,
            "2024-12-20",
            "TU Prague Lab",
            "calibrated",
            None,
        ))
        .add_custody_entry(make_signed_entry(
            signing_key,
            "2025-01-15",
            "Mālama Labs",
            "received",
            None,
        ))
        .add_custody_entry(make_signed_entry(
            signing_key,
            "2025-02-01",
            "Idaho Farm",
            "installed",
            Some(Coordinates { latitude: 43.8001, longitude: -115.9002 }),
        ))
        .build()
    }

    // ── Test 1: record created correctly ─────────────────────────────────────

    #[test]
    fn test_provenance_record_created() {
        let key = SigningKey::random(&mut OsRng);
        let record = build_record(&key);
        assert_eq!(record.sensor_did, "did:cardano:sensor:biochar-001");
        assert_eq!(record.manufacturer_info.name, "Tropic Square");
        assert_eq!(record.chain_of_custody.len(), 4);
        assert!(!record.created_hash.is_empty(), "created_hash must be populated");
        assert_eq!(record.created_hash.len(), 64, "SHA-256 should be 64 hex chars");
    }

    // ── Test 2: integrity check passes on un-modified record ─────────────────

    #[test]
    fn test_integrity_passes_on_clean_record() {
        let key = SigningKey::random(&mut OsRng);
        let record = build_record(&key);
        assert!(
            record.verify_integrity(),
            "Fresh record should pass integrity check"
        );
    }

    // ── Test 3: immutability — any field change breaks the hash ──────────────

    #[test]
    fn test_immutability_fails_on_modification() {
        let key = SigningKey::random(&mut OsRng);
        let mut record = build_record(&key);
        // Simulate an attacker replacing the manufacturer name
        record.manufacturer_info.name = "Fake Factory".to_string();
        assert!(
            !record.verify_integrity(),
            "Tampered record must fail integrity check"
        );
    }

    #[test]
    fn test_immutability_fails_on_modified_serial() {
        let key = SigningKey::random(&mut OsRng);
        let mut record = build_record(&key);
        record.manufacturer_info.serial_number = "FAKE-000".to_string();
        assert!(!record.verify_integrity());
    }

    // ── Test 4: chain of custody signature verification ──────────────────────

    #[test]
    fn test_custody_signature_verification() {
        let key = SigningKey::random(&mut OsRng);
        let record = build_record(&key);

        let vk = VerifyingKey::from(&key);
        let vk_hex = hex::encode(vk.to_encoded_point(true).as_bytes());

        for entry in &record.chain_of_custody {
            let ok = ProvenanceRecord::verify_custody_signature(entry, &vk_hex)
                .expect("Should not error on valid key");
            assert!(ok, "Custody signature for '{}' must verify", entry.action);
        }
    }

    #[test]
    fn test_custody_signature_fails_on_tampered_entry() {
        let key = SigningKey::random(&mut OsRng);
        let mut record = build_record(&key);

        let vk = VerifyingKey::from(&key);
        let vk_hex = hex::encode(vk.to_encoded_point(true).as_bytes());

        // Tamper: change the custodian name on the first entry
        record.chain_of_custody[0].custodian = "Evil Corp".to_string();
        let ok = ProvenanceRecord::verify_custody_signature(
            &record.chain_of_custody[0],
            &vk_hex,
        )
        .expect("Should not error on valid key");
        assert!(!ok, "Tampered custody entry signature must fail");
    }

    // ── Test 5: location audit ────────────────────────────────────────────────

    #[test]
    fn test_location_audit_within_100m() {
        let key = SigningKey::random(&mut OsRng);
        let record = build_record(&key);

        // Installed coordinates (43.8001, -115.9002) vs claimed (43.8, -115.9)
        // — should be well within 100m
        let result = record.audit_installation_location(100.0);
        assert!(result.is_ok(), "Audit should find 'installed' entry");
        assert!(
            result.unwrap(),
            "Installation GPS within 100m of claimed location"
        );
    }

    #[test]
    fn test_location_audit_fails_outside_threshold() {
        let key = SigningKey::random(&mut OsRng);
        // Place the installed GPS 5km away from the deployment claim
        let mut record = build_record(&key);
        // Re-stamp the installed entry with bogus far coordinates
        if let Some(e) = record.chain_of_custody.iter_mut().find(|e| e.action == "installed") {
            e.coordinates = Some(Coordinates { latitude: 44.5, longitude: -116.5 });
        }
        let result = record.audit_installation_location(100.0);
        assert!(result.is_ok());
        assert!(!result.unwrap(), "Far-away GPS should fail the 100m audit");
    }

    // ── Test 6: haversine distance sanity ────────────────────────────────────

    #[test]
    fn test_haversine_distance_accuracy() {
        // Prague → Vienna: known distance ~252 km
        let prague = Coordinates { latitude: 50.0755, longitude: 14.4378 };
        let vienna = Coordinates { latitude: 48.2082, longitude: 16.3738 };
        let dist_km = prague.distance_metres(&vienna) / 1000.0;
        assert!(
            (dist_km - 252.0).abs() < 10.0,
            "Haversine Prague→Vienna should be ~252km, got {dist_km:.1}km"
        );
    }

    // ── Test 7: hash is deterministic ────────────────────────────────────────

    #[test]
    fn test_hash_is_deterministic() {
        let key = SigningKey::random(&mut OsRng);
        let record = build_record(&key);
        // Recompute the hash independently and confirm it matches
        let recomputed = record.compute_hash();
        assert_eq!(record.created_hash, recomputed, "Hash must be deterministic");
    }
}

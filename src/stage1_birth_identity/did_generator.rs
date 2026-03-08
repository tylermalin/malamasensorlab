use k256::ecdsa::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use base64ct::Encoding;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidDocument {
    #[serde(rename = "@context")]
    pub context: String,
    pub id: String,
    #[serde(rename = "publicKey")]
    pub public_key: Vec<PublicKeyEntry>,
    pub authentication: Vec<String>,
    pub created: DateTime<Utc>,
    pub metadata: SensorMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub key_type: String,
    pub controller: String,
    #[serde(rename = "publicKeyJwk")]
    pub public_key_jwk: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorMetadata {
    #[serde(rename = "sensorType")]
    pub sensor_type: String,
    pub location: Location,
    pub manufacturer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
}

pub struct DidResult {
    pub did: String,
    pub doc: DidDocument,
    pub private_key: SigningKey,
}

pub fn generate_sensor_did(sensor_type: &str, manufacturer: &str, lat: f64, lon: f64) -> DidResult {
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = VerifyingKey::from(&signing_key);
    
    let uuid = Uuid::new_v4();
    let did = format!("did:cardano:sensor:{}", uuid);
    let key_id = format!("{}#key-1", did);
    
    // Convert public key to JWK manually
    let encoded_point = verifying_key.to_encoded_point(false);
    let x = base64ct::Base64UrlUnpadded::encode_string(encoded_point.x().unwrap());
    let y = base64ct::Base64UrlUnpadded::encode_string(encoded_point.y().unwrap());

    let jwk_json = serde_json::json!({
        "kty": "EC",
        "crv": "secp256k1",
        "x": x,
        "y": y
    });

    let doc = DidDocument {
        context: "https://www.w3.org/ns/did/v1".to_string(),
        id: did.clone(),
        public_key: vec![PublicKeyEntry {
            id: key_id.clone(),
            key_type: "EcdsaSecp256k1VerificationKey2019".to_string(),
            controller: did.clone(),
            public_key_jwk: jwk_json,
        }],
        authentication: vec![key_id],
        created: Utc::now(),
        metadata: SensorMetadata {
            sensor_type: sensor_type.to_string(),
            location: Location { latitude: lat, longitude: lon },
            manufacturer: manufacturer.to_string(),
        },
    };

    DidResult {
        did,
        doc,
        private_key: signing_key,
    }
}

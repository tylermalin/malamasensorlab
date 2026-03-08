/**
 * Stage 1: Birth of Data & Identity
 * "Every journey begins with an identity..."
 * — Odyssey of a Data Point
 *
 * TypeScript type definitions mirroring the Rust DID structures,
 * for use in the Mālama Protocol frontend and SDK clients.
 */

// ---------------------------------------------------------------------------
// Core W3C DID Types
// ---------------------------------------------------------------------------

/** W3C JSON Web Key (JWK) representation of a public key */
export interface PublicKeyJwk {
  /** Key type — always "EC" for Secp256k1 */
  kty: "EC";
  /** Curve — always "secp256k1" */
  crv: "secp256k1";
  /** X coordinate (base64url-encoded) */
  x: string;
  /** Y coordinate (base64url-encoded) */
  y: string;
  /** Key use — optional, "sig" for signing */
  use?: "sig";
}

/** A verification method entry in a DID Document */
export interface VerificationMethod {
  /** Fully-qualified key ID, e.g. "did:cardano:sensor:{uuid}#key-1" */
  id: string;
  /** Always "EcdsaSecp256k1VerificationKey2019" per W3C spec */
  type: "EcdsaSecp256k1VerificationKey2019";
  /** The DID that controls this key */
  controller: string;
  /** Public key in JWK format */
  publicKeyJwk: PublicKeyJwk;
}

/** Geographic location of a sensor */
export interface SensorLocation {
  latitude: number;
  longitude: number;
}

/** Device-specific metadata embedded in the DID Document */
export interface SensorMetadata {
  /** e.g. "temperature", "humidity", "co2" */
  sensorType: string;
  /** Physical deployment location */
  location: SensorLocation;
  /** Hardware manufacturer, e.g. "Tropic Square" */
  manufacturer: string;
}

/**
 * W3C DID Document for a Mālama sensor.
 * Stored in IPFS with an on-chain reference (Cardano anchor).
 *
 * @example
 * {
 *   "@context": "https://www.w3.org/ns/did/v1",
 *   "id": "did:cardano:sensor:6ba7b810-9dad-11d1-80b4-00c04fd430c8",
 *   ...
 * }
 */
export interface DidDocument {
  /** Always "https://www.w3.org/ns/did/v1" */
  "@context": "https://www.w3.org/ns/did/v1";
  /** The sensor's fully-qualified DID: "did:cardano:sensor:{uuid}" */
  id: string;
  /** Array of public key / verification method entries */
  publicKey: VerificationMethod[];
  /** Key IDs authorized to authenticate as this DID */
  authentication: string[];
  /** ISO 8601 timestamp of DID creation */
  created: string;
  /** Sensor-specific metadata */
  metadata: SensorMetadata;
}

// ---------------------------------------------------------------------------
// Sensor State Machine
// ---------------------------------------------------------------------------

/**
 * Lifecycle states for a Mālama sensor's Digital Twin.
 * Transitions are signed + timestamped for an immutable audit trail.
 */
export type SensorState =
  | "UNREGISTERED"  // Just powered on, no DID registered yet
  | "REGISTERED"    // DID recorded on-chain, awaiting activation
  | "ACTIVE"        // Streaming validated data
  | "OFFLINE"       // Unreachable but not decommissioned
  | "QUARANTINED"   // Flagged by AI validation (anomalous data)
  | "RETIRED";      // Permanently decommissioned

/** A single state transition in the sensor's lifecycle audit trail */
export interface StateTransition {
  from: SensorState;
  to: SensorState;
  /** ISO 8601 timestamp */
  timestamp: string;
  /** ECDSA signature of (from + to + timestamp), hex-encoded */
  signature: string;
}

/** Full Digital Twin record for a sensor */
export interface SensorDigitalTwin {
  /** The sensor's DID */
  did: string;
  /** Current lifecycle state */
  currentState: SensorState;
  /** Immutable ordered history of state transitions */
  stateHistory: StateTransition[];
  /** The sensor's resolved DID Document */
  didDocument: DidDocument;
}

// ---------------------------------------------------------------------------
// Ownership Proof
// ---------------------------------------------------------------------------

/**
 * A cryptographic challenge issued to prove sensor identity.
 * Used to prevent spoofing: "Prove you are sensor X."
 */
export interface OwnershipChallenge {
  /** Random hex nonce (16 bytes / 32 hex chars) */
  nonce: string;
  /** ISO 8601 timestamp of challenge creation */
  timestamp: string;
}

/** A sensor's response to an ownership challenge */
export interface OwnershipProof {
  challenge: OwnershipChallenge;
  /** ECDSA signature of hash(nonce || timestamp), hex-encoded */
  signature: string;
  /** The DID of the sensor claiming ownership */
  sensorDid: string;
}

// ---------------------------------------------------------------------------
// API Response types (for SDK / REST clients)
// ---------------------------------------------------------------------------

/** Response from the DID generation endpoint */
export interface GenerateSensorDidResponse {
  did: string;
  didDocument: DidDocument;
  /** IPFS CID where the DID Document is stored */
  ipfsCid: string;
  /** Cardano transaction ID anchoring the DID */
  cardanoTxId?: string;
}

/** Result of verifying an ownership proof */
export interface VerifyOwnershipResponse {
  verified: boolean;
  sensorDid: string;
  timestamp: string;
}

/**
 * Stage 5: Safety Net & Integrity
 * "No data point travels without protection... authenticated and free from tampering."
 * — Odyssey of a Data Point
 */

/**
 * A fingerprint generated via Locality Sensitive Hashing (LSH).
 * Expressed as a hex string for portability.
 */
export interface LshFingerprint {
    hash: string;
    /** The specific LSH algorithm/seed used */
    version: string;
}

/**
 * A single link in the verifiable Chain of Custody.
 * Tracks the "Who, What, When" of a data point's handling.
 */
export interface CustodyLink {
    /** DID of the agent/node handling the data */
    handlerDid: string;
    /** Hash of the data point as it was received */
    inputHash: string;
    /** Hash of the data point as it was handed off (may be same if no transformation) */
    outputHash: string;
    /** Unix timestamp of the handover */
    timestamp: number;
    /** ECDSA signature of (handlerDid + inputHash + outputHash + timestamp) */
    signature: string;
}

/**
 * Complete integrity report for an end-to-end data journey.
 */
export interface IntegrityReport {
    batchId: string;
    /** The sequence of verifiable handovers */
    chainOfCustody: CustodyLink[];
    /** The LSH fingerprint for anomaly detection comparison */
    fingerprint: LshFingerprint;
    /** Result of cross-checking all cryptographic proofs (DID, Merkle, Consensus, Storage) */
    isVerified: boolean;
    verificationLogs: string[];
}

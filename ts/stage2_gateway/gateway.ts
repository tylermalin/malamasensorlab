/**
 * Stage 2: The Gateway
 * "Data arrives raw and chaotic. The Gateway batches it, builds a Merkle tree..."
 * — Odyssey of a Data Point
 */

/** A single sensor reading as received by the Gateway */
export interface SensorReading {
    /** The sensor's DID */
    sensorId: string;
    /** The recorded value (e.g. temperature) */
    value: number;
    /** ISO 8601 timestamp */
    timestamp: string;
    /** ECDSA signature from the sensor proving authenticity */
    signature: string;
}

/** 
 * A batch of sensor readings grouped by a time window.
 * This is the unit of work for cryptographic anchoring and multi-chain settlement.
 */
export interface DataBatch {
    /** Unique UUID for the batch */
    batchId: string;
    /** Array of sensor readings included in this batch */
    readings: SensorReading[];
    /** Start of the collection window */
    windowStart: string;
    /** End of the collection window */
    windowEnd: string;
}

/**
 * Merkle Tree Metadata for a batch.
 * Used for verifying the integrity of individual readings within the batch.
 */
export interface MerkleMetadata {
    /** SHA-256 Merkle Root of the batch, hex-encoded */
    merkleRoot: string;
    /** Total number of readings in the tree */
    leafCount: number;
}

/**
 * A Merkle Inclusion Proof for a specific data point.
 */
export interface MerkleInclusionProof {
    /** The data point being proven */
    reading: SensorReading;
    /** The batch ID containing the data point */
    batchId: string;
    /** The Merkle root of the batch */
    merkleRoot: string;
    /** The siblings along the path from leaf to root (base64 or hex encoded) */
    proofPath: string[];
    /** The index of the leaf in the tree */
    leafIndex: number;
}

/** Lifecycle states for the Gateway Node */
export type GatewayState =
    | "COLLECTING"    // Aggregating readings into the current window
    | "SEALING"       // Window closed, building Merkle tree and writing to WAL
    | "BROADCASTING"  // Sending batch to consensus/storage layers
    | "CONFIRMED";    // Batch successfully anchored and acknowledged

/** Status of the Gateway Node */
export interface GatewayStatus {
    state: GatewayState;
    /** Current window duration in seconds */
    windowDuration: number;
    /** Number of readings in the current unsealed batch */
    pendingReadingsCount: number;
    /** The Merkle root of the last sealed batch */
    lastMerkleRoot?: string;
}

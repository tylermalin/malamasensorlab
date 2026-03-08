/**
 * Stage 4: Storage Provider
 * "Our data point now finds its home... anchored on-chain for immutability."
 * — Odyssey of a Data Point
 */

/**
 * A receipt confirming a batch anchor on a specific blockchain.
 */
export interface AnchorReceipt {
    /** The blockchain where the data was anchored (e.g., "Cardano", "BASE") */
    chain: string;
    /** The transaction identity anchoring the CID */
    txId: string;
    /** The IPFS Content Identifier (CID) for the off-chain data */
    cid: string;
}

/**
 * Status of the storage process for a batch.
 */
export interface StorageStatus {
    batchId: string;
    /** The CID assigned by IPFS (off-chain) */
    ipfsCid?: string;
    /** Collection of anchor receipts from different chains */
    anchors: AnchorReceipt[];
    /** Whether the storage process is completed (uploaded + anchored) */
    isFinalized: boolean;
}

/** Configuration for the Multi-chain Storage Manager */
export interface StorageConfig {
    ipfsGatewayUrl: string;
    /** List of chains to anchor on */
    enabledChains: string[];
}

/**
 * Stage 3: Satellite & Consensus
 * "In the next stage, our data point is no longer alone..."
 * — Odyssey of a Data Point
 */

/** Types of votes a consensus node can cast */
export type VoteType = "APPROVE" | "REJECT";

/** 
 * A vote from a single consensus node for a specific batch.
 */
export interface Vote {
    /** Unique ID of the node casting the vote */
    nodeId: string;
    /** The batch ID being voted on */
    batchId: string;
    /** The vote decision */
    voteType: VoteType;
    /** ECDSA signature of (nodeId + batchId + voteType), proving the vote came from the node */
    signature: string;
}

/**
 * An aggregated proof of consensus.
 * This is the certificate used to anchor the batch on multiple blockchains.
 */
export interface ConsensusProof {
    /** The batch ID that reached consensus */
    batchId: string;
    /** Collection of signatures from the approving nodes */
    signatures: string[];
    /** List of node IDs that provided the signatures */
    nodeIds: string[];
    /** Unix timestamp of when the proof was finalized */
    timestamp: number;
}

/** Current consensus state for a batch */
export type ConsensusStatus =
    | "PENDING"   // Batch assigned, waiting to start voting
    | "VOTING"    // Votes are being collected
    | "COMMITTED" // Threshold reached, consensus proof generated
    | "REJECTED";  // Consensus failed (e.g. invalid data or node failure)

/** 
 * Statistics about a voting session.
 */
export interface VotingRecord {
    batchId: string;
    /** Total number of nodes assigned to this partition */
    totalNodesAssigned: number;
    /** Number of APPROVE votes cast */
    approveCount: number;
    /** Number of REJECT votes cast */
    rejectCount: number;
    /** Whether the BFT threshold (2/3 + 1) was met */
    thresholdMet: boolean;
}

import { DidDocument } from "../stage1_birth_identity/did.js";
import { ConsensusProof } from "../stage3_consensus/consensus.js";
import { AnchorReceipt } from "../stage4_storage/storage.js";
import { SettlementReceipt } from "../stage6_settlement/settlement.js";

/**
 * Stage 7: Verification & Proof
 * "The journey is complete, but the truth remains..."
 * — Odyssey of a Data Point
 */

/**
 * A comprehensive audit trail summarizing the entire data life-cycle.
 */
export interface AuditTrail {
    batchId: string;
    didDoc: DidDocument;
    merkleRoot: string;
    consensusProof: ConsensusProof;
    storageAnchors: AnchorReceipt[];
    settlementReceipts: SettlementReceipt[];
    timestamp: number;
}

/**
 * The final "Proof of Journey" certificate.
 */
export interface ProofOfJourney {
    auditTrail: AuditTrail;
    /** LSH fingerprint for anomaly cross-referencing */
    fingerprint: string;
    /** Status of the data: "VERIFIED" | "TAMPERED" | "PENDING" */
    status: "VERIFIED" | "TAMPERED" | "PENDING";
    /** Boolean flag for quick machine verification */
    verificationPassed: boolean;
    /** Human-readable explorer links for all on-chain events */
    explorerLinks: { [chain: string]: string };
}

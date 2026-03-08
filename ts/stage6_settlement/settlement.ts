/**
 * Stage 6: Multi-Chain Settlement
 * "Verified data point transformed into a liquid asset..."
 * — Odyssey of a Data Point
 */

/** Types of carbon offset tokens */
export type TokenType = "LCO2" | "VCO2";

/**
 * Representation of a carbon credit token minted from sensor data.
 */
export interface CarbonToken {
    tokenType: TokenType;
    /** Amount in metric tons of CO2 equivalent (tCO2e) */
    amount: number;
    /** The batch ID that generated this token */
    batchId: string;
    /** Unix timestamp of the credit calculation */
    timestamp: number;
}

/**
 * A receipt for a successful settlement (minting) on a blockchain.
 */
export interface SettlementReceipt {
    /** The blockchain (e.g., "Cardano", "Hedera", "Base", "Celo") */
    chain: string;
    /** The transaction ID of the minting/settlement transaction */
    txId: string;
    /** The specific on-chain token/asset identifier */
    tokenId: string;
    /** The amount minted on this specific chain */
    amount: number;
}

/** 
 * Final settlement status for a data batch.
 */
export interface SettlementStatus {
    batchId: string;
    token?: CarbonToken;
    /** Collection of receipts from all enabled blockchains */
    receipts: SettlementReceipt[];
    /** Whether the settlement process has successfully completed across all chains */
    isSettled: boolean;
}

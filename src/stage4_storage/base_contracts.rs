//! Stage 4 — Prompt 28: BASE Solidity Contract Simulation
//!
//! Three Solidity contracts for BASE (EVM L2):
//!   1. SensorRegistry.sol   — ERC-721 NFT per sensor
//!   2. MerkleRootAnchor.sol — Record Merkle roots on-chain
//!   3. CarbonTokens.sol     — ERC-20 LCO₂ and VCO₂ tokens
//!
//! This Rust module simulates the off-chain ethers.js equivalent.
//! In production, use alloy-rs or ethers-rs to call BASE RPC.
//!
//! BASE testnet RPC: https://sepolia.base.org
//! Basescan verification: https://sepolia.basescan.org

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// ── EVM address type ──────────────────────────────────────────────────────────

pub type EvmAddress = String; // "0x..." 42-char hex

// ── Solidity contract types ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BaseContract {
    SensorRegistryERC721,
    MerkleRootAnchor,
    CarbonTokensERC20,
}

impl std::fmt::Display for BaseContract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BaseContract::SensorRegistryERC721 => write!(f, "SensorRegistry.sol"),
            BaseContract::MerkleRootAnchor => write!(f, "MerkleRootAnchor.sol"),
            BaseContract::CarbonTokensERC20 => write!(f, "CarbonTokens.sol"),
        }
    }
}

// ── Transaction receipt ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmTxReceipt {
    pub tx_hash: String,   // 0x + 64 hex chars
    pub block_number: u64,
    pub gas_used: u64,
    pub gas_price_gwei: u64,
    pub fee_usd_cents: u64,   // estimated at $0.001 per gwei at 2000 ETH/USD
    pub contract: BaseContract,
    pub submitted_at: DateTime<Utc>,
    pub status: bool,          // true = success (1), false = reverted (0)
}

impl EvmTxReceipt {
    pub fn fee_usd(&self) -> f64 { self.fee_usd_cents as f64 / 100.0 }
}

// ── ERC-721 Sensor NFT ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorNft {
    pub token_id: u64,
    pub owner: EvmAddress,
    pub sensor_did: String,
    pub token_uri: String, // ipfs://<CID>
    pub minted_at_block: u64,
}

// ── Merkle root anchor event ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleAnchorEvent {
    pub batch_id: String,
    pub merkle_root: [u8; 32],
    pub ipfs_cid: String,
    pub quorum_size: u8,
    pub block_number: u64,
}

// ── Carbon token balances ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarbonBalance {
    /// LCO₂ balance in wei (1e18 = 1 tonne).
    pub lco2_wei: u128,
    /// VCO₂ balance in wei.
    pub vco2_wei: u128,
}

impl CarbonBalance {
    pub fn lco2_tonnes(&self) -> f64 { self.lco2_wei as f64 / 1e18 }
    pub fn vco2_tonnes(&self) -> f64 { self.vco2_wei as f64 / 1e18 }
}

// ── Mock BASE/EVM client ──────────────────────────────────────────────────────

pub struct MockBaseClient {
    pub current_block: u64,
    pub nfts: HashMap<u64, SensorNft>,
    pub merkle_events: Vec<MerkleAnchorEvent>,
    pub carbon_balances: HashMap<EvmAddress, CarbonBalance>,
    pub next_token_id: u64,
}

impl MockBaseClient {
    pub fn new() -> Self {
        Self {
            current_block: 1_000_000,
            nfts: HashMap::new(),
            merkle_events: Vec::new(),
            carbon_balances: HashMap::new(),
            next_token_id: 1,
        }
    }

    fn next_block(&mut self) -> u64 {
        self.current_block += 1;
        self.current_block
    }

    fn tx_hash(data: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(data);
        format!("0x{}", hex::encode(h.finalize()))
    }

    /// ERC-721: Mint a sensor NFT.
    pub fn mint_sensor_nft(
        &mut self,
        owner: &str,
        sensor_did: &str,
        ipfs_cid: &str,
    ) -> Result<EvmTxReceipt, String> {
        let token_id = self.next_token_id;
        self.next_token_id += 1;
        let block = self.next_block();

        let nft = SensorNft {
            token_id,
            owner: owner.to_string(),
            sensor_did: sensor_did.to_string(),
            token_uri: format!("ipfs://{ipfs_cid}"),
            minted_at_block: block,
        };
        self.nfts.insert(token_id, nft);

        let data = format!("{owner}{sensor_did}{token_id}");
        Ok(EvmTxReceipt {
            tx_hash: Self::tx_hash(data.as_bytes()),
            block_number: block,
            gas_used: 150_000,
            gas_price_gwei: 1,
            fee_usd_cents: 1, // ~$0.01
            contract: BaseContract::SensorRegistryERC721,
            submitted_at: Utc::now(),
            status: true,
        })
    }

    /// Anchor a Merkle root on BASE.
    pub fn anchor_merkle_root(
        &mut self,
        batch_id: &str,
        merkle_root_hex: &str,
        ipfs_cid: &str,
        quorum_size: u8,
    ) -> Result<EvmTxReceipt, String> {
        if quorum_size < 2 {
            return Err("Quorum < 2".to_string());
        }
        if merkle_root_hex.len() != 64 {
            return Err("Merkle root must be 64-char hex".to_string());
        }

        let mut root_bytes = [0u8; 32];
        hex::decode_to_slice(merkle_root_hex, &mut root_bytes)
            .map_err(|e| e.to_string())?;

        let block = self.next_block();
        let event = MerkleAnchorEvent {
            batch_id: batch_id.to_string(),
            merkle_root: root_bytes,
            ipfs_cid: ipfs_cid.to_string(),
            quorum_size,
            block_number: block,
        };
        self.merkle_events.push(event);

        let data = format!("{batch_id}{merkle_root_hex}");
        Ok(EvmTxReceipt {
            tx_hash: Self::tx_hash(data.as_bytes()),
            block_number: block,
            gas_used: 80_000,
            gas_price_gwei: 1,
            fee_usd_cents: 0, // sub-cent on BASE
            contract: BaseContract::MerkleRootAnchor,
            submitted_at: Utc::now(),
            status: true,
        })
    }

    /// ERC-20: Mint LCO₂ tokens.
    pub fn mint_lco2(
        &mut self,
        to: &str,
        amount_wei: u128,
    ) -> Result<EvmTxReceipt, String> {
        let block = self.next_block();
        let entry = self.carbon_balances.entry(to.to_string()).or_insert(CarbonBalance { lco2_wei: 0, vco2_wei: 0 });
        entry.lco2_wei += amount_wei;

        let data = format!("{to}{amount_wei}lco2");
        Ok(EvmTxReceipt {
            tx_hash: Self::tx_hash(data.as_bytes()),
            block_number: block,
            gas_used: 60_000,
            gas_price_gwei: 1,
            fee_usd_cents: 0,
            contract: BaseContract::CarbonTokensERC20,
            submitted_at: Utc::now(),
            status: true,
        })
    }

    /// ERC-20: Mint VCO₂ tokens.
    pub fn mint_vco2(&mut self, to: &str, amount_wei: u128) -> Result<EvmTxReceipt, String> {
        let block = self.next_block();
        let entry = self.carbon_balances.entry(to.to_string()).or_insert(CarbonBalance { lco2_wei: 0, vco2_wei: 0 });
        entry.vco2_wei += amount_wei;

        let data = format!("{to}{amount_wei}vco2");
        Ok(EvmTxReceipt {
            tx_hash: Self::tx_hash(data.as_bytes()),
            block_number: block,
            gas_used: 60_000,
            gas_price_gwei: 1,
            fee_usd_cents: 0,
            contract: BaseContract::CarbonTokensERC20,
            submitted_at: Utc::now(),
            status: true,
        })
    }

    pub fn balance_of(&self, address: &str) -> Option<&CarbonBalance> {
        self.carbon_balances.get(address)
    }

    pub fn get_nft(&self, token_id: u64) -> Option<&SensorNft> {
        self.nfts.get(&token_id)
    }

    pub fn merkle_events_for(&self, batch_id: &str) -> Vec<&MerkleAnchorEvent> {
        self.merkle_events.iter().filter(|e| e.batch_id == batch_id).collect()
    }
}

impl Default for MockBaseClient { fn default() -> Self { Self::new() } }

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: &str = "0x1234567890123456789012345678901234567890";
    fn root64() -> String { "a".repeat(64) }

    // ── Test 1: mint NFT returns token ID 1 ──────────────────────────────────

    #[test]
    fn test_mint_nft_token_id_starts_at_1() {
        let mut client = MockBaseClient::new();
        client.mint_sensor_nft(OWNER, "did:cardano:sensor:s1", "QmCID").unwrap();
        assert!(client.get_nft(1).is_some(), "Token ID 1 must exist");
        let nft = client.get_nft(1).unwrap();
        assert!(nft.token_uri.starts_with("ipfs://"));
    }

    // ── Test 2: token IDs increment ───────────────────────────────────────────

    #[test]
    fn test_token_ids_increment() {
        let mut client = MockBaseClient::new();
        let r1 = client.mint_sensor_nft(OWNER, "s1", "cid1").unwrap();
        let r2 = client.mint_sensor_nft(OWNER, "s2", "cid2").unwrap();
        assert!(r2.block_number > r1.block_number);
        assert!(client.get_nft(2).is_some());
    }

    // ── Test 3: anchor merkle root succeeds ──────────────────────────────────

    #[test]
    fn test_anchor_merkle_root_success() {
        let mut client = MockBaseClient::new();
        let recv = client.anchor_merkle_root("b1", &root64(), "QmCID", 2).unwrap();
        assert!(recv.status);
        assert_eq!(recv.contract, BaseContract::MerkleRootAnchor);
        assert_eq!(client.merkle_events_for("b1").len(), 1);
    }

    // ── Test 4: anchor rejects < 2 quorum ────────────────────────────────────

    #[test]
    fn test_anchor_rejects_low_quorum() {
        let mut client = MockBaseClient::new();
        let result = client.anchor_merkle_root("b1", &root64(), "QmCID", 1);
        assert!(result.is_err());
    }

    // ── Test 5: mint LCO₂ increases balance ───────────────────────────────────

    #[test]
    fn test_mint_lco2_increases_balance() {
        let mut client = MockBaseClient::new();
        let one_tonne_wei: u128 = 1_000_000_000_000_000_000;
        client.mint_lco2(OWNER, one_tonne_wei).unwrap();
        let bal = client.balance_of(OWNER).unwrap();
        assert_eq!(bal.lco2_wei, one_tonne_wei);
        assert!((bal.lco2_tonnes() - 1.0).abs() < 1e-9);
    }

    // ── Test 6: mint VCO₂ independent of LCO₂ ─────────────────────────────────

    #[test]
    fn test_mint_vco2_independent() {
        let mut client = MockBaseClient::new();
        let one_tonne: u128 = 1_000_000_000_000_000_000;
        client.mint_lco2(OWNER, one_tonne).unwrap();
        client.mint_vco2(OWNER, one_tonne * 2).unwrap();
        let bal = client.balance_of(OWNER).unwrap();
        assert!((bal.lco2_tonnes() - 1.0).abs() < 1e-9);
        assert!((bal.vco2_tonnes() - 2.0).abs() < 1e-9);
    }

    // ── Test 7: invalid merkle root hex rejected ──────────────────────────────

    #[test]
    fn test_invalid_merkle_root_rejected() {
        let mut client = MockBaseClient::new();
        let result = client.anchor_merkle_root("b1", "tooshort", "QmCID", 2);
        assert!(result.is_err());
    }

    // ── Test 8: tx hash is 0x + 64 hex ────────────────────────────────────────

    #[test]
    fn test_tx_hash_format() {
        let mut client = MockBaseClient::new();
        let recv = client.mint_sensor_nft(OWNER, "s1", "CID").unwrap();
        assert!(recv.tx_hash.starts_with("0x"));
        assert_eq!(recv.tx_hash.len(), 66, "0x + 64 hex chars");
    }
}

//! Stage 4 — Prompt 29: HEDERA HCS & HTS Integration
//!
//! Two Hedera services:
//!   HCS (Hashgraph Consensus Service) — Submit Merkle roots as consensus messages
//!                                        Topic: malama.merkle-roots (fixed TopicID)
//!   HTS (Hashgraph Token Service)    — Create and transfer LCO₂ and VCO₂ tokens
//!
//! Article 6.4 sovereign program support:
//!   Each VCO₂ token carries a country-code attribute and ITMOReference
//!   (International Transferred Mitigation Outcome) as per Paris Agreement.
//!
//! Production: use hedera-sdk-rust or HTTP Mirror Node API.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// ── HCS Topic ─────────────────────────────────────────────────────────────────

/// A Hedera Consensus Service message submitted to a topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HcsMessage {
    pub topic_id: String,          // "0.0.12345678"
    pub sequence_number: u64,
    pub consensus_timestamp: DateTime<Utc>,
    pub message_bytes: Vec<u8>,
    pub running_hash: String,      // SHA-384 of accumulated messages
    pub transaction_id: String,
}

/// Content of a Merkle-root HCS message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleRootHcsPayload {
    pub batch_id: String,
    pub merkle_root: String,
    pub ipfs_cid: String,
    pub quorum_size: u8,
    pub protocol_version: String,
}

// ── HTS Token ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HtsTokenType {
    /// Logged Carbon (preliminary measurement).
    LCO2,
    /// Verified Carbon (Verra-confirmed removal tonne).
    VCO2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtsToken {
    pub token_id: String,       // "0.0.98765432"
    pub token_type: HtsTokenType,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,           // 6 decimals (1 token = 1 micro-tonne for precision)
    pub treasury_account: String,
}

/// Article 6.4 ITMO metadata on a VCO₂ token transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItmoMetadata {
    /// ISO 3166-1 alpha-2 country code of the sovereign program.
    pub country_code: String,
    /// ITMO reference code assigned by national registry.
    pub itmo_reference: String,
    /// Corresponding adjustment: amount subtracted from host country's NDC.
    pub corresponding_adjustment_tonnes: f64,
    pub activity_type: String,   // e.g. "Biochar sequestration"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtsTransferReceipt {
    pub transaction_id: String,
    pub token_id: String,
    pub from: String,
    pub to: String,
    pub amount: u64,             // in token's smallest unit (10^-6 tonne)
    pub itmo: Option<ItmoMetadata>,
    pub timestamp: DateTime<Utc>,
}

// ── Mock Hedera client ─────────────────────────────────────────────────────────

pub struct MockHederaClient {
    pub hcs_messages: Vec<HcsMessage>,
    pub hts_tokens: HashMap<String, HtsToken>,
    pub balances: HashMap<String, HashMap<String, u64>>, // account → token_id → balance
    pub next_seq: u64,
}

impl MockHederaClient {
    pub fn new() -> Self {
        Self {
            hcs_messages: Vec::new(),
            hts_tokens: HashMap::new(),
            balances: HashMap::new(),
            next_seq: 1,
        }
    }

    fn tx_id(seed: &str) -> String {
        let mut h = Sha256::new();
        h.update(seed.as_bytes());
        format!("0.0.1234@{}", hex::encode(&h.finalize()[..8]))
    }

    // ── HCS ──

    /// Submit a Merkle root message to the HCS topic.
    pub fn submit_hcs_message(
        &mut self,
        topic_id: &str,
        payload: &MerkleRootHcsPayload,
    ) -> Result<HcsMessage, String> {
        let bytes = serde_json::to_vec(payload).map_err(|e| e.to_string())?;

        // Simulated running hash (SHA-256 of seq + content)
        let mut h = Sha256::new();
        h.update(self.next_seq.to_le_bytes());
        h.update(&bytes);
        let running_hash = hex::encode(h.finalize());

        let msg = HcsMessage {
            topic_id: topic_id.to_string(),
            sequence_number: self.next_seq,
            consensus_timestamp: Utc::now(),
            message_bytes: bytes,
            running_hash,
            transaction_id: Self::tx_id(&format!("{topic_id}{}", self.next_seq)),
        };
        self.next_seq += 1;
        self.hcs_messages.push(msg.clone());
        Ok(msg)
    }

    /// Get all messages for a topic in sequence order.
    pub fn topic_messages(&self, topic_id: &str) -> Vec<&HcsMessage> {
        let mut msgs: Vec<&HcsMessage> = self.hcs_messages.iter()
            .filter(|m| m.topic_id == topic_id)
            .collect();
        msgs.sort_by_key(|m| m.sequence_number);
        msgs
    }

    // ── HTS ──

    /// Create a new HTS token.
    pub fn create_token(
        &mut self,
        token_type: HtsTokenType,
        treasury: &str,
    ) -> Result<String, String> {
        let (name, symbol) = match token_type {
            HtsTokenType::LCO2 => ("Logged Carbon", "LCO2"),
            HtsTokenType::VCO2 => ("Verified Carbon", "VCO2"),
        };
        let token_id = format!("0.0.{}", 90_000_000 + self.hts_tokens.len() as u64);
        let token = HtsToken {
            token_id: token_id.clone(),
            token_type,
            name: name.to_string(),
            symbol: symbol.to_string(),
            decimals: 6,
            treasury_account: treasury.to_string(),
        };
        self.hts_tokens.insert(token_id.clone(), token);
        // Seed treasury with 0 balance
        self.balances.entry(treasury.to_string())
            .or_default()
            .insert(token_id.clone(), 0);
        Ok(token_id)
    }

    /// Mint tokens to treasury.
    pub fn mint(&mut self, token_id: &str, amount: u64) -> Result<(), String> {
        let treasury = self.hts_tokens.get(token_id)
            .map(|t| t.treasury_account.clone())
            .ok_or_else(|| format!("Token {token_id} not found"))?;
        *self.balances.entry(treasury).or_default().entry(token_id.to_string()).or_insert(0) += amount;
        Ok(())
    }

    /// Transfer tokens between accounts (with optional ITMO metadata for VCO₂).
    pub fn transfer(
        &mut self,
        token_id: &str,
        from: &str,
        to: &str,
        amount: u64,
        itmo: Option<ItmoMetadata>,
    ) -> Result<HtsTransferReceipt, String> {
        // Check token exists
        if !self.hts_tokens.contains_key(token_id) {
            return Err(format!("Token {token_id} not found"));
        }

        // Check balance
        let from_bal = self.balances.entry(from.to_string()).or_default()
            .entry(token_id.to_string()).or_insert(0);
        if *from_bal < amount {
            return Err(format!("Insufficient balance: have {from_bal}, need {amount}"));
        }
        *from_bal -= amount;

        *self.balances.entry(to.to_string()).or_default()
            .entry(token_id.to_string()).or_insert(0) += amount;

        Ok(HtsTransferReceipt {
            transaction_id: Self::tx_id(&format!("{token_id}{from}{to}{amount}")),
            token_id: token_id.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            amount,
            itmo,
            timestamp: Utc::now(),
        })
    }

    pub fn balance_of(&self, account: &str, token_id: &str) -> u64 {
        self.balances.get(account)
            .and_then(|m| m.get(token_id))
            .copied()
            .unwrap_or(0)
    }
}

impl Default for MockHederaClient { fn default() -> Self { Self::new() } }

pub const MALAMA_HCS_TOPIC: &str = "0.0.4847200"; // production topic ID

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const TOPIC: &str = MALAMA_HCS_TOPIC;
    const TREASURY: &str = "0.0.111111";
    const FARMER:   &str = "0.0.222222";

    fn payload(batch_id: &str) -> MerkleRootHcsPayload {
        MerkleRootHcsPayload {
            batch_id: batch_id.to_string(),
            merkle_root: "a".repeat(64),
            ipfs_cid: "QmXyz".to_string(),
            quorum_size: 2,
            protocol_version: "1.0.0".to_string(),
        }
    }

    // ── Test 1: HCS message submitted with sequence number ────────────────────

    #[test]
    fn test_hcs_message_has_sequence_number() {
        let mut client = MockHederaClient::new();
        let msg = client.submit_hcs_message(TOPIC, &payload("b1")).unwrap();
        assert_eq!(msg.sequence_number, 1);
        assert_eq!(msg.topic_id, TOPIC);
    }

    // ── Test 2: sequence increments per topic ─────────────────────────────────

    #[test]
    fn test_hcs_sequence_increments() {
        let mut client = MockHederaClient::new();
        let m1 = client.submit_hcs_message(TOPIC, &payload("b1")).unwrap();
        let m2 = client.submit_hcs_message(TOPIC, &payload("b2")).unwrap();
        assert_eq!(m2.sequence_number, m1.sequence_number + 1);
    }

    // ── Test 3: topic_messages returns in order ───────────────────────────────

    #[test]
    fn test_topic_messages_in_order() {
        let mut client = MockHederaClient::new();
        client.submit_hcs_message(TOPIC, &payload("b1")).unwrap();
        client.submit_hcs_message(TOPIC, &payload("b2")).unwrap();
        let msgs = client.topic_messages(TOPIC);
        assert_eq!(msgs.len(), 2);
        assert!(msgs[0].sequence_number < msgs[1].sequence_number);
    }

    // ── Test 4: create LCO₂ token ─────────────────────────────────────────────

    #[test]
    fn test_create_lco2_token() {
        let mut client = MockHederaClient::new();
        let token_id = client.create_token(HtsTokenType::LCO2, TREASURY).unwrap();
        assert!(token_id.starts_with("0.0."));
        let token = client.hts_tokens.get(&token_id).unwrap();
        assert_eq!(token.symbol, "LCO2");
        assert_eq!(token.decimals, 6);
    }

    // ── Test 5: mint tokens to treasury ──────────────────────────────────────

    #[test]
    fn test_mint_increases_treasury_balance() {
        let mut client = MockHederaClient::new();
        let tid = client.create_token(HtsTokenType::LCO2, TREASURY).unwrap();
        client.mint(&tid, 1_000_000).unwrap(); // 1 tonne
        assert_eq!(client.balance_of(TREASURY, &tid), 1_000_000);
    }

    // ── Test 6: transfer moves balance ────────────────────────────────────────

    #[test]
    fn test_transfer_moves_balance() {
        let mut client = MockHederaClient::new();
        let tid = client.create_token(HtsTokenType::VCO2, TREASURY).unwrap();
        client.mint(&tid, 5_000_000).unwrap();
        client.transfer(&tid, TREASURY, FARMER, 2_000_000, None).unwrap();
        assert_eq!(client.balance_of(TREASURY, &tid), 3_000_000);
        assert_eq!(client.balance_of(FARMER, &tid), 2_000_000);
    }

    // ── Test 7: insufficient balance rejected ─────────────────────────────────

    #[test]
    fn test_insufficient_balance_rejected() {
        let mut client = MockHederaClient::new();
        let tid = client.create_token(HtsTokenType::LCO2, TREASURY).unwrap();
        client.mint(&tid, 100).unwrap();
        let result = client.transfer(&tid, TREASURY, FARMER, 200, None);
        assert!(result.is_err(), "Must reject over-transfer");
    }

    // ── Test 8: VCO₂ transfer with Article 6.4 ITMO metadata ─────────────────

    #[test]
    fn test_vco2_transfer_with_itmo() {
        let mut client = MockHederaClient::new();
        let tid = client.create_token(HtsTokenType::VCO2, TREASURY).unwrap();
        client.mint(&tid, 1_000_000).unwrap();
        let itmo = ItmoMetadata {
            country_code: "FJ".to_string(),
            itmo_reference: "FJI-2025-BIOCHAR-001".to_string(),
            corresponding_adjustment_tonnes: 1.0,
            activity_type: "Biochar sequestration".to_string(),
        };
        let receipt = client.transfer(&tid, TREASURY, FARMER, 1_000_000, Some(itmo)).unwrap();
        assert!(receipt.itmo.is_some());
        assert_eq!(receipt.itmo.unwrap().country_code, "FJ");
    }

    // ── Test 9: running hash changes each message ──────────────────────────────

    #[test]
    fn test_running_hash_changes() {
        let mut client = MockHederaClient::new();
        let m1 = client.submit_hcs_message(TOPIC, &payload("b1")).unwrap();
        let m2 = client.submit_hcs_message(TOPIC, &payload("b2")).unwrap();
        assert_ne!(m1.running_hash, m2.running_hash, "Running hash must advance");
    }
}

//! Stage 4 — Prompt 30: CELO Mobile SDK Integration
//!
//! Three components:
//!   1. USSD Gateway — SMS-based sensor reading submission (feature phones)
//!   2. cUSD Settlement — Stablecoin payments to farmers via CELO
//!   3. Smallholder Onboarding — Simplified KYC and sensor registration flow
//!
//! This bridges web3 to the developing world where smartphones may not exist.
//! A farmer dials *384*MALAMA# → enters readings → data auto-validates → payment.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// ── USSD session states ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UssdState {
    /// Initial USSD session opened.
    MenuShown,
    /// Waiting for farmer's phone number verification.
    AwaitingVerification,
    /// Waiting for sensor reading input.
    AwaitingReading,
    /// Reading submitted — waiting for validation.
    ReadingValidating,
    /// Session complete — payment issued.
    PaymentIssued,
    /// Session timed out.
    SessionExpired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UssdSession {
    pub session_id: String,
    pub phone_number: String,
    pub farmer_id: Option<String>,
    pub state: UssdState,
    pub readings: Vec<f64>,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

impl UssdSession {
    pub fn new(phone_number: &str) -> Self {
        let now = Utc::now();
        let mut h = Sha256::new();
        h.update(phone_number.as_bytes());
        h.update(now.timestamp().to_le_bytes());
        let id = hex::encode(&h.finalize()[..8]);

        Self {
            session_id: id,
            phone_number: phone_number.to_string(),
            farmer_id: None,
            state: UssdState::MenuShown,
            readings: Vec::new(),
            started_at: now,
            last_activity: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        (Utc::now() - self.last_activity).num_seconds() > 120 // 2-min USSD timeout
    }
}

// ── USSD menu responses ───────────────────────────────────────────────────────

/// Menu strings sent back to the phone (max 182 chars for USSD).
pub fn ussd_menu_main() -> &'static str {
    "CON Welcome to Mālama Protocol\n1. Submit sensor reading\n2. Check carbon balance\n3. Contact support"
}

pub fn ussd_menu_reading() -> &'static str {
    "CON Enter temperature (°C):\nExample: 23.4"
}

pub fn ussd_response_submitted(cusd_amount: f64) -> String {
    format!("END Reading submitted ✓\nPayment: {cusd_amount:.2} cUSD\nThank you!")
}

// ── cUSD settlement ───────────────────────────────────────────────────────────

/// cUSD token precision: 18 decimals (same as ETH).
const CUSD_DECIMALS: u128 = 1_000_000_000_000_000_000;

/// Payment rate: 0.01 cUSD per valid sensor reading.
pub const CUSD_PER_READING: f64 = 0.01;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CusdPayment {
    pub tx_hash: String,
    pub from: String,  // protocol treasury address
    pub to: String,    // farmer's CELO address or phone-linked account
    pub amount_wei: u128,
    pub readings_count: usize,
    pub timestamp: DateTime<Utc>,
}

impl CusdPayment {
    pub fn amount_cusd(&self) -> f64 { self.amount_wei as f64 / CUSD_DECIMALS as f64 }
}

// ── Farmer onboarding ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KycStatus {
    Pending,
    Approved,
    Rejected { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmallholderFarmer {
    pub farmer_id: String,
    pub phone_number: String,
    pub celo_address: Option<String>,
    pub location: String,   // "Village, District, Country"
    pub kyc_status: KycStatus,
    pub registered_at: DateTime<Utc>,
    pub total_readings_submitted: u64,
    pub total_cusd_earned: f64,
}

impl SmallholderFarmer {
    pub fn new(phone: &str, location: &str) -> Self {
        let mut h = Sha256::new();
        h.update(phone.as_bytes());
        let id = format!("farmer-{}", hex::encode(&h.finalize()[..4]));
        Self {
            farmer_id: id,
            phone_number: phone.to_string(),
            celo_address: None,
            location: location.to_string(),
            kyc_status: KycStatus::Pending,
            registered_at: Utc::now(),
            total_readings_submitted: 0,
            total_cusd_earned: 0.0,
        }
    }
}

// ── Celo gateway (mock) ───────────────────────────────────────────────────────

pub struct CeloGateway {
    pub farmers: HashMap<String, SmallholderFarmer>, // phone → farmer
    pub sessions: HashMap<String, UssdSession>,      // session_id → session
    pub payments: Vec<CusdPayment>,
    pub treasury_address: String,
    cusd_balances: HashMap<String, u128>,            // address → wei
}

impl CeloGateway {
    pub fn new(treasury: &str) -> Self {
        let mut g = Self {
            farmers: HashMap::new(),
            sessions: HashMap::new(),
            payments: Vec::new(),
            treasury_address: treasury.to_string(),
            cusd_balances: HashMap::new(),
        };
        // Seed treasury
        g.cusd_balances.insert(treasury.to_string(), 100_000 * CUSD_DECIMALS);
        g
    }

    /// Register a new smallholder farmer via phone number.
    pub fn register_farmer(&mut self, phone: &str, location: &str) -> &SmallholderFarmer {
        let farmer = SmallholderFarmer::new(phone, location);
        self.farmers.insert(phone.to_string(), farmer);
        self.farmers.get(phone).unwrap()
    }

    /// Approve KYC for a farmer.
    pub fn approve_kyc(&mut self, phone: &str) -> bool {
        if let Some(f) = self.farmers.get_mut(phone) {
            f.kyc_status = KycStatus::Approved;
            return true;
        }
        false
    }

    /// Open a USSD session for a phone number.
    pub fn open_ussd(&mut self, phone: &str) -> &UssdSession {
        let session = UssdSession::new(phone);
        let id = session.session_id.clone();
        self.sessions.insert(id.clone(), session);
        self.sessions.get(&id).unwrap()
    }

    /// Submit a reading via USSD session for a registered farmer.
    pub fn submit_reading_via_ussd(
        &mut self,
        session_id: &str,
        reading: f64,
    ) -> Result<CusdPayment, String> {
        // Look up session
        let phone = self.sessions.get(session_id)
            .map(|s| s.phone_number.clone())
            .ok_or("Session not found")?;

        if self.sessions.get(session_id).map(|s| s.is_expired()).unwrap_or(true) {
            return Err("Session expired".to_string());
        }

        // Farmer must be KYC approved
        let farmer_approved = self.farmers.get(&phone)
            .map(|f| f.kyc_status == KycStatus::Approved)
            .unwrap_or(false);

        if !farmer_approved {
            return Err("Farmer KYC not approved".to_string());
        }

        // Record reading
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.readings.push(reading);
            session.state = UssdState::PaymentIssued;
        }

        // Issue cUSD payment
        let amount_wei = (CUSD_PER_READING * CUSD_DECIMALS as f64) as u128;
        let farmer_addr = format!("0xCELO_{phone}");

        // Debit treasury
        let treasury_bal = self.cusd_balances.entry(self.treasury_address.clone()).or_insert(0);
        if *treasury_bal < amount_wei {
            return Err("Treasury insufficient cUSD".to_string());
        }
        *treasury_bal -= amount_wei;
        *self.cusd_balances.entry(farmer_addr.clone()).or_insert(0) += amount_wei;

        // Update farmer stats
        if let Some(f) = self.farmers.get_mut(&phone) {
            f.total_readings_submitted += 1;
            f.total_cusd_earned += CUSD_PER_READING;
        }

        let mut h = Sha256::new();
        h.update(session_id.as_bytes());
        h.update(reading.to_bits().to_le_bytes());
        let tx_hash = format!("0xCELO-{}", hex::encode(&h.finalize()[..16]));

        let payment = CusdPayment {
            tx_hash,
            from: self.treasury_address.clone(),
            to: farmer_addr,
            amount_wei,
            readings_count: 1,
            timestamp: Utc::now(),
        };
        self.payments.push(payment.clone());
        Ok(payment)
    }

    pub fn cusd_balance_of(&self, address: &str) -> f64 {
        self.cusd_balances.get(address).copied().unwrap_or(0) as f64 / CUSD_DECIMALS as f64
    }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const TREASURY: &str = "0xMALAMA_TREASURY";
    const PHONE: &str = "+679-123-4567";
    const LOC: &str = "Nadi, Western, Fiji";

    fn setup() -> CeloGateway {
        let mut gw = CeloGateway::new(TREASURY);
        gw.register_farmer(PHONE, LOC);
        gw.approve_kyc(PHONE);
        gw
    }

    // ── Test 1: farmer registered with pending KYC ────────────────────────────

    #[test]
    fn test_farmer_registered_pending_kyc() {
        let mut gw = CeloGateway::new(TREASURY);
        let farmer = gw.register_farmer(PHONE, LOC);
        assert_eq!(farmer.kyc_status, KycStatus::Pending);
        assert_eq!(farmer.phone_number, PHONE);
    }

    // ── Test 2: KYC approval transitions status ───────────────────────────────

    #[test]
    fn test_kyc_approval() {
        let mut gw = CeloGateway::new(TREASURY);
        gw.register_farmer(PHONE, LOC);
        assert!(gw.approve_kyc(PHONE));
        assert_eq!(gw.farmers[PHONE].kyc_status, KycStatus::Approved);
    }

    // ── Test 3: USSD session opened ──────────────────────────────────────────

    #[test]
    fn test_ussd_session_opened() {
        let mut gw = CeloGateway::new(TREASURY);
        let session = gw.open_ussd(PHONE);
        assert_eq!(session.phone_number, PHONE);
        assert_eq!(session.state, UssdState::MenuShown);
    }

    // ── Test 4: reading submission succeeds with KYC ──────────────────────────

    #[test]
    fn test_reading_submission_success() {
        let mut gw = setup();
        let session = gw.open_ussd(PHONE);
        let sid = session.session_id.clone();
        let payment = gw.submit_reading_via_ussd(&sid, 23.4).unwrap();
        assert!((payment.amount_cusd() - CUSD_PER_READING).abs() < 1e-9);
    }

    // ── Test 5: reading rejected without KYC ─────────────────────────────────

    #[test]
    fn test_reading_rejected_without_kyc() {
        let mut gw = CeloGateway::new(TREASURY);
        gw.register_farmer(PHONE, LOC); // pending KYC
        let session = gw.open_ussd(PHONE);
        let sid = session.session_id.clone();
        let result = gw.submit_reading_via_ussd(&sid, 23.4);
        assert!(result.is_err());
    }

    // ── Test 6: cUSD payment moves from treasury to farmer ────────────────────

    #[test]
    fn test_cusd_payment_transfers() {
        let mut gw = setup();
        let farmer_addr = format!("0xCELO_{PHONE}");
        let session = gw.open_ussd(PHONE);
        let sid = session.session_id.clone();
        gw.submit_reading_via_ussd(&sid, 23.4).unwrap();
        assert!((gw.cusd_balance_of(&farmer_addr) - CUSD_PER_READING).abs() < 1e-9);
    }

    // ── Test 7: farmer stats updated after submission ──────────────────────────

    #[test]
    fn test_farmer_stats_updated() {
        let mut gw = setup();
        let session = gw.open_ussd(PHONE);
        let sid = session.session_id.clone();
        gw.submit_reading_via_ussd(&sid, 23.4).unwrap();
        let farmer = &gw.farmers[PHONE];
        assert_eq!(farmer.total_readings_submitted, 1);
        assert!((farmer.total_cusd_earned - CUSD_PER_READING).abs() < 1e-9);
    }

    // ── Test 8: USSD menu strings are short enough ────────────────────────────

    #[test]
    fn test_ussd_menu_length() {
        assert!(ussd_menu_main().len() <= 182, "USSD message must be ≤182 chars");
        assert!(ussd_menu_reading().len() <= 182);
    }

    // ── Test 9: CUSD_PER_READING is 0.01 ─────────────────────────────────────

    #[test]
    fn test_payment_rate() {
        assert!((CUSD_PER_READING - 0.01).abs() < 1e-9);
    }
}

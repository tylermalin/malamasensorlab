//! Stage 4 — Prompt 31: Transaction Fee Optimization & Cost Calculator
//!
//! Before submitting to any chain, estimate fees and route intelligently:
//!   - Critical data (Merkle roots) → Cardano (most secure, immutable)
//!   - High-frequency events (reputation) → HEDERA (fast, cheap)
//!   - DeFi / token ops → BASE (EVM ecosystem, low L2 fees)
//!   - Offline/mobile → CELO (USSD, sub-cent fees)
//!
//! Optimization policy:
//!   1. Cost ceiling: never pay > $1.00 per submission
//!   2. Prefer Cardano for critical data
//!   3. Batch non-critical submissions to HEDERA if cost > 3× HEDERA
//!   4. Generate monthly savings report comparing actual vs. naive routing

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

// ── Chain identifiers ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Chain {
    Cardano,
    Base,
    Hedera,
    Celo,
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Chain::Cardano => "Cardano",
            Chain::Base    => "BASE",
            Chain::Hedera  => "HEDERA",
            Chain::Celo    => "CELO",
        })
    }
}

// ── Data criticality ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataCriticality {
    /// Merkle roots, sensor registrations — must go to Cardano.
    Critical,
    /// Reputation updates, batch status — HEDERA preferred.
    Standard,
    /// Aggregated stats, analytics — cheapest chain.
    NonCritical,
}

// ── Fee estimate ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeEstimate {
    pub chain: Chain,
    /// Estimated fee in USD cents (1 cent = $0.01).
    pub fee_usd_cents: u64,
    pub fee_native: String,   // human-readable: "0.17 ADA", "0.0001 HBAR", etc.
    pub estimated_at: DateTime<Utc>,
    pub tps_capacity: u32,    // transactions per second available
    pub finality_secs: u32,   // time to finality in seconds
}

impl FeeEstimate {
    pub fn fee_usd(&self) -> f64 { self.fee_usd_cents as f64 / 100.0 }
}

// ── Fee model (baseline averages, 2025) ──────────────────────────────────────

pub fn estimate_fee(chain: &Chain, data_size_bytes: usize) -> FeeEstimate {
    match chain {
        Chain::Cardano => FeeEstimate {
            chain: Chain::Cardano,
            fee_usd_cents: 25,  // ~$0.25 per tx (0.17 ADA @ $1.5/ADA)
            fee_native: format!("0.17 ADA ({data_size_bytes}B)"),
            estimated_at: Utc::now(),
            tps_capacity: 250,
            finality_secs: 20,
        },
        Chain::Base => FeeEstimate {
            chain: Chain::Base,
            fee_usd_cents: 1,   // ~$0.01 on L2
            fee_native: format!("~0.000005 ETH ({data_size_bytes}B)"),
            estimated_at: Utc::now(),
            tps_capacity: 2000,
            finality_secs: 2,
        },
        Chain::Hedera => FeeEstimate {
            chain: Chain::Hedera,
            fee_usd_cents: 1,   // ~$0.0001–0.01 per tx
            fee_native: format!("0.0001 HBAR ({data_size_bytes}B)"),
            estimated_at: Utc::now(),
            tps_capacity: 10_000,
            finality_secs: 3,
        },
        Chain::Celo => FeeEstimate {
            chain: Chain::Celo,
            fee_usd_cents: 0,   // sub-cent (< $0.001)
            fee_native: format!("<0.001 cUSD ({data_size_bytes}B)"),
            estimated_at: Utc::now(),
            tps_capacity: 1000,
            finality_secs: 5,
        },
    }
}

// ── Routing decision ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub payload_description: String,
    pub criticality: DataCriticality,
    pub recommended_chain: Chain,
    pub reason: String,
    pub estimated_fee: FeeEstimate,
    /// Naive cost: what it would cost to send everything to Cardano.
    pub naive_fee_cents: u64,
    /// Savings vs. naive routing.
    pub savings_cents: u64,
}

impl RoutingDecision {
    pub fn savings_usd(&self) -> f64 { self.savings_cents as f64 / 100.0 }
}

// ── Fee optimizer ─────────────────────────────────────────────────────────────

pub struct FeeOptimizer {
    pub cost_ceiling_cents: u64,
    pub decisions: Vec<RoutingDecision>,
}

impl FeeOptimizer {
    pub fn new() -> Self {
        Self { cost_ceiling_cents: 100, decisions: Vec::new() }  // $1.00 ceiling
    }

    /// Choose the optimal chain for a given payload.
    pub fn route(
        &mut self,
        payload_desc: &str,
        criticality: DataCriticality,
        data_size_bytes: usize,
    ) -> &RoutingDecision {
        let (chain, reason) = match criticality {
            DataCriticality::Critical => (
                Chain::Cardano,
                "Critical data must be on Cardano (immutable, most secure)".to_string(),
            ),
            DataCriticality::Standard => {
                let hedera = estimate_fee(&Chain::Hedera, data_size_bytes);
                let cardano = estimate_fee(&Chain::Cardano, data_size_bytes);
                if hedera.fee_usd_cents * 3 < cardano.fee_usd_cents {
                    (Chain::Hedera, "Standard data: HEDERA is <1/3rd cost of Cardano".to_string())
                } else {
                    (Chain::Cardano, "Standard data: cost difference < 3×, prefer Cardano".to_string())
                }
            }
            DataCriticality::NonCritical => (
                Chain::Celo,
                "Non-critical: CELO is cheapest for low-priority operations".to_string(),
            ),
        };

        let estimated_fee = estimate_fee(&chain, data_size_bytes);

        // Cap: if recommended chain exceeds ceiling, try cheaper fallback
        let (final_chain, final_fee) = if estimated_fee.fee_usd_cents > self.cost_ceiling_cents {
            let fallback = Chain::Hedera;
            let fallback_fee = estimate_fee(&fallback, data_size_bytes);
            (fallback, fallback_fee)
        } else {
            (chain, estimated_fee)
        };

        let naive_fee_cents = estimate_fee(&Chain::Cardano, data_size_bytes).fee_usd_cents;
        let savings_cents = naive_fee_cents.saturating_sub(final_fee.fee_usd_cents);

        let decision = RoutingDecision {
            payload_description: payload_desc.to_string(),
            criticality,
            recommended_chain: final_chain,
            reason,
            estimated_fee: final_fee,
            naive_fee_cents,
            savings_cents,
        };
        self.decisions.push(decision);
        self.decisions.last().unwrap()
    }

    /// Generate a savings report for all routing decisions made.
    pub fn savings_report(&self) -> SavingsReport {
        let total_actual: u64 = self.decisions.iter().map(|d| d.estimated_fee.fee_usd_cents).sum();
        let total_naive: u64 = self.decisions.iter().map(|d| d.naive_fee_cents).sum();
        let total_saved = total_naive.saturating_sub(total_actual);

        let mut by_chain: HashMap<String, ChainUsageStats> = HashMap::new();
        for d in &self.decisions {
            let entry = by_chain.entry(d.recommended_chain.to_string()).or_insert(ChainUsageStats {
                chain: d.recommended_chain.to_string(),
                tx_count: 0,
                total_cost_cents: 0,
            });
            entry.tx_count += 1;
            entry.total_cost_cents += d.estimated_fee.fee_usd_cents;
        }

        SavingsReport {
            total_transactions: self.decisions.len(),
            total_actual_cost_cents: total_actual,
            total_naive_cost_cents: total_naive,
            total_saved_cents: total_saved,
            savings_percentage: if total_naive > 0 {
                (total_saved as f64 / total_naive as f64) * 100.0
            } else { 0.0 },
            by_chain,
        }
    }
}

impl Default for FeeOptimizer { fn default() -> Self { Self::new() } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainUsageStats {
    pub chain: String,
    pub tx_count: usize,
    pub total_cost_cents: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavingsReport {
    pub total_transactions: usize,
    pub total_actual_cost_cents: u64,
    pub total_naive_cost_cents: u64,
    pub total_saved_cents: u64,
    pub savings_percentage: f64,
    pub by_chain: HashMap<String, ChainUsageStats>,
}

impl SavingsReport {
    pub fn total_saved_usd(&self) -> f64 { self.total_saved_cents as f64 / 100.0 }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test 1: critical data routed to Cardano ───────────────────────────────

    #[test]
    fn test_critical_routed_to_cardano() {
        let mut opt = FeeOptimizer::new();
        let d = opt.route("Merkle Root", DataCriticality::Critical, 256);
        assert_eq!(d.recommended_chain, Chain::Cardano);
    }

    // ── Test 2: standard data routed to Hedera (cheaper) ─────────────────────

    #[test]
    fn test_standard_routed_to_hedera() {
        let mut opt = FeeOptimizer::new();
        let d = opt.route("Reputation Update", DataCriticality::Standard, 64);
        // Hedera = 1 cent, Cardano = 25 cents → 3× threshold → Hedera
        assert_eq!(d.recommended_chain, Chain::Hedera);
    }

    // ── Test 3: non-critical data routed to Celo ──────────────────────────────

    #[test]
    fn test_non_critical_routed_to_celo() {
        let mut opt = FeeOptimizer::new();
        let d = opt.route("Analytics event", DataCriticality::NonCritical, 32);
        assert_eq!(d.recommended_chain, Chain::Celo);
    }

    // ── Test 4: estimate_fee returns correct values ───────────────────────────

    #[test]
    fn test_estimate_fee_cardano() {
        let fee = estimate_fee(&Chain::Cardano, 256);
        assert_eq!(fee.fee_usd_cents, 25);
        assert_eq!(fee.finality_secs, 20);
    }

    // ── Test 5: savings = naive - actual ─────────────────────────────────────

    #[test]
    fn test_savings_calculation() {
        let mut opt = FeeOptimizer::new();
        opt.route("R1", DataCriticality::Standard, 100);
        opt.route("R2", DataCriticality::NonCritical, 100);
        let report = opt.savings_report();
        assert!(report.total_saved_cents > 0, "Routing optimization must save money");
    }

    // ── Test 6: savings report summary is correct ─────────────────────────────

    #[test]
    fn test_savings_report_totals() {
        let mut opt = FeeOptimizer::new();
        for _ in 0..10 {
            opt.route("Reputation", DataCriticality::Standard, 64);
        }
        let report = opt.savings_report();
        assert_eq!(report.total_transactions, 10);
        assert!(report.savings_percentage > 0.0);
    }

    // ── Test 7: Hedera is cheaper than Cardano ────────────────────────────────

    #[test]
    fn test_hedera_cheaper_than_cardano() {
        let h = estimate_fee(&Chain::Hedera, 256);
        let c = estimate_fee(&Chain::Cardano, 256);
        assert!(h.fee_usd_cents < c.fee_usd_cents, "HEDERA must be cheaper than Cardano");
    }

    // ── Test 8: cost ceiling fallback logic ───────────────────────────────────

    #[test]
    fn test_cost_ceiling_triggers_fallback() {
        let mut opt = FeeOptimizer::new();
        opt.cost_ceiling_cents = 0; // force any fee to exceed ceiling
        let d = opt.route("Critical Root", DataCriticality::Critical, 256);
        // Cardano = 25 cents > 0 ceiling → fallback to HEDERA
        assert_eq!(d.recommended_chain, Chain::Hedera);
    }

    // ── Test 9: HEDERA has highest TPS capacity ────────────────────────────────

    #[test]
    fn test_hedera_highest_tps() {
        let chains = [Chain::Cardano, Chain::Base, Chain::Hedera, Chain::Celo];
        let fees: Vec<FeeEstimate> = chains.iter().map(|c| estimate_fee(c, 128)).collect();
        // HEDERA has 10,000 TPS — highest of all chains
        let max_tps = fees.iter().map(|f| f.tps_capacity).max().unwrap();
        let hedera_tps = fees.iter().find(|f| f.chain == Chain::Hedera).unwrap().tps_capacity;
        assert_eq!(hedera_tps, max_tps, "HEDERA must have highest TPS capacity");
    }
}

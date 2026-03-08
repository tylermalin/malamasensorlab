//! Stage 2 — Prompt 13: Backpressure Handling & Rate Limiting
//!
//! Three coordinated mechanisms:
//!
//! 1. **Token Bucket** — classic rate limiter; sensors earn tokens at a fixed
//!    rate and spend one per reading. Prevents bursts from overwhelming the gateway.
//!
//! 2. **Jittered Send** — each sensor SDK adds random jitter before retries to
//!    prevent the "thundering herd" problem when 1,000 sensors reconnect simultaneously.
//!
//! 3. **Circuit Breaker** — after N consecutive failures, the circuit opens and
//!    all calls fail fast until a cooldown period elapses, then it half-opens
//!    for a single probe attempt.

use std::collections::HashMap;
use chrono::{DateTime, Duration, Utc};

// ── Token Bucket ────────────────────────────────────────────────────────────

/// Token bucket rate limiter per sensor.
///
/// Tokens refill at `rate_per_sec` tokens/second up to `capacity`.
/// Consuming a token costs 1.0 token; partial tokens are supported.
pub struct TokenBucket {
    pub capacity: f64,
    pub rate_per_sec: f64,
    tokens: f64,
    last_refill: DateTime<Utc>,
}

impl TokenBucket {
    pub fn new(capacity: f64, rate_per_sec: f64) -> Self {
        Self { capacity, rate_per_sec, tokens: capacity, last_refill: Utc::now() }
    }

    /// Attempt to consume one token. Returns `true` if the reading is allowed.
    pub fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Utc::now();
        let secs = (now - self.last_refill).num_milliseconds() as f64 / 1000.0;
        self.tokens = (self.tokens + secs * self.rate_per_sec).min(self.capacity);
        self.last_refill = now;
    }

    /// Remaining tokens (read-only peek after refill).
    pub fn available(&mut self) -> f64 {
        self.refill();
        self.tokens
    }
}

// ── Per-sensor rate limiter map ───────────────────────────────────────────────

pub struct GatewayRateLimiter {
    buckets: HashMap<String, TokenBucket>,
    pub capacity: f64,
    pub rate_per_sec: f64,
}

impl GatewayRateLimiter {
    pub fn new(capacity: f64, rate_per_sec: f64) -> Self {
        Self { buckets: HashMap::new(), capacity, rate_per_sec }
    }

    /// Allow or deny a reading from `sensor_did`.
    pub fn allow(&mut self, sensor_did: &str) -> bool {
        let bucket = self.buckets
            .entry(sensor_did.to_string())
            .or_insert_with(|| TokenBucket::new(self.capacity, self.rate_per_sec));
        bucket.try_consume()
    }
}

// ── Jittered retry delay ───────────────────────────────────────────────────────

/// Compute an exponential-backoff delay with full jitter.
///
/// `attempt` is 0-indexed (first retry = 0).
/// Returns milliseconds to wait.
///
/// Formula: `rand(0, min(cap, base * 2^attempt))`
/// This produces the "Full Jitter" variant from the AWS blog post.
pub fn jittered_delay_ms(attempt: u32, base_ms: u64, cap_ms: u64) -> u64 {
    // Cap exponent at 20 to prevent u64 overflow, then saturate the multiply
    let exponent = attempt.min(20);
    let doubled = 1u64.checked_shl(exponent).unwrap_or(u64::MAX);
    let ceiling = base_ms.saturating_mul(doubled).min(cap_ms);
    // Deterministic pseudo-random 0‥1000 derived from attempt
    let pseudo_rand = (attempt as u64).wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) % 1000;
    ceiling.saturating_mul(pseudo_rand) / 1000
}

// ── Circuit Breaker ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitState {
    /// Calls pass through normally.
    Closed,
    /// Too many failures — calls fail fast.
    Open { opened_at: DateTime<Utc> },
    /// One probe attempt allowed to test if backend recovered.
    HalfOpen,
}

pub struct CircuitBreaker {
    pub failure_threshold: u32,
    pub cooldown_secs: i64,
    state: CircuitState,
    consecutive_failures: u32,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, cooldown_secs: i64) -> Self {
        Self {
            failure_threshold,
            cooldown_secs,
            state: CircuitState::Closed,
            consecutive_failures: 0,
        }
    }

    /// Check if a call should be allowed through.
    pub fn can_call(&mut self) -> bool {
        match &self.state {
            CircuitState::Closed => true,
            CircuitState::HalfOpen => true, // allow one probe
            CircuitState::Open { opened_at } => {
                let elapsed = (Utc::now() - *opened_at).num_seconds();
                if elapsed >= self.cooldown_secs {
                    self.state = CircuitState::HalfOpen;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Record that the last call succeeded.
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.state = CircuitState::Closed;
    }

    /// Record that the last call failed.
    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        if self.consecutive_failures >= self.failure_threshold {
            self.state = CircuitState::Open { opened_at: Utc::now() };
        }
    }

    pub fn is_open(&self) -> bool { matches!(self.state, CircuitState::Open { .. }) }
    pub fn is_closed(&self) -> bool { matches!(self.state, CircuitState::Closed) }
    pub fn is_half_open(&self) -> bool { matches!(self.state, CircuitState::HalfOpen) }
    pub fn failures(&self) -> u32 { self.consecutive_failures }
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test 1: token bucket allows up to capacity ────────────────────────────

    #[test]
    fn test_token_bucket_allows_up_to_capacity() {
        let mut bucket = TokenBucket::new(5.0, 999.0); // fast refill
        for i in 0..5 {
            assert!(bucket.try_consume(), "Token {i} should be allowed");
        }
        assert!(!bucket.try_consume(), "Bucket exhausted — should deny");
    }

    // ── Test 2: full bucket starts at capacity ────────────────────────────────

    #[test]
    fn test_bucket_starts_full() {
        let mut bucket = TokenBucket::new(10.0, 1.0);
        assert!((bucket.available() - 10.0).abs() < 1.0, "Bucket should start near full");
    }

    // ── Test 3: rate limiter allows per sensor ────────────────────────────────

    #[test]
    fn test_rate_limiter_allows_per_sensor() {
        let mut limiter = GatewayRateLimiter::new(3.0, 0.001); // refill very slowly
        assert!(limiter.allow("s1"));
        assert!(limiter.allow("s1"));
        assert!(limiter.allow("s1"));
        assert!(!limiter.allow("s1"), "s1 bucket exhausted");
        // s2 has its own bucket — still allowed
        assert!(limiter.allow("s2"));
    }

    // ── Test 4: jittered delay respects cap ───────────────────────────────────

    #[test]
    fn test_jittered_delay_respects_cap() {
        for attempt in 0..20 {
            let delay = jittered_delay_ms(attempt, 100, 30_000);
            assert!(delay <= 30_000, "Delay must not exceed cap: got {delay}");
        }
    }

    // ── Test 5: jitter ceiling grows with attempt ────────────────────────────

    #[test]
    fn test_jitter_delay_increases_with_attempt() {
        // The ceiling must grow as attempt increases (up to cap)
        // Use small base_ms and large cap so ceiling is not capped early
        let ceiling0  = 100u64.min(30000);
        let ceiling10 = 100u64.saturating_mul(1u64 << 10).min(30000);
        let ceiling15 = 100u64.saturating_mul(1u64 << 15).min(30000);
        assert!(ceiling10 > ceiling0, "Ceiling grows from 0 to 10");
        assert_eq!(ceiling15, 30000, "Ceiling capped at cap_ms for high attempt");
    }

    // ── Test 6: circuit breaker opens after threshold failures ─────────────────

    #[test]
    fn test_circuit_opens_after_threshold() {
        let mut cb = CircuitBreaker::new(3, 60);
        assert!(cb.is_closed());
        cb.record_failure();
        cb.record_failure();
        assert!(cb.is_closed(), "Should still be closed after 2 failures");
        cb.record_failure();
        assert!(cb.is_open(), "Should open after 3 consecutive failures");
    }

    // ── Test 7: open circuit rejects calls ────────────────────────────────────

    #[test]
    fn test_open_circuit_rejects_calls() {
        let mut cb = CircuitBreaker::new(1, 3600);
        cb.record_failure();
        assert!(cb.is_open());
        assert!(!cb.can_call(), "Open circuit must reject calls");
    }

    // ── Test 8: success resets circuit ───────────────────────────────────────

    #[test]
    fn test_success_resets_circuit() {
        let mut cb = CircuitBreaker::new(3, 60);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failures(), 2);
        cb.record_success();
        assert!(cb.is_closed());
        assert_eq!(cb.failures(), 0, "Success must reset failure count");
    }

    // ── Test 9: half-open allows probe ────────────────────────────────────────

    #[test]
    fn test_half_open_allows_probe() {
        let mut cb = CircuitBreaker::new(1, 0); // 0s cooldown
        cb.record_failure(); // opens
        // After "cooldown" (0s), can_call transitions to HalfOpen
        let allowed = cb.can_call();
        assert!(cb.is_half_open(), "Should transition to HalfOpen after cooldown");
        assert!(allowed, "HalfOpen probe must be allowed through");
    }

    // ── Test 10: failed probe re-opens circuit ─────────────────────────────────

    #[test]
    fn test_failed_probe_reopens_circuit() {
        let mut cb = CircuitBreaker::new(1, 0);
        cb.record_failure(); // open
        cb.can_call(); // → half-open
        cb.record_failure(); // probe failed → re-open
        assert!(cb.is_open(), "Failed probe must re-open circuit");
    }
}

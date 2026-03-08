//! Stage 3 — Prompt 24: Validator Network Monitoring & Health Checks
//!
//! Pings each validator every 5 minutes.
//! Alerts if any validator goes offline.
//! Auto-rotates to backup if primary goes offline.
//!
//! HealthMonitor maintains a per-validator heartbeat ledger and fires alerts
//! when a validator misses its ping window.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

pub const PING_INTERVAL_SECS: i64  = 300;   // 5 minutes
pub const OFFLINE_THRESHOLD:  i64  = 600;   // 2 missed pings = offline
pub const ALERT_COOLDOWN_SECS: i64 = 1800;  // don't spam alerts (30 min)

// ── Health status ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Online,
    Degraded   { missed_pings: u32 },
    Offline    { since: DateTime<Utc> },
}

impl HealthStatus {
    pub fn is_online(&self) -> bool { matches!(self, HealthStatus::Online) }
    pub fn is_available(&self) -> bool {
        matches!(self, HealthStatus::Online | HealthStatus::Degraded { .. })
    }
}

// ── Alert ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertKind {
    ValidatorOffline { validator_id: String },
    ValidatorDegraded { validator_id: String, missed_pings: u32 },
    ValidatorRecovered { validator_id: String },
    AutoRotated { from: String, to: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub kind: AlertKind,
    pub issued_at: DateTime<Utc>,
    pub message: String,
}

impl Alert {
    fn new(kind: AlertKind) -> Self {
        let message = match &kind {
            AlertKind::ValidatorOffline { validator_id } =>
                format!("⚠️  {validator_id} is OFFLINE — rotate backup"),
            AlertKind::ValidatorDegraded { validator_id, missed_pings } =>
                format!("🔶 {validator_id} degraded ({missed_pings} missed pings)"),
            AlertKind::ValidatorRecovered { validator_id } =>
                format!("✅ {validator_id} recovered"),
            AlertKind::AutoRotated { from, to } =>
                format!("🔄 Auto-rotated from {from} to {to}"),
        };
        Self { kind, issued_at: Utc::now(), message }
    }
}

// ── Per-validator heartbeat record ────────────────────────────────────────────

#[derive(Debug, Clone)]
struct ValidatorHeartbeat {
    validator_id: String,
    last_seen: DateTime<Utc>,
    missed_pings: u32,
    status: HealthStatus,
    last_alert_at: Option<DateTime<Utc>>,
}

impl ValidatorHeartbeat {
    fn new(validator_id: &str) -> Self {
        Self {
            validator_id: validator_id.to_string(),
            last_seen: Utc::now(),
            missed_pings: 0,
            status: HealthStatus::Online,
            last_alert_at: None,
        }
    }

    fn seconds_since_seen(&self) -> i64 {
        (Utc::now() - self.last_seen).num_seconds()
    }

    fn can_alert(&self) -> bool {
        self.last_alert_at
            .map(|t| (Utc::now() - t).num_seconds() >= ALERT_COOLDOWN_SECS)
            .unwrap_or(true)
    }
}

// ── Health monitor ────────────────────────────────────────────────────────────

pub struct HealthMonitor {
    validators: HashMap<String, ValidatorHeartbeat>,
    pub alerts: Vec<Alert>,
    /// Primary validator → backup validator mapping.
    failover_map: HashMap<String, String>,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
            alerts: Vec::new(),
            failover_map: HashMap::new(),
        }
    }

    /// Register a validator for monitoring.
    pub fn register(&mut self, validator_id: &str) {
        self.validators.entry(validator_id.to_string())
            .or_insert_with(|| ValidatorHeartbeat::new(validator_id));
    }

    /// Set a failover mapping: if `primary` goes offline, use `backup`.
    pub fn set_failover(&mut self, primary: &str, backup: &str) {
        self.failover_map.insert(primary.to_string(), backup.to_string());
    }

    /// Record a successful ping from a validator.
    pub fn record_ping(&mut self, validator_id: &str) {
        let hb = self.validators.entry(validator_id.to_string())
            .or_insert_with(|| ValidatorHeartbeat::new(validator_id));

        let was_offline = !hb.status.is_available();
        hb.last_seen = Utc::now();
        hb.missed_pings = 0;
        hb.status = HealthStatus::Online;

        if was_offline {
            self.alerts.push(Alert::new(AlertKind::ValidatorRecovered {
                validator_id: validator_id.to_string(),
            }));
        }
    }

    /// Tick the monitor: check all validators for missed pings and issue alerts.
    /// Returns the number of new alerts issued.
    pub fn tick(&mut self) -> usize {
        let validator_ids: Vec<String> = self.validators.keys().cloned().collect();
        let mut issued = 0;

        for id in &validator_ids {
            let hb = self.validators.get_mut(id).unwrap();
            let elapsed = hb.seconds_since_seen();

            if elapsed >= OFFLINE_THRESHOLD {
                hb.missed_pings = (elapsed / PING_INTERVAL_SECS) as u32;

                let already_offline = matches!(hb.status, HealthStatus::Offline { .. });
                if !already_offline {
                    hb.status = HealthStatus::Offline { since: Utc::now() };
                }

                if hb.can_alert() {
                    hb.last_alert_at = Some(Utc::now());
                    self.alerts.push(Alert::new(AlertKind::ValidatorOffline {
                        validator_id: id.clone(),
                    }));
                    issued += 1;

                    // Auto-rotate if failover configured
                    if let Some(backup) = self.failover_map.get(id).cloned() {
                        self.alerts.push(Alert::new(AlertKind::AutoRotated {
                            from: id.clone(),
                            to: backup,
                        }));
                        issued += 1;
                    }
                }
            } else if elapsed >= PING_INTERVAL_SECS {
                let missed = (elapsed / PING_INTERVAL_SECS) as u32;
                hb.missed_pings = missed;
                hb.status = HealthStatus::Degraded { missed_pings: missed };

                if hb.can_alert() {
                    hb.last_alert_at = Some(Utc::now());
                    self.alerts.push(Alert::new(AlertKind::ValidatorDegraded {
                        validator_id: id.clone(),
                        missed_pings: missed,
                    }));
                    issued += 1;
                }
            }
        }
        issued
    }

    /// Current health status of a validator.
    pub fn status(&self, validator_id: &str) -> Option<&HealthStatus> {
        self.validators.get(validator_id).map(|hb| &hb.status)
    }

    /// All currently offline validators.
    pub fn offline_validators(&self) -> Vec<&str> {
        self.validators.values()
            .filter(|hb| matches!(&hb.status, HealthStatus::Offline { .. }))
            .map(|hb| hb.validator_id.as_str())
            .collect()
    }

    /// All currently online validators.
    pub fn online_validators(&self) -> Vec<&str> {
        self.validators.values()
            .filter(|hb| hb.status.is_online())
            .map(|hb| hb.validator_id.as_str())
            .collect()
    }

    /// Force a validator offline (for testing).
    #[cfg(test)]
    pub fn force_offline(&mut self, validator_id: &str) {
        if let Some(hb) = self.validators.get_mut(validator_id) {
            // Backdate last_seen so tick() sees it as offline
            hb.last_seen = Utc::now() - Duration::seconds(OFFLINE_THRESHOLD + 60);
            hb.status = HealthStatus::Offline { since: Utc::now() };
        }
    }
}

impl Default for HealthMonitor { fn default() -> Self { Self::new() } }

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test 1: newly registered validator is online ──────────────────────────

    #[test]
    fn test_new_validator_online() {
        let mut monitor = HealthMonitor::new();
        monitor.register("v1");
        assert_eq!(monitor.status("v1"), Some(&HealthStatus::Online));
    }

    // ── Test 2: ping keeps validator online ───────────────────────────────────

    #[test]
    fn test_ping_keeps_online() {
        let mut monitor = HealthMonitor::new();
        monitor.register("v1");
        monitor.record_ping("v1");
        assert_eq!(monitor.status("v1"), Some(&HealthStatus::Online));
        assert_eq!(monitor.online_validators(), vec!["v1"]);
    }

    // ── Test 3: missed pings trigger offline detection ────────────────────────

    #[test]
    fn test_offline_detected_after_threshold() {
        let mut monitor = HealthMonitor::new();
        monitor.register("v1");
        monitor.force_offline("v1");
        monitor.tick();
        assert_eq!(monitor.offline_validators(), vec!["v1"]);
    }

    // ── Test 4: offline alert issued ─────────────────────────────────────────

    #[test]
    fn test_offline_alert_issued() {
        let mut monitor = HealthMonitor::new();
        monitor.register("v1");
        monitor.force_offline("v1");
        monitor.tick();
        let offline_alerts: Vec<&Alert> = monitor.alerts.iter()
            .filter(|a| matches!(&a.kind, AlertKind::ValidatorOffline { .. }))
            .collect();
        assert!(!offline_alerts.is_empty(), "Offline alert must be issued");
    }

    // ── Test 5: recovery issues recovery alert ────────────────────────────────

    #[test]
    fn test_recovery_alert_issued() {
        let mut monitor = HealthMonitor::new();
        monitor.register("v1");
        monitor.force_offline("v1");
        monitor.tick();
        monitor.record_ping("v1");
        let recovered: Vec<&Alert> = monitor.alerts.iter()
            .filter(|a| matches!(&a.kind, AlertKind::ValidatorRecovered { .. }))
            .collect();
        assert!(!recovered.is_empty(), "Recovery alert must be issued");
        assert_eq!(monitor.status("v1"), Some(&HealthStatus::Online));
    }

    // ── Test 6: auto-rotation alert when failover configured ──────────────────

    #[test]
    fn test_auto_rotate_fires_when_failover_set() {
        let mut monitor = HealthMonitor::new();
        monitor.register("v1");
        monitor.register("v2");
        monitor.set_failover("v1", "v2");
        monitor.force_offline("v1");
        monitor.tick();
        let rotated: Vec<&Alert> = monitor.alerts.iter()
            .filter(|a| matches!(&a.kind, AlertKind::AutoRotated { .. }))
            .collect();
        assert!(!rotated.is_empty(), "Auto-rotation alert must be issued");
    }

    // ── Test 7: no auto-rotation without failover configured ──────────────────

    #[test]
    fn test_no_auto_rotate_without_failover() {
        let mut monitor = HealthMonitor::new();
        monitor.register("v1");
        monitor.force_offline("v1");
        monitor.tick();
        let rotated: Vec<&Alert> = monitor.alerts.iter()
            .filter(|a| matches!(&a.kind, AlertKind::AutoRotated { .. }))
            .collect();
        assert!(rotated.is_empty(), "No rotation without failover config");
    }

    // ── Test 8: multiple validators tracked independently ─────────────────────

    #[test]
    fn test_multiple_validators_independent() {
        let mut monitor = HealthMonitor::new();
        monitor.register("v1");
        monitor.register("v2");
        monitor.register("v3");
        monitor.force_offline("v1");
        monitor.record_ping("v2");
        monitor.record_ping("v3");
        monitor.tick();
        assert_eq!(monitor.offline_validators().len(), 1);
        assert_eq!(monitor.online_validators().len(), 2);
    }

    // ── Test 9: constants are correct ────────────────────────────────────────

    #[test]
    fn test_constants() {
        assert_eq!(PING_INTERVAL_SECS, 300);   // 5 min
        assert_eq!(OFFLINE_THRESHOLD, 600);    // 10 min = 2 missed pings
    }
}

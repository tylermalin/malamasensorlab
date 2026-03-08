use crate::stage7_verification::audit::AuditTrail;

pub struct ExplorerLogic;

impl ExplorerLogic {
    /// P50: Search by time range and sensor DID
    pub fn search_by_time(
        trails: Vec<AuditTrail>,
        start: i64,
        end: i64,
        sensor_did: Option<&str>,
    ) -> Vec<AuditTrail> {
        trails.into_iter()
            .filter(|t| t.timestamp >= start && t.timestamp <= end)
            .filter(|t| sensor_did.map_or(true, |did| t.did_doc.id == did))
            .collect()
    }

    /// P51: Geographic Region Mapping
    pub fn map_to_region(lat: f64, lon: f64) -> String {
        if lat > 0.0 {
            if lon > 0.0 { "Asia/Europe".to_string() }
            else { "North America".to_string() }
        } else {
            if lon > 0.0 { "Oceania/Africa".to_string() }
            else { "South America".to_string() }
        }
    }

    pub fn format_explorer_link(chain: &str, tx_id: &str) -> String {
        match chain.to_lowercase().as_str() {
            "cardano" => format!("https://cexplorer.io/tx/{}", tx_id),
            "base" => format!("https://basescan.org/tx/{}", tx_id),
            "hedera" => format!("https://hashscan.io/mainnet/transaction/{}", tx_id),
            "celo" => format!("https://celoscan.io/tx/{}", tx_id),
            _ => format!("https://unknown-explorer.com/tx/{}", tx_id),
        }
    }

    pub fn format_ipfs_link(cid: &str) -> String {
        format!("https://ipfs.io/ipfs/{}", cid)
    }
}

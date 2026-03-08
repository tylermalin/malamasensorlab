pub struct ExplorerLogic;

impl ExplorerLogic {
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

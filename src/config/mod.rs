use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "config.json")]
    pub config: PathBuf,

    #[arg(long)]
    pub redeem: bool,

    #[arg(long, requires = "redeem")]
    pub condition_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub polymarket: PolymarketConfig,
    pub strategy: StrategyConfig,
}

/// 15m vs 5m arbitrage: trade overlap window; per-symbol price-to-beat tolerance (USD).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// Symbols to arb (15m vs 5m overlap). e.g. ["btc", "eth", "sol", "xrp"].
    #[serde(default = "default_symbols")]
    pub symbols: Vec<String>,
    /// Max sum of (15m one side ask + 5m opposite side ask) to trigger arb (e.g. 0.99).
    #[serde(default = "default_sum_threshold")]
    pub sum_threshold: f64,
    /// Seconds to wait after placing an arb before placing the next one (cooldown).
    #[serde(default = "default_trade_interval_secs")]
    pub trade_interval_secs: u64,
    #[serde(default)]
    pub simulation_mode: bool,
    /// Size in shares per leg (15m and 5m).
    #[serde(default = "default_arb_shares")]
    pub arb_shares: String,
    /// Per-symbol max |15m price-to-beat − 5m price-to-beat| (USD) to allow arb.
    #[serde(default, alias = "price_to_beat_tolerance_usd")]
    pub btc_price_to_beat_tolerance_usd: f64,
    #[serde(default = "default_eth_tolerance")]
    pub eth_price_to_beat_tolerance_usd: f64,
    #[serde(default = "default_sol_tolerance")]
    pub sol_price_to_beat_tolerance_usd: f64,
    #[serde(default = "default_xrp_tolerance")]
    pub xrp_price_to_beat_tolerance_usd: f64,
    /// Seconds between polls when checking if markets are closed/resolved (e.g. 30).
    #[serde(default = "default_resolution_poll_interval_secs")]
    pub resolution_poll_interval_secs: u64,
    /// Max seconds to wait for resolution before giving up (e.g. 600 = 10 min).
    #[serde(default = "default_resolution_max_wait_secs")]
    pub resolution_max_wait_secs: u64,
    /// Automatically redeem winning tokens after resolution.
    #[serde(default = "default_auto_redeem")]
    pub auto_redeem: bool,
}

fn default_symbols() -> Vec<String> {
    vec!["btc".into(), "eth".into(), "sol".into(), "xrp".into()]
}
fn default_sum_threshold() -> f64 {
    0.99
}
fn default_trade_interval_secs() -> u64 {
    60
}
fn default_arb_shares() -> String {
    "10".to_string()
}
fn default_eth_tolerance() -> f64 {
    1.0
}
fn default_sol_tolerance() -> f64 {
    0.05
}
fn default_xrp_tolerance() -> f64 {
    0.0003
}
fn default_resolution_poll_interval_secs() -> u64 {
    30
}
fn default_resolution_max_wait_secs() -> u64 {
    600
}
fn default_auto_redeem() -> bool {
    true
}

impl StrategyConfig {
    /// Price-to-beat tolerance (USD) for the given symbol.
    pub fn price_to_beat_tolerance_for(&self, symbol: &str) -> f64 {
        match symbol.to_lowercase().as_str() {
            "btc" => self.btc_price_to_beat_tolerance_usd,
            "eth" => self.eth_price_to_beat_tolerance_usd,
            "sol" => self.sol_price_to_beat_tolerance_usd,
            "xrp" => self.xrp_price_to_beat_tolerance_usd,
            _ => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketConfig {
    pub gamma_api_url: String,
    pub clob_api_url: String,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub api_passphrase: Option<String>,
    pub private_key: Option<String>,
    pub proxy_wallet_address: Option<String>,
    pub signature_type: Option<u8>,
    /// Polygon RPC URL for redemption (Safe reads + sendTransaction). Defaults to polygon-rpc.com if unset.
    #[serde(default)]
    pub rpc_url: Option<String>,
    /// WebSocket base URL for market channel (e.g. wss://ws-subscriptions-clob.polymarket.com).
    #[serde(default = "default_ws_url")]
    pub ws_url: String,
    /// RTDS WebSocket URL for Chainlink BTC price (price-to-beat). Topic: crypto_prices_chainlink, symbol: btc/usd.
    #[serde(default = "default_rtds_ws_url")]
    pub rtds_ws_url: String,
}

fn default_ws_url() -> String {
    "wss://ws-subscriptions-clob.polymarket.com".to_string()
}

fn default_rtds_ws_url() -> String {
    "wss://ws-live-data.polymarket.com".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            polymarket: PolymarketConfig {
                gamma_api_url: "https://gamma-api.polymarket.com".to_string(),
                clob_api_url: "https://clob.polymarket.com".to_string(),
                api_key: None,
                api_secret: None,
                api_passphrase: None,
                private_key: None,
                proxy_wallet_address: None,
                signature_type: None,
                rpc_url: None,
                ws_url: default_ws_url(),
                rtds_ws_url: default_rtds_ws_url(),
            },
            strategy: StrategyConfig {
                symbols: default_symbols(),
                sum_threshold: 0.99,
                trade_interval_secs: default_trade_interval_secs(),
                simulation_mode: false,
                arb_shares: default_arb_shares(),
                btc_price_to_beat_tolerance_usd: 10.0,
                eth_price_to_beat_tolerance_usd: default_eth_tolerance(),
                sol_price_to_beat_tolerance_usd: default_sol_tolerance(),
                xrp_price_to_beat_tolerance_usd: default_xrp_tolerance(),
                resolution_poll_interval_secs: default_resolution_poll_interval_secs(),
                resolution_max_wait_secs: default_resolution_max_wait_secs(),
                auto_redeem: default_auto_redeem(),
            },
        }
    }
}

impl Config {
    pub fn load(path: &PathBuf) -> anyhow::Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            let config = Config::default();
            let content = serde_json::to_string_pretty(&config)?;
            std::fs::write(path, content)?;
            Ok(config)
        }
    }
}

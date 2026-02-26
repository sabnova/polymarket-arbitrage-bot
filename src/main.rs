mod adapters;
mod domain;
mod services;
mod utils;
mod models;
mod config;
mod api;

use anyhow::Result;
use clap::Parser;
use config::{Args, Config};
use std::io::Write;
use std::sync::Arc;
use adapters::polymarket::PolymarketApi;
use services::arbitrage_orchestrator::ArbStrategy;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .init();

    let args = Args::parse();
    let config = Config::load(&args.config)?;

    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    eprintln!("📋 15m vs 5m arbitrage (BTC, ETH, SOL, XRP) — overlap, per-symbol tolerance");
    eprintln!("   All symbols via parallel WebSocket; arb when sum < threshold & |P15−P5| ≤ tolerance");
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let api = Arc::new(PolymarketApi::new(
        config.polymarket.gamma_api_url.clone(),
        config.polymarket.clob_api_url.clone(),
        config.polymarket.api_key.clone(),
        config.polymarket.api_secret.clone(),
        config.polymarket.api_passphrase.clone(),
        config.polymarket.private_key.clone(),
        config.polymarket.proxy_wallet_address.clone(),
        config.polymarket.signature_type,
        config.polymarket.rpc_url.clone(),
    ));

    if args.redeem {
        run_redeem_only(api.as_ref(), &config, args.condition_id.as_deref()).await?;
        return Ok(());
    }

    if config.polymarket.private_key.is_some() {
        if let Err(e) = api.authenticate().await {
            log::error!("Authentication failed: {}", e);
            anyhow::bail!("Authentication failed. Please check your credentials.");
        }
    } else {
        log::warn!("⚠️ No private key provided. Bot can only monitor (no orders).");
    }

    let strategy = ArbStrategy::new(api, config);
    strategy.run().await
}

async fn run_redeem_only(
    api: &PolymarketApi,
    config: &Config,
    condition_id: Option<&str>,
) -> Result<()> {
    let proxy = config
        .polymarket
        .proxy_wallet_address
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("--redeem requires proxy_wallet_address in config.json"))?;

    eprintln!("Redeem-only mode (proxy: {})", proxy);
    let cids: Vec<String> = if let Some(cid) = condition_id {
        let cid = if cid.starts_with("0x") {
            cid.to_string()
        } else {
            format!("0x{}", cid)
        };
        eprintln!("Redeeming condition: {}", cid);
        vec![cid]
    } else {
        eprintln!("Fetching redeemable positions...");
        let list = api.get_redeemable_positions(proxy).await?;
        if list.is_empty() {
            eprintln!("No redeemable positions found.");
            return Ok(());
        }
        eprintln!("Found {} condition(s) to redeem.", list.len());
        list
    };

    let mut ok_count = 0u32;
    let mut fail_count = 0u32;
    for cid in &cids {
        eprintln!("\n--- Redeeming condition {} ---", &cid[..cid.len().min(18)]);
        match api.redeem_tokens(cid, "", "Up").await {
            Ok(_) => {
                eprintln!("Success: {}", cid);
                ok_count += 1;
            }
            Err(e) => {
                eprintln!("Failed to redeem {}: {} (skipping)", cid, e);
                fail_count += 1;
            }
        }
    }
    eprintln!(
        "\nRedeem complete. Succeeded: {}, Failed: {}",
        ok_count, fail_count
    );
    Ok(())
}

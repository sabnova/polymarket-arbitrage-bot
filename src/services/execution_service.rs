use crate::adapters::polymarket::ws_market::{run_market_ws, PricesSnapshot};
use crate::adapters::polymarket::PolymarketApi;
use crate::config::Config;
use crate::domain::arbitrage::select_arb_legs;
use crate::models::{OrderRequest, TradeRecord};
use anyhow::Result;
use chrono::Utc;
use log::{info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

const MARKET_15M_DURATION_SECS: i64 = 15 * 60;
const LIVE_PRICE_POLL_MS: u64 = 10;

#[allow(clippy::too_many_arguments)]
pub async fn run_overlap_round(
    api: Arc<PolymarketApi>,
    config: &Config,
    symbol: &str,
    cid_15: &str,
    cid_5: &str,
    t15_up: &str,
    t15_down: &str,
    t5_up: &str,
    t5_down: &str,
    period_15: i64,
    period_5: i64,
) -> Result<Vec<TradeRecord>> {
    let prices: PricesSnapshot = Arc::new(RwLock::new(HashMap::new()));
    let asset_ids = vec![
        t15_up.to_string(),
        t15_down.to_string(),
        t5_up.to_string(),
        t5_down.to_string(),
    ];
    let ws_url = config.polymarket.ws_url.clone();
    let prices_clone = Arc::clone(&prices);
    let symbol_ws = symbol.to_string();
    let ws_handle = tokio::spawn(async move {
        if let Err(e) = run_market_ws(&ws_url, asset_ids, prices_clone).await {
            warn!("{} overlap WebSocket exited: {}", symbol_ws.to_uppercase(), e);
        }
    });

    let threshold = config.strategy.sum_threshold;
    let shares = config.strategy.arb_shares.clone();
    let interval_secs = config.strategy.trade_interval_secs;
    let simulation = config.strategy.simulation_mode;
    let sym_upper = symbol.to_uppercase();

    let mut last_trade_at: Option<std::time::Instant> = None;
    let mut trades: Vec<TradeRecord> = Vec::new();

    while Utc::now().timestamp() < period_15 + MARKET_15M_DURATION_SECS {
        let snap = prices.read().await;
        let ask_15_up = snap.get(t15_up).and_then(|p| p.ask);
        let ask_15_down = snap.get(t15_down).and_then(|p| p.ask);
        let ask_5_up = snap.get(t5_up).and_then(|p| p.ask);
        let ask_5_down = snap.get(t5_down).and_then(|p| p.ask);
        drop(snap);

        if let Some(t) = last_trade_at {
            if t.elapsed().as_secs() < interval_secs {
                sleep(Duration::from_millis(LIVE_PRICE_POLL_MS)).await;
                continue;
            }
        }

        let Some(selection) = select_arb_legs(
            ask_15_up,
            ask_15_down,
            ask_5_up,
            ask_5_down,
            threshold,
            t15_up,
            t15_down,
            t5_up,
            t5_down,
        ) else {
            sleep(Duration::from_millis(LIVE_PRICE_POLL_MS)).await;
            continue;
        };

        if simulation {
            info!(
                "[SIM] {} arb would place: 15m {} @ {:.4} + 5m {} @ {:.4} (sum {:.4} < {})",
                sym_upper,
                selection.leg1_outcome,
                selection.leg1_price,
                selection.leg2_outcome,
                selection.leg2_price,
                selection.leg1_price + selection.leg2_price,
                threshold
            );
            last_trade_at = Some(std::time::Instant::now());
            let size_f64: f64 = shares.parse().unwrap_or(0.0);
            trades.push(TradeRecord {
                symbol: symbol.to_string(),
                period_15,
                period_5,
                cid_15: cid_15.to_string(),
                cid_5: cid_5.to_string(),
                leg1_token: selection.leg1_token.to_string(),
                leg1_price: selection.leg1_price,
                leg1_cid: cid_15.to_string(),
                leg1_outcome: selection.leg1_outcome.to_string(),
                leg2_token: selection.leg2_token.to_string(),
                leg2_price: selection.leg2_price,
                leg2_cid: cid_5.to_string(),
                leg2_outcome: selection.leg2_outcome.to_string(),
                size: size_f64,
            });
            sleep(Duration::from_millis(LIVE_PRICE_POLL_MS)).await;
            continue;
        }

        let order1 = OrderRequest {
            token_id: selection.leg1_token.to_string(),
            side: "BUY".to_string(),
            size: shares.clone(),
            price: format!("{:.4}", selection.leg1_price),
            order_type: "GTC".to_string(),
        };
        let order2 = OrderRequest {
            token_id: selection.leg2_token.to_string(),
            side: "BUY".to_string(),
            size: shares.clone(),
            price: format!("{:.4}", selection.leg2_price),
            order_type: "GTC".to_string(),
        };

        let r1 = api.place_order(&order1).await;
        let r2 = api.place_order(&order2).await;

        match (&r1, &r2) {
            (Ok(res1), Ok(res2)) => {
                let id1 = res1.order_id.as_deref().unwrap_or("");
                let id2 = res2.order_id.as_deref().unwrap_or("");
                info!(
                    "{} arb placed: 15m {} @ {:.4} ({}), 5m {} @ {:.4} ({}), next in {}s",
                    sym_upper,
                    selection.leg1_outcome,
                    selection.leg1_price,
                    id1,
                    selection.leg2_outcome,
                    selection.leg2_price,
                    id2,
                    interval_secs
                );
                last_trade_at = Some(std::time::Instant::now());
                let size_f64: f64 = shares.parse().unwrap_or(0.0);
                trades.push(TradeRecord {
                    symbol: symbol.to_string(),
                    period_15,
                    period_5,
                    cid_15: cid_15.to_string(),
                    cid_5: cid_5.to_string(),
                    leg1_token: selection.leg1_token.to_string(),
                    leg1_price: selection.leg1_price,
                    leg1_cid: cid_15.to_string(),
                    leg1_outcome: selection.leg1_outcome.to_string(),
                    leg2_token: selection.leg2_token.to_string(),
                    leg2_price: selection.leg2_price,
                    leg2_cid: cid_5.to_string(),
                    leg2_outcome: selection.leg2_outcome.to_string(),
                    size: size_f64,
                });
            }
            (Err(e), _) => {
                warn!("{} arb leg1 place failed: {}", sym_upper, e);
            }
            (_, Err(e)) => {
                warn!("{} arb leg2 place failed: {}", sym_upper, e);
            }
        }

        sleep(Duration::from_millis(LIVE_PRICE_POLL_MS)).await;
    }

    ws_handle.abort();
    info!(
        "{} overlap window ended (period {}), {} trade(s) placed.",
        sym_upper,
        period_15,
        trades.len()
    );
    Ok(trades)
}

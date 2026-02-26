use crate::adapters::polymarket::PolymarketApi;
use crate::config::Config;
use crate::domain::pnl::compute_trade_pnl;
use crate::models::TradeRecord;
use anyhow::Result;
use log::{info, warn};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

const RESOLUTION_INITIAL_DELAY_SECS: u64 = 60;

pub async fn resolve_and_compute_pnl(
    api: Arc<PolymarketApi>,
    config: &Config,
    trades: &[TradeRecord],
    cumulative_pnl: Arc<RwLock<f64>>,
) -> Result<(Vec<(String, String)>, f64)> {
    if trades.is_empty() {
        return Ok((Vec::new(), 0.0));
    }

    let poll_interval = config.strategy.resolution_poll_interval_secs;
    let max_wait = config.strategy.resolution_max_wait_secs;
    let first = trades.first().expect("non-empty trades");
    let cid_15 = &first.cid_15;
    let cid_5 = &first.cid_5;
    info!(
        "Resolution: waiting {}s, then polling every {}s (max {}s) for {} trade(s).",
        RESOLUTION_INITIAL_DELAY_SECS,
        poll_interval,
        max_wait,
        trades.len()
    );
    sleep(Duration::from_secs(RESOLUTION_INITIAL_DELAY_SECS)).await;

    let started = std::time::Instant::now();
    let mut m15_resolved = None;
    let mut m5_resolved = None;
    while started.elapsed().as_secs() < max_wait {
        let m15 = api.get_market(cid_15).await.ok();
        let m5 = api.get_market(cid_5).await.ok();
        let (closed_15, winner_15) = m15
            .as_ref()
            .map(|m| {
                (
                    m.closed,
                    m.tokens
                        .iter()
                        .find(|t| t.winner)
                        .map(|t| (t.token_id.as_str(), t.outcome.as_str())),
                )
            })
            .unwrap_or((false, None));
        let (closed_5, winner_5) = m5
            .as_ref()
            .map(|m| {
                (
                    m.closed,
                    m.tokens
                        .iter()
                        .find(|t| t.winner)
                        .map(|t| (t.token_id.as_str(), t.outcome.as_str())),
                )
            })
            .unwrap_or((false, None));

        if closed_15 && closed_5 && winner_15.is_some() && winner_5.is_some() {
            m15_resolved = m15;
            m5_resolved = m5;
            break;
        }
        sleep(Duration::from_secs(poll_interval)).await;
    }

    let (winner_15, winner_5) = match (m15_resolved.as_ref(), m5_resolved.as_ref()) {
        (Some(m15), Some(m5)) => (
            m15.tokens
                .iter()
                .find(|t| t.winner)
                .map(|t| (t.token_id.as_str(), t.outcome.as_str())),
            m5.tokens
                .iter()
                .find(|t| t.winner)
                .map(|t| (t.token_id.as_str(), t.outcome.as_str())),
        ),
        _ => {
            warn!(
                "Resolution timeout for {} trades (cid_15={}, cid_5={}).",
                trades.len(),
                cid_15,
                cid_5
            );
            return Ok((Vec::new(), 0.0));
        }
    };

    let (win_token_15, win_token_5, outcome_15, outcome_5) = match (winner_15, winner_5) {
        (Some((t15, o15)), Some((t5, o5))) => (t15, t5, o15, o5),
        _ => return Ok((Vec::new(), 0.0)),
    };

    let mut period_pnl = 0.0f64;
    let mut redeem_targets: Vec<(String, String)> = Vec::new();

    for trade in trades {
        let sym = trade.symbol.to_uppercase();
        let pnl_result = compute_trade_pnl(trade, win_token_15, win_token_5);
        period_pnl += pnl_result.pnl;

        let result_msg = match (pnl_result.won_15m, pnl_result.won_5m) {
            (true, true) => "Won both legs",
            (true, false) => "Won 15m leg",
            (false, true) => "Won 5m leg",
            (false, false) => "Lost both legs",
        };
        info!(
            "{} resolved: Won 15m {} 5m {} | {} | cost={:.2}, payout={:.2}, PnL={:.2} | period PnL={:.2}",
            sym,
            outcome_15,
            outcome_5,
            result_msg,
            pnl_result.cost,
            pnl_result.payout,
            pnl_result.pnl,
            period_pnl
        );

        if pnl_result.won_15m {
            let out = if win_token_15 == trade.leg1_token {
                trade.leg1_outcome.clone()
            } else {
                trade.leg2_outcome.clone()
            };
            redeem_targets.push((trade.cid_15.clone(), out));
        }
        if pnl_result.won_5m {
            let out = if win_token_5 == trade.leg1_token {
                trade.leg1_outcome.clone()
            } else {
                trade.leg2_outcome.clone()
            };
            redeem_targets.push((trade.cid_5.clone(), out));
        }
    }

    if period_pnl != 0.0 {
        let mut cum = cumulative_pnl.write().await;
        *cum += period_pnl;
        info!("Period PnL: {:.2} | Cumulative PnL: {:.2}", period_pnl, *cum);
    }

    Ok((redeem_targets, period_pnl))
}

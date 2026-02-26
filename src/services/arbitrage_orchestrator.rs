use crate::adapters::polymarket::ws_rtds::{run_chainlink_multi_poller, PriceCacheMulti};
use crate::adapters::polymarket::PolymarketApi;
use crate::config::Config;
use crate::domain::window::{current_15m_period_start, current_5m_period_start, is_last_5min_of_15m};
use crate::models::TradeRecord;
use crate::services::discovery_service::MarketDiscovery;
use crate::services::execution_service::run_overlap_round;
use crate::services::redemption_service::auto_redeem_winners;
use crate::services::resolution_service::resolve_and_compute_pnl;
use anyhow::Result;
use chrono::Utc;
use log::{error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

const OVERLAP_POLL_SECS: u64 = 5;
const WAIT_FOR_PRICE_POLL_SECS: u64 = 10;

pub struct ArbStrategy {
    api: Arc<PolymarketApi>,
    config: Config,
    discovery: MarketDiscovery,
    price_cache_15: PriceCacheMulti,
    price_cache_5: PriceCacheMulti,
}

impl ArbStrategy {
    pub fn new(api: Arc<PolymarketApi>, config: Config) -> Self {
        Self {
            discovery: MarketDiscovery::new(api.clone()),
            api,
            config,
            price_cache_15: Arc::new(RwLock::new(HashMap::new())),
            price_cache_5: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn wait_for_overlap_and_prices(
        &self,
        symbol: &str,
    ) -> Result<(
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
        i64,
        f64,
        f64,
    )> {
        loop {
            let now = Utc::now().timestamp();
            let period_15 = current_15m_period_start();
            let period_5 = current_5m_period_start();

            if !is_last_5min_of_15m(now, period_15) {
                sleep(Duration::from_secs(OVERLAP_POLL_SECS)).await;
                continue;
            }

            let (cid_15, cid_5) = {
                let m15 = self.discovery.get_15m_market(symbol, period_15);
                let m5 = self.discovery.get_5m_market(symbol, period_5);
                let (r15, r5) = tokio::try_join!(m15, m5)?;
                let cid_15 = match r15 {
                    Some((cid, _)) => cid,
                    None => {
                        warn!(
                            "15m {} market not found for period {}. Retrying.",
                            symbol, period_15
                        );
                        sleep(Duration::from_secs(OVERLAP_POLL_SECS)).await;
                        continue;
                    }
                };
                let cid_5 = match r5 {
                    Some((cid, _)) => cid,
                    None => {
                        warn!(
                            "5m {} market not found for period {}. Retrying.",
                            symbol, period_5
                        );
                        sleep(Duration::from_secs(OVERLAP_POLL_SECS)).await;
                        continue;
                    }
                };
                (cid_15, cid_5)
            };

            let (price_15, price_5) = {
                let c15 = self.price_cache_15.read().await;
                let c5 = self.price_cache_5.read().await;
                let p15 = c15.get(symbol).and_then(|m| m.get(&period_15).copied());
                let p5 = c5.get(symbol).and_then(|m| m.get(&period_5).copied());
                (p15, p5)
            };

            let (price_15, price_5) = match (price_15, price_5) {
                (Some(a), Some(b)) => (a, b),
                _ => {
                    info!(
                        "{}: waiting for price-to-beat 15m={:?}, 5m={:?}",
                        symbol.to_uppercase(),
                        price_15,
                        price_5
                    );
                    sleep(Duration::from_secs(WAIT_FOR_PRICE_POLL_SECS)).await;
                    continue;
                }
            };

            let tolerance = self.config.strategy.price_to_beat_tolerance_for(symbol);
            if (price_15 - price_5).abs() > tolerance {
                info!(
                    "{}: |15m - 5m| price-to-beat = {:.6} > tolerance {:.6} USD; skipping.",
                    symbol.to_uppercase(),
                    (price_15 - price_5).abs(),
                    tolerance
                );
                sleep(Duration::from_secs(OVERLAP_POLL_SECS)).await;
                continue;
            }

            let (t15_up, t15_down, t5_up, t5_down) = {
                let tok15 = self.discovery.get_market_tokens(&cid_15);
                let tok5 = self.discovery.get_market_tokens(&cid_5);
                let ((u15, d15), (u5, d5)) = tokio::try_join!(tok15, tok5)?;
                (u15, d15, u5, d5)
            };

            info!(
                "{} overlap active: 15m period {} (P2B {:.4}), 5m period {} (P2B {:.4}), tolerance {:.6}",
                symbol.to_uppercase(),
                period_15,
                price_15,
                period_5,
                price_5,
                tolerance
            );
            return Ok((
                cid_15, cid_5, t15_up, t15_down, t5_up, t5_down, period_15, period_5, price_15,
                price_5,
            ));
        }
    }

    async fn run_symbol_loop(
        api: Arc<PolymarketApi>,
        config: Config,
        price_cache_15: PriceCacheMulti,
        price_cache_5: PriceCacheMulti,
        cumulative_pnl: Arc<RwLock<f64>>,
        symbol: String,
    ) -> Result<()> {
        let discovery = MarketDiscovery::new(api.clone());
        let strategy = Self {
            api: api.clone(),
            config: config.clone(),
            discovery,
            price_cache_15,
            price_cache_5,
        };
        loop {
            let (cid_15, cid_5, t15_up, t15_down, t5_up, t5_down, period_15, period_5, _p15, _p5) =
                strategy.wait_for_overlap_and_prices(&symbol).await?;

            match run_overlap_round(
                strategy.api.clone(),
                &strategy.config,
                &symbol,
                &cid_15,
                &cid_5,
                &t15_up,
                &t15_down,
                &t5_up,
                &t5_down,
                period_15,
                period_5,
            )
            .await
            {
                Ok(trades) => {
                    if !trades.is_empty() {
                        strategy
                            .resolve_redeem_and_track(trades, cumulative_pnl.clone())
                            .await?;
                    }
                }
                Err(e) => {
                    error!("{} overlap round error: {}", symbol.to_uppercase(), e);
                }
            }
            sleep(Duration::from_secs(5)).await;
        }
    }

    async fn resolve_redeem_and_track(
        &self,
        trades: Vec<TradeRecord>,
        cumulative_pnl: Arc<RwLock<f64>>,
    ) -> Result<()> {
        let (redeem_targets, _) = resolve_and_compute_pnl(
            self.api.clone(),
            &self.config,
            &trades,
            cumulative_pnl,
        )
        .await?;
        auto_redeem_winners(self.api.clone(), &self.config, &redeem_targets).await?;
        Ok(())
    }

    pub async fn run(&self) -> Result<()> {
        let symbols = &self.config.strategy.symbols;
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!(
            "15m vs 5m arbitrage (symbols: {:?}) — overlap window, parallel WS",
            symbols
        );
        info!(
            "   Price-to-beat: RTDS Chainlink (all symbols in one WS); per-symbol tolerance"
        );
        info!(
            "   Place both legs when sum of asks < {}; next arb after {}s cooldown.",
            self.config.strategy.sum_threshold, self.config.strategy.trade_interval_secs
        );
        info!(
            "   Post-arb: poll resolution every {}s, auto_redeem={}",
            self.config.strategy.resolution_poll_interval_secs, self.config.strategy.auto_redeem
        );
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        let cumulative_pnl: Arc<RwLock<f64>> = Arc::new(RwLock::new(0.0));
        let rtds_url = self.config.polymarket.rtds_ws_url.clone();
        let cache_15 = Arc::clone(&self.price_cache_15);
        let cache_5 = Arc::clone(&self.price_cache_5);
        let symbols_rtds = symbols.clone();
        if let Err(e) = run_chainlink_multi_poller(rtds_url, symbols_rtds, cache_15, cache_5).await {
            warn!("RTDS Chainlink poller start: {}", e);
        }
        sleep(Duration::from_secs(2)).await;

        let mut handles = Vec::new();
        for symbol in symbols.clone() {
            let api = Arc::clone(&self.api);
            let config = self.config.clone();
            let price_cache_15 = Arc::clone(&self.price_cache_15);
            let price_cache_5 = Arc::clone(&self.price_cache_5);
            let cumulative_pnl = Arc::clone(&cumulative_pnl);
            handles.push(tokio::spawn(async move {
                if let Err(e) = Self::run_symbol_loop(
                    api,
                    config,
                    price_cache_15,
                    price_cache_5,
                    cumulative_pnl,
                    symbol.clone(),
                )
                .await
                {
                    error!("Symbol loop {} failed: {}", symbol, e);
                }
            }));
        }
        futures_util::future::try_join_all(handles).await?;
        Ok(())
    }
}

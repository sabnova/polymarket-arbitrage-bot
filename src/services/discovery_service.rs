use crate::adapters::polymarket::PolymarketApi;
use crate::utils::slug_builder::{build_15m_slug, build_5m_slug, parse_price_to_beat_from_question};
use anyhow::Result;
use std::sync::Arc;

pub struct MarketDiscovery {
    api: Arc<PolymarketApi>,
}

impl MarketDiscovery {
    pub fn new(api: Arc<PolymarketApi>) -> Self {
        Self { api }
    }

    pub async fn get_market_tokens(&self, condition_id: &str) -> Result<(String, String)> {
        let details = self.api.get_market(condition_id).await?;
        let mut up_token = None;
        let mut down_token = None;

        for token in details.tokens {
            let outcome = token.outcome.to_uppercase();
            if outcome.contains("UP") || outcome == "1" {
                up_token = Some(token.token_id);
            } else if outcome.contains("DOWN") || outcome == "0" {
                down_token = Some(token.token_id);
            }
        }

        let up = up_token.ok_or_else(|| anyhow::anyhow!("Up token not found"))?;
        let down = down_token.ok_or_else(|| anyhow::anyhow!("Down token not found"))?;
        Ok((up, down))
    }

    pub async fn get_15m_market(
        &self,
        symbol: &str,
        period_start: i64,
    ) -> Result<Option<(String, Option<f64>)>> {
        let slug = build_15m_slug(symbol, period_start);
        let market = match self.api.get_market_by_slug(&slug).await {
            Ok(m) => m,
            Err(_) => return Ok(None),
        };
        if !market.active || market.closed {
            return Ok(None);
        }
        let price_to_beat = parse_price_to_beat_from_question(&market.question);
        Ok(Some((market.condition_id, price_to_beat)))
    }

    pub async fn get_5m_market(
        &self,
        symbol: &str,
        period_start: i64,
    ) -> Result<Option<(String, Option<f64>)>> {
        let slug = build_5m_slug(symbol, period_start);
        let market = match self.api.get_market_by_slug(&slug).await {
            Ok(m) => m,
            Err(_) => return Ok(None),
        };
        if !market.active || market.closed {
            return Ok(None);
        }
        let price_to_beat = parse_price_to_beat_from_question(&market.question);
        Ok(Some((market.condition_id, price_to_beat)))
    }
}

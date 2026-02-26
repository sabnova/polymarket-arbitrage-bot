use crate::adapters::polymarket::PolymarketApi;
use crate::models::{Market, MarketDetails, OrderBook, TokenPrice};
use anyhow::Result;
use rust_decimal::Decimal;

pub async fn get_market_by_slug(api: &PolymarketApi, slug: &str) -> Result<Market> {
    api.get_market_by_slug(slug).await
}

pub async fn get_market(api: &PolymarketApi, condition_id: &str) -> Result<MarketDetails> {
    api.get_market(condition_id).await
}

pub async fn get_orderbook(api: &PolymarketApi, token_id: &str) -> Result<OrderBook> {
    api.get_orderbook(token_id).await
}

pub async fn get_price(api: &PolymarketApi, token_id: &str, side: &str) -> Result<Decimal> {
    api.get_price(token_id, side).await
}

pub async fn get_best_price(api: &PolymarketApi, token_id: &str) -> Result<Option<TokenPrice>> {
    api.get_best_price(token_id).await
}

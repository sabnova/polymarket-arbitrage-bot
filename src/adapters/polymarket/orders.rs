use crate::adapters::polymarket::PolymarketApi;
use crate::models::{OrderRequest, OrderResponse, OrderStatus};
use anyhow::Result;

pub async fn place_order(api: &PolymarketApi, order: &OrderRequest) -> Result<OrderResponse> {
    api.place_order(order).await
}

pub async fn place_market_order(
    api: &PolymarketApi,
    token_id: &str,
    amount: f64,
    side: &str,
    order_type: Option<&str>,
) -> Result<OrderResponse> {
    api.place_market_order(token_id, amount, side, order_type).await
}

pub async fn cancel_order(api: &PolymarketApi, order_id: &str) -> Result<()> {
    api.cancel_order(order_id).await
}

pub async fn get_order_status(api: &PolymarketApi, order_id: &str) -> Result<OrderStatus> {
    api.get_order_status(order_id).await
}

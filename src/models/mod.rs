use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    #[serde(rename = "conditionId")]
    pub condition_id: String,
    #[serde(rename = "id")]
    pub market_id: Option<String>,
    pub question: String,
    pub slug: String,
    #[serde(rename = "endDateISO")]
    pub end_date_iso: Option<String>,
    pub active: bool,
    pub closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDetails {
    #[serde(rename = "condition_id")]
    pub condition_id: String,
    pub question: String,
    pub tokens: Vec<MarketToken>,
    pub active: bool,
    pub closed: bool,
    #[serde(rename = "end_date_iso")]
    pub end_date_iso: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketToken {
    pub outcome: String,
    #[serde(rename = "token_id")]
    pub token_id: String,
    pub winner: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub bids: Vec<OrderBookEntry>,
    pub asks: Vec<OrderBookEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookEntry {
    pub price: Decimal,
    pub size: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub token_id: String,
    pub side: String,
    pub size: String,
    pub price: String,
    #[serde(rename = "type")]
    pub order_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    pub order_id: Option<String>,
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderStatus {
    pub id: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "original_size")]
    pub original_size: Option<String>,
    #[serde(rename = "size_matched")]
    pub size_matched: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedeemResponse {
    pub success: bool,
    pub message: Option<String>,
    pub transaction_hash: Option<String>,
    pub amount_redeemed: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    pub token_id: String,
    pub bid: Option<Decimal>,
    pub ask: Option<Decimal>,
}

/// Record of an arb trade for PnL tracking and redeem.
#[derive(Debug, Clone)]
pub struct TradeRecord {
    pub symbol: String,
    pub period_15: i64,
    pub period_5: i64,
    pub cid_15: String,
    pub cid_5: String,
    /// Leg 1: token_id, price, condition_id (15m or 5m), outcome "Up" or "Down"
    pub leg1_token: String,
    pub leg1_price: f64,
    pub leg1_cid: String,
    pub leg1_outcome: String,
    /// Leg 2
    pub leg2_token: String,
    pub leg2_price: f64,
    pub leg2_cid: String,
    pub leg2_outcome: String,
    pub size: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    #[serde(rename = "tokenID")]
    pub token_id: Option<String>,
    pub side: String,
    pub size: f64,
    pub price: f64,
    pub timestamp: u64,
    #[serde(rename = "conditionId")]
    pub condition_id: Option<String>,
}

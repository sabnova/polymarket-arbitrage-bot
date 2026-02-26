//! CLOB Market WebSocket: subscribe to asset_ids and stream best bid/ask updates.

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const WS_MARKET_PATH: &str = "ws/market";

#[derive(Debug, Deserialize)]
struct WsBookLevel {
    price: String,
    #[allow(dead_code)]
    size: String,
}

#[derive(Debug, Deserialize)]
struct WsBookMessage {
    #[serde(rename = "event_type")]
    #[allow(dead_code)]
    event_type: Option<String>,
    #[serde(rename = "asset_id")]
    asset_id: String,
    #[serde(default, alias = "bids")]
    buys: Vec<WsBookLevel>,
    #[serde(default, alias = "asks")]
    sells: Vec<WsBookLevel>,
}

#[derive(Debug, Deserialize)]
struct WsPriceChangeItem {
    #[serde(rename = "asset_id")]
    asset_id: String,
    #[serde(rename = "best_bid")]
    best_bid: Option<String>,
    #[serde(rename = "best_ask")]
    best_ask: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WsPriceChangeMessage {
    #[serde(rename = "event_type")]
    #[allow(dead_code)]
    event_type: Option<String>,
    #[serde(rename = "price_changes")]
    price_changes: Vec<WsPriceChangeItem>,
}

#[derive(Debug, Clone, Default)]
pub struct BestPrices {
    pub bid: Option<f64>,
    pub ask: Option<f64>,
}

pub type PricesSnapshot = Arc<RwLock<HashMap<String, BestPrices>>>;

fn parse_f64(s: &str) -> Option<f64> {
    s.trim().parse().ok()
}

fn is_placeholder_quote(bid: Option<f64>, ask: Option<f64>) -> bool {
    match (bid, ask) {
        (Some(b), Some(a)) => b < 0.05 && a > 0.95,
        (Some(b), None) => b < 0.05,
        (None, Some(a)) => a > 0.95,
        (None, None) => false,
    }
}

const WS_RECONNECT_DELAY_SECS: u64 = 3;

pub async fn run_market_ws(
    ws_base_url: &str,
    asset_ids: Vec<String>,
    prices: PricesSnapshot,
) -> Result<()> {
    let url = format!("{}/{}", ws_base_url.trim_end_matches('/'), WS_MARKET_PATH);
    let sub = serde_json::json!({
        "assets_ids": asset_ids.clone(),
        "type": "market"
    });
    let sub_body = serde_json::to_string(&sub)?;

    loop {
        info!("Connecting to market WebSocket: {}", url);
        let (ws_stream, _) = match connect_async(&url).await {
            Ok(s) => s,
            Err(e) => {
                error!(
                    "WebSocket connect failed: {}. Reconnecting in {}s.",
                    e, WS_RECONNECT_DELAY_SECS
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(WS_RECONNECT_DELAY_SECS)).await;
                continue;
            }
        };

        let (mut write, mut read) = ws_stream.split();
        let sub_msg = Message::Text(sub_body.clone());
        if let Err(e) = write.send(sub_msg).await {
            error!(
                "WebSocket send subscribe failed: {}. Reconnecting in {}s.",
                e, WS_RECONNECT_DELAY_SECS
            );
            tokio::time::sleep(tokio::time::Duration::from_secs(WS_RECONNECT_DELAY_SECS)).await;
            continue;
        }
        info!("Subscribed to {} assets", asset_ids.len());

        let mut disconnected = false;
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if text == "PONG" || text == "pong" {
                        continue;
                    }
                    if let Err(e) = process_message(&text, &prices).await {
                        debug!("WS parse error: {} for message: {}", e, &text[..text.len().min(200)]);
                    }
                }
                Ok(Message::Ping(data)) => {
                    let _ = write.send(Message::Pong(data)).await;
                }
                Ok(Message::Close(_)) => {
                    info!(
                        "WebSocket closed by server. Reconnecting in {}s.",
                        WS_RECONNECT_DELAY_SECS
                    );
                    disconnected = true;
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}. Reconnecting in {}s.", e, WS_RECONNECT_DELAY_SECS);
                    disconnected = true;
                    break;
                }
                _ => {}
            }
        }
        if disconnected {
            tokio::time::sleep(tokio::time::Duration::from_secs(WS_RECONNECT_DELAY_SECS)).await;
        } else {
            break;
        }
    }

    Ok(())
}

async fn process_message(text: &str, prices: &PricesSnapshot) -> Result<()> {
    let v: serde_json::Value = serde_json::from_str(text).context("Parse JSON")?;
    let event_type = v.get("event_type").and_then(|t| t.as_str());

    if event_type == Some("book") {
        let book: WsBookMessage = serde_json::from_value(v).context("Parse book")?;
        let bid = book.buys.first().and_then(|b| parse_f64(&b.price));
        let ask = book.sells.first().and_then(|a| parse_f64(&a.price));
        if (bid.is_some() || ask.is_some()) && !is_placeholder_quote(bid, ask) {
            let mut w = prices.write().await;
            let entry = w.entry(book.asset_id).or_default();
            if let Some(b) = bid {
                entry.bid = Some(b);
            }
            if let Some(a) = ask {
                entry.ask = Some(a);
            }
        }
        return Ok(());
    }

    if event_type == Some("price_change") {
        let msg: WsPriceChangeMessage = serde_json::from_value(v).context("Parse price_change")?;
        let mut w = prices.write().await;
        for pc in msg.price_changes {
            let bid = pc.best_bid.and_then(|s| parse_f64(&s));
            let ask = pc.best_ask.and_then(|s| parse_f64(&s));
            if (bid.is_some() || ask.is_some()) && !is_placeholder_quote(bid, ask) {
                let entry = w.entry(pc.asset_id).or_default();
                if let Some(b) = bid {
                    entry.bid = Some(b);
                }
                if let Some(a) = ask {
                    entry.ask = Some(a);
                }
            }
        }
        return Ok(());
    }

    Ok(())
}

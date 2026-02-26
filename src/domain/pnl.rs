use crate::models::TradeRecord;

pub struct TradePnl {
    pub cost: f64,
    pub payout: f64,
    pub pnl: f64,
    pub won_15m: bool,
    pub won_5m: bool,
}

pub fn compute_trade_pnl(trade: &TradeRecord, win_token_15: &str, win_token_5: &str) -> TradePnl {
    let cost = (trade.leg1_price + trade.leg2_price) * trade.size;
    let won_15m = win_token_15 == trade.leg1_token || win_token_15 == trade.leg2_token;
    let won_5m = win_token_5 == trade.leg1_token || win_token_5 == trade.leg2_token;
    let payout = trade.size * ((won_15m as i32 + won_5m as i32) as f64);
    let pnl = payout - cost;
    TradePnl {
        cost,
        payout,
        pnl,
        won_15m,
        won_5m,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_trade() -> TradeRecord {
        TradeRecord {
            symbol: "btc".to_string(),
            period_15: 1,
            period_5: 1,
            cid_15: "c15".to_string(),
            cid_5: "c5".to_string(),
            leg1_token: "a".to_string(),
            leg1_price: 0.45,
            leg1_cid: "c15".to_string(),
            leg1_outcome: "Up".to_string(),
            leg2_token: "b".to_string(),
            leg2_price: 0.47,
            leg2_cid: "c5".to_string(),
            leg2_outcome: "Down".to_string(),
            size: 10.0,
        }
    }

    #[test]
    fn computes_two_leg_win_pnl() {
        let result = compute_trade_pnl(&sample_trade(), "a", "b");
        assert_eq!(result.cost, 9.2);
        assert_eq!(result.payout, 20.0);
        assert_eq!(result.pnl, 10.8);
    }
}

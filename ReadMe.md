# Polymarket BTC Arbitrage Bot

Systematic execution for the BTC 15m vs 5m overlap on Polymarket.

Built for traders who care about one thing: consistent, risk-controlled performance.

---

## Why Use This

- Captures short-lived pricing dislocations between related BTC markets.
- Enters only when pricing is favorable enough to target positive expectancy.
- Uses strict two-leg execution logic to reduce one-sided exposure.
- Includes simulation mode to validate settings before risking capital.

This is not a signal product. This is an execution engine.

---

## Strategy In 30 Seconds

The bot monitors the overlap between BTC 15-minute and BTC 5-minute Up/Down markets.

When either pair is mispriced enough:

- `15m Up + 5m Down < threshold`, or
- `15m Down + 5m Up < threshold`,

it attempts to buy both legs and lock in spread.

If both legs fill, the trade is complete.
If only one leg fills, the bot exits the filled leg and cancels the other order.

---

## What Matters (Performance View)

Track these metrics every day:

- Net PnL
- ROI on deployed capital
- Fill rate (both legs filled)
- Missed opportunities
- Slippage and fees paid
- Max intraday drawdown

If you want this README to convert serious traders, keep this section updated with real numbers from your logs.

### Example Performance Block (replace with your real results)

| Metric | Last 7D | Last 30D | Since Launch |
|---|---:|---:|---:|
| Net PnL | +$0.00 | +$0.00 | +$0.00 |
| ROI | 0.00% | 0.00% | 0.00% |
| Trades Executed | 0 | 0 | 0 |
| Two-Leg Fill Rate | 0.00% | 0.00% | 0.00% |
| Avg Edge Captured | 0.000 | 0.000 | 0.000 |
| Max Drawdown | 0.00% | 0.00% | 0.00% |

---

## Risk Controls

- Executes only during valid overlap windows.
- Requires matching price-to-beat context before entry.
- Verifies fills shortly after placement.
- Auto-unwinds one-leg fills to limit directional risk.
- Supports simulation mode before live deployment.

No bot eliminates risk. This reduces execution risk, not market risk.

---

## Quick Start (Non-Technical)

1. Create a Polymarket account and API credentials.
2. Fund your proxy wallet.
3. Set strategy size (`shares`) and entry threshold (`sum_threshold`).
4. Run in simulation mode first.
5. Go live only after you verify logs and behavior.

If you have a technical operator, they can use the setup section below.

---

## Technical Setup

### Install

```bash
git clone https://github.com/gamma-trade-lab/polymarket-arbitrage-bot.git
cd polymarket-arbitrage-bot
cargo build --release
```

Binary path: `target/release/polymarket-arbitrage-bot`

### Configure `config.json`

```json
{
  "polymarket": {
    "gamma_api_url": "https://gamma-api.polymarket.com",
    "clob_api_url": "https://clob.polymarket.com",
    "api_key": "YOUR_API_KEY",
    "api_secret": "YOUR_API_SECRET",
    "api_passphrase": "YOUR_PASSPHRASE",
    "private_key": "YOUR_POLYGON_PRIVATE_KEY_HEX",
    "proxy_wallet_address": "0x...",
    "signature_type": 2,
    "ws_url": "wss://ws-subscriptions-clob.polymarket.com"
  },
  "strategy": {
    "sum_threshold": 0.99,
    "shares": 5,
    "verify_fill_secs": 10,
    "simulation_mode": true,
    "price_to_beat_delay_secs": 30,
    "price_to_beat_poll_interval_secs": 10
  }
}
```

Important:

- `sum_threshold`: lower usually means higher selectivity.
- `shares`: position size per leg.
- `simulation_mode`: set `true` before going live.

Never commit real keys to git.

### Run

```bash
cargo run --release
```

or

```bash
./target/release/polymarket-arbitrage-bot
```

Custom config:

```bash
./target/release/polymarket-arbitrage-bot -c /path/to/config.json
```

Redeem winning positions:

```bash
./target/release/polymarket-arbitrage-bot --redeem
```

---

## Compliance And Disclaimer

This software is for research and execution automation. Trading prediction markets and crypto involves substantial risk, including total loss. Past performance does not guarantee future results. You are responsible for legal and tax compliance in your jurisdiction and for adhering to Polymarket terms.

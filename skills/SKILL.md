---
name: fintool
description: "Financial trading CLI — spot and perp trading on Hyperliquid, Binance, Coinbase. Deposit and withdraw across chains (Unit bridge, Across Protocol). LLM-enriched price quotes with trend analysis. News and SEC filings. Use when: user asks about stock/crypto prices, wants to trade, deposit, withdraw, or check portfolio."
homepage: https://github.com/second-state/fintool
metadata: { "openclaw": { "emoji": "📈", "requires": { "bins": ["curl"] } } }
---

# fintool — Financial Trading Skill

A CLI tool for market intelligence and trading across multiple exchanges.

## Tool

- **Binary**: `{baseDir}/scripts/fintool`
- **Config**: `~/.fintool/config.toml`
- **Mode**: Always use JSON mode — `fintool --json '<JSON>'`. All input and output is structured JSON.

## Setup Check (MANDATORY — do this FIRST)

Before running any command, verify the user's configuration:

```bash
cat ~/.fintool/config.toml 2>/dev/null
```

**If the file doesn't exist**, run:
```bash
{baseDir}/scripts/fintool --json '{"command":"init"}'
```

**Check for these requirements:**

1. **OpenAI API key** — Required for enriched quotes (trend/momentum analysis).
   - Look for: `openai_api_key = "sk-..."` (uncommented) in `[api_keys]`
   - If missing: Ask the user for their OpenAI API key, or tell them to add it to `~/.fintool/config.toml` under `[api_keys]`

2. **At least one exchange** — Required for trading commands.
   - **Hyperliquid**: `private_key` or `wallet_json` + `wallet_passcode` in `[wallet]` (spot + perps)
   - **Binance**: `binance_api_key` + `binance_api_secret` in `[api_keys]` (spot + perps)
   - **Coinbase**: `coinbase_api_key` + `coinbase_api_secret` in `[api_keys]` (spot only)
   - If none configured: Ask the user which exchange they want to use and request the credentials.

**If the user provides credentials**, edit `~/.fintool/config.toml` directly to add them.

## Exchange Capabilities

| Feature | Hyperliquid | Binance | Coinbase |
|---------|-------------|---------|----------|
| Spot orders | ✅ | ✅ | ✅ |
| Perp orders | ✅ | ✅ | ❌ |
| Orderbook | ✅ | ✅ | ✅ |
| Deposit | ✅ (Unit + Across) | ✅ (API) | ✅ (API) |
| Withdraw | ✅ (Bridge2 + Unit + Across) | ✅ (API) | ✅ (API) |
| Balance | ✅ | ✅ | ✅ |
| Open orders | ✅ | ✅ | ✅ |
| Cancel | ✅ | ✅ | ✅ |
| Positions | ✅ | ✅ | ❌ |

**Auto exchange priority**: Hyperliquid > Coinbase > Binance for spot. Hyperliquid > Binance for perps.

Use `"exchange": "hyperliquid"` (or `"binance"`, `"coinbase"`) to force a specific exchange. Defaults to `"auto"`.

## Error Handling

- All errors are returned as `{"error": "..."}`. Check for an `error` key in every response.
- If a command returns an **exchange error**, suggest the user try a different exchange with `"exchange": "<name>"`.
- If the selected exchange is **not configured**, tell the user which credentials are needed and offer to add them to config.
- If a trading command fails with **insufficient balance** or **invalid symbol**, relay the error clearly.

## JSON Command Reference

All commands use: `{baseDir}/scripts/fintool --json '<JSON>'`

### Market Data

```json
{"command": "quote", "symbol": "BTC"}
{"command": "perp_quote", "symbol": "ETH"}
{"command": "orderbook", "symbol": "HYPE"}
{"command": "orderbook", "symbol": "BTC", "levels": 10, "exchange": "binance"}
{"command": "perp_orderbook", "symbol": "BTC"}
{"command": "perp_orderbook", "symbol": "ETH", "levels": 20}
{"command": "news", "symbol": "AAPL"}
```

### Spot Trading

```json
{"command": "order_buy", "symbol": "ETH", "amount": 0.1, "price": 3800}
{"command": "order_sell", "symbol": "ETH", "amount": 0.1, "price": 4000}
{"command": "order_buy", "symbol": "BTC", "amount": 0.01, "price": 95000, "exchange": "binance"}
```

### Perpetual Futures

```json
{"command": "perp_buy", "symbol": "ETH", "amount": 0.5, "price": 3800}
{"command": "perp_sell", "symbol": "BTC", "amount": 0.01, "price": 100000}
{"command": "perp_buy", "symbol": "ETH", "amount": 0.5, "price": 3900, "close": true}
{"command": "perp_sell", "symbol": "BTC", "amount": 0.01, "price": 95000, "close": true}
{"command": "perp_leverage", "symbol": "ETH", "leverage": 5, "cross": true}
{"command": "perp_set_mode", "mode": "unified"}
```

### Portfolio

```json
{"command": "balance"}
{"command": "balance", "exchange": "binance"}
{"command": "positions"}
{"command": "orders"}
{"command": "orders", "symbol": "BTC"}
{"command": "cancel", "order_id": "BTC:91490942"}
{"command": "cancel", "order_id": "binance_spot:BTCUSDT:123"}
```

### Deposits

```json
{"command": "deposit", "asset": "ETH"}
{"command": "deposit", "asset": "BTC"}
{"command": "deposit", "asset": "USDC", "amount": 100, "from": "ethereum"}
{"command": "deposit", "asset": "USDC", "amount": 500, "from": "base"}
{"command": "deposit", "asset": "USDC", "amount": 100, "from": "ethereum", "dry_run": true}
{"command": "deposit", "asset": "USDC", "exchange": "binance", "from": "ethereum"}
{"command": "deposit", "asset": "ETH", "exchange": "coinbase"}
```

### Withdrawals

```json
{"command": "withdraw", "asset": "USDC", "amount": 100}
{"command": "withdraw", "asset": "USDC", "amount": 100, "to": "0xOtherAddress"}
{"command": "withdraw", "asset": "USDC", "amount": 100, "to": "ethereum"}
{"command": "withdraw", "asset": "USDC", "amount": 100, "to": "base"}
{"command": "withdraw", "asset": "USDC", "amount": 100, "to": "ethereum", "dry_run": true}
{"command": "withdraw", "asset": "ETH", "amount": 0.5}
{"command": "withdraw", "asset": "BTC", "amount": 0.01, "to": "bc1q..."}
{"command": "withdraw", "asset": "USDC", "amount": 100, "to": "0x...", "network": "ethereum"}
```

### Transfers (between spot/perp/HIP-3 dex)

```json
{"command": "transfer", "asset": "USDT0", "amount": 50, "from": "spot", "to": "cash"}
{"command": "transfer", "asset": "USDT0", "amount": 50, "from": "cash", "to": "spot"}
```

### Bridge Status

```json
{"command": "bridge_status"}
```

### SEC Reports

```json
{"command": "report_list", "symbol": "AAPL", "limit": 5}
{"command": "report_annual", "symbol": "AAPL"}
{"command": "report_quarterly", "symbol": "AAPL"}
{"command": "report_get", "symbol": "AAPL", "accession": "0000320193-24-000123"}
```

### Prediction Markets (Polymarket)

```json
{"command": "predict_list", "query": "bitcoin"}
{"command": "predict_list", "query": "election", "limit": 5}
{"command": "predict_list", "query": "bitcoin", "min_end_days": 7}
{"command": "predict_list", "query": "bitcoin", "min_end_days": 0}
{"command": "predict_quote", "market": "will-bitcoin-hit-100k"}
{"command": "predict_buy", "market": "will-bitcoin-hit-100k", "outcome": "yes", "amount": 10, "price": 0.65}
{"command": "predict_sell", "market": "will-bitcoin-hit-100k", "outcome": "yes", "amount": 10, "price": 0.70}
{"command": "predict_positions"}
{"command": "predict_deposit", "amount": 100, "from": "base"}
{"command": "predict_balance"}
{"command": "predict_withdraw", "amount": 50}
```

**Notes:**
- `predict_list` defaults to `min_end_days: 3`, filtering out markets that close within 3 days (which often have odds near 1:0). Set to `0` to see all markets.
- `predict_list` and `predict_quote` are read-only and don't require Polymarket credentials.
- Trading commands (`predict_buy`, `predict_sell`, `predict_deposit`, `predict_withdraw`) require `wallet.private_key` in config.
- Use the market slug (from `predict_list`) or condition ID as the `market` value.

## Workflows

### Workflow 1: Spot Trading

**Goal**: Research a symbol and place a spot trade.

**Step 1 — Quote price with analysis:**
```bash
{baseDir}/scripts/fintool --json '{"command":"quote","symbol":"BTC"}'
```
Returns: price, 24h/7d/30d changes, trend (bullish/bearish/neutral), momentum, volume analysis, summary. Uses data from Hyperliquid + Yahoo Finance + CoinGecko, merged by OpenAI.

**Step 1b — Check orderbook depth and spread:**
```bash
{baseDir}/scripts/fintool --json '{"command":"orderbook","symbol":"BTC"}'
```
Returns: bids, asks, spread, spreadPct, midPrice. Use to assess liquidity before trading.

**Step 2 — Check recent news:**
```bash
{baseDir}/scripts/fintool --json '{"command":"news","symbol":"BTC"}'
```
Returns: up to 10 recent headlines from Google News RSS.

**Step 3 — Place the trade:**
```bash
# Buy: buy 0.01 BTC at max price $95000
{baseDir}/scripts/fintool --json '{"command":"order_buy","symbol":"BTC","amount":0.01,"price":95000}'

# Sell: sell 0.01 BTC at min price $100000
{baseDir}/scripts/fintool --json '{"command":"order_sell","symbol":"BTC","amount":0.01,"price":100000}'

# Force a specific exchange:
{baseDir}/scripts/fintool --json '{"command":"order_buy","symbol":"BTC","amount":0.01,"price":95000,"exchange":"binance"}'
```

**Step 4 — Verify:**
```bash
{baseDir}/scripts/fintool --json '{"command":"orders"}'
{baseDir}/scripts/fintool --json '{"command":"balance"}'
```

### Workflow 2: Perpetual Futures Trading

**Goal**: Research and take a leveraged position via perpetual futures.

**Step 1 — Get perp quote with funding/OI:**
```bash
{baseDir}/scripts/fintool --json '{"command":"perp_quote","symbol":"ETH"}'
```
Returns: mark price, oracle price, funding rate, open interest, premium, max leverage.

**Step 1b — Check perp orderbook depth and spread:**
```bash
{baseDir}/scripts/fintool --json '{"command":"perp_orderbook","symbol":"ETH"}'
```
Returns: bids, asks, spread, spreadPct, midPrice. Use to assess liquidity and set limit prices.

**Step 2 — Check spot price for context:**
```bash
{baseDir}/scripts/fintool --json '{"command":"quote","symbol":"ETH"}'
```

**Step 3 — Check news:**
```bash
{baseDir}/scripts/fintool --json '{"command":"news","symbol":"ETH"}'
```

**Step 4 — Place the trade:**
```bash
# Long: buy 0.5 ETH at limit price $3800
{baseDir}/scripts/fintool --json '{"command":"perp_buy","symbol":"ETH","amount":0.5,"price":3800}'

# Short: sell 0.5 ETH at limit price $4000
{baseDir}/scripts/fintool --json '{"command":"perp_sell","symbol":"ETH","amount":0.5,"price":4000}'
```

**Step 5 — Monitor:**
```bash
{baseDir}/scripts/fintool --json '{"command":"positions"}'
{baseDir}/scripts/fintool --json '{"command":"orders"}'
```

**Step 6 — Close a position:**
```bash
# Close a long (reduce-only) — sells without opening a new short
{baseDir}/scripts/fintool --json '{"command":"perp_sell","symbol":"ETH","amount":0.5,"price":4000,"close":true}'

# Close a short (reduce-only) — buys without opening a new long
{baseDir}/scripts/fintool --json '{"command":"perp_buy","symbol":"ETH","amount":0.5,"price":3800,"close":true}'
```
Use `"close": true` to ensure the order only reduces an existing position. Without it, the order could flip you into the opposite direction.

**Note**: Perps only available on Hyperliquid and Binance. If the user only has Coinbase configured, tell them perps are not supported on Coinbase.

### Workflow 3: Portfolio Overview

**Goal**: Check current positions and balances across exchanges.

```bash
# Account balance
{baseDir}/scripts/fintool --json '{"command":"balance"}'
{baseDir}/scripts/fintool --json '{"command":"balance","exchange":"binance"}'

# Open positions (perps)
{baseDir}/scripts/fintool --json '{"command":"positions"}'

# Open orders
{baseDir}/scripts/fintool --json '{"command":"orders"}'
{baseDir}/scripts/fintool --json '{"command":"orders","symbol":"BTC"}'

# Cancel an order
{baseDir}/scripts/fintool --json '{"command":"cancel","order_id":"BTC:91490942"}'
{baseDir}/scripts/fintool --json '{"command":"cancel","order_id":"binance_spot:BTCUSDT:123"}'
{baseDir}/scripts/fintool --json '{"command":"cancel","order_id":"binance_futures:BTCUSDT:456"}'
{baseDir}/scripts/fintool --json '{"command":"cancel","order_id":"coinbase:uuid-here"}'
```

### Workflow 4: Depositing Funds

**Goal**: Fund an exchange account with crypto from an external wallet or another chain.

**Hyperliquid — ETH/BTC/SOL (permanent deposit address via HyperUnit):**
```bash
{baseDir}/scripts/fintool --json '{"command":"deposit","asset":"ETH"}'
{baseDir}/scripts/fintool --json '{"command":"deposit","asset":"BTC"}'
{baseDir}/scripts/fintool --json '{"command":"deposit","asset":"SOL"}'
```
Returns a reusable deposit address on the native chain. User sends any amount, any time.

**Hyperliquid — USDC from Ethereum or Base (automated bridge):**
```bash
# Bridge 100 USDC from Ethereum mainnet → Hyperliquid
{baseDir}/scripts/fintool --json '{"command":"deposit","asset":"USDC","amount":100,"from":"ethereum"}'

# Bridge 500 USDC from Base → Hyperliquid
{baseDir}/scripts/fintool --json '{"command":"deposit","asset":"USDC","amount":500,"from":"base"}'

# Preview the route and fees without executing
{baseDir}/scripts/fintool --json '{"command":"deposit","asset":"USDC","amount":100,"from":"ethereum","dry_run":true}'
```
Automatically signs 3 transactions: approval → Across bridge → HL Bridge2 deposit.

**Binance — get deposit address:**
```bash
{baseDir}/scripts/fintool --json '{"command":"deposit","asset":"USDC","exchange":"binance","from":"ethereum"}'
{baseDir}/scripts/fintool --json '{"command":"deposit","asset":"ETH","exchange":"binance"}'
{baseDir}/scripts/fintool --json '{"command":"deposit","asset":"BTC","exchange":"binance","from":"bitcoin"}'
```

**Coinbase — get deposit address:**
```bash
{baseDir}/scripts/fintool --json '{"command":"deposit","asset":"ETH","exchange":"coinbase"}'
{baseDir}/scripts/fintool --json '{"command":"deposit","asset":"USDC","exchange":"coinbase"}'
```

### Workflow 5: Withdrawing Funds

**Goal**: Move assets from an exchange to an external wallet or another chain.

**Hyperliquid — USDC to Arbitrum (default, ~3-4 min):**
```bash
{baseDir}/scripts/fintool --json '{"command":"withdraw","asset":"USDC","amount":100}'
{baseDir}/scripts/fintool --json '{"command":"withdraw","asset":"USDC","amount":100,"to":"0xOtherAddress"}'
```

**Hyperliquid — USDC to Ethereum or Base (chained bridge, ~5-7 min):**
```bash
{baseDir}/scripts/fintool --json '{"command":"withdraw","asset":"USDC","amount":100,"to":"ethereum"}'
{baseDir}/scripts/fintool --json '{"command":"withdraw","asset":"USDC","amount":100,"to":"base"}'
{baseDir}/scripts/fintool --json '{"command":"withdraw","asset":"USDC","amount":100,"to":"ethereum","dry_run":true}'
```
Automatically chains: HL Bridge2 → Arbitrum → Across bridge → destination.

**Hyperliquid — ETH/BTC/SOL to native chain (via HyperUnit):**
```bash
{baseDir}/scripts/fintool --json '{"command":"withdraw","asset":"ETH","amount":0.5}'
{baseDir}/scripts/fintool --json '{"command":"withdraw","asset":"BTC","amount":0.01,"to":"bc1q..."}'
{baseDir}/scripts/fintool --json '{"command":"withdraw","asset":"SOL","amount":1,"to":"SomeSolanaAddress"}'
```

**Binance:**
```bash
{baseDir}/scripts/fintool --json '{"command":"withdraw","asset":"USDC","amount":100,"to":"0x...","network":"ethereum"}'
{baseDir}/scripts/fintool --json '{"command":"withdraw","asset":"ETH","amount":0.5,"to":"0x...","network":"arbitrum"}'
```

**Coinbase:**
```bash
{baseDir}/scripts/fintool --json '{"command":"withdraw","asset":"USDC","amount":100,"to":"0x..."}'
{baseDir}/scripts/fintool --json '{"command":"withdraw","asset":"ETH","amount":0.5,"to":"0x...","network":"base"}'
```

**Track HyperUnit bridge operations:**
```bash
{baseDir}/scripts/fintool --json '{"command":"bridge_status"}'
```

### Workflow 6: Prediction Market Trading (Polymarket)

**Goal**: Research and trade on prediction markets.

**Step 1 — Search for markets:**
```bash
{baseDir}/scripts/fintool --json '{"command":"predict_list","query":"bitcoin"}'
```
Returns: matching markets with question, outcomes, prices, volume, liquidity, and end date. By default filters out markets closing within 3 days (which have odds near 1:0). Use `"min_end_days": 7` for markets ending further out, or `"min_end_days": 0` to see all.

**Step 2 — Get detailed quote:**
```bash
{baseDir}/scripts/fintool --json '{"command":"predict_quote","market":"will-bitcoin-hit-100k"}'
```
Returns: full market details including condition ID, CLOB token IDs, prices, and volume.

**Step 3 — Place a trade:**
```bash
# Buy "yes" shares at $0.65
{baseDir}/scripts/fintool --json '{"command":"predict_buy","market":"will-bitcoin-hit-100k","outcome":"yes","amount":10,"price":0.65}'

# Sell "yes" shares at $0.70
{baseDir}/scripts/fintool --json '{"command":"predict_sell","market":"will-bitcoin-hit-100k","outcome":"yes","amount":10,"price":0.70}'
```

**Step 4 — Monitor positions:**
```bash
{baseDir}/scripts/fintool --json '{"command":"predict_positions"}'
{baseDir}/scripts/fintool --json '{"command":"predict_balance"}'
```

**Note**: Use the market slug from `predict_list` output as the `market` value.

## Symbol Aliases

Common indices and commodities have convenient aliases:

| Alias | Description |
|-------|-------------|
| `SP500`, `SPX` | S&P 500 |
| `NASDAQ`, `NDX` | Nasdaq |
| `DOW`, `DJI` | Dow Jones |
| `VIX` | Volatility Index |
| `GOLD` | Gold Futures |
| `SILVER` | Silver Futures |
| `OIL`, `CRUDE` | Crude Oil |
| `10Y`, `30Y` | Treasury Yields |
| `NIKKEI`, `FTSE`, `DAX`, `HSI` | International indices |

## Tips

- **Always quote before trading** — The enriched quote gives trend/momentum context that helps with timing.
- **Check news before large trades** — Headlines can explain sudden price moves.
- **Use `"exchange"` when ambiguous** — If the user has multiple exchanges, explicitly select one to avoid confusion.
- **All output is JSON** — Parse the response and present relevant fields to the user in a readable format.
- **Amount is in symbol units** — `"amount": 0.1` on ETH means 0.1 ETH, not $0.10. Calculate the size from the price quote.
- **Check for errors** — Every response may contain `{"error": "..."}`. Always check before presenting results.

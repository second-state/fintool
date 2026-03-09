---
name: fintool
description: "Financial trading CLIs — spot and perp trading on Hyperliquid, Binance, Coinbase. Prediction markets on Polymarket. Deposit and withdraw across chains. LLM-enriched price quotes with trend analysis. News and SEC filings. Use when: user asks about stock/crypto prices, wants to trade, deposit, withdraw, or check portfolio."
homepage: https://github.com/second-state/fintool
metadata: { "openclaw": { "emoji": "📈", "requires": { "bins": ["curl"] } } }
---

# fintool — Financial Trading Skill

A suite of CLI tools for market intelligence and trading across multiple exchanges. Each exchange has its own binary.

## Tools

| Binary | Purpose | Path |
|--------|---------|------|
| `fintool` | Market intelligence (quotes, news, SEC filings) | `{baseDir}/scripts/fintool` |
| `hyperliquid` | Hyperliquid trading (spot, perp, HIP-3, deposits) | `{baseDir}/scripts/hyperliquid` |
| `binance` | Binance trading (spot, perp, deposits) | `{baseDir}/scripts/binance` |
| `coinbase` | Coinbase trading (spot, deposits) | `{baseDir}/scripts/coinbase` |
| `polymarket` | Polymarket prediction markets | `{baseDir}/scripts/polymarket` |

- **Config**: `~/.fintool/config.toml`
- **Mode**: Always use JSON mode — `<binary> --json '<JSON>'`. All input and output is structured JSON.
- **No `exchange` field**: Each binary handles one exchange. No `"exchange"` parameter needed.

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
   - **Polymarket**: Uses `wallet.private_key` (same as Hyperliquid) for prediction market trading
   - If none configured: Ask the user which exchange they want to use and request the credentials.

**If the user provides credentials**, edit `~/.fintool/config.toml` directly to add them.

## Exchange Capabilities

| Feature | `hyperliquid` | `binance` | `coinbase` | `polymarket` |
|---------|---------------|-----------|------------|--------------|
| Spot orders | buy, sell | buy, sell | buy, sell | — |
| Perp orders | perp buy/sell | perp buy/sell | — | — |
| Prediction markets | — | — | — | buy, sell, list, quote |
| Orderbook | spot + perp | spot + perp | spot | — |
| Deposit | Unit + Across | API | API | bridge |
| Withdraw | Bridge2 + Unit + Across | API | API | bridge |
| Balance | balance | balance | balance | balance |
| Open orders | orders | orders | orders | — |
| Cancel | cancel | cancel | cancel | — |
| Positions | positions | positions | — | positions |

## Error Handling

- All errors are returned as `{"error": "..."}`. Check for an `error` key in every response.
- If a command fails with **insufficient balance** or **invalid symbol**, relay the error clearly.
- If the exchange is **not configured**, tell the user which credentials are needed and offer to add them to config.

## JSON Command Reference

### Market Data (`fintool`)

```json
// fintool --json '<JSON>'
{"command": "quote", "symbol": "BTC"}
{"command": "news", "symbol": "AAPL"}
{"command": "report_list", "symbol": "AAPL", "limit": 5}
{"command": "report_annual", "symbol": "AAPL"}
{"command": "report_quarterly", "symbol": "AAPL"}
{"command": "report_get", "symbol": "AAPL", "accession": "0000320193-24-000123"}
```

### Hyperliquid Trading (`hyperliquid`)

```json
// hyperliquid --json '<JSON>'
{"command": "address"}
{"command": "quote", "symbol": "ETH"}
{"command": "buy", "symbol": "ETH", "amount": 0.1, "price": 3800}
{"command": "sell", "symbol": "ETH", "amount": 0.1, "price": 4000}
{"command": "orderbook", "symbol": "HYPE"}
{"command": "orderbook", "symbol": "BTC", "levels": 10}
{"command": "perp_quote", "symbol": "ETH"}
{"command": "perp_orderbook", "symbol": "BTC"}
{"command": "perp_buy", "symbol": "ETH", "amount": 0.5, "price": 3800}
{"command": "perp_sell", "symbol": "BTC", "amount": 0.01, "price": 100000}
{"command": "perp_buy", "symbol": "ETH", "amount": 0.5, "price": 3900, "close": true}
{"command": "perp_sell", "symbol": "BTC", "amount": 0.01, "price": 95000, "close": true}
{"command": "perp_leverage", "symbol": "ETH", "leverage": 5, "cross": true}
{"command": "perp_set_mode", "mode": "unified"}
{"command": "balance"}
{"command": "positions"}
{"command": "orders"}
{"command": "orders", "symbol": "BTC"}
{"command": "cancel", "order_id": "BTC:91490942"}
{"command": "deposit", "asset": "ETH"}
{"command": "deposit", "asset": "USDC", "amount": 100, "from": "base"}
{"command": "deposit", "asset": "USDC", "amount": 100, "from": "ethereum", "dry_run": true}
{"command": "withdraw", "asset": "USDC", "amount": 100}
{"command": "withdraw", "asset": "USDC", "amount": 100, "to": "base"}
{"command": "withdraw", "asset": "ETH", "amount": 0.5}
{"command": "transfer", "asset": "USDT0", "amount": 50, "from": "spot", "to": "cash"}
{"command": "transfer", "asset": "USDT0", "amount": 50, "from": "cash", "to": "spot"}
{"command": "bridge_status"}
```

### Binance Trading (`binance`)

```json
// binance --json '<JSON>'
{"command": "buy", "symbol": "BTC", "amount": 0.01, "price": 95000}
{"command": "sell", "symbol": "BTC", "amount": 0.01, "price": 100000}
{"command": "orderbook", "symbol": "BTC"}
{"command": "perp_buy", "symbol": "ETH", "amount": 0.5, "price": 3800}
{"command": "perp_sell", "symbol": "ETH", "amount": 0.5, "price": 4000, "close": true}
{"command": "perp_leverage", "symbol": "ETH", "leverage": 5}
{"command": "balance"}
{"command": "positions"}
{"command": "orders"}
{"command": "cancel", "order_id": "binance_spot:BTCUSDT:123"}
{"command": "deposit", "asset": "USDC", "from": "ethereum"}
{"command": "withdraw", "asset": "USDC", "amount": 100, "to": "0x...", "network": "ethereum"}
```

### Coinbase Trading (`coinbase`)

```json
// coinbase --json '<JSON>'
{"command": "buy", "symbol": "ETH", "amount": 0.1, "price": 3800}
{"command": "sell", "symbol": "ETH", "amount": 0.1, "price": 4000}
{"command": "orderbook", "symbol": "BTC"}
{"command": "balance"}
{"command": "orders"}
{"command": "cancel", "order_id": "coinbase:uuid-here"}
{"command": "deposit", "asset": "ETH"}
{"command": "withdraw", "asset": "USDC", "amount": 100, "to": "0x..."}
```

### Polymarket Prediction Markets (`polymarket`)

```json
// polymarket --json '<JSON>'
{"command": "list", "query": "bitcoin"}
{"command": "list", "query": "election", "limit": 5}
{"command": "list", "query": "bitcoin", "min_end_days": 7}
{"command": "list", "query": "bitcoin", "min_end_days": 0}
{"command": "quote", "market": "will-bitcoin-hit-100k"}
{"command": "buy", "market": "will-bitcoin-hit-100k", "outcome": "yes", "amount": 10, "price": 0.65}
{"command": "sell", "market": "will-bitcoin-hit-100k", "outcome": "yes", "amount": 10, "price": 0.70}
{"command": "positions"}
{"command": "balance"}
{"command": "deposit", "amount": 100, "from": "base"}
{"command": "withdraw", "amount": 50}
```

**Notes:**
- `list` defaults to `min_end_days: 3`, filtering out markets that close within 3 days (which often have odds near 1:0). Set to `0` to see all markets.
- `list` and `quote` are read-only and don't require Polymarket credentials.
- Trading commands (`buy`, `sell`, `deposit`, `withdraw`) require `wallet.private_key` in config.
- Use the market slug (from `list`) or condition ID as the `market` value.

## Workflows

### Workflow 1: Spot Trading

**Goal**: Research a symbol and place a spot trade.

**Step 1 — Quote price with analysis:**
```bash
{baseDir}/scripts/fintool --json '{"command":"quote","symbol":"BTC"}'
```
Returns: price, 24h/7d/30d changes, trend (bullish/bearish/neutral), momentum, volume analysis, summary.

**Step 1b — Check orderbook depth and spread:**
```bash
{baseDir}/scripts/hyperliquid --json '{"command":"orderbook","symbol":"BTC"}'
```
Returns: bids, asks, spread, spreadPct, midPrice. Use to assess liquidity before trading.

**Step 2 — Check recent news:**
```bash
{baseDir}/scripts/fintool --json '{"command":"news","symbol":"BTC"}'
```

**Step 3 — Place the trade:**
```bash
# Buy 0.01 BTC at max price $95000 on Hyperliquid
{baseDir}/scripts/hyperliquid --json '{"command":"buy","symbol":"BTC","amount":0.01,"price":95000}'

# Or on Binance
{baseDir}/scripts/binance --json '{"command":"buy","symbol":"BTC","amount":0.01,"price":95000}'

# Or on Coinbase
{baseDir}/scripts/coinbase --json '{"command":"buy","symbol":"BTC","amount":0.01,"price":95000}'
```

**Step 4 — Verify:**
```bash
{baseDir}/scripts/hyperliquid --json '{"command":"orders"}'
{baseDir}/scripts/hyperliquid --json '{"command":"balance"}'
```

### Workflow 2: Perpetual Futures Trading

**Goal**: Research and take a leveraged position via perpetual futures.

**Step 1 — Get perp quote with funding/OI:**
```bash
{baseDir}/scripts/hyperliquid --json '{"command":"perp_quote","symbol":"ETH"}'
```

**Step 1b — Check perp orderbook depth and spread:**
```bash
{baseDir}/scripts/hyperliquid --json '{"command":"perp_orderbook","symbol":"ETH"}'
```

**Step 2 — Check spot price for context:**
```bash
{baseDir}/scripts/fintool --json '{"command":"quote","symbol":"ETH"}'
```

**Step 3 — Check news:**
```bash
{baseDir}/scripts/fintool --json '{"command":"news","symbol":"ETH"}'
```

**Step 4 — Set leverage and place the trade:**
```bash
{baseDir}/scripts/hyperliquid --json '{"command":"perp_leverage","symbol":"ETH","leverage":5}'
{baseDir}/scripts/hyperliquid --json '{"command":"perp_buy","symbol":"ETH","amount":0.5,"price":3800}'
```

**Step 5 — Monitor:**
```bash
{baseDir}/scripts/hyperliquid --json '{"command":"positions"}'
{baseDir}/scripts/hyperliquid --json '{"command":"orders"}'
```

**Step 6 — Close a position:**
```bash
# Close a long (reduce-only)
{baseDir}/scripts/hyperliquid --json '{"command":"perp_sell","symbol":"ETH","amount":0.5,"price":4000,"close":true}'
```
Use `"close": true` to ensure the order only reduces an existing position.

**Note**: Perps are available on `hyperliquid` and `binance`. Coinbase does not support perps.

### Workflow 3: Portfolio Overview

**Goal**: Check current positions and balances.

```bash
# Hyperliquid
{baseDir}/scripts/hyperliquid --json '{"command":"balance"}'
{baseDir}/scripts/hyperliquid --json '{"command":"positions"}'
{baseDir}/scripts/hyperliquid --json '{"command":"orders"}'

# Binance
{baseDir}/scripts/binance --json '{"command":"balance"}'
{baseDir}/scripts/binance --json '{"command":"positions"}'

# Coinbase
{baseDir}/scripts/coinbase --json '{"command":"balance"}'

# Polymarket
{baseDir}/scripts/polymarket --json '{"command":"balance"}'
{baseDir}/scripts/polymarket --json '{"command":"positions"}'

# Cancel an order
{baseDir}/scripts/hyperliquid --json '{"command":"cancel","order_id":"BTC:91490942"}'
{baseDir}/scripts/binance --json '{"command":"cancel","order_id":"binance_spot:BTCUSDT:123"}'
```

### Workflow 4: Depositing Funds

**Goal**: Fund an exchange account.

**Hyperliquid — ETH/BTC/SOL (permanent deposit address via HyperUnit):**
```bash
{baseDir}/scripts/hyperliquid --json '{"command":"deposit","asset":"ETH"}'
{baseDir}/scripts/hyperliquid --json '{"command":"deposit","asset":"BTC"}'
```

**Hyperliquid — USDC from Ethereum or Base (automated bridge):**
```bash
{baseDir}/scripts/hyperliquid --json '{"command":"deposit","asset":"USDC","amount":100,"from":"base"}'
{baseDir}/scripts/hyperliquid --json '{"command":"deposit","asset":"USDC","amount":100,"from":"ethereum","dry_run":true}'
```

**Binance / Coinbase — get deposit address:**
```bash
{baseDir}/scripts/binance --json '{"command":"deposit","asset":"USDC","from":"ethereum"}'
{baseDir}/scripts/coinbase --json '{"command":"deposit","asset":"ETH"}'
```

**Polymarket — deposit USDC:**
```bash
{baseDir}/scripts/polymarket --json '{"command":"deposit","amount":100,"from":"base"}'
```

### Workflow 5: Withdrawing Funds

**Goal**: Move assets from an exchange to an external wallet.

**Hyperliquid:**
```bash
{baseDir}/scripts/hyperliquid --json '{"command":"withdraw","asset":"USDC","amount":100}'
{baseDir}/scripts/hyperliquid --json '{"command":"withdraw","asset":"USDC","amount":100,"to":"base"}'
{baseDir}/scripts/hyperliquid --json '{"command":"withdraw","asset":"ETH","amount":0.5}'
```

**Binance / Coinbase:**
```bash
{baseDir}/scripts/binance --json '{"command":"withdraw","asset":"USDC","amount":100,"to":"0x...","network":"ethereum"}'
{baseDir}/scripts/coinbase --json '{"command":"withdraw","asset":"USDC","amount":100,"to":"0x..."}'
```

**Polymarket:**
```bash
{baseDir}/scripts/polymarket --json '{"command":"withdraw","amount":50}'
```

**Track HyperUnit bridge operations:**
```bash
{baseDir}/scripts/hyperliquid --json '{"command":"bridge_status"}'
```

### Workflow 6: Prediction Market Trading (Polymarket)

**Goal**: Research and trade on prediction markets.

**Step 1 — Search for markets:**
```bash
{baseDir}/scripts/polymarket --json '{"command":"list","query":"bitcoin"}'
```
Returns: matching markets with question, outcomes, prices, volume, liquidity. By default filters out markets closing within 3 days. Use `"min_end_days": 0` to see all.

**Step 2 — Get detailed quote:**
```bash
{baseDir}/scripts/polymarket --json '{"command":"quote","market":"will-bitcoin-hit-100k"}'
```

**Step 3 — Place a trade:**
```bash
{baseDir}/scripts/polymarket --json '{"command":"buy","market":"will-bitcoin-hit-100k","outcome":"yes","amount":10,"price":0.65}'
{baseDir}/scripts/polymarket --json '{"command":"sell","market":"will-bitcoin-hit-100k","outcome":"yes","amount":10,"price":0.70}'
```

**Step 4 — Monitor positions:**
```bash
{baseDir}/scripts/polymarket --json '{"command":"positions"}'
{baseDir}/scripts/polymarket --json '{"command":"balance"}'
```

## Symbol Aliases

Common indices and commodities have convenient aliases (used with `fintool quote`):

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
- **Use the right binary** — `hyperliquid` for Hyperliquid, `binance` for Binance, etc. No `exchange` field needed.
- **All output is JSON** — Parse the response and present relevant fields to the user in a readable format.
- **Amount is in symbol units** — `"amount": 0.1` on ETH means 0.1 ETH, not $0.10. Calculate the size from the price quote.
- **Check for errors** — Every response may contain `{"error": "..."}`. Always check before presenting results.

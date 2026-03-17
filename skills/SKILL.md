---
name: fintool
description: "Financial trading CLIs — spot and perp trading on Hyperliquid, Binance, Coinbase, OKX. Prediction markets on Polymarket. Deposit and withdraw across chains. LLM-enriched price quotes with trend analysis. News and SEC filings. Historical backtesting with forward PnL analysis. Use when: user asks about stock/crypto prices, wants to trade, deposit, withdraw, check portfolio, or backtest a strategy."
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
| `okx` | OKX trading (spot, perp, deposits, withdrawals) | `{baseDir}/scripts/okx` |
| `polymarket` | Polymarket prediction markets | `{baseDir}/scripts/polymarket` |
| `backtest` | Historical simulation + forward PnL (no keys needed) | `{baseDir}/scripts/backtest` |

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
   - **OKX**: `okx_api_key` + `okx_secret_key` + `okx_passphrase` in `[api_keys]` (spot + perps)
   - **Polymarket**: Uses `wallet.private_key` (same as Hyperliquid) for prediction market trading
   - If none configured: Ask the user which exchange they want to use and request the credentials.

**Exception — `backtest`**: The backtest binary requires **no configuration** — no API keys, no wallet. It uses public Yahoo Finance and CoinGecko data. You can use it immediately.

**If the user provides credentials**, edit `~/.fintool/config.toml` directly to add them.

## Exchange Capabilities

| Feature | `hyperliquid` | `binance` | `coinbase` | `okx` | `polymarket` | `backtest` |
|---------|---------------|-----------|------------|-------|--------------|------------|
| Spot orders | buy, sell | buy, sell | buy, sell | buy, sell | — | simulated buy/sell |
| Perp orders | perp buy/sell | perp buy/sell | — | perp buy/sell | — | simulated perp buy/sell |
| Prediction markets | — | — | — | — | buy, sell, list, quote | — |
| Orderbook | spot + perp | spot + perp | spot | spot + perp | — | — |
| Deposit | Unit + Across | API | API | API | bridge | — |
| Withdraw | Bridge2 + Unit + Across | API | API | API | bridge | — |
| Transfer | spot ↔ perp ↔ dex | spot ↔ futures | — | funding ↔ trading | — | — |
| Balance | balance | balance | balance | balance | balance | simulated |
| Open orders | orders | orders | orders | orders | — | — |
| Cancel | cancel | cancel | cancel | cancel | — | — |
| Positions | positions | positions | — | positions | positions | simulated |
| Funding rate | — | — | — | perp funding_rate | — | — |
| Historical quote | — | — | — | — | — | quote |
| Forward PnL | — | — | — | — | — | +1d/+2d/+4d/+7d |
| SEC filings (dated) | — | — | — | — | — | report list/annual/quarterly |

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
{"command": "buy", "symbol": "ETH", "amount": 0.1, "price": 3800}
{"command": "sell", "symbol": "ETH", "amount": 0.1, "price": 4000}
{"command": "orderbook", "symbol": "HYPE"}
{"command": "orderbook", "symbol": "BTC", "levels": 10}
{"command": "quote", "symbol": "ETH"}
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
{"command": "deposit", "asset": "ETH", "amount": 0.01}
{"command": "deposit", "asset": "ETH", "amount": 0.05, "dry_run": true}
{"command": "deposit", "asset": "BTC", "amount": 0.001}
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

### OKX Trading (`okx`)

```json
// okx --json '<JSON>'
{"command": "quote", "symbol": "BTC"}
{"command": "buy", "symbol": "ETH", "amount": 0.01, "price": 2000}
{"command": "sell", "symbol": "ETH", "amount": 0.01, "price": 2100}
{"command": "orderbook", "symbol": "BTC"}
{"command": "perp_orderbook", "symbol": "ETH"}
{"command": "perp_buy", "symbol": "ETH", "amount": 0.1, "price": 2000}
{"command": "perp_sell", "symbol": "ETH", "amount": 0.1, "price": 2100, "close": true}
{"command": "perp_leverage", "symbol": "ETH", "leverage": 5, "cross": true}
{"command": "perp_funding_rate", "symbol": "BTC"}
{"command": "balance"}
{"command": "positions"}
{"command": "orders"}
{"command": "cancel", "inst_id": "BTC-USDT", "order_id": "12345"}
{"command": "deposit", "asset": "USDC", "network": "base"}
{"command": "deposit", "asset": "ETH", "network": "ethereum"}
{"command": "withdraw", "asset": "USDC", "amount": 100, "network": "base"}
{"command": "withdraw", "asset": "USDC", "amount": 100, "to": "0x...", "network": "ethereum"}
{"command": "transfer", "asset": "USDT", "amount": 100, "from": "funding", "to": "trading"}
{"command": "transfer", "asset": "USDT", "amount": 100, "from": "trading", "to": "funding"}
```

**OKX Notes:**
- `quote` and `orderbook` are public endpoints — no API keys needed.
- OKX uses instrument IDs: spot = `BTC-USDT`, perp = `BTC-USDT-SWAP`. The CLI auto-formats from symbol.
- OKX has `funding` (for deposits/withdrawals) and `trading` (unified) accounts. Use `transfer` to move between them.
- Withdrawal fee is auto-fetched if `--fee` is not specified.
- Cancel requires both `inst_id` (e.g. `BTC-USDT`) and `order_id`.

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

### Backtesting (`backtest`)

**Important**: The `backtest` binary requires `--at YYYY-MM-DD` as a CLI flag (not in the JSON body). No API keys or wallet needed.

```bash
# backtest --at <DATE> --json '<JSON>'
```

```json
// Historical price
{"command": "quote", "symbol": "BTC"}
{"command": "quote", "symbol": "AAPL"}
{"command": "quote", "symbol": "GOLD"}

// Simulated spot trades — returns forward PnL at +1/+2/+4/+7 days
{"command": "buy", "symbol": "ETH", "amount": 0.5}
{"command": "buy", "symbol": "AAPL", "amount": 10, "price": 237}
{"command": "sell", "symbol": "BTC", "amount": 0.01, "price": 105000}

// Simulated perp trades — returns leveraged forward PnL
{"command": "perp_leverage", "symbol": "ETH", "leverage": 5}
{"command": "perp_buy", "symbol": "ETH", "amount": 0.5, "price": 3300}
{"command": "perp_sell", "symbol": "BTC", "amount": 0.01, "price": 100000}

// SEC filings filtered by date
{"command": "report_list", "symbol": "AAPL", "limit": 5}
{"command": "report_annual", "symbol": "TSLA"}
{"command": "report_quarterly", "symbol": "AAPL"}

// Portfolio management
{"command": "balance"}
{"command": "positions"}
{"command": "reset"}

// News stub (historical news not available)
{"command": "news", "symbol": "BTC"}
```

**Notes:**
- If `price` is omitted on buy/sell, the historical close price at the `--at` date is used automatically.
- Portfolio state persists to `~/.fintool/backtest_portfolio.json`. Use `reset` to clear.
- `balance` returns `cashBalance` (spot trades only), `positions`, `totalTrades`, `leverageSettings`.
- Trade output includes a `pnl` array with forward price, dollar PnL, and percentage PnL at each offset.
- Data sources: Yahoo Finance (stocks, crypto, commodities, indices) with CoinGecko fallback for crypto.

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

# Or on OKX
{baseDir}/scripts/okx --json '{"command":"buy","symbol":"BTC","amount":0.01,"price":95000}'
```

**Step 4 — Verify:**
```bash
{baseDir}/scripts/hyperliquid --json '{"command":"orders"}'
{baseDir}/scripts/hyperliquid --json '{"command":"balance"}'
```

### Workflow 2: Perpetual Futures Trading

**Goal**: Research and take a leveraged position via perpetual futures.

**Step 1 — Get quote with funding/OI:**
```bash
{baseDir}/scripts/hyperliquid --json '{"command":"quote","symbol":"ETH"}'
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

**Note**: Perps are available on `hyperliquid`, `binance`, and `okx`. Coinbase does not support perps.

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

# OKX
{baseDir}/scripts/okx --json '{"command":"balance"}'
{baseDir}/scripts/okx --json '{"command":"positions"}'
{baseDir}/scripts/okx --json '{"command":"orders"}'

# Polymarket
{baseDir}/scripts/polymarket --json '{"command":"balance"}'
{baseDir}/scripts/polymarket --json '{"command":"positions"}'

# Cancel an order
{baseDir}/scripts/hyperliquid --json '{"command":"cancel","order_id":"BTC:91490942"}'
{baseDir}/scripts/binance --json '{"command":"cancel","order_id":"binance_spot:BTCUSDT:123"}'
```

### Workflow 4: Depositing Funds

**Goal**: Fund an exchange account.

**Hyperliquid — ETH (auto-bridge via Unit):**
```bash
{baseDir}/scripts/hyperliquid --json '{"command":"deposit","asset":"ETH","amount":0.01}'
{baseDir}/scripts/hyperliquid --json '{"command":"deposit","asset":"ETH","amount":0.05,"dry_run":true}'
```
Sends ETH from your wallet on Ethereum L1 to a Unit bridge deposit address. Minimum: 0.007 ETH.

**Hyperliquid — USDC from Ethereum or Base (auto-bridge via Across):**
```bash
{baseDir}/scripts/hyperliquid --json '{"command":"deposit","asset":"USDC","amount":100,"from":"base"}'
{baseDir}/scripts/hyperliquid --json '{"command":"deposit","asset":"USDC","amount":100,"from":"ethereum","dry_run":true}'
```

**Hyperliquid — BTC/SOL (manual — shows deposit address):**
```bash
{baseDir}/scripts/hyperliquid --json '{"command":"deposit","asset":"BTC","amount":0.001}'
```
BTC and SOL cannot be bridged automatically. The command returns a Unit deposit address for manual transfer.

**Binance / Coinbase / OKX — get deposit address:**
```bash
{baseDir}/scripts/binance --json '{"command":"deposit","asset":"USDC","from":"ethereum"}'
{baseDir}/scripts/coinbase --json '{"command":"deposit","asset":"ETH"}'
{baseDir}/scripts/okx --json '{"command":"deposit","asset":"USDC","network":"base"}'
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

**Binance / Coinbase / OKX:**
```bash
{baseDir}/scripts/binance --json '{"command":"withdraw","asset":"USDC","amount":100,"to":"0x...","network":"ethereum"}'
{baseDir}/scripts/coinbase --json '{"command":"withdraw","asset":"USDC","amount":100,"to":"0x..."}'
{baseDir}/scripts/okx --json '{"command":"withdraw","asset":"USDC","amount":100,"network":"base"}'
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

### Workflow 7: Backtesting a Trading Strategy

**Goal**: Develop a thesis, simulate trades at historical dates, and evaluate forward PnL before live trading.

**Use backtest when** the agent is developing a strategy, validating a thesis with historical data, or the user asks "what if I had bought X on date Y?"

**Step 1 — Develop a thesis using current data:**
```bash
{baseDir}/scripts/fintool --json '{"command":"news","symbol":"BTC"}'
{baseDir}/scripts/fintool --json '{"command":"report_list","symbol":"AAPL","limit":5}'
```
Use news and SEC filings to identify a catalyst or thesis (e.g., "NVDA earnings blowout", "oil supply shock").

**Step 2 — Reset the backtest portfolio:**
```bash
{baseDir}/scripts/backtest --at 2025-01-15 --json '{"command":"reset"}'
```

**Step 3 — Scout historical prices:**
```bash
{baseDir}/scripts/backtest --at 2025-01-15 --json '{"command":"quote","symbol":"BTC"}'
{baseDir}/scripts/backtest --at 2025-01-15 --json '{"command":"quote","symbol":"GOLD"}'
```

**Step 4 — Execute simulated trades:**
```bash
# Spot buy (auto-price from historical close)
{baseDir}/scripts/backtest --at 2025-01-15 --json '{"command":"buy","symbol":"BTC","amount":0.01}'

# Spot short
{baseDir}/scripts/backtest --at 2025-01-15 --json '{"command":"sell","symbol":"SP500","amount":1.5,"price":5900}'

# Leveraged perp
{baseDir}/scripts/backtest --at 2025-01-15 --json '{"command":"perp_leverage","symbol":"ETH","leverage":5}'
{baseDir}/scripts/backtest --at 2025-01-15 --json '{"command":"perp_buy","symbol":"ETH","amount":0.5,"price":3300}'
```
Each trade returns forward PnL at +1, +2, +4, +7 days — review these to evaluate the thesis.

**Step 5 — Review portfolio state:**
```bash
{baseDir}/scripts/backtest --at 2025-01-15 --json '{"command":"balance"}'
{baseDir}/scripts/backtest --at 2025-01-15 --json '{"command":"positions"}'
```

**Step 6 — Iterate:**
- Try different entry dates to test timing sensitivity
- Adjust position sizes for risk management
- Test multi-leg strategies (e.g., long one asset + short another)
- Check SEC filings before the trade date for fundamental context:
  ```bash
  {baseDir}/scripts/backtest --at 2024-06-01 --json '{"command":"report_list","symbol":"AAPL","limit":3}'
  ```

**Step 7 — If the backtest validates the thesis, proceed to live trading** using the appropriate exchange binary (hyperliquid, binance, etc.).

## Symbol Aliases

Common indices and commodities have convenient aliases (used with `fintool quote` and `backtest quote`):

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
- **Backtest before trading** — When developing a thesis or strategy, use `backtest` to simulate at historical dates and validate with forward PnL before committing real capital.
- **No config for backtest** — The `backtest` binary needs no API keys or wallet. Use it freely for research and strategy development.

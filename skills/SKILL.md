---
name: fintool
description: "Financial trading CLI — spot/perp/options trading on Hyperliquid, Binance, Coinbase. LLM-enriched price quotes with trend analysis. Prediction markets (Polymarket + Kalshi). News and SEC filings. Use when: user asks about stock/crypto prices, wants to trade, check portfolio, or browse prediction markets."
homepage: https://github.com/second-state/fintool
metadata: { "openclaw": { "emoji": "📈", "requires": { "bins": ["curl"] } } }
---

# fintool — Financial Trading Skill

A CLI tool for market intelligence and trading across multiple exchanges.

## Tool

- **Binary**: `{baseDir}/scripts/fintool`
- **Config**: `~/.fintool/config.toml`
- **Output**: JSON by default, `--human` for colored terminal output

## Setup Check (MANDATORY — do this FIRST)

Before running any command, verify the user's configuration:

```bash
cat ~/.fintool/config.toml 2>/dev/null
```

**If the file doesn't exist**, run:
```bash
{baseDir}/scripts/fintool init
```

**Check for these requirements:**

1. **OpenAI API key** — Required for enriched quotes (trend/momentum analysis).
   - Look for: `openai_api_key = "sk-..."` (uncommented) in `[api_keys]`
   - If missing: Ask the user for their OpenAI API key, or tell them to add it to `~/.fintool/config.toml` under `[api_keys]`

2. **At least one exchange** — Required for trading commands.
   - **Hyperliquid**: `private_key` or `wallet_json` + `wallet_passcode` in `[wallet]` (spot + perps)
   - **Binance**: `binance_api_key` + `binance_api_secret` in `[api_keys]` (spot + perps + options)
   - **Coinbase**: `coinbase_api_key` + `coinbase_api_secret` in `[api_keys]` (spot only)
   - If none configured: Ask the user which exchange they want to use and request the credentials.

**If the user provides credentials**, edit `~/.fintool/config.toml` directly to add them.

## Exchange Capabilities

| Feature | Hyperliquid | Binance | Coinbase |
|---------|-------------|---------|----------|
| Spot orders | ✅ | ✅ | ✅ |
| Perp orders | ✅ | ✅ | ❌ |
| Options | ❌ | ✅ | ❌ |
| Balance | ✅ | ✅ | ✅ |
| Open orders | ✅ | ✅ | ✅ |
| Cancel | ✅ | ✅ | ✅ |
| Positions | ✅ | ✅ | ❌ |

**Auto exchange priority**: Hyperliquid > Coinbase > Binance for spot/perp. Binance-only for options.

Use `--exchange hyperliquid|binance|coinbase` to force a specific exchange.

## Error Handling

- If a command returns an **exchange error**, suggest the user try a different exchange with `--exchange <name>`.
- If the selected exchange is **not configured**, tell the user which credentials are needed and offer to add them to config.
- If a trading command fails with **insufficient balance** or **invalid symbol**, relay the error clearly.

## Workflows

### Workflow 1: Spot Trading

**Goal**: Research a symbol and place a spot trade.

**Step 1 — Quote price with analysis:**
```bash
{baseDir}/scripts/fintool quote <SYMBOL>
```
Returns: price, 24h/7d/30d changes, trend (bullish/bearish/neutral), momentum, volume analysis, summary. Uses data from Hyperliquid + Yahoo Finance + CoinGecko, merged by OpenAI.

**Step 2 — Check recent news:**
```bash
{baseDir}/scripts/fintool news <SYMBOL>
```
Returns: up to 10 recent headlines from Google News RSS.

**Step 3 — Place the trade:**
```bash
# Buy: spend $<AMOUNT> at max price $<PRICE>
{baseDir}/scripts/fintool order buy <SYMBOL> <AMOUNT_USDC> <MAX_PRICE>

# Sell: sell <AMOUNT> units at min price $<PRICE>
{baseDir}/scripts/fintool order sell <SYMBOL> <AMOUNT> <MIN_PRICE>

# Force a specific exchange:
{baseDir}/scripts/fintool order buy <SYMBOL> <AMOUNT> <PRICE> --exchange binance
```

**Step 4 — Verify:**
```bash
{baseDir}/scripts/fintool orders
{baseDir}/scripts/fintool balance
```

### Workflow 2: Perpetual Futures Trading

**Goal**: Research and take a leveraged position via perpetual futures.

**Step 1 — Get perp quote with funding/OI:**
```bash
{baseDir}/scripts/fintool perp quote <SYMBOL>
```
Returns: mark price, oracle price, funding rate, open interest, premium, max leverage.

**Step 2 — Check spot price for context:**
```bash
{baseDir}/scripts/fintool quote <SYMBOL>
```

**Step 3 — Check news:**
```bash
{baseDir}/scripts/fintool news <SYMBOL>
```

**Step 4 — Place the trade:**
```bash
# Long: spend $<AMOUNT> at limit price $<PRICE>
{baseDir}/scripts/fintool perp buy <SYMBOL> <AMOUNT_USDC> <PRICE>

# Short: sell <SIZE> units at limit price $<PRICE>
{baseDir}/scripts/fintool perp sell <SYMBOL> <SIZE> <PRICE>
```

**Step 5 — Monitor:**
```bash
{baseDir}/scripts/fintool positions
{baseDir}/scripts/fintool orders
```

**Note**: Perps only available on Hyperliquid and Binance. If the user only has Coinbase configured, tell them perps are not supported on Coinbase.

### Workflow 3: Options Trading

**Goal**: Buy or sell options contracts.

**IMPORTANT**: Options only work on Binance. If user tries with Hyperliquid or Coinbase, explain this and ask them to configure Binance credentials.

**Step 1 — Research the underlying:**
```bash
{baseDir}/scripts/fintool quote <SYMBOL>
{baseDir}/scripts/fintool news <SYMBOL>
```

**Step 2 — Place options trade:**
```bash
# Buy a call option
{baseDir}/scripts/fintool options buy <SYMBOL> call <STRIKE> <EXPIRY> <SIZE>

# Buy a put option
{baseDir}/scripts/fintool options buy <SYMBOL> put <STRIKE> <EXPIRY> <SIZE>

# Sell a call option
{baseDir}/scripts/fintool options sell <SYMBOL> call <STRIKE> <EXPIRY> <SIZE>
```

- `EXPIRY` format: `YYYY-MM-DD` (e.g., `2026-03-28`)
- Binance converts to: `BTC-260328-80000-C` internally

### Workflow 4: Prediction Markets

**Goal**: Browse and trade on prediction markets (Polymarket + Kalshi).

**Step 1 — Browse markets:**
```bash
{baseDir}/scripts/fintool predict list
{baseDir}/scripts/fintool predict search "election"
{baseDir}/scripts/fintool predict search "BTC" --platform kalshi
```

**Step 2 — Get detailed quote:**
```bash
{baseDir}/scripts/fintool predict quote kalshi:TICKER
{baseDir}/scripts/fintool predict quote polymarket:slug
```

**Step 3 — Trade (stub — requires platform-specific auth):**
```bash
{baseDir}/scripts/fintool predict buy kalshi:TICKER yes 10
```

### Workflow 5: Portfolio Overview

**Goal**: Check current positions and balances across exchanges.

```bash
# Account balance
{baseDir}/scripts/fintool balance
{baseDir}/scripts/fintool balance --exchange binance

# Open positions (perps)
{baseDir}/scripts/fintool positions

# Open orders
{baseDir}/scripts/fintool orders
{baseDir}/scripts/fintool orders BTC

# Cancel an order
{baseDir}/scripts/fintool cancel BTC:91490942              # Hyperliquid
{baseDir}/scripts/fintool cancel binance_spot:BTCUSDT:123   # Binance spot
{baseDir}/scripts/fintool cancel binance_futures:BTCUSDT:456 # Binance futures
{baseDir}/scripts/fintool cancel coinbase:uuid-here          # Coinbase
```

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
- **Use `--exchange` when ambiguous** — If the user has multiple exchanges, explicitly select one to avoid confusion.
- **JSON output is default** — Parse it programmatically. Use `--human` only when showing to the user in terminal.

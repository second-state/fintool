# fintool

A Rust CLI for financial trading and market intelligence — spot and perpetual futures on **Hyperliquid**, **Unit**, **Binance**, and **Coinbase**, stock quotes, LLM-enriched analysis, prediction markets on **Polymarket** and **Kalshi**, SEC filings, and news.

## Install as an OpenClaw Skill

Tell your [OpenClaw](https://openclaw.ai) agent:

> Read https://raw.githubusercontent.com/second-state/fintool/refs/heads/main/skills/install.md and install the fintool skill.

The agent will download the correct binary for your platform, set up the skill, and walk you through configuration.

## Installation (Manual)

```bash
cd fintool
cargo build --release
# Binary at ./target/release/fintool
```

Or download a pre-built binary from [Releases](https://github.com/second-state/fintool/releases).

## Quick Start

```bash
# Create config file
fintool init

# Edit config with your keys
vim ~/.fintool/config.toml

# Get an enriched quote with trend analysis
fintool quote BTC
fintool quote AAPL --human

# Index and commodity aliases
fintool quote SP500
fintool quote GOLD
fintool quote VIX

# Perp quotes with funding/OI
fintool perp quote BTC

# News
fintool news ETH

# SEC filings
fintool report annual AAPL
fintool report list TSLA

# Prediction markets
fintool predict list
fintool predict search "election"

# Spot trading (auto-selects exchange)
fintool order buy TSLA 100 410
fintool order sell TSLA 1 420

# Force specific exchange
fintool order buy BTC 100 65000 --exchange coinbase
fintool order buy BTC 100 65000 --exchange binance

# Perp trading
fintool perp buy BTC 100 65000
fintool perp sell BTC 0.01 70000

# Options trading (Binance only)
fintool options buy BTC call 70000 260328 0.1 --exchange binance
```

## Output Modes

**JSON (default):** Machine-readable output for scripting and piping.

```bash
fintool quote BTC
fintool quote BTC | jq '.price'
```

**Human-friendly:** Colored, formatted terminal output.

```bash
fintool quote BTC --human
fintool --human balance
```

The `--human` flag is global and works with any subcommand.

---

## Exchange Support

`fintool` supports three exchanges with automatic routing: **Hyperliquid**, **Binance**, and **Coinbase**.

### Exchange Capability Matrix

| Feature | Hyperliquid | Binance | Coinbase |
|---------|-------------|---------|----------|
| Spot Trading | ✅ | ✅ | ✅ |
| Perpetual Futures | ✅ | ✅ | ❌ |
| Options | ❌ | ✅ | ❌ |
| Balance/Positions | ✅ | ✅ | ✅ |
| Orders/Cancellation | ✅ | ✅ | ✅ |

### Global Exchange Flag

All trading commands support `--exchange <EXCHANGE>`:

| Value | Behavior |
|-------|----------|
| `auto` (default) | Auto-select based on configured exchanges and command type |
| `hyperliquid` | Force Hyperliquid (requires wallet config) |
| `binance` | Force Binance (requires API keys) |
| `coinbase` | Force Coinbase (requires API keys) |

### Auto Mode Routing

When `--exchange auto` (default):

1. **Options commands** → Always Binance (only exchange that supports options)
2. **Perpetual futures** → Hyperliquid > Binance (Coinbase doesn't support perps)
3. **Spot trading** → Hyperliquid > Coinbase > Binance (priority order)
4. **If only one exchange configured** → Use that one

### Symbol Formats by Exchange

| Exchange | Spot Format | Perp Format | Notes |
|----------|-------------|-------------|-------|
| Hyperliquid | `BTC`, `TSLA` | `BTC`, `ETH` | Symbol only, no pair suffix |
| Binance | `BTCUSDT` | `BTCUSDT` | Auto-appends USDT in code |
| Coinbase | `BTC-USD` | N/A | Dash-separated, USD quote |

**Note:** `fintool` handles format conversion automatically. Just use the base symbol (e.g., `BTC`) and it will convert to the right format for each exchange.

### Examples

```bash
# Auto routing (uses configured exchange with priority)
fintool order buy BTC 100 65000

# Force Hyperliquid
fintool order buy BTC 100 65000 --exchange hyperliquid

# Force Binance
fintool order buy BTC 100 65000 --exchange binance

# Force Coinbase (uses BTC-USD internally)
fintool order buy BTC 100 65000 --exchange coinbase

# Options require Binance
fintool options buy BTC call 70000 260328 0.1 --exchange binance
```

---

## Configuration

Config file: `~/.fintool/config.toml`

Run `fintool init` to generate a template, or copy `config.toml.default` from the release zip.

### Example Configuration (All Three Exchanges)

```toml
[wallet]
# Hyperliquid private key (hex, with or without 0x prefix)
private_key = "0xabcdef1234567890..."

# Alternative: encrypted keystore file
# wallet_json = "/path/to/wallet.json"
# wallet_passcode = "your-passcode"

[network]
testnet = false

[api_keys]
# OpenAI — enables LLM-enriched quote analysis (trend, momentum, summary)
openai_api_key = "sk-..."
openai_model = "gpt-4.1-mini"

# Binance — enables spot/futures/options trading
binance_api_key = "..."
binance_api_secret = "..."

# Coinbase Advanced Trade — enables spot trading
coinbase_api_key = "..."
coinbase_api_secret = "..."

# Kalshi — prediction market trading
# kalshi_api_key = "..."
# kalshi_api_secret = "..."
```

### Config Options

| Section | Key | Type | Default | Description |
|---------|-----|------|---------|-------------|
| `wallet` | `private_key` | string | — | Hyperliquid wallet hex private key (with or without `0x`). **Takes priority** over keystore. |
| `wallet` | `wallet_json` | string | — | Path to encrypted Ethereum keystore JSON file. Supports `~` expansion. |
| `wallet` | `wallet_passcode` | string | — | Passcode to decrypt the keystore file. |
| `network` | `testnet` | bool | `false` | Use Hyperliquid testnet. |
| `api_keys` | `openai_api_key` | string | — | OpenAI API key. Enables LLM-enriched quotes with trend/momentum analysis. |
| `api_keys` | `openai_model` | string | `gpt-4.1-mini` | OpenAI model for quote analysis. Any chat completions model works. |
| `api_keys` | `binance_api_key` | string | — | Binance API key for spot/futures/options trading. |
| `api_keys` | `binance_api_secret` | string | — | Binance API secret (HMAC-SHA256 signing). |
| `api_keys` | `coinbase_api_key` | string | — | Coinbase Advanced Trade API key. |
| `api_keys` | `coinbase_api_secret` | string | — | Coinbase Advanced Trade API secret (HMAC-SHA256 signing). |
| `api_keys` | `kalshi_api_key` | string | — | Kalshi API key (for prediction market trading). |
| `api_keys` | `kalshi_api_secret` | string | — | Kalshi API secret. |

### What Needs Configuration

| Command | Hyperliquid Wallet | Binance Keys | Coinbase Keys | OpenAI Key | Exchange Support |
|---------|-------------------|--------------|---------------|------------|------------------|
| `quote` | No | No | No | Optional (enriches) | N/A (read-only) |
| `perp quote` | No | No | No | No | N/A (read-only) |
| `news`, `init` | No | No | No | No | N/A |
| `report` | No | No | No | No | N/A |
| `predict list/search/quote` | No | No | No | No | N/A |
| `order buy/sell` | Yes (HL) | Yes (Binance) | Yes (Coinbase) | No | All three |
| `perp buy/sell` | Yes (HL) | Yes (Binance) | No | No | HL + Binance |
| `orders` | Yes (HL) | Yes (Binance) | Yes (Coinbase) | No | All three |
| `cancel` | Yes (HL) | Yes (Binance) | Yes (Coinbase) | No | All three |
| `balance` | Yes (HL) | Yes (Binance) | Yes (Coinbase) | No | All three |
| `positions` | Yes (HL) | Yes (Binance) | Yes (Coinbase) | No | All three |
| `options buy/sell` | No | Yes (Binance) | No | No | Binance only |
| `predict buy/sell` | Yes (HL) | No | No | No | Polymarket/Kalshi |
| `deposit` (HL) | Yes | No | No | No | Hyperliquid |
| `deposit` (Binance) | No | Yes | No | No | Binance |
| `deposit` (Coinbase) | No | No | Yes | No | Coinbase |
| `withdraw` (HL) | Yes | No | No | No | Hyperliquid |
| `withdraw` (Binance) | No | Yes | No | No | Binance |
| `withdraw` (Coinbase) | No | No | Yes | No | Coinbase |
| `bridge-status` | Yes | No | No | No | Hyperliquid |

---

## Commands

### `fintool init`

Create a default config file at `~/.fintool/config.toml`.

```bash
fintool init
```

---

### `fintool quote <SYMBOL>`

Get the current price with multi-source aggregation and optional LLM analysis.

**Data sources** (fetched in parallel):
1. **Hyperliquid spot** — tokenized stocks and crypto
2. **Yahoo Finance** — traditional stocks, indices, commodities
3. **CoinGecko** — crypto prices with 7d/30d trends, market cap

**With OpenAI key configured:** All raw data is sent to the LLM to produce merged analysis with trend direction, momentum, volume context, and a market summary.

**Without OpenAI key:** Returns merged data from the best available source.

#### Symbol Aliases

Common indices, commodities, and treasuries are aliased for convenience:

| Alias | Resolves To | Description |
|-------|-------------|-------------|
| `SP500`, `SPX` | `^GSPC` | S&P 500 |
| `NASDAQ`, `NDX` | `^IXIC`, `^NDX` | Nasdaq Composite / 100 |
| `DOW`, `DJI`, `DJIA` | `^DJI` | Dow Jones |
| `RUSSELL`, `RUT` | `^RUT` | Russell 2000 |
| `VIX` | `^VIX` | CBOE Volatility Index |
| `NIKKEI` | `^N225` | Nikkei 225 |
| `FTSE` | `^FTSE` | FTSE 100 |
| `DAX` | `^GDAXI` | DAX |
| `HSI`, `HANGSENG` | `^HSI` | Hang Seng |
| `GOLD` | `GC=F` | Gold Futures |
| `SILVER` | `SI=F` | Silver Futures |
| `OIL`, `CRUDE` | `CL=F` | Crude Oil Futures |
| `NATGAS` | `NG=F` | Natural Gas Futures |
| `10Y`, `TNX` | `^TNX` | 10-Year Treasury Yield |
| `30Y`, `TYX` | `^TYX` | 30-Year Treasury Yield |

#### Examples

```bash
fintool quote BTC          # crypto — Hyperliquid + CoinGecko + Yahoo
fintool quote AAPL         # stock — Yahoo Finance
fintool quote SP500        # index alias
fintool quote GOLD         # commodity alias
fintool quote USD1         # stablecoin
fintool quote ETH --human  # colored terminal output
```

#### JSON Schema — Enriched (with OpenAI)

```json
{
  "symbol": "BTC",
  "name": "Bitcoin",
  "price": 64700.0,
  "price_currency": "USD",
  "change_24h_pct": -4.28,
  "change_7d_pct": -5.97,
  "change_30d_pct": -27.54,
  "volume_24h": 56772864157.0,
  "market_cap": 1291834932905.0,
  "trend": "bearish",
  "trend_strength": "strong",
  "momentum": "Bitcoin has declined 4.28% in the last 24 hours and nearly 6% over the past week, indicating sustained bearish pressure.",
  "volume_analysis": "The 24-hour volume of $56.8B indicates significant market activity despite the downturn.",
  "summary": "Bitcoin is in a strong bearish trend with a 27.5% decline over the past month. High trading volumes suggest active selling pressure rather than low-liquidity drift.",
  "sources_used": ["Yahoo Finance", "CoinGecko"],
  "confidence": "high"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `symbol` | string | Asset symbol |
| `name` | string | Full asset name |
| `price` | number | Best available current price (USD) |
| `price_currency` | string | Always `"USD"` |
| `change_24h_pct` | number\|null | 24-hour price change (%) |
| `change_7d_pct` | number\|null | 7-day price change (%) — crypto only via CoinGecko |
| `change_30d_pct` | number\|null | 30-day price change (%) — crypto only via CoinGecko |
| `volume_24h` | number\|null | 24-hour trading volume (USD) |
| `market_cap` | number\|null | Market capitalization (USD) |
| `trend` | string | `"bullish"`, `"bearish"`, or `"neutral"` |
| `trend_strength` | string | `"strong"`, `"moderate"`, or `"weak"` |
| `momentum` | string | 1-2 sentence momentum analysis |
| `volume_analysis` | string | 1 sentence volume context |
| `summary` | string | 2-3 sentence market overview |
| `sources_used` | array | Data sources that contributed to the analysis |
| `confidence` | string | `"high"`, `"medium"`, or `"low"` |

#### JSON Schema — Basic (without OpenAI)

When no OpenAI key is configured, returns merged raw data:

```json
{
  "symbol": "AAPL",
  "price": "266.18",
  "change24h": "4.07",
  "volume24h": 34926242.0,
  "sources_used": ["Yahoo Finance"]
}
```

#### Human Output Example

```
  📊 BTC (Bitcoin)
  Price:      $64,683.00
  24h Change: -4.31%

  📉 Trend:  bearish (strong)

  💫 Momentum: Bitcoin has declined over 4% in the last 24 hours and
     nearly 6% over the past week, indicating sustained bearish pressure.
  📊 Volume:   The 24-hour volume of $56.8B indicates significant activity.

  📝 Summary:
     Bitcoin is in a strong bearish trend with a 27.5% decline over
     the past month. High volumes suggest active selling pressure.

  Sources: Yahoo Finance, CoinGecko | Confidence: high
```

---

### `fintool perp quote <SYMBOL>`

Get the current **perpetual futures** price with funding rate, open interest, premium, and leverage info.

**Supported exchanges:** Hyperliquid (default, including HIP-3 dexes), Binance (via `--exchange binance`)

Fintool automatically searches across Hyperliquid's main perp universe and HIP-3 builder-deployed dexes (like `cash`/dreamcash) to find the most liquid market for any symbol.

#### Examples

```bash
# Crypto perps (main HL dex)
fintool perp quote BTC
fintool perp quote ETH
fintool perp quote SOL --human

# Commodity perps (HIP-3 cash dex)
fintool perp quote SILVER        # silver ~$89/oz, 20x leverage
fintool perp quote GOLD          # gold ~$5,184/oz, 20x leverage

# Stock perps (HIP-3 cash dex)
fintool perp quote TSLA          # Tesla stock perp
fintool perp quote NVDA          # NVIDIA stock perp
fintool perp quote GOOGL         # Alphabet stock perp

# US index perps (HIP-3 cash dex)
fintool perp quote USA500        # S&P 500 index perp

# Binance
fintool perp quote BTC --exchange binance
```

#### Available HIP-3 Assets (cash dex)

| Category | Symbols |
|----------|---------|
| Commodities | `SILVER`, `GOLD` |
| US Stocks | `TSLA`, `NVDA`, `GOOGL`, `AMZN`, `MSFT`, `META`, `INTC`, `HOOD` |
| Indices | `USA500` (S&P 500) |

Aliases: `XAG` → SILVER, `XAU` → GOLD, `SP500`/`SPX` → USA500

#### JSON Schema (main perps)

```json
{
  "symbol": "BTC",
  "markPx": "65785.0",
  "oraclePx": "65825.0",
  "change24h": "-3.27",
  "funding": "0.0000083791",
  "premium": "-0.0003797949",
  "openInterest": "21206.37062",
  "volume24h": "1862590179.97",
  "prevDayPx": "68011.0",
  "maxLeverage": 40,
  "source": "Hyperliquid"
}
```

#### JSON Schema (HIP-3 perps)

```json
{
  "symbol": "SILVER",
  "hip3Asset": "cash:SILVER",
  "dex": "cash",
  "markPx": "89.478",
  "oraclePx": "89.425",
  "change24h": "2.83",
  "funding": "0.0000079076",
  "premium": "0.0005093654",
  "openInterest": "37801.7",
  "volume24h": "37711686.30",
  "prevDayPx": "87.015",
  "maxLeverage": 20,
  "source": "Hyperliquid HIP-3 (cash)"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `symbol` | string | User-facing symbol |
| `hip3Asset` | string | HIP-3 dex-qualified asset name (e.g. `cash:SILVER`) |
| `dex` | string | HIP-3 dex name (e.g. `cash`) |
| `markPx` | string | Current mark price (USD) |
| `oraclePx` | string | Oracle price (USD) |
| `change24h` | string | 24-hour price change (%) |
| `funding` | string | Current funding rate (per 8h) |
| `premium` | string | Mark-oracle premium |
| `openInterest` | string | Open interest in asset units |
| `volume24h` | string | 24-hour notional volume (USD) |
| `prevDayPx` | string | Previous day price (USD) |
| `maxLeverage` | number | Maximum allowed leverage |
| `source` | string | `"Hyperliquid"`, `"Hyperliquid HIP-3 (cash)"`, or `"Binance"` |

---

### `fintool news <SYMBOL>`

Get the latest news headlines via Google News RSS.

#### Examples

```bash
fintool news ETH
fintool news TSLA
fintool news AAPL --human
```

#### JSON Schema

Returns an array of up to 10 articles:

```json
[
  {
    "title": "Bitcoin Surges Past $70K",
    "source": "CoinDesk",
    "url": "https://news.google.com/rss/articles/...",
    "published": "Mon, 23 Feb 2026 04:51:05 GMT"
  }
]
```

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Article headline |
| `source` | string | Publisher name |
| `url` | string | Article URL |
| `published` | string | Publication date (RFC 2822) |

---

### `fintool report annual <SYMBOL>`

Fetch the latest 10-K (annual) filing from SEC EDGAR. Use `--output <file>` to save the full text.

### `fintool report quarterly <SYMBOL>`

Fetch the latest 10-Q (quarterly) filing from SEC EDGAR.

### `fintool report list <SYMBOL>`

List recent SEC filings for a company.

### `fintool report get <SYMBOL> <ACCESSION>`

Fetch a specific filing by accession number.

#### Examples

```bash
fintool report annual AAPL
fintool report quarterly TSLA --output tsla_10q.txt
fintool report list MSFT
fintool report list AAPL --human
fintool report get AAPL 0000320193-24-000123
```

---

### `fintool order buy <SYMBOL> <AMOUNT_USDC> <MAX_PRICE>`

Place a **spot** limit buy order. The price is the **maximum price** you're willing to pay per unit. Size is calculated as `AMOUNT_USDC / MAX_PRICE`.

**Exchanges:** Hyperliquid, Binance, Coinbase (auto-routed based on config and `--exchange` flag)

The symbol is auto-resolved:
- **Hyperliquid:** `TSLA` → `TSLA/USDC` spot pair
- **Binance:** `TSLA` → `TSLAUSDT` spot pair
- **Coinbase:** `BTC` → `BTC-USD` product ID

#### Examples

```bash
fintool order buy TSLA 1 410      # buy $1 of TSLA at max $410
fintool order buy HYPE 100 25     # buy $100 of HYPE at max $25
fintool order buy BTC 50 66000    # buy $50 of BTC spot at max $66,000

# Force specific exchange
fintool order buy BTC 100 65000 --exchange binance
fintool order buy BTC 100 65000 --exchange coinbase
```

#### JSON Schema

```json
{
  "action": "spot_buy",
  "symbol": "TSLA",
  "size": "0.002439",
  "maxPrice": "410",
  "total_usdc": "1",
  "network": "mainnet",
  "result": "Ok(...)"
}
```

---

### `fintool order sell <SYMBOL> <AMOUNT> <MIN_PRICE>`

Place a **spot** limit sell order. The price is the **minimum price** you'll accept per unit.

**Exchanges:** Hyperliquid, Binance, Coinbase (auto-routed based on config and `--exchange` flag)

#### Examples

```bash
fintool order sell TSLA 1 420     # sell 1 TSLA at minimum $420
fintool order sell HYPE 10 30     # sell 10 HYPE at minimum $30

# Force specific exchange
fintool order sell BTC 0.01 67000 --exchange binance
fintool order sell BTC 0.01 67000 --exchange coinbase
```

#### JSON Schema

```json
{
  "action": "spot_sell",
  "symbol": "TSLA",
  "size": "1",
  "minPrice": "420",
  "network": "mainnet",
  "result": "Ok(...)"
}
```

---

### `fintool perp buy <SYMBOL> <AMOUNT_USDC> <PRICE>`

Place a **perpetual futures** limit buy (long) order.

**Exchanges:** Hyperliquid (including HIP-3), Binance (Coinbase doesn't support perps)

#### Examples

```bash
# Crypto perps (main HL dex)
fintool perp buy BTC 100 65000    # long $100 of BTC at $65,000
fintool perp buy ETH 500 1800     # long $500 of ETH at $1,800

# Commodity/stock perps (HIP-3 cash dex — auto-detected)
fintool perp buy SILVER 1000 89.50   # long $1000 of silver at $89.50
fintool perp buy GOLD 5000 5200      # long $5000 of gold at $5,200
fintool perp buy TSLA 1000 410       # long $1000 of TSLA at $410
fintool perp buy NVDA 2000 193       # long $2000 of NVDA at $193

# Force Binance
fintool perp buy BTC 100 65000 --exchange binance
```

---

### `fintool perp sell <SYMBOL> <AMOUNT> <PRICE>`

Place a **perpetual futures** limit sell (short) order.

**Exchanges:** Hyperliquid (including HIP-3), Binance (Coinbase doesn't support perps)

#### Examples

```bash
# Crypto perps
fintool perp sell BTC 0.01 70000  # short 0.01 BTC at $70,000
fintool perp sell ETH 1 2000      # short 1 ETH at $2,000

# Commodity/stock perps (HIP-3)
fintool perp sell SILVER 10 95    # short 10 silver at $95
fintool perp sell GOLD 0.5 5300   # short 0.5 gold at $5,300
fintool perp sell TSLA 5 420      # short 5 TSLA at $420

# Force Binance
fintool perp sell BTC 0.01 70000 --exchange binance
```

---

### `fintool orders [SYMBOL]`

List open orders (both spot and perp). Optionally filter by symbol.

**Exchanges:** All three supported (Hyperliquid, Binance, Coinbase)

```bash
fintool orders
fintool orders BTC
fintool orders --human
fintool orders --exchange binance
fintool orders --exchange coinbase
```

---

### `fintool cancel <ORDER_ID>`

Cancel an open order.

**Order ID formats:**

| Exchange | Format | Example |
|----------|--------|---------|
| Hyperliquid | `SYMBOL:OID` | `BTC:91490942` |
| Binance spot | `binance_spot:SYMBOL:ORDERID` | `binance_spot:BTCUSDT:12345678` |
| Binance futures | `binance_futures:SYMBOL:ORDERID` | `binance_futures:BTCUSDT:87654321` |
| Coinbase | `coinbase:UUID` | `coinbase:abc123-def456-...` |

**Note:** Use `fintool orders` to get the correct order ID format for each exchange.

#### Examples

```bash
# Hyperliquid
fintool cancel BTC:91490942

# Binance spot
fintool cancel binance_spot:BTCUSDT:12345678

# Binance futures
fintool cancel binance_futures:BTCUSDT:87654321

# Coinbase
fintool cancel coinbase:abc123-def456-ghi789
```

---

### `fintool balance`

Show account balances and margin summary.

**Exchanges:** All three supported (Hyperliquid, Binance, Coinbase)

```bash
fintool balance
fintool balance --human
fintool balance --exchange binance
fintool balance --exchange coinbase
```

---

### `fintool positions`

Show open positions with PnL.

**Exchanges:** All three supported (Hyperliquid, Binance, Coinbase)

```bash
fintool positions
fintool positions --human
fintool positions --exchange binance
fintool positions --exchange coinbase
```

---

### `fintool options buy/sell <SYMBOL> <TYPE> <STRIKE> <EXPIRY> <SIZE>`

Place an options order. **Binance only** — Hyperliquid and Coinbase don't support options.

**Binance options symbol format:** `BTC-260328-80000-C`
- Format: `BASE-YYMMDD-STRIKE-C/P`
- `C` = Call, `P` = Put

#### Examples

```bash
# Buy a BTC call option (strike $80k, expiry 2026-03-28)
fintool options buy BTC call 80000 260328 0.1

# Sell a BTC put option
fintool options sell BTC put 60000 260328 0.1

# Explicit exchange flag (optional for options)
fintool options buy BTC call 70000 260328 0.1 --exchange binance
```

**Note:** Hyperliquid and Coinbase will return an error: *"Options trading requires Binance"*

---

### `fintool deposit <ASSET>`

Deposit assets to an exchange. The behavior depends on the asset and exchange.

#### Hyperliquid (default)

**ETH, BTC, SOL** — Generates a permanent deposit address via the [HyperUnit](https://docs.hyperunit.xyz) bridge. Send any amount, any number of times.

```bash
fintool deposit ETH              # get your ETH deposit address
fintool deposit BTC              # get your BTC deposit address
fintool deposit SOL              # get your SOL deposit address
```

Minimums: 0.007 ETH, 0.0003 BTC, 0.12 SOL. Estimated time: ~3 min (ETH), ~20 min (BTC), ~1 min (SOL).

**USDC** — Bridges USDC from Ethereum or Base to Hyperliquid via [Across Protocol](https://across.to) → Arbitrum → HL Bridge2. Executes automatically.

```bash
fintool deposit USDC --amount 100 --from ethereum    # ETH mainnet → HL
fintool deposit USDC --amount 500 --from base         # Base → HL
fintool deposit USDC --amount 100 --from ethereum --dry-run   # quote only
```

The USDC bridge executes 3 transactions with your configured private key:
1. Approve USDC spend on source chain (if needed)
2. Bridge via Across SpokePool (source → Arbitrum, ~2-10s)
3. Transfer USDC to HL Bridge2 on Arbitrum (auto-credited to your HL account)

#### Binance

Fetches your Binance deposit address via API. Use `--from` to specify the network.

```bash
fintool deposit ETH --exchange binance                    # default network
fintool deposit USDC --exchange binance --from ethereum   # USDC on Ethereum
fintool deposit USDC --exchange binance --from base       # USDC on Base
fintool deposit BTC --exchange binance --from bitcoin     # BTC on Bitcoin
```

#### Coinbase

Creates a Coinbase deposit address via API.

```bash
fintool deposit ETH --exchange coinbase
fintool deposit USDC --exchange coinbase
fintool deposit BTC --exchange coinbase
```

#### JSON Schema (Hyperliquid ETH/BTC/SOL)

```json
{
  "action": "deposit",
  "exchange": "hyperliquid",
  "asset": "ETH",
  "source_chain": "ethereum",
  "destination": "hyperliquid",
  "hl_address": "0x...",
  "deposit_address": "0x...",
  "minimum": "0.007 ETH",
  "estimated_fees": { ... }
}
```

#### JSON Schema (USDC bridge)

```json
{
  "action": "deposit_usdc",
  "exchange": "hyperliquid",
  "status": "completed",
  "source_chain": "ethereum",
  "amount_in": "100 USDC",
  "amount_deposited": "99.98 USDC",
  "hl_address": "0x...",
  "bridge_tx": "0x...",
  "hl_deposit_tx": "0x..."
}
```

---

### `fintool withdraw <AMOUNT> <ASSET>`

Withdraw assets from an exchange to an external address. Executes by default; use `--dry-run` for quote-only.

#### Hyperliquid (default)

**USDC** — Withdraws via HL Bridge2. Default destination is Arbitrum; use `--network` for Ethereum or Base (chained via Across).

```bash
# USDC → Arbitrum (direct, ~3-4 min)
fintool withdraw 100 USDC
fintool withdraw 100 USDC --to 0xOtherAddress

# USDC → Ethereum mainnet (HL → Arbitrum → Ethereum, ~5-7 min)
fintool withdraw 100 USDC --network ethereum

# USDC → Base (HL → Arbitrum → Base, ~5-6 min)
fintool withdraw 100 USDC --network base

# Quote only
fintool withdraw 100 USDC --network ethereum --dry-run
```

For `--network ethereum` or `--network base`, the chained withdrawal:
1. Withdraws USDC from HL to your Arbitrum address (~4 min)
2. Bridges USDC from Arbitrum to destination via Across (~2-10s)

**ETH, BTC, SOL** — Withdraws via [HyperUnit](https://docs.hyperunit.xyz) bridge. Transfers the tokenized asset (uETH/uBTC/uSOL) on Hyperliquid to a Unit withdrawal address, which releases native asset on the destination chain.

```bash
# ETH → Ethereum (defaults to your wallet address)
fintool withdraw 0.5 ETH

# SOL → Solana
fintool withdraw 1 SOL --to SomeSolanaAddress

# BTC → Bitcoin (--to required)
fintool withdraw 0.01 BTC --to bc1q...
```

#### Binance

Withdraws via Binance API. Requires `--to` and optionally `--network`.

```bash
fintool withdraw 100 USDC --to 0x... --exchange binance --network ethereum
fintool withdraw 0.5 ETH --to 0x... --exchange binance --network arbitrum
fintool withdraw 0.01 BTC --to bc1q... --exchange binance
```

Network name mapping: `ethereum`→ETH, `base`→BASE, `arbitrum`→ARBITRUM, `solana`→SOL, `bitcoin`→BTC, `bsc`→BSC, `polygon`→MATIC, `optimism`→OPTIMISM, `avalanche`→AVAXC.

#### Coinbase

Withdraws (sends) via Coinbase API. Requires `--to`.

```bash
fintool withdraw 100 USDC --to 0x... --exchange coinbase
fintool withdraw 0.5 ETH --to 0x... --exchange coinbase --network base
```

#### JSON Schema

```json
{
  "action": "withdraw",
  "exchange": "hyperliquid",
  "status": "submitted",
  "asset": "USDC",
  "amount": "100",
  "destination_chain": "arbitrum",
  "destination_address": "0x...",
  "result": "Ok(...)"
}
```

---

### `fintool bridge-status`

Show all HyperUnit bridge operations (deposits and withdrawals) for your configured wallet.

```bash
fintool bridge-status
fintool bridge-status --human
```

#### JSON Schema

```json
{
  "address": "0x...",
  "operations": [
    {
      "id": "0xabc...",
      "asset": "eth",
      "source_chain": "ethereum",
      "destination_chain": "hyperliquid",
      "amount": "0.500000 ETH",
      "state": "done",
      "source_tx": "0x...",
      "destination_tx": "0x..."
    }
  ]
}
```

---

### `fintool predict list [--platform <PLATFORM>] [--limit <N>]`

List trending prediction markets from Polymarket and/or Kalshi.

```bash
fintool predict list
fintool predict list --platform polymarket --limit 5
fintool predict list --platform kalshi --human
```

---

### `fintool predict search <QUERY> [--platform <PLATFORM>] [--limit <N>]`

Search prediction markets by keyword.

```bash
fintool predict search "trump"
fintool predict search "BTC" --platform kalshi
```

---

### `fintool predict quote <MARKET_ID>`

Get detailed quote for a specific market. Market ID format: `platform:identifier`.

```bash
fintool predict quote kalshi:KXBALANCE-29
fintool predict quote polymarket:china-coup-attempt-before-2027
```

---

### `fintool predict buy/sell <MARKET_ID> <SIDE> <AMOUNT>`

> ⚠️ **Stub** — Requires Polymarket CLOB signing or Kalshi API credentials.

```bash
fintool predict buy kalshi:KXBALANCE-29 yes 10
fintool predict sell polymarket:some-market no 50 --min-price 90
```

---

## Command Summary

| Command | Description | Exchanges |
|---------|-------------|-----------|
| `fintool init` | Create config file | N/A |
| `fintool quote <SYM>` | Multi-source price + LLM analysis | N/A (read-only) |
| `fintool perp quote <SYM>` | Perp price + funding/OI/premium | Hyperliquid, Binance |
| `fintool news <SYM>` | Latest news headlines | N/A |
| `fintool report annual/quarterly <SYM>` | SEC 10-K/10-Q filings | N/A |
| `fintool report list <SYM>` | List recent SEC filings | N/A |
| `fintool report get <SYM> <ACC>` | Fetch specific filing | N/A |
| `fintool order buy <SYM> <USDC> <MAX>` | Spot limit buy | Hyperliquid, Binance, Coinbase |
| `fintool order sell <SYM> <AMT> <MIN>` | Spot limit sell | Hyperliquid, Binance, Coinbase |
| `fintool perp buy <SYM> <USDC> <PX>` | Perp limit long | Hyperliquid, Binance |
| `fintool perp sell <SYM> <AMT> <PX>` | Perp limit short | Hyperliquid, Binance |
| `fintool orders [SYM]` | List open orders | Hyperliquid, Binance, Coinbase |
| `fintool cancel <ORDER_ID>` | Cancel an order | Hyperliquid, Binance, Coinbase |
| `fintool balance` | Account balances | Hyperliquid, Binance, Coinbase |
| `fintool positions` | Open positions + PnL | Hyperliquid, Binance, Coinbase |
| `fintool options buy/sell ...` | Options trading | Binance only |
| `fintool predict list` | List prediction markets | Polymarket, Kalshi |
| `fintool predict search <Q>` | Search prediction markets | Polymarket, Kalshi |
| `fintool predict quote <ID>` | Quote prediction market | Polymarket, Kalshi |
| `fintool deposit <ASSET>` | Deposit to exchange | Hyperliquid, Binance, Coinbase |
| `fintool withdraw <AMT> <ASSET>` | Withdraw from exchange | Hyperliquid, Binance, Coinbase |
| `fintool bridge-status` | Unit bridge operation status | Hyperliquid |
| `fintool predict buy/sell <ID> ...` | Trade predictions (stub) | Polymarket, Kalshi |

## Data Sources

| Data | Source | Auth Required | Notes |
|------|--------|---------------|-------|
| Spot prices (crypto + tokenized stocks) | Hyperliquid Spot API | No | |
| Traditional stock prices, indices, commodities | Yahoo Finance | No | |
| Crypto prices, 7d/30d trends, market cap | CoinGecko | No | |
| Quote analysis (trend, momentum, summary) | OpenAI API | `openai_api_key` | |
| Perp prices, funding, OI (Hyperliquid) | Hyperliquid Perps API | No | |
| Perp prices, funding, OI (Binance) | Binance Futures API | No | |
| News | Google News RSS | No | |
| SEC filings (10-K, 10-Q) | SEC EDGAR | No | |
| Trading — Hyperliquid spot + perps | Hyperliquid Exchange API | Wallet private key | EIP-712 signing |
| Trading — Binance spot | Binance Spot API `/api/v3/order` | API key + secret | HMAC-SHA256 signing |
| Trading — Binance futures | Binance Futures API `/fapi/v1/order` | API key + secret | HMAC-SHA256 signing |
| Trading — Binance options | Binance Options API `/eapi/v1/order` | API key + secret | HMAC-SHA256 signing |
| Trading — Coinbase spot | Coinbase Advanced Trade API `/api/v3/brokerage/orders` | API key + secret | HMAC-SHA256 signing |
| Prediction markets (quotes) | Polymarket Gamma API | No | |
| Prediction markets (quotes) | Kalshi REST API | No | |
| Prediction markets (trading) | Polymarket CLOB | Wallet private key | |
| Prediction markets (trading) | Kalshi REST API | API key + secret | |
| Deposit/Withdraw — HyperUnit bridge | HyperUnit API | Wallet private key | ETH, BTC, SOL ↔ Hyperliquid |
| Deposit — USDC cross-chain bridge | Across Protocol API | Wallet private key | Ethereum/Base → Arbitrum → HL |
| Deposit/Withdraw — HL USDC | Hyperliquid Bridge2 | Wallet private key | Arbitrum ↔ Hyperliquid |
| Deposit/Withdraw — Binance | Binance SAPI | API key + secret | `/sapi/v1/capital/` endpoints |
| Deposit/Withdraw — Coinbase | Coinbase v2 API | API key + secret | `/v2/accounts/` endpoints |

## Technical Notes

- **[HIP-3 Implementation](docs/HIP3-IMPLEMENTATION.md)** — How fintool implements EIP-712 signing for Hyperliquid's builder-deployed perpetuals (commodities, stocks, indices). Covers asset index resolution, msgpack wire format, and the signing flow that bypasses the Rust SDK's limitations.

## Architecture

```
fintool/
├── src/
│   ├── main.rs          # Entry point, command dispatch
│   ├── cli.rs           # Clap CLI definitions (global --exchange flag)
│   ├── config.rs        # Config loading (~/.fintool/config.toml)
│   ├── signing.rs       # Hyperliquid wallet signing, asset resolution, order execution
│   ├── binance.rs       # Binance API client (spot/futures/options, deposit/withdraw, HMAC-SHA256)
│   ├── coinbase.rs      # Coinbase Advanced Trade API client (spot, deposit/withdraw, HMAC-SHA256)
│   ├── bridge.rs        # Across Protocol cross-chain USDC bridge (Ethereum/Base ↔ Arbitrum)
│   ├── unit.rs          # HyperUnit bridge (ETH/BTC/SOL deposit/withdraw, fee estimation)
│   ├── polymarket.rs    # Polymarket CLOB client
│   ├── format.rs        # Color formatting helpers
│   └── commands/
│       ├── quote.rs     # Multi-source quotes + LLM enrichment
│       ├── news.rs      # News via Google News RSS
│       ├── report.rs    # SEC filings via EDGAR
│       ├── order.rs     # Spot limit buy/sell with exchange routing
│       ├── perp.rs      # Perp limit buy/sell with exchange routing
│       ├── orders.rs    # List open orders
│       ├── cancel.rs    # Cancel orders (supports all three exchange formats)
│       ├── balance.rs   # Account balance
│       ├── positions.rs # Open positions
│       ├── options.rs   # Options trading (Binance only)
│       ├── predict.rs   # Prediction markets (Polymarket + Kalshi)
│       ├── deposit.rs   # Multi-exchange deposit (Unit, Across, Binance, Coinbase)
│       ├── withdraw.rs  # Multi-exchange withdraw (Bridge2, Unit, Across, Binance, Coinbase)
│       └── bridge_status.rs # HyperUnit bridge operation tracker
├── config.toml.default  # Config template
├── Cargo.toml
└── README.md
```

### Key Modules

**`binance.rs`** — Binance exchange integration:
- HMAC-SHA256 request signing
- Spot orders: `/api/v3/order`
- Futures orders: `/fapi/v1/order`
- Options orders: `/eapi/v1/order`
- Account balances, positions, open orders, cancellation

**`coinbase.rs`** — Coinbase Advanced Trade integration:
- HMAC-SHA256 request signing (timestamp + method + path + body)
- Spot orders: `/api/v3/brokerage/orders`
- Product ID format: `BTC-USD` (not `BTCUSDT`)
- Account balances, open orders, cancellation
- **No perp or options support** (Coinbase doesn't offer these products)

**Exchange Routing** — Commands with `--exchange` flag:
- `resolve_exchange()` in each command module
- Auto mode logic: check configured exchanges, priority = Hyperliquid > Coinbase > Binance for spot
- Perps: Hyperliquid > Binance (Coinbase excluded)
- Options always require Binance

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `hyperliquid_rust_sdk` | Hyperliquid exchange client with EIP-712 signing |
| `ethers` | Ethereum wallet and signing primitives |
| `reqwest` | HTTP client (rustls TLS — no OpenSSL) |
| `hmac`, `sha2`, `hex` | HMAC-SHA256 signing for Binance and Coinbase APIs |
| `clap` | CLI argument parsing |
| `serde` / `serde_json` | JSON serialization |
| `colored` | Terminal colors (`--human` mode) |
| `tabled` | Table formatting (`--human` mode) |
| `rust_decimal` | Precise financial math |

## License

MIT

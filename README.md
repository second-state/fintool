# fintool

A Rust CLI for financial trading and market intelligence — spot and perpetual futures on Hyperliquid, stock quotes, LLM-enriched analysis, prediction markets, SEC filings, and news.

## Installation

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

# Spot trading (requires wallet config)
fintool order buy TSLA 100 410
fintool order sell TSLA 1 420

# Perp trading
fintool perp buy BTC 100 65000
fintool perp sell BTC 0.01 70000
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

## Configuration

Config file: `~/.fintool/config.toml`

Run `fintool init` to generate a template, or copy `config.toml.default` from the release zip.

```toml
[wallet]
# Private key (hex, with or without 0x prefix)
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

# Kalshi — prediction market trading
# kalshi_api_key = "..."
# kalshi_api_secret = "..."
```

### Config Options

| Section | Key | Type | Default | Description |
|---------|-----|------|---------|-------------|
| `wallet` | `private_key` | string | — | Hex private key (with or without `0x`). **Takes priority** over keystore. |
| `wallet` | `wallet_json` | string | — | Path to encrypted Ethereum keystore JSON file. Supports `~` expansion. |
| `wallet` | `wallet_passcode` | string | — | Passcode to decrypt the keystore file. |
| `network` | `testnet` | bool | `false` | Use Hyperliquid testnet. |
| `api_keys` | `openai_api_key` | string | — | OpenAI API key. Enables LLM-enriched quotes with trend/momentum analysis. |
| `api_keys` | `openai_model` | string | `gpt-4.1-mini` | OpenAI model for quote analysis. Any chat completions model works. |
| `api_keys` | `kalshi_api_key` | string | — | Kalshi API key (for prediction market trading). |
| `api_keys` | `kalshi_api_secret` | string | — | Kalshi API secret. |

### What Needs a Wallet

| Command | Wallet Required | OpenAI Key |
|---------|----------------|------------|
| `quote` | No | Optional (enriches output) |
| `perp quote` | No | No |
| `news`, `init` | No | No |
| `report` | No | No |
| `predict list/search/quote` | No | No |
| `order buy/sell`, `perp buy/sell` | Yes | No |
| `orders`, `cancel`, `balance`, `positions` | Yes | No |
| `predict buy/sell` | Yes | No |
| `options` | — | — (stub) |

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

#### Examples

```bash
fintool perp quote BTC
fintool perp quote ETH
fintool perp quote SOL --human
```

#### JSON Schema

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

| Field | Type | Description |
|-------|------|-------------|
| `symbol` | string | Asset symbol |
| `markPx` | string | Current mark price (USD) |
| `oraclePx` | string | Oracle price (USD) |
| `change24h` | string | 24-hour price change (%) |
| `funding` | string | Current funding rate (per 8h) |
| `premium` | string | Mark-oracle premium |
| `openInterest` | string | Open interest in asset units |
| `volume24h` | string | 24-hour notional volume (USD) |
| `prevDayPx` | string | Previous day price (USD) |
| `maxLeverage` | number | Maximum allowed leverage |
| `source` | string | `"Hyperliquid"` |

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

The symbol is auto-resolved to its Hyperliquid spot pair (e.g. `TSLA` → `TSLA/USDC`).

#### Examples

```bash
fintool order buy TSLA 1 410      # buy $1 of TSLA at max $410
fintool order buy HYPE 100 25     # buy $100 of HYPE at max $25
fintool order buy BTC 50 66000    # buy $50 of BTC spot at max $66,000
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

#### Examples

```bash
fintool order sell TSLA 1 420     # sell 1 TSLA at minimum $420
fintool order sell HYPE 10 30     # sell 10 HYPE at minimum $30
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

#### Examples

```bash
fintool perp buy BTC 100 65000    # long $100 of BTC at $65,000
fintool perp buy ETH 500 1800     # long $500 of ETH at $1,800
```

---

### `fintool perp sell <SYMBOL> <AMOUNT> <PRICE>`

Place a **perpetual futures** limit sell (short) order.

#### Examples

```bash
fintool perp sell BTC 0.01 70000  # short 0.01 BTC at $70,000
fintool perp sell ETH 1 2000      # short 1 ETH at $2,000
```

---

### `fintool orders [SYMBOL]`

List open orders (both spot and perp). Optionally filter by symbol.

```bash
fintool orders
fintool orders BTC
fintool orders --human
```

---

### `fintool cancel <SYMBOL:OID>`

Cancel an open order. Format: `SYMBOL:ORDER_ID`.

```bash
fintool cancel BTC:91490942
```

---

### `fintool balance`

Show account balances and margin summary.

```bash
fintool balance
fintool balance --human
```

---

### `fintool positions`

Show open positions with PnL.

```bash
fintool positions
fintool positions --human
```

---

### `fintool options buy/sell <SYMBOL> <TYPE> <STRIKE> <EXPIRY> <SIZE>`

> ⚠️ **Stub** — Native options support coming with Hyperliquid HIP-4.

```bash
fintool options buy BTC call 70000 2026-03-28 0.1
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

| Command | Description |
|---------|-------------|
| `fintool init` | Create config file |
| `fintool quote <SYM>` | Multi-source price + LLM analysis |
| `fintool perp quote <SYM>` | Perp price + funding/OI/premium |
| `fintool news <SYM>` | Latest news headlines |
| `fintool report annual/quarterly <SYM>` | SEC 10-K/10-Q filings |
| `fintool report list <SYM>` | List recent SEC filings |
| `fintool report get <SYM> <ACC>` | Fetch specific filing |
| `fintool order buy <SYM> <USDC> <MAX>` | Spot limit buy |
| `fintool order sell <SYM> <AMT> <MIN>` | Spot limit sell |
| `fintool perp buy <SYM> <USDC> <PX>` | Perp limit long |
| `fintool perp sell <SYM> <AMT> <PX>` | Perp limit short |
| `fintool orders [SYM]` | List open orders |
| `fintool cancel <SYM:OID>` | Cancel an order |
| `fintool balance` | Account balances |
| `fintool positions` | Open positions + PnL |
| `fintool options buy/sell ...` | Options (stub, HIP-4) |
| `fintool predict list` | List prediction markets |
| `fintool predict search <Q>` | Search prediction markets |
| `fintool predict quote <ID>` | Quote prediction market |
| `fintool predict buy/sell <ID> ...` | Trade predictions (stub) |

## Data Sources

| Data | Source | Auth Required |
|------|--------|---------------|
| Spot prices (crypto + tokenized stocks) | Hyperliquid Spot API | No |
| Traditional stock prices, indices, commodities | Yahoo Finance | No |
| Crypto prices, 7d/30d trends, market cap | CoinGecko | No |
| Quote analysis (trend, momentum, summary) | OpenAI API | `openai_api_key` |
| Perp prices, funding, OI | Hyperliquid Perps API | No |
| News | Google News RSS | No |
| SEC filings (10-K, 10-Q) | SEC EDGAR | No |
| Trading (spot + perps) | Hyperliquid Exchange API | Wallet private key |
| Prediction markets (quotes) | Polymarket Gamma API | No |
| Prediction markets (quotes) | Kalshi REST API | No |
| Prediction markets (trading) | Polymarket CLOB | Wallet private key |
| Prediction markets (trading) | Kalshi REST API | API key + secret |

## Architecture

```
fintool/
├── src/
│   ├── main.rs          # Entry point, command dispatch
│   ├── cli.rs           # Clap CLI definitions
│   ├── config.rs        # Config loading (~/.fintool/config.toml)
│   ├── signing.rs       # Wallet signing, asset resolution, order execution
│   ├── format.rs        # Color formatting helpers
│   └── commands/
│       ├── quote.rs     # Multi-source quotes + LLM enrichment
│       ├── news.rs      # News via Google News RSS
│       ├── report.rs    # SEC filings via EDGAR
│       ├── order.rs     # Spot limit buy/sell
│       ├── perp.rs      # Perp limit buy/sell
│       ├── orders.rs    # List open orders
│       ├── cancel.rs    # Cancel orders
│       ├── balance.rs   # Account balance
│       ├── positions.rs # Open positions
│       ├── options.rs   # Options (stub, HIP-4)
│       └── predict.rs   # Prediction markets (Polymarket + Kalshi)
├── config.toml.default  # Config template
├── Cargo.toml
└── README.md
```

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `hyperliquid_rust_sdk` | Hyperliquid exchange client with EIP-712 signing |
| `ethers` | Ethereum wallet and signing primitives |
| `reqwest` | HTTP client (rustls TLS — no OpenSSL) |
| `clap` | CLI argument parsing |
| `serde` / `serde_json` | JSON serialization |
| `colored` | Terminal colors (`--human` mode) |
| `tabled` | Table formatting (`--human` mode) |
| `rust_decimal` | Precise financial math |

## License

MIT

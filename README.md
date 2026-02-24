# fintool

A Rust CLI for financial trading — spot and perpetual futures on Hyperliquid, stock quotes via Yahoo Finance, and news via Google News.

## Installation

```bash
cd fintool
cargo build --release
# Binary at ./target/release/fintool
```

## Quick Start

```bash
# Create config file
fintool init

# Edit config with your wallet key
vim ~/.fintool/config.toml

# Spot quotes
fintool quote BTC
fintool quote TSLA
fintool quote AAPL

# Perp quotes
fintool perp quote BTC

# News
fintool news ETH

# Spot trading (requires wallet config)
fintool order buy TSLA 100 410    # buy $100 of TSLA, max price $410
fintool order sell TSLA 1 420     # sell 1 TSLA, min price $420

# Perp trading
fintool perp buy BTC 100 65000    # long $100 of BTC at $65,000
fintool perp sell BTC 0.01 70000  # short 0.01 BTC at $70,000
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

Run `fintool init` to generate a template.

```toml
[wallet]
# Private key (hex, with or without 0x prefix)
# Takes priority over wallet_json + wallet_passcode
private_key = "0xabcdef1234567890..."

# Alternative: encrypted keystore file
# wallet_json = "/path/to/wallet.json"
# wallet_passcode = "your-passcode"

[network]
# Use Hyperliquid testnet (default: false)
testnet = false

[api_keys]
# Reserved for future use
# newsapi_key = "..."
```

### Config Options

| Section | Key | Type | Default | Description |
|---------|-----|------|---------|-------------|
| `wallet` | `private_key` | string | — | Hex private key (with or without `0x`). **Takes priority** over keystore. |
| `wallet` | `wallet_json` | string | — | Path to encrypted Ethereum keystore JSON file. Supports `~` expansion. |
| `wallet` | `wallet_passcode` | string | — | Passcode to decrypt the keystore file. |
| `network` | `testnet` | bool | `false` | Use Hyperliquid testnet (`api.hyperliquid-testnet.xyz`). |
| `api_keys` | `newsapi_key` | string | — | NewsAPI key (reserved for future use). |
| `api_keys` | `kalshi_api_key` | string | — | Kalshi API key (for prediction market trading). |
| `api_keys` | `kalshi_api_secret` | string | — | Kalshi API secret. |

### What Needs a Wallet

| Command | Wallet Required |
|---------|----------------|
| `quote`, `news`, `init` | No |
| `perp quote` | No |
| `order buy/sell`, `perp buy/sell` | Yes |
| `orders`, `cancel`, `balance`, `positions` | Yes |
| `options`, `predict` | Yes (stubs) |

---

## Commands

### `fintool init`

Create a default config file at `~/.fintool/config.toml`.

```bash
fintool init
```

---

### `fintool quote <SYMBOL>`

Get the current **spot** price for a crypto asset or stock.

**Resolution order:**
1. Hyperliquid spot (TSLA, HYPE, BTC, ETH, etc. — tokenized assets)
2. Yahoo Finance fallback (AAPL, GOOGL, MSFT, etc.)

#### Examples

```bash
fintool quote TSLA       # tokenized stock on HL spot
fintool quote BTC        # crypto on HL spot
fintool quote AAPL       # stock via Yahoo Finance
fintool quote ETH --human
```

#### JSON Schema — Hyperliquid Spot

```json
{
  "symbol": "TSLA",
  "price": "407.28",
  "change24h": "0.00",
  "volume24h": "48.8736",
  "prevDayPx": "407.28",
  "source": "Hyperliquid"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `symbol` | string | Asset symbol |
| `price` | string | Current spot price (USD) |
| `change24h` | string | 24-hour price change (%) |
| `volume24h` | string | 24-hour notional volume (USD) |
| `prevDayPx` | string | Previous day price (USD) |
| `source` | string | `"Hyperliquid"` |

#### JSON Schema — Yahoo Finance

```json
{
  "symbol": "AAPL",
  "price": "245.12",
  "change24h": "1.25",
  "currency": "USD",
  "exchange": "NMS",
  "source": "Yahoo Finance"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `currency` | string | Trading currency |
| `exchange` | string | Exchange code (NMS = NASDAQ, NYQ = NYSE) |

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

### `fintool order buy <SYMBOL> <AMOUNT_USDC> <MAX_PRICE>`

Place a **spot** limit buy order. The price is the **maximum price** you're willing to pay per unit. Size is calculated as `AMOUNT_USDC / MAX_PRICE`.

The symbol is auto-resolved to its Hyperliquid spot pair (e.g. `TSLA` → `TSLA/USDC`).

#### Examples

```bash
# Buy $1 of TSLA at max $410 per share
fintool order buy TSLA 1 410

# Buy $100 of HYPE at max $25
fintool order buy HYPE 100 25

# Buy $50 of BTC spot at max $66,000
fintool order buy BTC 50 66000
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

| Field | Type | Description |
|-------|------|-------------|
| `action` | string | `"spot_buy"` |
| `symbol` | string | Input symbol |
| `size` | string | Calculated order size (AMOUNT_USDC / MAX_PRICE) |
| `maxPrice` | string | Maximum price per unit (limit price) |
| `total_usdc` | string | Total USDC amount |
| `network` | string | `"mainnet"` or `"testnet"` |
| `result` | string | Exchange response |

---

### `fintool order sell <SYMBOL> <AMOUNT> <MIN_PRICE>`

Place a **spot** limit sell order. The price is the **minimum price** you'll accept per unit.

#### Examples

```bash
# Sell 1 TSLA at minimum $420
fintool order sell TSLA 1 420

# Sell 10 HYPE at minimum $30
fintool order sell HYPE 10 30
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

| Field | Type | Description |
|-------|------|-------------|
| `action` | string | `"spot_sell"` |
| `symbol` | string | Input symbol |
| `size` | string | Amount of asset to sell |
| `minPrice` | string | Minimum price per unit (limit price) |
| `network` | string | `"mainnet"` or `"testnet"` |
| `result` | string | Exchange response |

---

### `fintool perp buy <SYMBOL> <AMOUNT_USDC> <PRICE>`

Place a **perpetual futures** limit buy (long) order.

#### Examples

```bash
# Long $100 of BTC at $65,000
fintool perp buy BTC 100 65000

# Long $500 of ETH at $1,800
fintool perp buy ETH 500 1800

# Long $50 of SOL at $150
fintool perp buy SOL 50 150
```

#### JSON Schema

```json
{
  "action": "perp_buy",
  "symbol": "BTC",
  "size": "0.001538",
  "price": "65000",
  "total_usdc": "100",
  "network": "mainnet",
  "result": "Ok(...)"
}
```

---

### `fintool perp sell <SYMBOL> <AMOUNT> <PRICE>`

Place a **perpetual futures** limit sell (short) order.

#### Examples

```bash
# Short 0.01 BTC at $70,000
fintool perp sell BTC 0.01 70000

# Short 1 ETH at $2,000
fintool perp sell ETH 1 2000
```

#### JSON Schema

```json
{
  "action": "perp_sell",
  "symbol": "BTC",
  "size": "0.01",
  "price": "70000",
  "network": "mainnet",
  "result": "Ok(...)"
}
```

---

### `fintool orders [SYMBOL]`

List open orders (both spot and perp). Optionally filter by symbol.

#### Examples

```bash
fintool orders
fintool orders BTC
fintool orders --human
```

#### JSON Schema

```json
[
  {
    "coin": "BTC",
    "limitPx": "65000.0",
    "oid": 91490942,
    "side": "B",
    "sz": "0.001538",
    "timestamp": 1681247412573
  }
]
```

| Field | Type | Description |
|-------|------|-------------|
| `coin` | string | Asset symbol |
| `limitPx` | string | Limit price |
| `oid` | number | Order ID |
| `side` | string | `"B"` (buy) or `"A"` (sell) |
| `sz` | string | Remaining size |
| `timestamp` | number | Order creation time (ms epoch) |

---

### `fintool cancel <SYMBOL:OID>`

Cancel an open order. Format: `SYMBOL:ORDER_ID`.

#### Examples

```bash
fintool cancel BTC:91490942
fintool cancel TSLA:12345678
```

#### JSON Schema

```json
{
  "action": "cancel",
  "symbol": "BTC",
  "orderId": 91490942,
  "network": "mainnet",
  "result": "Ok(...)"
}
```

---

### `fintool balance`

Show account balances and margin summary.

#### Examples

```bash
fintool balance
fintool balance --human
```

#### JSON Schema

Returns the raw Hyperliquid `clearinghouseState` response:

```json
{
  "marginSummary": {
    "accountValue": "10000.00",
    "totalMarginUsed": "500.00",
    "totalNtlPos": "5000.00"
  },
  "crossMarginSummary": {
    "accountValue": "10000.00",
    "totalMarginUsed": "500.00",
    "totalNtlPos": "5000.00"
  },
  "assetPositions": [...]
}
```

---

### `fintool positions`

Show open positions with PnL.

#### Examples

```bash
fintool positions
fintool positions --human
```

#### JSON Schema

```json
[
  {
    "position": {
      "coin": "BTC",
      "szi": "0.1",
      "entryPx": "65000.0",
      "positionValue": "6580.0",
      "unrealizedPnl": "80.0",
      "leverage": { "type": "cross", "value": 10 }
    },
    "type": "oneWay"
  }
]
```

| Field | Type | Description |
|-------|------|-------------|
| `coin` | string | Asset symbol |
| `szi` | string | Signed size (negative = short) |
| `entryPx` | string | Average entry price |
| `positionValue` | string | Current position value |
| `unrealizedPnl` | string | Unrealized profit/loss |
| `leverage` | object | Leverage type and value |

---

### `fintool options buy/sell <SYMBOL> <TYPE> <STRIKE> <EXPIRY> <SIZE>`

> ⚠️ **Stub** — Native options support coming with Hyperliquid HIP-4.

```bash
fintool options buy BTC call 70000 2026-03-28 0.1
fintool options sell ETH put 1500 2026-03-28 1.0
```

#### JSON Schema

```json
{
  "status": "not_implemented",
  "note": "Native options support coming with Hyperliquid HIP-4",
  "params": { "symbol": "BTC", "type": "call", "strike": "70000", "expiry": "2026-03-28", "size": "0.1" }
}
```

---

### `fintool predict list [--platform <PLATFORM>] [--limit <N>]`

List trending prediction markets from Polymarket and/or Kalshi, sorted by volume.

- `--platform`: `polymarket`, `kalshi`, or `all` (default: `all`)
- `--limit`: max results (default: 10)

#### Examples

```bash
# Top markets from both platforms
fintool predict list

# Only Polymarket, top 5
fintool predict list --platform polymarket --limit 5

# Only Kalshi
fintool predict list --platform kalshi --limit 10

# Human-readable
fintool predict list --human
```

#### JSON Schema

```json
[
  {
    "platform": "polymarket",
    "id": "polymarket:china-coup-attempt-before-2027",
    "question": "China coup attempt before 2027?",
    "yesPrice": 0.046,
    "noPrice": 0.954,
    "volume": "99934.32838",
    "liquidity": "13217.82684",
    "endDate": "2026-12-31T00:00:00Z",
    "outcomes": ["Yes", "No"],
    "url": "https://polymarket.com/event/china-coup-attempt-before-2027"
  },
  {
    "platform": "kalshi",
    "id": "kalshi:KXBALANCE-29",
    "question": "Will Trump balance the budget?",
    "yesPrice": 0.125,
    "noPrice": 0.875,
    "volume": "38049",
    "endDate": "2029-07-01T14:00:00Z",
    "outcomes": ["Yes", "No"],
    "url": "https://kalshi.com/markets/KXBALANCE-29"
  }
]
```

| Field | Type | Description |
|-------|------|-------------|
| `platform` | string | `"polymarket"` or `"kalshi"` |
| `id` | string | Market ID (`platform:slug` or `platform:TICKER`) |
| `question` | string | Market question |
| `yesPrice` | number | Yes probability (0-1) |
| `noPrice` | number | No probability (0-1) |
| `volume` | string | Total volume traded |
| `liquidity` | string | Current liquidity (Polymarket only) |
| `endDate` | string | Market close date (ISO 8601) |
| `outcomes` | array | Outcome labels |
| `url` | string | Market page URL |

---

### `fintool predict search <QUERY> [--platform <PLATFORM>] [--limit <N>]`

Search prediction markets by keyword across both platforms.

#### Examples

```bash
# Search across both platforms
fintool predict search "trump"

# Search only Kalshi for BTC markets
fintool predict search "BTC" --platform kalshi

# Search Polymarket for election markets
fintool predict search "election" --platform polymarket --limit 5
```

#### JSON Schema

Same schema as `predict list`.

---

### `fintool predict quote <MARKET_ID>`

Get detailed price/probability quote for a specific market.

**Market ID format:** `platform:identifier`
- Polymarket: `polymarket:<slug>` (slug from the URL)
- Kalshi: `kalshi:<TICKER>` (ticker from the market)

Use `predict list` or `predict search` to find market IDs.

#### Examples

```bash
# Quote a Kalshi market
fintool predict quote kalshi:KXBALANCE-29

# Quote a Polymarket market
fintool predict quote polymarket:china-coup-attempt-before-2027

# Human-readable
fintool predict quote kalshi:KXELONMARS-99 --human
```

#### JSON Schema

```json
{
  "platform": "kalshi",
  "id": "kalshi:KXBALANCE-29",
  "question": "Will Trump balance the budget?",
  "yesPrice": 0.125,
  "noPrice": 0.875,
  "volume": "38049",
  "endDate": "2029-07-01T14:00:00Z",
  "outcomes": ["Yes", "No"],
  "url": "https://kalshi.com/markets/KXBALANCE-29"
}
```

Same fields as list/search, with all available detail for the specific market.

---

### `fintool predict buy <MARKET_ID> <SIDE> <AMOUNT> [--max-price <CENTS>]`

Buy prediction contracts on a specific market.

- `SIDE`: `yes` or `no`
- `AMOUNT`: USDC (Polymarket) or USD (Kalshi)
- `--max-price`: optional max price in cents (1-99)

> ⚠️ **Trading requires additional configuration:**
> - **Polymarket:** wallet `private_key` in config (trades on Polygon)
> - **Kalshi:** `kalshi_api_key` and `kalshi_api_secret` in config

#### Examples

```bash
# Buy $10 of YES on a Kalshi market
fintool predict buy kalshi:KXBALANCE-29 yes 10

# Buy $50 of NO on Polymarket, max 60¢
fintool predict buy polymarket:china-coup-attempt-before-2027 no 50 --max-price 60
```

#### JSON Schema

```json
{
  "action": "predict_buy",
  "market": "kalshi:KXBALANCE-29",
  "side": "yes",
  "amount": "10",
  "maxPrice": null,
  "status": "not_implemented",
  "note": "Trading on kalshi requires additional configuration."
}
```

---

### `fintool predict sell <MARKET_ID> <SIDE> <AMOUNT> [--min-price <CENTS>]`

Sell prediction contracts.

- `SIDE`: `yes` or `no`
- `AMOUNT`: number of contracts
- `--min-price`: optional min price in cents (1-99)

#### Examples

```bash
fintool predict sell kalshi:KXBALANCE-29 yes 5
fintool predict sell polymarket:china-coup-attempt-before-2027 no 10 --min-price 90
```

#### JSON Schema

```json
{
  "action": "predict_sell",
  "market": "kalshi:KXBALANCE-29",
  "side": "yes",
  "amount": "5",
  "minPrice": null,
  "status": "not_implemented",
  "note": "Trading on kalshi requires additional configuration."
}
```

---

## Command Summary

| Command | Description |
|---------|-------------|
| `fintool init` | Create config file |
| `fintool quote <SYM>` | Spot price (HL spot → Yahoo) |
| `fintool perp quote <SYM>` | Perp price + funding/OI/premium |
| `fintool news <SYM>` | Latest news headlines |
| `fintool order buy <SYM> <USDC> <MAX_PRICE>` | Spot limit buy |
| `fintool order sell <SYM> <AMT> <MIN_PRICE>` | Spot limit sell |
| `fintool perp buy <SYM> <USDC> <PRICE>` | Perp limit long |
| `fintool perp sell <SYM> <AMT> <PRICE>` | Perp limit short |
| `fintool orders [SYM]` | List open orders |
| `fintool cancel <SYM:OID>` | Cancel an order |
| `fintool balance` | Account balances |
| `fintool positions` | Open positions + PnL |
| `fintool options buy/sell ...` | Options (stub, HIP-4) |
| `fintool predict list [--platform X]` | List prediction markets (Polymarket + Kalshi) |
| `fintool predict search <QUERY>` | Search prediction markets |
| `fintool predict quote <ID>` | Quote a specific prediction market |
| `fintool predict buy <ID> <SIDE> <AMT>` | Buy prediction contracts (stub) |
| `fintool predict sell <ID> <SIDE> <AMT>` | Sell prediction contracts (stub) |

## Data Sources

| Data | Source | Auth Required |
|------|--------|---------------|
| Spot prices (crypto + tokenized stocks) | Hyperliquid Spot API | No |
| Traditional stock prices | Yahoo Finance | No |
| Perp prices, funding, OI | Hyperliquid Perps API | No |
| News | Google News RSS | No |
| Trading (spot + perps) | Hyperliquid Exchange API | Wallet private key |
| Prediction markets (quotes) | Polymarket Gamma API | No |
| Prediction markets (quotes) | Kalshi REST API | No |
| Prediction markets (trading) | Polymarket CLOB | Wallet private key |
| Prediction markets (trading) | Kalshi REST API | API key + secret |

## API Endpoints

| Endpoint | URL |
|----------|-----|
| Mainnet Info | `https://api.hyperliquid.xyz/info` |
| Mainnet Exchange | `https://api.hyperliquid.xyz/exchange` |
| Testnet Info | `https://api.hyperliquid-testnet.xyz/info` |
| Testnet Exchange | `https://api.hyperliquid-testnet.xyz/exchange` |
| Polymarket Gamma | `https://gamma-api.polymarket.com` |
| Kalshi API | `https://api.elections.kalshi.com/trade-api/v2` |

## Supported Assets

### Spot
All assets on Hyperliquid spot — tokenized stocks (TSLA, AAPL, GOOGL via Wagyu.xyz), crypto (BTC, ETH, HYPE), and more. Auto-resolved as `SYMBOL/USDC`.

### Perps
All perpetual futures on Hyperliquid — BTC, ETH, SOL, AVAX, ARB, DOGE, and 200+ more.

### Stocks (quotes only)
Any ticker on Yahoo Finance as fallback — AAPL, GOOGL, MSFT, AMZN, etc.

## Architecture

```
fintool/
├── src/
│   ├── main.rs          # Entry point, command dispatch
│   ├── cli.rs           # Clap CLI definitions
│   ├── config.rs        # Config file loading (~/.fintool/config.toml)
│   ├── signing.rs       # Wallet signing, asset resolution, order execution
│   ├── format.rs        # Color formatting helpers
│   └── commands/
│       ├── quote.rs     # Spot + perp price quotes
│       ├── news.rs      # News via Google News RSS
│       ├── order.rs     # Spot limit buy/sell
│       ├── perp.rs      # Perp limit buy/sell
│       ├── orders.rs    # List open orders
│       ├── cancel.rs    # Cancel orders
│       ├── balance.rs   # Account balance
│       ├── positions.rs # Open positions
│       ├── options.rs   # Options (stub, HIP-4)
│       └── predict.rs   # Prediction markets (Polymarket + Kalshi)
├── Cargo.toml
└── README.md
```

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `hyperliquid_rust_sdk` | Hyperliquid exchange client with EIP-712 signing |
| `ethers` | Ethereum wallet and signing primitives |
| `clap` | CLI argument parsing |
| `reqwest` | HTTP client |
| `serde` / `serde_json` | JSON serialization |
| `colored` | Terminal colors (`--human` mode) |
| `tabled` | Table formatting (`--human` mode) |
| `rust_decimal` | Precise financial math |
| `eth-keystore` | Encrypted wallet JSON decryption |
| `urlencoding` | URL parameter encoding |

## License

MIT

# fintool

A Rust CLI tool for agentic trading and market intelligence — spot and perpetual futures on **Hyperliquid**, **Binance**, and **Coinbase**. Supports crypto, stocks, and commodities. Seamlessly deposit, withdraw, and bridge across major blockchains and wallets with a single command. Get real-time price quotes, momentums, trends, funding rates, LLM-enriched analysis, SEC filings, and news.

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

## Quick Guides

### Setup

```bash
fintool init                    # create config file
vim ~/.fintool/config.toml      # add your wallet key and API keys
```

### Deposit funds into the exchange

Bridge USDC from Base (or Ethereum) to Hyperliquid. You must bridge more than $5 USDC. Your Hyperliquid address is the same as your Base/Ethereum address. The command handles all bridging transactions automatically.

```bash
fintool deposit USDC --amount 15 --from base
```

The deposited USDC goes into the Hyperliquid perp margin account. To use it for spot trading as well, set the account to unified mode:

```bash
fintool perp set-mode unified
```

Check your balance:

```bash
fintool balance
```

### Withdraw funds from the exchange

Withdraw USDC from Hyperliquid back to Base. The command reverses the deposit bridges (Hyperliquid → Arbitrum → Base).

```bash
fintool withdraw USDC --amount 10 --to base
```

You can also withdraw to Arbitrum (default, fastest) or Ethereum:

```bash
fintool withdraw USDC --amount 10                      # → Arbitrum (~3-4 min)
fintool withdraw USDC --amount 10 --to ethereum         # → Ethereum (~5-7 min)
```

### Get price quotes and news

Get an enriched spot price quote with trend analysis (uses Hyperliquid + Yahoo Finance + CoinGecko, merged by OpenAI):

```bash
fintool quote BTC
fintool quote AAPL
fintool quote SP500               # index alias
fintool quote GOLD                # commodity alias
```

Get a perpetual futures quote with funding rate, open interest, and leverage info:

```bash
fintool perp quote ETH
fintool perp quote SILVER         # HIP-3 commodity perp
```

Get the latest news headlines and SEC filings:

```bash
fintool news ETH
fintool report annual AAPL
fintool report list TSLA
```

### Spot buy and sell

Get the current price, then place a limit buy order. The command below buys 1.0 HYPE at a max price of $25/HYPE:

```bash
fintool quote HYPE
fintool order buy HYPE --amount 1.0 --price 25.00
```

Check your balance, then sell. The command below sells 0.48 HYPE at a minimum price of $30/HYPE:

```bash
fintool balance
fintool order sell HYPE --amount 0.48 --price 30.00
```

You can force a specific exchange with `--exchange`:

```bash
fintool order buy BTC --amount 0.002 --price 65000 --exchange coinbase
fintool order buy BTC --amount 0.002 --price 65000 --exchange binance
```

### Open and close perp positions

Get the perp quote, set leverage, and open a long position:

```bash
fintool perp quote ETH
fintool perp leverage ETH --leverage 2
fintool perp buy ETH --amount 0.006 --price 2100.00
```

Check positions and balance:

```bash
fintool positions
fintool balance
```

Close the position with `--close` (reduce-only — won't open a new short):

```bash
fintool perp sell ETH --amount 0.006 --price 2150.00 --close
```

### Commodity perp on Hyperliquid (USDT0 conversion)

The HIP-3 commodity/stock perp market on Hyperliquid (SILVER, GOLD, TSLA, etc.) uses USDT0 as collateral instead of USDC. You need to swap USDC → USDT0 first.

**Buy USDT0 on the spot market and transfer to the HIP-3 dex:**

```bash
fintool order buy USDT0 --amount 30 --price 1.002
fintool transfer USDT0 --amount 30 --from spot --to cash
```

**Trade the commodity perp:**

```bash
fintool perp quote SILVER
fintool perp leverage SILVER --leverage 2
fintool perp buy SILVER --amount 0.13 --price 89.00
```

**Close the position and convert back to USDC:**

```bash
fintool perp sell SILVER --amount 0.14 --price 91.00 --close
fintool transfer USDT0 --amount 30 --from cash --to spot
fintool order sell USDT0 --amount 30 --price 0.998
```

Check everything:

```bash
fintool positions
fintool orders
fintool balance
```

## Output Modes

**Human-readable (default):** Colored, formatted terminal output.

```bash
fintool quote BTC
fintool balance
```

**JSON mode:** Pass a full command as JSON for programmatic use. Output is always JSON.

```bash
fintool --json '{"command":"quote","symbol":"BTC"}'
fintool --json '{"command":"balance"}'
```

See [JSON Mode](#json-mode) for the full schema.

---

## Exchange Support

`fintool` supports three exchanges with automatic routing: **Hyperliquid**, **Binance**, and **Coinbase**.

### Exchange Capability Matrix

| Feature | Hyperliquid | Binance | Coinbase |
|---------|-------------|---------|----------|
| Spot Trading | ✅ | ✅ | ✅ |
| Perpetual Futures | ✅ | ✅ | ❌ |
| Options | ❌ | ✅ | ❌ |
| Balance | ✅ | ✅ | ✅ |
| Positions | ✅ | ✅ | ❌ |
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
fintool order buy BTC --amount 0.002 --price 65000

# Force Hyperliquid
fintool order buy BTC --amount 0.002 --price 65000 --exchange hyperliquid

# Force Binance
fintool order buy BTC --amount 0.002 --price 65000 --exchange binance

# Force Coinbase (uses BTC-USD internally)
fintool order buy BTC --amount 0.002 --price 65000 --exchange coinbase

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
### What Needs Configuration

| Command | Hyperliquid Wallet | Binance Keys | Coinbase Keys | OpenAI Key | Exchange Support |
|---------|-------------------|--------------|---------------|------------|------------------|
| `quote` | No | No | No | Optional (enriches) | N/A (read-only) |
| `perp quote` | No | No | No | No | Hyperliquid (read-only) |
| `news`, `init` | No | No | No | No | N/A |
| `address` | Yes | No | No | No | Hyperliquid |
| `report` | No | No | No | No | N/A |
| `order buy/sell` | Yes (HL) | Yes (Binance) | Yes (Coinbase) | No | All three |
| `perp buy/sell` | Yes (HL) | Yes (Binance) | No | No | HL + Binance |
| `perp leverage` | Yes (HL) | Yes (Binance) | No | No | HL + Binance |
| `perp set-mode` | Yes | No | No | No | Hyperliquid only |
| `orders` | Yes (HL) | Yes (Binance) | Yes (Coinbase) | No | All three |
| `cancel` | Yes (HL) | Yes (Binance) | Yes (Coinbase) | No | All three |
| `balance` | Yes (HL) | Yes (Binance) | Yes (Coinbase) | No | All three |
| `positions` | Yes (HL) | Yes (Binance) | No | No | HL + Binance |
| `options buy/sell` | No | Yes (Binance) | No | No | Binance only |
| `deposit` (HL) | Yes | No | No | No | Hyperliquid |
| `deposit` (Binance) | No | Yes | No | No | Binance |
| `deposit` (Coinbase) | No | No | Yes | No | Coinbase |
| `withdraw` (HL) | Yes | No | No | No | Hyperliquid |
| `withdraw` (Binance) | No | Yes | No | No | Binance |
| `withdraw` (Coinbase) | No | No | Yes | No | Coinbase |
| `bridge-status` | Yes | No | No | No | Hyperliquid |
| `transfer` | Yes | No | No | No | Hyperliquid only |

---

## Commands

### `fintool init`

Create a default config file at `~/.fintool/config.toml`.

```bash
fintool init
```

---

### `fintool address`

Print the configured Hyperliquid wallet address (derived from the private key in config).

```bash
fintool address          # 0x...
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
fintool quote ETH
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

**Supported exchanges:** Hyperliquid only (including HIP-3 dexes like `cash`/dreamcash)

Fintool automatically searches across Hyperliquid's main perp universe and HIP-3 builder-deployed dexes (like `cash`/dreamcash) to find the most liquid market for any symbol.

#### Examples

```bash
# Crypto perps (main HL dex)
fintool perp quote BTC
fintool perp quote ETH
fintool perp quote SOL

# Commodity perps (HIP-3 cash dex)
fintool perp quote SILVER        # silver ~$89/oz, 20x leverage
fintool perp quote GOLD          # gold ~$5,184/oz, 20x leverage

# Stock perps (HIP-3 cash dex)
fintool perp quote TSLA          # Tesla stock perp
fintool perp quote NVDA          # NVIDIA stock perp
fintool perp quote GOOGL         # Alphabet stock perp

# US index perps (HIP-3 cash dex)
fintool perp quote USA500        # S&P 500 index perp
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
| `source` | string | `"Hyperliquid"` or `"Hyperliquid HIP-3 (cash)"` |

---

### `fintool news <SYMBOL>`

Get the latest news headlines via Google News RSS.

#### Examples

```bash
fintool news ETH
fintool news TSLA
fintool news AAPL
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
fintool report list AAPL
fintool report get AAPL 0000320193-24-000123
```

---

### `fintool order buy <SYMBOL> --amount <SIZE> --price <PRICE>`

Place a **spot** limit buy order. `--amount` is in symbol units (e.g. 1.0 HYPE). `--price` is the **maximum price** you're willing to pay per unit.

**Exchanges:** Hyperliquid, Binance, Coinbase (auto-routed based on config and `--exchange` flag)

The symbol is auto-resolved:
- **Hyperliquid:** `TSLA` → `TSLA/USDC` spot pair
- **Binance:** `TSLA` → `TSLAUSDT` spot pair
- **Coinbase:** `BTC` → `BTC-USD` product ID

#### Examples

```bash
fintool order buy TSLA --amount 0.01 --price 410     # buy 0.01 TSLA at max $410
fintool order buy HYPE --amount 1.0 --price 25.00    # buy 1.0 HYPE at max $25
fintool order buy BTC --amount 0.001 --price 66000   # buy 0.001 BTC at max $66,000

# Force specific exchange
fintool order buy BTC --amount 0.002 --price 65000 --exchange binance
fintool order buy BTC --amount 0.002 --price 65000 --exchange coinbase
```

#### JSON Schema

```json
{
  "action": "spot_buy",
  "symbol": "HYPE",
  "size": "1.0",
  "price": "25.00",
  "total_usdc": "25.00",
  "network": "mainnet",
  "result": "Ok(...)"
}
```

---

### `fintool order sell <SYMBOL> --amount <SIZE> --price <PRICE>`

Place a **spot** limit sell order. `--amount` is in symbol units. `--price` is the **minimum price** you'll accept per unit.

**Exchanges:** Hyperliquid, Binance, Coinbase (auto-routed based on config and `--exchange` flag)

#### Examples

```bash
fintool order sell TSLA --amount 1 --price 420       # sell 1 TSLA at minimum $420
fintool order sell HYPE --amount 10 --price 30.00    # sell 10 HYPE at minimum $30

# Force specific exchange
fintool order sell BTC --amount 0.01 --price 67000 --exchange binance
fintool order sell BTC --amount 0.01 --price 67000 --exchange coinbase
```

#### JSON Schema

```json
{
  "action": "spot_sell",
  "symbol": "TSLA",
  "size": "1",
  "price": "420",
  "network": "mainnet",
  "result": "Ok(...)"
}
```

---

### `fintool perp buy <SYMBOL> --amount <SIZE> --price <PRICE> [--close]`

Place a **perpetual futures** limit buy (long) order. `--amount` is in symbol units (e.g. 0.1 ETH).

**Exchanges:** Hyperliquid (including HIP-3), Binance (Coinbase doesn't support perps)

Use `--close` to close an existing short position (reduce-only order). Without `--close`, the order opens a new long position.

#### Examples

```bash
# Crypto perps (main HL dex)
fintool perp buy BTC --amount 0.002 --price 65000     # long 0.002 BTC at $65,000
fintool perp buy ETH --amount 0.1 --price 1800        # long 0.1 ETH at $1,800

# Close an existing short position (reduce-only)
fintool perp buy ETH --amount 0.1 --price 1800 --close

# Commodity/stock perps (HIP-3 cash dex — auto-detected)
fintool perp buy SILVER --amount 11.2 --price 89.50   # long 11.2 oz silver at $89.50
fintool perp buy GOLD --amount 1.0 --price 5200       # long 1.0 oz gold at $5,200
fintool perp buy TSLA --amount 2.5 --price 410        # long 2.5 TSLA at $410
fintool perp buy NVDA --amount 10 --price 193         # long 10 NVDA at $193

# Force Binance
fintool perp buy BTC --amount 0.002 --price 65000 --exchange binance
```

---

### `fintool perp sell <SYMBOL> --amount <SIZE> --price <PRICE> [--close]`

Place a **perpetual futures** limit sell (short) order. `--amount` is in symbol units.

**Exchanges:** Hyperliquid (including HIP-3), Binance (Coinbase doesn't support perps)

Use `--close` to close an existing long position (reduce-only order). Without `--close`, the order opens a new short position.

#### Examples

```bash
# Crypto perps
fintool perp sell BTC --amount 0.01 --price 70000    # short 0.01 BTC at $70,000
fintool perp sell ETH --amount 1 --price 2000        # short 1 ETH at $2,000

# Close an existing long position (reduce-only)
fintool perp sell ETH --amount 0.5 --price 2000 --close

# Commodity/stock perps (HIP-3)
fintool perp sell SILVER --amount 10 --price 95      # short 10 silver at $95
fintool perp sell GOLD --amount 0.5 --price 5300     # short 0.5 gold at $5,300
fintool perp sell TSLA --amount 5 --price 420        # short 5 TSLA at $420

# Force Binance
fintool perp sell BTC --amount 0.01 --price 70000 --exchange binance
```

---

### `fintool perp leverage <SYMBOL> --leverage <N> [--cross]`

Set leverage for a perpetual futures asset.

**Exchanges:** Hyperliquid (including HIP-3), Binance

By default, uses isolated margin. Use `--cross` for cross margin (main perps only — HIP-3 dex perps only support isolated margin).

#### Examples

```bash
# Crypto perps (main HL dex)
fintool perp leverage ETH --leverage 5              # 5x isolated
fintool perp leverage BTC --leverage 10 --cross     # 10x cross margin

# HIP-3 perps (commodities, stocks — isolated only)
fintool perp leverage SILVER --leverage 2
fintool perp leverage TSLA --leverage 3

# Binance
fintool perp leverage ETH --leverage 5 --exchange binance
```

#### JSON Schema

```json
{
  "action": "set_leverage",
  "exchange": "hyperliquid",
  "symbol": "ETH",
  "leverage": 5,
  "marginType": "isolated",
  "network": "mainnet",
  "result": "Ok(...)"
}
```

---

### `fintool perp set-mode <MODE>`

Set the account abstraction mode on Hyperliquid. **Hyperliquid only.**

| Mode | Description |
|------|-------------|
| `unified` | Single USDC balance shared across all perp dexes and spot |
| `standard` | Separate balances per dex (default for new accounts) |
| `disabled` | No abstraction |

#### Examples

```bash
fintool perp set-mode unified    # share margin across all dexes
fintool perp set-mode standard   # separate balances per dex
```

---

### `fintool orders [SYMBOL]`

List open orders (both spot and perp). Optionally filter by symbol.

**Exchanges:** All three supported (Hyperliquid, Binance, Coinbase)

```bash
fintool orders
fintool orders BTC
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
fintool balance --exchange binance
fintool balance --exchange coinbase
```

---

### `fintool positions`

Show open positions with PnL. Includes HIP-3 dex positions on Hyperliquid.

**Exchanges:** Hyperliquid, Binance (Coinbase is spot-only — no positions)

```bash
fintool positions
fintool positions --exchange binance
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

### `fintool withdraw <ASSET> --amount <AMT>`

Withdraw assets from an exchange to an external address. Executes by default; use `--dry-run` for quote-only.

#### Hyperliquid (default)

**USDC** — Withdraws via HL Bridge2. Default destination is Arbitrum; use `--to` with a chain name for Ethereum or Base (chained via Across).

```bash
# USDC → Arbitrum (direct, ~3-4 min)
fintool withdraw USDC --amount 100
fintool withdraw USDC --amount 100 --to 0xOtherAddress

# USDC → Ethereum mainnet (HL → Arbitrum → Ethereum, ~5-7 min)
fintool withdraw USDC --amount 100 --to ethereum

# USDC → Base (HL → Arbitrum → Base, ~5-6 min)
fintool withdraw USDC --amount 100 --to base

# Quote only
fintool withdraw USDC --amount 100 --to ethereum --dry-run
```

For `--to ethereum` or `--to base`, the chained withdrawal:
1. Withdraws USDC from HL to your Arbitrum address (~4 min)
2. Bridges USDC from Arbitrum to destination via Across (~2-10s)

**ETH, BTC, SOL** — Withdraws via [HyperUnit](https://docs.hyperunit.xyz) bridge. Transfers the tokenized asset (uETH/uBTC/uSOL) on Hyperliquid to a Unit withdrawal address, which releases native asset on the destination chain.

```bash
# ETH → Ethereum (defaults to your wallet address)
fintool withdraw ETH --amount 0.5

# SOL → Solana
fintool withdraw SOL --amount 1 --to SomeSolanaAddress

# BTC → Bitcoin (--to required)
fintool withdraw BTC --amount 0.01 --to bc1q...
```

#### Binance

Withdraws via Binance API. Requires `--to` (address) and optionally `--network`.

```bash
fintool withdraw USDC --amount 100 --to 0x... --exchange binance --network ethereum
fintool withdraw ETH --amount 0.5 --to 0x... --exchange binance --network arbitrum
fintool withdraw BTC --amount 0.01 --to bc1q... --exchange binance
```

Network name mapping: `ethereum`→ETH, `base`→BASE, `arbitrum`→ARBITRUM, `solana`→SOL, `bitcoin`→BTC, `bsc`→BSC, `polygon`→MATIC, `optimism`→OPTIMISM, `avalanche`→AVAXC.

#### Coinbase

Withdraws (sends) via Coinbase API. Requires `--to` (address).

```bash
fintool withdraw USDC --amount 100 --to 0x... --exchange coinbase
fintool withdraw ETH --amount 0.5 --to 0x... --exchange coinbase --network base
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

### `fintool transfer <ASSET> --amount <AMT> --from <SRC> --to <DST>`

Transfer assets between perp, spot, and HIP-3 dex accounts on Hyperliquid. **Hyperliquid only** — other exchanges will return an error.

On Hyperliquid, perp margin, spot balances, and HIP-3 dex margins are separate pools. This command lets you move funds between them.

#### Source/Destination Values

| Value | Description |
|-------|-------------|
| `spot` | Spot account |
| `perp` | Main perp margin account |
| `cash` | HIP-3 cash dex (SILVER, GOLD, stocks — uses USDT0 collateral) |
| `xyz` | HIP-3 xyz dex (uses USDC collateral) |
| `km`, `flx`, etc. | Other HIP-3 dexes |

**Note:** One side must always be `spot`. Direct perp-to-dex transfers are not supported.

#### HIP-3 Dex Collateral

| Dex | Auto-Resolved Assets | Collateral |
|-----|---------------------|------------|
| `cash` | SILVER, GOLD, TSLA, NVDA, GOOGL, AMZN, MSFT, META, INTC, HOOD, USA500 | USDT0 |
| `xyz` | (use `xyz:SYMBOL` prefix) | USDC |
| `km` | (use `km:SYMBOL` prefix) | varies |
| `flx` | (use `flx:SYMBOL` prefix) | varies |

#### Examples

```bash
# Spot ↔ main perp
fintool transfer USDC --amount 10 --from perp --to spot
fintool transfer USDC --amount 10 --from spot --to perp

# Spot ↔ HIP-3 dex (required for SILVER, GOLD, stocks)
# Note: cash dex uses USDT0 collateral — swap USDC→USDT0 on spot first
fintool transfer USDT0 --amount 10 --from spot --to cash
fintool transfer USDT0 --amount 10 --from cash --to spot
```

#### JSON Schema

```json
{
  "action": "transfer",
  "asset": "USDT0",
  "amount": "10",
  "from": "spot",
  "to": "cash",
  "token": "USDT0",
  "status": "ok"
}
```

---

### `fintool bridge-status`

Show all HyperUnit bridge operations (deposits and withdrawals) for your configured wallet.

```bash
fintool bridge-status
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

## Command Summary

| Command | Description | Exchanges |
|---------|-------------|-----------|
| `fintool init` | Create config file | N/A |
| `fintool address` | Print wallet address | Hyperliquid |
| `fintool quote <SYM>` | Multi-source price + LLM analysis | N/A (read-only) |
| `fintool perp quote <SYM>` | Perp price + funding/OI/premium | Hyperliquid |
| `fintool news <SYM>` | Latest news headlines | N/A |
| `fintool report annual/quarterly <SYM>` | SEC 10-K/10-Q filings | N/A |
| `fintool report list <SYM>` | List recent SEC filings | N/A |
| `fintool report get <SYM> <ACC>` | Fetch specific filing | N/A |
| `fintool order buy <SYM> --amount N --price P` | Spot limit buy | Hyperliquid, Binance, Coinbase |
| `fintool order sell <SYM> --amount N --price P` | Spot limit sell | Hyperliquid, Binance, Coinbase |
| `fintool perp buy <SYM> --amount N --price P` | Perp limit long / close short (`--close`) | Hyperliquid, Binance |
| `fintool perp sell <SYM> --amount N --price P` | Perp limit short / close long (`--close`) | Hyperliquid, Binance |
| `fintool perp leverage <SYM> --leverage N` | Set perp leverage (incl. HIP-3) | Hyperliquid, Binance |
| `fintool perp set-mode <MODE>` | Set account abstraction mode | Hyperliquid only |
| `fintool orders [SYM]` | List open orders | Hyperliquid, Binance, Coinbase |
| `fintool cancel <ORDER_ID>` | Cancel an order | Hyperliquid, Binance, Coinbase |
| `fintool balance` | Account balances | Hyperliquid, Binance, Coinbase |
| `fintool positions` | Open positions + PnL (incl. HIP-3) | Hyperliquid, Binance |
| `fintool options buy/sell ...` | Options trading | Binance only |
| `fintool deposit <ASSET>` | Deposit to exchange | Hyperliquid, Binance, Coinbase |
| `fintool withdraw <ASSET> --amount N` | Withdraw from exchange | Hyperliquid, Binance, Coinbase |
| `fintool bridge-status` | Unit bridge operation status | Hyperliquid |
| `fintool transfer <ASSET> --amount N --from X --to Y` | Transfer: perp ↔ spot ↔ dex | Hyperliquid only |

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
│   ├── signing.rs       # Hyperliquid wallet signing, asset resolution, order execution, dex transfers
│   ├── hip3.rs          # HIP-3 builder-deployed perps: EIP-712 signing, order/leverage for dex assets
│   ├── binance.rs       # Binance API client (spot/futures/options, deposit/withdraw, HMAC-SHA256)
│   ├── coinbase.rs      # Coinbase Advanced Trade API client (spot, deposit/withdraw, HMAC-SHA256)
│   ├── bridge.rs        # Across Protocol cross-chain USDC bridge (Ethereum/Base ↔ Arbitrum)
│   ├── unit.rs          # HyperUnit bridge (ETH/BTC/SOL deposit/withdraw, fee estimation)
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

## JSON Mode

For scripts, bots, and programmatic use, pass the entire command as a JSON string via the `--json` flag. In this mode, **all output is JSON** (including errors).

```bash
fintool --json '{"command":"quote","symbol":"BTC"}'
fintool --json '{"command":"balance"}'
fintool --json '{"command":"order_buy","symbol":"HYPE","amount":"1.0","price":"25.00"}'
fintool --json '{"command":"withdraw","asset":"USDC","amount":"10","to":"base"}'
```

Errors are returned as JSON too:

```json
{"error": "Invalid JSON command: missing field `symbol`"}
```

### JSON Command Schema

| `command` | Required fields | Optional fields |
|-----------|----------------|-----------------|
| `init` | — | — |
| `address` | — | — |
| `quote` | `symbol` | — |
| `news` | `symbol` | — |
| `order_buy` | `symbol`, `amount`, `price` | `exchange` |
| `order_sell` | `symbol`, `amount`, `price` | `exchange` |
| `orders` | — | `symbol`, `exchange` |
| `cancel` | `order_id` | `exchange` |
| `balance` | — | `exchange` |
| `positions` | — | `exchange` |
| `perp_quote` | `symbol` | — |
| `perp_buy` | `symbol`, `amount`, `price` | `close`, `exchange` |
| `perp_sell` | `symbol`, `amount`, `price` | `close`, `exchange` |
| `perp_leverage` | `symbol`, `leverage` | `cross`, `exchange` |
| `perp_set_mode` | `mode` | — |
| `options_buy` | `symbol`, `option_type`, `strike`, `expiry`, `size` | `exchange` |
| `options_sell` | `symbol`, `option_type`, `strike`, `expiry`, `size` | `exchange` |
| `deposit` | `asset` | `amount`, `from`, `exchange`, `dry_run` |
| `withdraw` | `asset`, `amount` | `to`, `network`, `dry_run` |
| `transfer` | `asset`, `amount`, `from`, `to` | — |
| `bridge_status` | — | — |
| `report_annual` | `symbol` | `output` |
| `report_quarterly` | `symbol` | `output` |
| `report_list` | `symbol` | `limit` |
| `report_get` | `symbol`, `accession` | `output` |

**Notes:**
- `exchange` defaults to `"auto"` when omitted
- `amount` and `price` are strings (e.g. `"0.1"`, `"2500.00"`)
- `leverage` is a number (e.g. `10`)
- `close` and `dry_run` are booleans (default `false`)
- `limit` is a number (default `10`)

### Examples

```bash
# Get a price quote
fintool --json '{"command":"quote","symbol":"ETH"}'

# Place a perp buy with leverage
fintool --json '{"command":"perp_leverage","symbol":"ETH","leverage":5}'
fintool --json '{"command":"perp_buy","symbol":"ETH","amount":"0.1","price":"3000"}'

# Close a perp position
fintool --json '{"command":"perp_sell","symbol":"ETH","amount":"0.1","price":"3100","close":true}'

# Transfer USDT0 to cash dex for commodity trading
fintool --json '{"command":"transfer","asset":"USDT0","amount":"30","from":"spot","to":"cash"}'

# Deposit and withdraw
fintool --json '{"command":"deposit","asset":"USDC","amount":"15","from":"base"}'
fintool --json '{"command":"withdraw","asset":"USDC","amount":"10","to":"base"}'

# Check account state
fintool --json '{"command":"balance"}'
fintool --json '{"command":"positions"}'
fintool --json '{"command":"orders"}'
```

---

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `hyperliquid_rust_sdk` | Hyperliquid exchange client with EIP-712 signing |
| `ethers` | Ethereum wallet and signing primitives |
| `reqwest` | HTTP client (rustls TLS — no OpenSSL) |
| `hmac`, `sha2`, `hex` | HMAC-SHA256 signing for Binance and Coinbase APIs |
| `clap` | CLI argument parsing |
| `serde` / `serde_json` | JSON serialization |
| `colored` | Terminal colors (human-readable output) |
| `tabled` | Table formatting (human-readable output) |
| `rust_decimal` | Precise financial math |

## License

MIT

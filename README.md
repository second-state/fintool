# fintool

A suite of Rust CLI tools for agentic trading and market intelligence. Each exchange has its own dedicated binary — **`hyperliquid`**, **`binance`**, **`coinbase`**, **`okx`**, **`polymarket`** — plus a shared **`fintool`** for exchange-agnostic market intelligence (quotes, news, SEC filings). Supports crypto, stocks, commodities, and prediction markets. All CLIs support `--json` mode for scripting and agent integration.

## Table of Contents

- [Install as an OpenClaw Skill](#install-as-an-openclaw-skill)
- [Installation (Manual)](#installation-manual)
- [CLI Overview](#cli-overview)
- [Quick Guides](#quick-guides)
  - [Setup](#setup)
  - [Deposit funds](#deposit-funds)
  - [Withdraw funds](#withdraw-funds)
  - [Get price quotes and news](#get-price-quotes-and-news)
  - [Spot buy and sell](#spot-buy-and-sell)
  - [Open and close perp positions](#open-and-close-perp-positions)
  - [Commodity perp on Hyperliquid (USDT0 conversion)](#commodity-perp-on-hyperliquid-usdt0-conversion)
  - [Prediction market trading (Polymarket)](#prediction-market-trading-polymarket)
- [Configuration](#configuration)
  - [Config Options](#config-options)
  - [What Needs Configuration](#what-needs-configuration)
- [CLIs and Commands](#clis-and-commands)
  - [`fintool` — Market Intelligence](#fintool--market-intelligence)
  - [`hyperliquid` — Hyperliquid Exchange](#hyperliquid--hyperliquid-exchange)
  - [`binance` — Binance Exchange](#binance--binance-exchange)
  - [`coinbase` — Coinbase Exchange](#coinbase--coinbase-exchange)
  - [`okx` — OKX Exchange](#okx--okx-exchange)
  - [`polymarket` — Polymarket Prediction Markets](#polymarket--polymarket-prediction-markets)
- [Common Commands Reference](#common-commands-reference)
  - [`quote`](#quote-symbol)
  - [`buy / sell` (spot)](#buy--sell-spot)
  - [`perp buy / perp sell`](#perp-buy--perp-sell)
  - [`perp leverage`](#perp-leverage)
  - [`orderbook / perp orderbook`](#orderbook--perp-orderbook)
  - [`orders`](#orders)
  - [`cancel`](#cancel-order_id)
  - [`balance`](#balance)
  - [`positions`](#positions)
  - [`deposit`](#deposit-asset)
  - [`withdraw`](#withdraw-asset---amount-amt)
- [Command Summary](#command-summary)
- [Data Sources](#data-sources)
- [JSON Mode](#json-mode)
- [Architecture](#architecture)
- [Key Dependencies](#key-dependencies)
- [License](#license)

## Install as an OpenClaw Skill

Tell your [OpenClaw](https://openclaw.ai) agent:

> Read https://raw.githubusercontent.com/second-state/fintool/refs/heads/main/skills/install.md and install the fintool skill.

The agent will download the correct binaries for your platform, set up the skill, and walk you through configuration.

## Installation (Manual)

```bash
cd fintool
cargo build --release
# Binaries at ./target/release/{fintool,hyperliquid,binance,coinbase,okx,polymarket}
```

Or download pre-built binaries from [Releases](https://github.com/second-state/fintool/releases).

## CLI Overview

| Binary | Purpose | Exchange |
|--------|---------|----------|
| `fintool` | Market intelligence — quotes, news, SEC filings | None (read-only data) |
| `hyperliquid` | Spot + perp + HIP-3 trading, deposits, withdrawals, transfers | Hyperliquid |
| `binance` | Spot + perp trading, deposits, withdrawals | Binance |
| `coinbase` | Spot trading, deposits, withdrawals | Coinbase |
| `okx` | Spot + perp trading, deposits, withdrawals, transfers | OKX |
| `polymarket` | Prediction market trading, deposits, withdrawals | Polymarket (Polygon) |

All CLIs support `--json` mode for programmatic use. See [JSON Mode](#json-mode).

### Exchange Capability Matrix

| Feature | `hyperliquid` | `binance` | `coinbase` | `okx` | `polymarket` |
|---------|---------------|-----------|------------|-------|--------------|
| Spot Trading | buy, sell | buy, sell | buy, sell | buy, sell | — |
| Perpetual Futures | perp buy/sell | perp buy/sell | — | perp buy/sell | — |
| Prediction Markets | — | — | — | — | buy, sell, list, quote |
| Orderbook | spot + perp | spot + perp | spot | spot + perp | — |
| Options | options buy/sell | — | — | — | — |
| Balance | balance | balance | balance | balance | balance |
| Positions | positions | positions | — | positions | positions |
| Orders/Cancel | orders, cancel | orders, cancel | orders, cancel | orders, cancel | — |
| Deposit | deposit | deposit | deposit | deposit | deposit |
| Withdraw | withdraw | withdraw | withdraw | withdraw | withdraw |
| Transfer | transfer | transfer | — | transfer | — |
| Funding Rate | — | — | — | perp funding-rate | — |
| Bridge Status | bridge-status | — | — | — | — |

## Quick Guides

### Setup

```bash
fintool init                    # create config file
vim ~/.fintool/config.toml      # add your wallet key and API keys
```

### Deposit funds

Bridge ETH or USDC to Hyperliquid:

```bash
hyperliquid deposit ETH --amount 0.01          # auto-bridges ETH via Unit
hyperliquid deposit USDC --amount 15 --from base  # bridges USDC via Across (min 5 USDC)
```

For assets that can't be bridged automatically (BTC, SOL), the command shows a deposit address:

```bash
hyperliquid deposit BTC --amount 0.001         # shows Unit deposit address
```

The deposited USDC goes into the Hyperliquid perp margin account. To use it for spot trading as well, set the account to unified mode:

```bash
hyperliquid perp set-mode unified
```

Check your balance:

```bash
hyperliquid balance
```

### Withdraw funds

Withdraw USDC from Hyperliquid back to Base:

```bash
hyperliquid withdraw USDC --amount 10 --to base
```

You can also withdraw to Arbitrum (default, fastest) or Ethereum:

```bash
hyperliquid withdraw USDC --amount 10                      # → Arbitrum (~3-4 min)
hyperliquid withdraw USDC --amount 10 --to ethereum         # → Ethereum (~5-7 min)
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
hyperliquid quote ETH
hyperliquid quote SILVER         # HIP-3 commodity perp
```

View the L2 orderbook (bids/asks, spread, depth):

```bash
hyperliquid orderbook HYPE             # spot orderbook (default 5 levels)
hyperliquid perp orderbook BTC         # perp orderbook
hyperliquid orderbook ETH --levels 20  # more depth
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
hyperliquid buy HYPE --amount 1.0 --price 25.00
```

Check your balance, then sell:

```bash
hyperliquid balance
hyperliquid sell HYPE --amount 0.48 --price 30.00
```

Use different exchange CLIs for different exchanges:

```bash
coinbase buy BTC --amount 0.002 --price 65000
binance buy BTC --amount 0.002 --price 65000
okx buy BTC --amount 0.002 --price 65000
```

### Open and close perp positions

Get the perp quote, set leverage, and open a long position:

```bash
hyperliquid quote ETH
hyperliquid perp leverage ETH --leverage 2
hyperliquid perp buy ETH --amount 0.006 --price 2100.00
```

Check positions and balance:

```bash
hyperliquid positions
hyperliquid balance
```

Close the position with `--close` (reduce-only — won't open a new short):

```bash
hyperliquid perp sell ETH --amount 0.006 --price 2150.00 --close
```

### Commodity perp on Hyperliquid (USDT0 conversion)

The HIP-3 commodity/stock perp market on Hyperliquid (SILVER, GOLD, TSLA, etc.) uses USDT0 as collateral instead of USDC. You need to swap USDC → USDT0 first.

**Buy USDT0 on the spot market and transfer to the HIP-3 dex:**

```bash
hyperliquid buy USDT0 --amount 30 --price 1.002
hyperliquid transfer USDT0 --amount 30 --from spot --to cash
```

**Trade the commodity perp:**

```bash
hyperliquid quote SILVER
hyperliquid perp leverage SILVER --leverage 2
hyperliquid perp buy SILVER --amount 0.13 --price 89.00
```

**Close the position and convert back to USDC:**

```bash
hyperliquid perp sell SILVER --amount 0.14 --price 91.00 --close
hyperliquid transfer USDT0 --amount 30 --from cash --to spot
hyperliquid sell USDT0 --amount 30 --price 0.998
```

Check everything:

```bash
hyperliquid positions
hyperliquid orders
hyperliquid balance
```

### Prediction market trading (Polymarket)

```bash
# List/search prediction markets
polymarket list --query "bitcoin"
polymarket list --query "election" --limit 5

# Only show markets ending 7+ days from now (default: 3)
polymarket list --query "bitcoin" --min-end-days 7

# Show all markets including ones closing today
polymarket list --query "bitcoin" --min-end-days 0

# Get market details/quote
polymarket quote will-bitcoin-hit-100k

# Buy shares (yes outcome at $0.65)
polymarket buy will-bitcoin-hit-100k --outcome yes --amount 10 --price 0.65

# Sell shares
polymarket sell will-bitcoin-hit-100k --outcome yes --amount 10 --price 0.70

# View positions
polymarket positions

# Deposit USDC to Polymarket
polymarket deposit --amount 100 --from base
```

> **Note:** All CLIs support a `--json` mode for scripting and agent integration — pass a full command as a JSON string and get JSON output. See [JSON Mode](#json-mode) for details.

---

## Configuration

Config file: `~/.fintool/config.toml`

Run `fintool init` to generate a template, or copy `config.toml.default` from the release zip.

### Example Configuration

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

# OKX — enables spot/perp trading
okx_api_key = "..."
okx_secret_key = "..."
okx_passphrase = "..."

# Polymarket — prediction market trading on Polygon
# private_key defaults to [wallet] private_key if omitted
[polymarket]
# private_key = "0x..."
# signature_type = "proxy"   # proxy (default), eoa, or gnosis-safe
```

### Config Options

| Section | Key | Type | Default | Description |
|---------|-----|------|---------|-------------|
| `wallet` | `private_key` | string | — | Wallet hex private key (with or without `0x`). Used by Hyperliquid and Polymarket. |
| `wallet` | `wallet_json` | string | — | Path to encrypted Ethereum keystore JSON file. Supports `~` expansion. |
| `wallet` | `wallet_passcode` | string | — | Passcode to decrypt the keystore file. |
| `network` | `testnet` | bool | `false` | Use Hyperliquid testnet. |
| `api_keys` | `openai_api_key` | string | — | OpenAI API key. Enables LLM-enriched quotes with trend/momentum analysis. |
| `api_keys` | `openai_model` | string | `gpt-4.1-mini` | OpenAI model for quote analysis. |
| `api_keys` | `binance_api_key` | string | — | Binance API key for spot/futures/options trading. |
| `api_keys` | `binance_api_secret` | string | — | Binance API secret (HMAC-SHA256 signing). |
| `api_keys` | `coinbase_api_key` | string | — | Coinbase Advanced Trade API key. |
| `api_keys` | `coinbase_api_secret` | string | — | Coinbase Advanced Trade API secret (HMAC-SHA256 signing). |
| `api_keys` | `okx_api_key` | string | — | OKX API key for spot/perp trading. |
| `api_keys` | `okx_secret_key` | string | — | OKX API secret (HMAC-SHA256 signing). |
| `api_keys` | `okx_passphrase` | string | — | OKX API passphrase. |
| `api_keys` | `okx_base_url` | string | `https://www.okx.com` | Custom OKX base URL. Set to `https://app.okx.com` for OKX US. |
| `polymarket` | `signature_type` | string | `proxy` | Polymarket signing mode: `proxy`, `eoa`, or `gnosis-safe`. Uses `wallet.private_key`. |

### What Needs Configuration

| CLI / Command | Wallet Key | Binance Keys | Coinbase Keys | OKX Keys | OpenAI Key |
|---------------|-----------|--------------|---------------|----------|------------|
| `fintool quote` | No | No | No | — | Optional (enriches) |
| `fintool news`, `fintool init` | No | No | No | — | No |
| `fintool report` | No | No | No | — | No |
| `hyperliquid` (all commands) | Yes | — | — | — | — |
| `hyperliquid quote` | No | — | — | — | — |
| `binance` (all commands) | — | Yes | — | — | — |
| `coinbase` (all commands) | — | — | Yes | — | — |
| `okx quote`, `okx orderbook` | — | — | — | No | — |
| `okx` (trading/balance/deposit/withdraw) | — | — | — | Yes | — |
| `polymarket list`, `polymarket quote` | No | — | — | — | — |
| `polymarket` (buy/sell/positions/deposit) | Yes | — | — | — | — |

---

## CLIs and Commands

### `fintool` — Market Intelligence

Exchange-agnostic price data, news, and SEC filings.

| Command | Description |
|---------|-------------|
| `fintool init` | Create config file at `~/.fintool/config.toml` |
| `fintool quote <SYMBOL>` | Multi-source price + LLM analysis |
| `fintool news <SYMBOL>` | Latest news headlines |
| `fintool report annual <SYMBOL>` | SEC 10-K annual filing |
| `fintool report quarterly <SYMBOL>` | SEC 10-Q quarterly filing |
| `fintool report list <SYMBOL>` | List recent SEC filings |
| `fintool report get <SYMBOL> <ACCESSION>` | Fetch specific filing |

#### `fintool quote <SYMBOL>`

Get the current price with multi-source aggregation and optional LLM analysis.

**Data sources** (fetched in parallel):
1. **Hyperliquid spot** — tokenized stocks and crypto
2. **Yahoo Finance** — traditional stocks, indices, commodities
3. **CoinGecko** — crypto prices with 7d/30d trends, market cap

**With OpenAI key configured:** All raw data is sent to the LLM to produce merged analysis with trend direction, momentum, volume context, and a market summary.

**Without OpenAI key:** Returns merged data from the best available source.

##### Symbol Aliases

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

##### JSON Schema — Enriched (with OpenAI)

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
  "momentum": "Bitcoin has declined 4.28% in the last 24 hours...",
  "volume_analysis": "The 24-hour volume of $56.8B indicates significant market activity.",
  "summary": "Bitcoin is in a strong bearish trend with a 27.5% decline over the past month.",
  "sources_used": ["Yahoo Finance", "CoinGecko"],
  "confidence": "high"
}
```

---

### `hyperliquid` — Hyperliquid Exchange

Spot and perpetual futures trading, HIP-3 dex (commodities, stocks), deposits, withdrawals, and transfers.

| Command | Description |
|---------|-------------|
| `hyperliquid address` | Print wallet address |
| `hyperliquid quote <SYMBOL>` | Price quote + funding/OI/premium (alias: `perp quote`) |
| `hyperliquid buy <SYMBOL> --amount N --price P` | Spot limit buy |
| `hyperliquid sell <SYMBOL> --amount N --price P` | Spot limit sell |
| `hyperliquid perp buy <SYM> --amount N --price P [--close]` | Perp long / close short |
| `hyperliquid perp sell <SYM> --amount N --price P [--close]` | Perp short / close long |
| `hyperliquid perp leverage <SYM> --leverage N [--cross]` | Set perp leverage |
| `hyperliquid perp set-mode <MODE>` | Set account mode (unified/standard/disabled) |
| `hyperliquid perp orderbook <SYM> [--levels N]` | Perp L2 orderbook |
| `hyperliquid orderbook <SYMBOL> [--levels N]` | Spot L2 orderbook |
| `hyperliquid orders [SYMBOL]` | List open orders |
| `hyperliquid cancel <ORDER_ID>` | Cancel an order |
| `hyperliquid balance` | Account balances |
| `hyperliquid positions` | Open positions + PnL |
| `hyperliquid options buy/sell ...` | Options trading |
| `hyperliquid deposit <ASSET> --amount N [--from CHAIN]` | Deposit (auto-bridge ETH/USDC, address for BTC/SOL) |
| `hyperliquid withdraw <ASSET> --amount N [--to DST]` | Withdraw (Bridge2 / Unit) |
| `hyperliquid transfer <ASSET> --amount N --from SRC --to DST` | Transfer: perp ↔ spot ↔ dex |
| `hyperliquid bridge-status` | Unit bridge operation status |

#### Hyperliquid-Specific Features

**HIP-3 Perps (commodities, stocks):** The `cash` dex on Hyperliquid supports commodity and stock perps using USDT0 collateral. Symbols like `SILVER`, `GOLD`, `TSLA` are auto-detected and routed to the correct dex.

**Quote:** `hyperliquid quote` (or `hyperliquid perp quote`) returns perpetual futures data including funding rate, open interest, premium, and max leverage.

**Transfer:** Move funds between `spot`, `perp`, `cash` (HIP-3), and other dex accounts. One side must always be `spot`.

**Deposit:** `--amount` is always required. ETH is auto-bridged via Unit (sends from your wallet on Ethereum L1). USDC is bridged via Across (requires `--from`). BTC/SOL cannot be bridged automatically — the command shows a Unit deposit address for manual transfer.

**Bridge Status:** Track HyperUnit bridge operations for ETH/BTC/SOL deposits and withdrawals.

---

### `binance` — Binance Exchange

Spot and perpetual futures trading.

| Command | Description |
|---------|-------------|
| `binance buy <SYMBOL> --amount N --price P` | Spot limit buy |
| `binance sell <SYMBOL> --amount N --price P` | Spot limit sell |
| `binance perp buy <SYM> --amount N --price P [--close]` | Perp long / close short |
| `binance perp sell <SYM> --amount N --price P [--close]` | Perp short / close long |
| `binance perp leverage <SYM> --leverage N [--cross]` | Set perp leverage |
| `binance perp orderbook <SYM> [--levels N]` | Perp L2 orderbook |
| `binance orderbook <SYMBOL> [--levels N]` | Spot L2 orderbook |
| `binance orders [SYMBOL]` | List open orders |
| `binance cancel <ORDER_ID>` | Cancel an order |
| `binance balance` | Account balances |
| `binance positions` | Open positions |
| `binance deposit <ASSET> [--from CHAIN]` | Deposit address |
| `binance withdraw <ASSET> --amount N [--to DST] [--network NET]` | Withdraw |

---

### `coinbase` — Coinbase Exchange

Spot trading only (no perps or options).

| Command | Description |
|---------|-------------|
| `coinbase buy <SYMBOL> --amount N --price P` | Spot limit buy |
| `coinbase sell <SYMBOL> --amount N --price P` | Spot limit sell |
| `coinbase orderbook <SYMBOL> [--levels N]` | Spot L2 orderbook |
| `coinbase orders [SYMBOL]` | List open orders |
| `coinbase cancel <ORDER_ID>` | Cancel an order |
| `coinbase balance` | Account balances |
| `coinbase deposit <ASSET> [--from CHAIN]` | Deposit address |
| `coinbase withdraw <ASSET> --amount N [--to DST] [--network NET]` | Withdraw |

---

### `okx` — OKX Exchange

Spot and perpetual futures trading, deposits, withdrawals, and account transfers.

| Command | Description |
|---------|-------------|
| `okx buy <SYMBOL> --amount N --price P` | Spot limit buy |
| `okx sell <SYMBOL> --amount N --price P` | Spot limit sell |
| `okx perp buy <SYM> --amount N --price P [--close]` | Perp long / close short |
| `okx perp sell <SYM> --amount N --price P [--close]` | Perp short / close long |
| `okx perp leverage <SYM> --leverage N [--cross]` | Set perp leverage |
| `okx perp funding-rate <SYM>` | Get funding rate for a swap |
| `okx perp orderbook <SYM> [--levels N]` | Perp L2 orderbook |
| `okx orderbook <SYMBOL> [--levels N]` | Spot L2 orderbook |
| `okx orders [SYMBOL]` | List open orders |
| `okx cancel --inst-id <ID> <ORDER_ID>` | Cancel an order |
| `okx balance` | Account balances (trading + funding) |
| `okx positions` | Open positions |
| `okx quote <SYMBOL>` | Price quote (no auth required) |
| `okx deposit <ASSET> [--network NET]` | Deposit address |
| `okx withdraw <ASSET> --amount N [--to DST] [--network NET] [--fee F]` | Withdraw |
| `okx transfer <ASSET> --amount N --from SRC --to DST` | Transfer: funding ↔ trading |

#### OKX-Specific Features

**Instrument IDs:** Spot uses `BTC-USDT`, perps use `BTC-USDT-SWAP`. The CLI auto-formats these from the symbol name.

**Account types:** OKX has a `funding` account (for deposits/withdrawals) and a `trading` (unified) account. Use `okx transfer` to move between them.

**Trade modes:** Spot orders use `cash` mode, perp orders use `cross` margin by default. Use `--cross` with leverage to explicitly select cross margin.

**Withdrawal fees:** If `--fee` is not specified, the CLI auto-fetches the minimum withdrawal fee from the OKX API.

**Base URL:** Default is `https://www.okx.com`. Set `okx_base_url` in config for OKX US (`https://app.okx.com`).

---

### `polymarket` — Polymarket Prediction Markets

Prediction market trading on Polygon.

| Command | Description |
|---------|-------------|
| `polymarket list [--query Q] [--limit N] [--min-end-days N]` | Search/browse markets |
| `polymarket quote <MARKET>` | Market details/prices |
| `polymarket buy <MARKET> --outcome O --amount N --price P` | Buy outcome shares |
| `polymarket sell <MARKET> --outcome O --amount N --price P` | Sell outcome shares |
| `polymarket positions` | Open positions |
| `polymarket balance` | USDC balance |
| `polymarket deposit [--amount N] [--from CHAIN]` | Deposit USDC |
| `polymarket withdraw --amount N` | Withdraw USDC |

#### Polymarket-Specific Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--query` | string | *(none)* | Search query for `list` (e.g., "bitcoin", "election") |
| `--limit` | integer | `10` | Max results for `list` |
| `--sort` | string | *(none)* | Sort by `"volume"` or `"liquidity"` |
| `--min-end-days` | integer | `3` | Minimum days before market closes |
| `--outcome` | string | — | Outcome name for buy/sell (e.g., `yes`, `no`) |

---

## Common Commands Reference

These commands work the same across exchange CLIs. The only difference is which binary you use.

### `quote <SYMBOL>`

| CLI | What it returns |
|-----|----------------|
| `fintool quote` | Multi-source spot price + LLM analysis |
| `hyperliquid quote` | Price + funding/OI/premium (alias: `perp quote`) |

### `buy / sell` (spot)

Place a spot limit buy or sell order. `--amount` is in symbol units. `--price` is the limit price.

```bash
hyperliquid buy HYPE --amount 1.0 --price 25.00
binance sell BTC --amount 0.01 --price 67000
coinbase buy ETH --amount 0.5 --price 2000
```

### `perp buy / perp sell`

Place a perpetual futures limit order. Use `--close` for reduce-only.

```bash
hyperliquid perp buy ETH --amount 0.1 --price 2000
hyperliquid perp sell BTC --amount 0.01 --price 70000 --close
binance perp buy ETH --amount 0.1 --price 2000
okx perp buy ETH --amount 0.1 --price 2000
```

### `perp leverage`

Set leverage for a perp asset. Use `--cross` for cross margin (isolated by default).

```bash
hyperliquid perp leverage ETH --leverage 5
hyperliquid perp leverage BTC --leverage 10 --cross
binance perp leverage ETH --leverage 5
okx perp leverage ETH --leverage 5 --cross
```

### `orderbook / perp orderbook`

Show L2 orderbook with bids, asks, spread, and depth. Default: 5 levels.

```bash
hyperliquid orderbook HYPE
hyperliquid perp orderbook BTC --levels 20
binance orderbook ETH
coinbase orderbook BTC
okx orderbook BTC
```

### `orders`

List open orders (spot and perp). Optionally filter by symbol.

```bash
hyperliquid orders
hyperliquid orders BTC
binance orders
okx orders
```

### `cancel <ORDER_ID>`

Cancel an open order.

**Order ID formats by CLI:**

| CLI | Format | Example |
|-----|--------|---------|
| `hyperliquid` | `SYMBOL:OID` | `BTC:91490942` |
| `binance` | `binance_spot:SYMBOL:ID` or `binance_futures:SYMBOL:ID` | `binance_spot:BTCUSDT:12345678` |
| `coinbase` | `coinbase:UUID` | `coinbase:abc123-def456-...` |
| `okx` | `--inst-id INST_ID ORDER_ID` | `okx cancel --inst-id BTC-USDT 12345` |

### `balance`

Show account balances and margin summary.

```bash
hyperliquid balance
binance balance
coinbase balance
okx balance
polymarket balance
```

### `positions`

Show open positions with PnL.

```bash
hyperliquid positions    # includes HIP-3 dex positions
binance positions
okx positions
polymarket positions     # prediction market positions
```

### `deposit <ASSET> --amount <AMT>`

Deposit assets to the exchange. Behavior varies by CLI:

**Hyperliquid:**
```bash
hyperliquid deposit ETH --amount 0.01                # auto-bridge ETH via Unit
hyperliquid deposit USDC --amount 100 --from base    # bridge USDC from Base via Across
hyperliquid deposit BTC --amount 0.001               # shows deposit address (manual)
```

**Binance / Coinbase / OKX:**
```bash
binance deposit ETH --from ethereum
coinbase deposit USDC
okx deposit USDC --network base
```

**Polymarket:**
```bash
polymarket deposit --amount 100 --from base
```

### `withdraw <ASSET> --amount <AMT>`

Withdraw assets from the exchange. Behavior varies by CLI:

**Hyperliquid:**
```bash
hyperliquid withdraw USDC --amount 100               # → Arbitrum (default)
hyperliquid withdraw USDC --amount 100 --to base     # → Base
hyperliquid withdraw ETH --amount 0.5                # → Ethereum (HyperUnit)
```

**Binance / Coinbase / OKX:**
```bash
binance withdraw USDC --amount 100 --to 0x... --network ethereum
coinbase withdraw ETH --amount 0.5 --to 0x...
okx withdraw USDC --amount 100 --network base
```

**Polymarket:**
```bash
polymarket withdraw --amount 100
```

---

## Command Summary

| Command | Description | CLIs |
|---------|-------------|------|
| `init` | Create config file | `fintool` |
| `address` | Print wallet address | `hyperliquid` |
| `quote <SYM>` | Price quote | `fintool` (spot), `hyperliquid` (perp) |
| `news <SYM>` | Latest news headlines | `fintool` |
| `report annual/quarterly/list/get` | SEC filings | `fintool` |
| `buy <SYM> --amount N --price P` | Spot limit buy | `hyperliquid`, `binance`, `coinbase`, `okx` |
| `sell <SYM> --amount N --price P` | Spot limit sell | `hyperliquid`, `binance`, `coinbase`, `okx` |
| `perp buy <SYM> --amount N --price P` | Perp long / close short | `hyperliquid`, `binance`, `okx` |
| `perp sell <SYM> --amount N --price P` | Perp short / close long | `hyperliquid`, `binance`, `okx` |
| `perp leverage <SYM> --leverage N` | Set perp leverage | `hyperliquid`, `binance`, `okx` |
| `perp funding-rate <SYM>` | Funding rate | `okx` |
| `perp set-mode <MODE>` | Account mode | `hyperliquid` |
| `orderbook <SYM>` | Spot L2 orderbook | `hyperliquid`, `binance`, `coinbase`, `okx` |
| `perp orderbook <SYM>` | Perp L2 orderbook | `hyperliquid`, `binance`, `okx` |
| `orders [SYM]` | List open orders | `hyperliquid`, `binance`, `coinbase`, `okx` |
| `cancel <ORDER_ID>` | Cancel an order | `hyperliquid`, `binance`, `coinbase`, `okx` |
| `balance` | Account balances | `hyperliquid`, `binance`, `coinbase`, `okx`, `polymarket` |
| `positions` | Open positions + PnL | `hyperliquid`, `binance`, `okx`, `polymarket` |
| `options buy/sell ...` | Options trading | `hyperliquid` |
| `deposit <ASSET>` | Deposit to exchange | `hyperliquid`, `binance`, `coinbase`, `okx`, `polymarket` |
| `withdraw <ASSET> --amount N` | Withdraw from exchange | `hyperliquid`, `binance`, `coinbase`, `okx`, `polymarket` |
| `transfer <ASSET> --amount N` | Transfer between accounts | `hyperliquid`, `okx` |
| `bridge-status` | Unit bridge status | `hyperliquid` |
| `list [--query Q]` | Search prediction markets | `polymarket` |
| `quote <MARKET>` | Market details/prices | `polymarket` |
| `buy <MARKET> --outcome O ...` | Buy prediction shares | `polymarket` |
| `sell <MARKET> --outcome O ...` | Sell prediction shares | `polymarket` |

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
| Prediction markets — Polymarket | Polymarket Gamma + CLOB APIs | Wallet key (for trading) | EIP-712 signing |
| Trading — Coinbase spot | Coinbase Advanced Trade API | API key + secret | HMAC-SHA256 signing |
| Trading — OKX spot + perps | OKX API `/api/v5/trade/order` | API key + secret + passphrase | HMAC-SHA256 + base64 signing |
| Quotes — OKX | OKX Public API `/api/v5/market/ticker` | No | No auth for public endpoints |
| Deposit/Withdraw — HyperUnit bridge | HyperUnit API | Wallet private key | ETH, BTC, SOL ↔ Hyperliquid |
| Deposit — USDC cross-chain bridge | Across Protocol API | Wallet private key | Ethereum/Base → Arbitrum → HL |

## JSON Mode

For scripts, bots, and programmatic use, pass the entire command as a JSON string via the `--json` flag. In this mode, **all output is JSON** (including errors).

Each CLI has its own JSON command set — no `exchange` field needed.

```bash
# Market intelligence
fintool --json '{"command":"quote","symbol":"BTC"}'
fintool --json '{"command":"news","symbol":"ETH"}'

# Hyperliquid trading
hyperliquid --json '{"command":"buy","symbol":"HYPE","amount":1.0,"price":25.00}'
hyperliquid --json '{"command":"perp_buy","symbol":"ETH","amount":0.1,"price":3000}'
hyperliquid --json '{"command":"perp_leverage","symbol":"ETH","leverage":5}'
hyperliquid --json '{"command":"balance"}'
hyperliquid --json '{"command":"transfer","asset":"USDT0","amount":30,"from":"spot","to":"cash"}'
hyperliquid --json '{"command":"deposit","asset":"ETH","amount":0.01}'
hyperliquid --json '{"command":"deposit","asset":"USDC","amount":15,"from":"base"}'

# Binance trading
binance --json '{"command":"buy","symbol":"BTC","amount":0.002,"price":65000}'
binance --json '{"command":"perp_sell","symbol":"ETH","amount":0.1,"price":3100,"close":true}'
binance --json '{"command":"balance"}'

# Coinbase trading
coinbase --json '{"command":"buy","symbol":"ETH","amount":0.5,"price":2000}'
coinbase --json '{"command":"balance"}'

# OKX trading
okx --json '{"command":"quote","symbol":"BTC"}'
okx --json '{"command":"buy","symbol":"ETH","amount":0.01,"price":2000}'
okx --json '{"command":"perp_buy","symbol":"BTC","amount":0.001,"price":60000}'
okx --json '{"command":"balance"}'
okx --json '{"command":"transfer","asset":"USDT","amount":100,"from":"funding","to":"trading"}'

# Polymarket prediction markets
polymarket --json '{"command":"list","query":"bitcoin"}'
polymarket --json '{"command":"buy","market":"will-btc-hit-100k","outcome":"yes","amount":20,"price":0.50}'
polymarket --json '{"command":"positions"}'
```

Errors are returned as JSON too:

```json
{"error": "Invalid JSON command: missing field `symbol`"}
```

### JSON Command Schema by CLI

#### `fintool`

| `command` | Required fields | Optional fields |
|-----------|----------------|-----------------|
| `init` | — | — |
| `quote` | `symbol` | — |
| `news` | `symbol` | — |
| `report_annual` | `symbol` | `output` |
| `report_quarterly` | `symbol` | `output` |
| `report_list` | `symbol` | `limit` |
| `report_get` | `symbol`, `accession` | `output` |

#### `hyperliquid`

| `command` | Required fields | Optional fields |
|-----------|----------------|-----------------|
| `address` | — | — |
| `buy` | `symbol`, `amount`, `price` | — |
| `sell` | `symbol`, `amount`, `price` | — |
| `orderbook` | `symbol` | `levels` |
| `orders` | — | `symbol` |
| `cancel` | `order_id` | — |
| `balance` | — | — |
| `positions` | — | — |
| `quote` / `perp_quote` | `symbol` | — |
| `perp_orderbook` | `symbol` | `levels` |
| `perp_buy` | `symbol`, `amount`, `price` | `close` |
| `perp_sell` | `symbol`, `amount`, `price` | `close` |
| `perp_leverage` | `symbol`, `leverage` | `cross` |
| `perp_set_mode` | `mode` | — |
| `options_buy` | `symbol`, `option_type`, `strike`, `expiry`, `size` | — |
| `options_sell` | `symbol`, `option_type`, `strike`, `expiry`, `size` | — |
| `deposit` | `asset`, `amount` | `from`, `dry_run` |
| `withdraw` | `asset`, `amount` | `to`, `network`, `dry_run` |
| `transfer` | `asset`, `amount`, `from`, `to` | — |
| `bridge_status` | — | — |

#### `binance`

| `command` | Required fields | Optional fields |
|-----------|----------------|-----------------|
| `buy` | `symbol`, `amount`, `price` | — |
| `sell` | `symbol`, `amount`, `price` | — |
| `orderbook` | `symbol` | `levels` |
| `orders` | — | `symbol` |
| `cancel` | `order_id` | — |
| `balance` | — | — |
| `positions` | — | — |
| `perp_orderbook` | `symbol` | `levels` |
| `perp_buy` | `symbol`, `amount`, `price` | `close` |
| `perp_sell` | `symbol`, `amount`, `price` | `close` |
| `perp_leverage` | `symbol`, `leverage` | `cross` |
| `deposit` | `asset` | `amount`, `from`, `dry_run` |
| `withdraw` | `asset`, `amount` | `to`, `network`, `dry_run` |

#### `coinbase`

| `command` | Required fields | Optional fields |
|-----------|----------------|-----------------|
| `buy` | `symbol`, `amount`, `price` | — |
| `sell` | `symbol`, `amount`, `price` | — |
| `orderbook` | `symbol` | `levels` |
| `orders` | — | `symbol` |
| `cancel` | `order_id` | — |
| `balance` | — | — |
| `deposit` | `asset` | `amount`, `from`, `dry_run` |
| `withdraw` | `asset`, `amount` | `to`, `network`, `dry_run` |

#### `okx`

| `command` | Required fields | Optional fields |
|-----------|----------------|-----------------|
| `buy` | `symbol`, `amount`, `price` | — |
| `sell` | `symbol`, `amount`, `price` | — |
| `orderbook` | `symbol` | `levels` |
| `orders` | — | `symbol` |
| `cancel` | `inst_id`, `order_id` | — |
| `balance` | — | — |
| `positions` | — | — |
| `quote` | `symbol` | — |
| `perp_orderbook` | `symbol` | `levels` |
| `perp_buy` | `symbol`, `amount`, `price` | `close` |
| `perp_sell` | `symbol`, `amount`, `price` | `close` |
| `perp_leverage` | `symbol`, `leverage` | `cross` |
| `perp_funding_rate` | `symbol` | — |
| `deposit` | `asset` | `network` |
| `withdraw` | `asset`, `amount` | `to`, `network`, `fee` |
| `transfer` | `asset`, `amount`, `from`, `to` | — |

#### `polymarket`

| `command` | Required fields | Optional fields |
|-----------|----------------|-----------------|
| `list` | — | `query`, `limit`, `active`, `sort`, `min_end_days` |
| `quote` | `market` | — |
| `buy` | `market`, `outcome`, `amount`, `price` | — |
| `sell` | `market`, `outcome`, `amount`, `price` | — |
| `positions` | — | — |
| `balance` | — | — |
| `deposit` | — | `amount`, `from`, `dry_run` |
| `withdraw` | `amount` | `dry_run` |

**Notes:**
- `amount` and `price` are numbers (e.g. `0.1`, `2500.00`)
- `leverage` is a number (e.g. `10`)
- `close` and `dry_run` are booleans (default `false`)
- `limit` is a number (default `10`)
- `min_end_days` is a number (default `3`)

---

## Architecture

```
fintool/
├── src/
│   ├── lib.rs          # Library crate — module re-exports, shared helpers
│   ├── bin/
│   │   ├── fintool.rs      # CLI: market intelligence (quote, news, report)
│   │   ├── hyperliquid.rs  # CLI: Hyperliquid exchange
│   │   ├── binance.rs      # CLI: Binance exchange
│   │   ├── coinbase.rs     # CLI: Coinbase exchange
│   │   ├── okx.rs          # CLI: OKX exchange
│   │   └── polymarket.rs   # CLI: Polymarket prediction markets
│   ├── config.rs        # Config loading (~/.fintool/config.toml)
│   ├── signing.rs       # Hyperliquid wallet signing, asset resolution, order execution
│   ├── hip3.rs          # HIP-3 builder-deployed perps: EIP-712 signing
│   ├── binance.rs       # Binance API client (spot/futures/options, HMAC-SHA256)
│   ├── coinbase.rs      # Coinbase Advanced Trade API client (HMAC-SHA256)
│   ├── okx.rs           # OKX API client (HMAC-SHA256 + base64)
│   ├── bridge.rs        # Across Protocol cross-chain USDC bridge
│   ├── unit.rs          # HyperUnit bridge (ETH/BTC/SOL deposit/withdraw)
│   ├── polymarket.rs    # Polymarket SDK client helpers
│   ├── format.rs        # Color formatting + number formatting helpers
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
│       ├── options.rs   # Options trading
│       ├── orderbook.rs # L2 orderbooks
│       ├── deposit.rs   # Multi-exchange deposit
│       ├── withdraw.rs  # Multi-exchange withdraw
│       ├── transfer.rs  # Spot ↔ perp ↔ dex transfers (Hyperliquid)
│       ├── predict.rs   # Prediction market commands (Polymarket)
│       └── bridge_status.rs # HyperUnit bridge tracker
├── tests/
│   ├── helpers.sh       # Shell test utilities
│   ├── hyperliquid/     # E2E tests for Hyperliquid
│   ├── binance/         # E2E tests for Binance
│   ├── okx/             # E2E tests for OKX
│   └── polymarket/      # E2E tests for Polymarket
├── examples/
│   ├── funding_arb/     # Funding rate arbitrage bot
│   └── metal_pair/      # Metal pairs trading bot
├── skills/
│   ├── SKILL.md         # OpenClaw skill reference
│   └── install.md       # OpenClaw install instructions
├── Cargo.toml
└── README.md
```

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `hyperliquid_rust_sdk` | Hyperliquid exchange client with EIP-712 signing |
| `ethers` | Ethereum wallet and signing primitives |
| `reqwest` | HTTP client (rustls TLS — no OpenSSL) |
| `hmac`, `sha2`, `hex`, `base64` | HMAC-SHA256 signing for Binance, Coinbase, and OKX APIs |
| `clap` | CLI argument parsing |
| `serde` / `serde_json` | JSON serialization |
| `colored` | Terminal colors (human-readable output) |
| `tabled` | Table formatting (human-readable output) |
| `rust_decimal` | Precise financial math |
| `polymarket-client-sdk` | Polymarket CLOB, Gamma, Data, and Bridge API clients |
| `alloy` | Ethereum primitives and signing for Polymarket integration |

## License

MIT

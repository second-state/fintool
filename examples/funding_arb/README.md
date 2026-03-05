# Funding Rate Arbitrage Bot

Delta-neutral funding rate arbitrage: **buy spot + short perp** on the Hyperliquid asset with the highest positive funding rate. Collect hourly funding payments while staying market-neutral.

## Files

| File | Description |
|------|-------------|
| `bot.sh` | Uses the **human CLI API** — standard fintool commands with human-readable output |
| `bot_json.sh` | Uses the **JSON API** — all fintool calls via `--json` with structured JSON output |
| `README.md` | This file |

Both scripts implement the same strategy. Choose `bot.sh` for readable logs and terminal output, or `bot_json.sh` for programmatic/agent-driven execution.

## Strategy

Every hour (matching Hyperliquid's funding interval), the bot:

1. **Scans funding rates** across 13 assets available on both spot and perp markets
2. **Filters** for positive funding rate (> 0.01%/hr) and minimum volume ($1M 24h)
3. **Fetches spot orderbook depth** to assess spread and liquidity
4. **Analyzes candidates via OpenAI** — picks the best asset considering funding magnitude, spot liquidity, spread, and risk
5. **Opens a delta-neutral position** — 50% spot buy + 50% perp short (1x leverage)
6. **Monitors hourly** — if funding turns negative, closes everything back to USDC
7. **Repeats** — waits for the next positive funding opportunity

### Supported Assets

Assets on both Hyperliquid spot and perp markets:

| Spot Ticker | Perp Ticker | Notes |
|------------|-------------|-------|
| HYPE | HYPE | Highest liquidity |
| PUMP | PUMP | High volume |
| XRP1 | XRP | Major asset |
| LINK0 | LINK | Major asset |
| AVAX0 | AVAX | Major asset |
| AAVE0 | AAVE | Major asset |
| BNB0 | BNB | Major asset |
| XMR1 | XMR | Privacy coin |
| PURR | PURR | HL native |
| TRUMP | TRUMP | Meme/political |
| BERA | BERA | Berachain |
| MON | MON | Monad |
| ANIME | ANIME | Animecoin |

## Setup

### 1. Configure fintool

```bash
vim ~/.fintool/config.toml
```

Required config:
```toml
[wallet]
private_key = "0x..."

[api_keys]
openai_api_key = "sk-..."
```

### 2. Fund your account

```bash
fintool deposit USDC --amount 100 --from base
fintool perp set-mode unified
```

### 3. Environment variables (optional overrides)

```bash
export FINTOOL=/path/to/fintool          # default: ./target/release/fintool
export OPENAI_API_KEY=sk-...             # default: from ~/.fintool/config.toml
```

### 4. Run (dry run first!)

```bash
# Dry run — scans and logs what it would do, no trades
./bot.sh --dry-run            # human CLI version
./bot_json.sh --dry-run       # JSON API version

# Live trading
./bot.sh

# Custom check interval (e.g., every 30 min)
./bot.sh --interval 1800
```

## Configuration

Tunable parameters at the top of `bot.sh` / `bot_json.sh`:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `CHECK_INTERVAL` | 3600 | Seconds between checks (1hr = funding interval) |
| `SLIPPAGE_BPS` | 50 | Limit order buffer in basis points (0.5%) |
| `MIN_FUNDING` | 0.0001 | Minimum hourly funding rate to enter (0.01%) |
| `MIN_VOLUME` | 1000000 | Minimum 24h perp volume in USD |
| `LEVERAGE` | 1 | Perp leverage (1x for delta neutral) |
| `POSITION_PCT` | 90 | % of available USDC to deploy (10% buffer) |

## Logs

All activity logged to `/tmp/funding_arb.log`. Each cycle logs:
- Account state (positions, USDC balance)
- Candidate assets with funding rates and spot liquidity
- OpenAI analysis and pick reasoning
- Order execution details
- Position monitoring results

## How It Works

```
┌─────────────────────────────────────────────┐
│              Every 1 Hour                    │
├─────────────────────────────────────────────┤
│                                             │
│  Has positions?                             │
│  ├─ YES → Check funding rate                │
│  │   ├─ Still positive → HOLD ✅            │
│  │   └─ Turned negative → CLOSE ALL → USDC │
│  │                                          │
│  └─ NO → Scan all 13 assets                 │
│      ├─ Filter: funding > 0, vol > $1M      │
│      ├─ Fetch spot depth & spread            │
│      ├─ OpenAI picks best candidate          │
│      └─ Open: buy spot + short perp          │
│                                             │
└─────────────────────────────────────────────┘
```

## Risk

This is a **delta-neutral** strategy — you're not betting on price direction, just collecting funding. However:

- **Spot-perp basis risk** — prices can diverge temporarily; forced unwind during dislocation = loss
- **Spot liquidity** — perp markets are much deeper; spot slippage is the main cost
- **Funding is variable** — rates change hourly and can flip quickly
- **Rotation cost** — switching assets costs spread + fees on 4 orders (spot sell + perp close + spot buy + perp open)
- **Smart contract risk** — Hyperliquid is onchain; standard DeFi risks apply
- This is NOT financial advice

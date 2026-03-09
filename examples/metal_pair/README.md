# Metal Pairs Trading Bot

Daily pairs trading bot: **long one metal, short the other** (GOLD vs SILVER) on Hyperliquid HIP-3 perps.

## Strategy

Each day the bot:

1. **Searches news** via Brave API for GOLD and SILVER headlines
2. **Analyzes sentiment** via OpenAI — which metal is more bullish/bearish
3. **Checks 24h price momentum** via Hyperliquid perp data
4. **Checks funding rates** on Hyperliquid perps
5. **Decides** which to long and which to short (or HOLD if signals are flat)
6. **Closes** all existing positions — returns everything to USDT0
7. **Normalizes USDT0 to exactly $50** — sells excess to USDC or buys more (auto-bridges from Base if USDC insufficient)
8. **Transfers $50 USDT0 to HIP-3 dex**
9. **Opens** two $50 notional positions at 2x leverage ($25 margin each = $50 total)

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
hyperliquid deposit USDC --amount 100 --from base
hyperliquid perp set-mode unified
```

### 3. Environment variables (optional overrides)

```bash
export FINTOOL=/path/to/fintool              # default: ./target/release/fintool
export HYPERLIQUID=/path/to/hyperliquid      # default: ./target/release/hyperliquid
export OPENAI_API_KEY=...                    # default: from ~/.fintool/config.toml
export BRAVE_API_KEY=...                     # default: from ~/.fintool/config.toml
```

### 4. Run manually

```bash
# Dry run — logs what it would do, no trades
python3 bot.py --dry-run

# Live trading
python3 bot.py

# Custom settings
python3 bot.py --target-usdt0 100 --position-size 100 --leverage 3
```

### 5. Schedule daily via cron

```bash
# Run at 9:00 AM CT every day
0 9 * * * cd /path/to/examples/metal_pair && python3 bot.py >> logs/cron.log 2>&1
```

## Configuration

All parameters are configurable via command-line flags:

| Flag | Default | Description |
|------|---------|-------------|
| `--target-usdt0` | 50 | Target USDT0 margin for both legs |
| `--position-size` | 50 | Notional per leg in USD |
| `--leverage` | 2 | Leverage multiplier |
| `--fintool` | `target/release/fintool` | Path to fintool binary (market intelligence) |
| `--hyperliquid` | `target/release/hyperliquid` | Path to hyperliquid binary (trading) |
| `--log-dir` | `logs/` | Log directory |
| `--dry-run` | off | Log actions without executing trades |

## Dependencies

- **Python 3.10+** (no third-party packages required — uses only stdlib)
- **fintool** CLI binary (market intelligence: quotes, news)
- **hyperliquid** CLI binary (trading: orders, positions, balances, transfers)

## Logs

Daily logs in `logs/YYYY-MM-DD.log`. Each run logs:
- News headlines fetched
- Sentiment analysis results
- Price quotes and funding rates
- Trading decision with reasoning
- Order execution details
- Final position verification

## How USDT0 & Rebalancing Works

HIP-3 commodity perps (GOLD, SILVER, etc.) use USDT0 as collateral, not USDC.

On every run, the bot resets to a clean state:

```
1. Close all positions → USDT0 freed in HIP-3 dex
2. Transfer all USDT0 from dex → spot
3. Normalize spot USDT0 to exactly $50:
   - Excess USDT0 → sell for USDC
   - Deficit → buy USDT0 with USDC
   - No USDC? → auto-bridge from Base
4. Transfer $50 USDT0 → HIP-3 dex
5. Open long + short ($50 notional each @ 2x = $25 margin each = $50 total)
```

## Risk

This is a **market-neutral pairs trade** — you're betting on relative performance, not absolute direction. However:
- Slippage and funding costs eat into returns
- Both metals can move against you if correlation breaks
- HIP-3 liquidity may be thin for large sizes
- This is NOT financial advice

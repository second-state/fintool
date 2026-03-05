# Metal Pairs Trading Bot

Daily pairs trading bot: **long one metal, short the other** (GOLD vs SILVER) on Hyperliquid HIP-3 perps.

## Files

| File | Description |
|------|-------------|
| `bot.sh` | Uses the **human CLI API** — standard fintool commands with human-readable output |
| `bot_json.sh` | Uses the **JSON API** — all fintool calls via `--json` with structured JSON output |
| `README.md` | This file |

Both scripts implement the same strategy. Choose `bot.sh` for readable logs and terminal output, or `bot_json.sh` for programmatic/agent-driven execution.

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
# Ensure wallet private key is set
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
export FINTOOL=/path/to/fintool          # default: ~/clawd/fintool-bin
export TARGET_USDT0=50                   # default: $50 USDT0 maintained as margin
export POSITION_SIZE_USD=50              # default: $50 notional per leg
export LEVERAGE=2                        # default: 2x
export BRAVE_API_KEY=...                 # default: from config
export OPENAI_API_KEY=...               # default: from ~/.fintool/config.toml
```

### 4. Run manually

```bash
./bot.sh          # human CLI — readable terminal output
./bot_json.sh     # JSON API — structured JSON output
```

### 5. Schedule daily via cron

```bash
# Run at 9:00 AM CT every day (human CLI version)
0 9 * * * cd /path/to/metal-pairs-bot && ./bot.sh >> logs/cron.log 2>&1

# Or use the JSON API version for programmatic log parsing
0 9 * * * cd /path/to/metal-pairs-bot && ./bot_json.sh >> logs/cron.log 2>&1
```

## API Differences

### bot.sh (Human CLI)

Uses standard fintool commands. Action output is human-readable (colored, formatted). Data extraction (prices, funding rates, positions) queries the Hyperliquid API directly via curl.

```bash
# Actions use fintool CLI
$FINTOOL perp buy GOLD --amount 0.01 --price 5200
$FINTOOL transfer USDT0 --amount 50 --from spot --to cash
$FINTOOL positions

# Data extraction uses Hyperliquid API
curl -s https://api.hyperliquid.xyz/info -d '{"type":"metaAndAssetCtxs","dex":"cash"}'
```

### bot_json.sh (JSON API)

All fintool calls use `--json`. Output is always structured JSON, parsed with jq.

```bash
# All calls use --json
ft '{"command":"perp_buy","symbol":"GOLD","amount":"0.01","price":"5200"}'
ft '{"command":"transfer","asset":"USDT0","amount":"50","from":"spot","to":"cash"}'
ft '{"command":"positions"}'
```

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

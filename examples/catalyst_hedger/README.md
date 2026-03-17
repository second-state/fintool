# Catalyst Hedger

A Python script that automatically hedges perpetual futures positions using prediction market bets on specific catalysts. Uses OpenAI to estimate event probabilities from news and adjusts hedge positions hourly.

## How It Works

```
                    ┌─────────────┐
                    │  News APIs  │
                    │(Brave Search│
                    └──────┬──────┘
                           │
                           ▼
              ┌────────────────────────┐
              │   OpenAI (gpt-4o)      │
              │   Estimate probability │
              │   of each catalyst     │
              └────────────┬───────────┘
                           │
                    ┌──────▼──────┐
                    │  Compare    │
                    │  AI prob vs │
                    │  Polymarket │
                    └──────┬──────┘
                           │
              ┌────────────▼───────────┐
              │   Hedge Math           │
              │   Shares = (N×L×D)     │
              │           / (1-P)      │
              │   Adjust by coverage   │
              │   & edge signal        │
              └────────────┬───────────┘
                           │
              ┌────────────▼───────────┐
              │   fintool CLI commands │
              │   polymarket buy/sell  │
              │   hyperliquid perp ... │
              └────────────────────────┘
```

Every hour, the script:

1. **Checks the perp position** — if the configured position doesn't exist yet, it opens it automatically (sets leverage, places the order)
2. **Fetches news** for each catalyst via Brave Search
3. **Asks OpenAI** to estimate the probability of each catalyst firing and the expected asset drawdown
4. **Fetches Polymarket prices** for each catalyst market
5. **Computes the optimal hedge** — how many prediction market shares to hold
6. **Generates fintool CLI commands** to rebalance (buy/sell Polymarket shares)
7. **Logs everything** to `hedger_log.jsonl`

## Prerequisites

- Python 3.10+
- [fintool](https://github.com/second-state/fintool) installed and configured (`hyperliquid`, `polymarket` binaries in PATH)
- OpenAI API key
- Brave Search API key (optional — has a fallback)

## Installation

```bash
pip install requests

# Ensure fintool CLIs are available
which hyperliquid polymarket fintool
```

## Configuration

Edit `hedger_config.json`:

```json
{
  "perp": {
    "exchange": "hyperliquid",
    "asset": "BTC",
    "side": "long",
    "size": 0.5,
    "entry_price": 95000,
    "leverage": 2.0
  },
  "catalysts": [
    {
      "name": "Fed rate hike March 2026",
      "polymarket_slug": "will-the-fed-cut-rates-in-march",
      "news_query": "Federal Reserve rate decision March 2026 FOMC",
      "expected_drawdown": 0.15,
      "coverage": 1.0,
      "max_premium_pct": 0.05,
      "current_shares": 0
    }
  ],
  "rebalance_threshold": 0.1,
  "dry_run": true,
  "log_file": "hedger_log.jsonl"
}
```

### Config Fields

#### `perp` — Your perpetual futures position

| Field | Description | Example |
|---|---|---|
| `exchange` | fintool exchange CLI name | `hyperliquid`, `binance`, `okx` |
| `asset` | Asset symbol | `BTC`, `ETH`, `SOL` |
| `side` | Position direction | `long` or `short` |
| `size` | Position size in asset units | `0.5` (= 0.5 BTC) |
| `entry_price` | Target entry price | `95000` |
| `leverage` | Leverage multiplier | `2.0` |

#### `catalysts[]` — Events to hedge against

| Field | Description | Example |
|---|---|---|
| `name` | Human-readable label | `"Fed rate hike March 2026"` |
| `polymarket_slug` | Polymarket market slug | `"will-the-fed-cut-rates-in-march"` |
| `news_query` | Search query for news gathering | `"Federal Reserve FOMC March 2026"` |
| `expected_drawdown` | Default expected % drop if catalyst fires | `0.15` (= 15%) |
| `coverage` | Fraction of loss to hedge (0.0–1.0) | `1.0` (= full hedge) |
| `max_premium_pct` | Max hedge cost as % of notional | `0.05` (= 5%) |
| `current_shares` | Polymarket shares currently held | `0` (update after trades) |

#### Top-level settings

| Field | Description | Default |
|---|---|---|
| `rebalance_threshold` | Only rebalance if position off by this fraction | `0.1` (10%) |
| `dry_run` | If true, print commands but don't execute | `true` |
| `log_file` | Path to JSONL log file | `"hedger_log.jsonl"` |

## Usage

### Dry Run (recommended first)

See what the script would do without executing any trades:

```bash
export OPENAI_API_KEY=sk-...
export BRAVE_API_KEY=...  # optional

python3 catalyst_hedger.py --config hedger_config.json
```

### Live Execution

Actually execute the fintool commands:

```bash
python3 catalyst_hedger.py --config hedger_config.json --execute
```

### Continuous Hourly Loop

Run continuously, rebalancing every hour:

```bash
python3 catalyst_hedger.py --config hedger_config.json --loop
```

Custom interval (e.g., every 30 minutes):

```bash
python3 catalyst_hedger.py --config hedger_config.json --loop --interval 1800
```

## How the AI Edge Detection Works

The script compares OpenAI's probability estimate with the Polymarket price:

| Condition | Interpretation | Coverage Adjustment |
|---|---|---|
| AI prob > market + 5% | Hedge is **cheap** (market underprices risk) | Coverage × 1.5 (up to 100%) |
| AI prob < market − 10% | Hedge is **expensive** (market overprices risk) | Coverage × 0.5 |
| Within band | No strong edge | Use default coverage |
| AI confidence = "low" | Unreliable estimate | Don't rebalance |

## Hedging Math

**Core formula:**

```
Shares needed = (Notional × Leverage × Drawdown) / (1 - Polymarket price)
Premium cost  = Shares × Polymarket price
Premium %     = Premium cost / Notional
```

**Example:** Long 0.5 BTC at $95k (2x leverage), hedging a 15% drawdown catalyst at $0.20 on Polymarket:

```
Loss if catalyst = $95,000 × 0.5 × 2 × 0.15 = $14,250
Shares needed    = $14,250 / (1 - 0.20)       = 17,813
Premium cost     = 17,813 × $0.20              = $3,563
Premium %        = $3,563 / $47,500            = 7.5%
```

## Perp Position Auto-Setup

On each run, the script checks whether the configured perp position exists:

```
$ hyperliquid positions --json
```

- **Position exists** → proceeds to hedge calculation
- **Position doesn't exist** → automatically opens it:
  1. Sets leverage: `hyperliquid perp leverage BTC --leverage 2`
  2. Opens position: `hyperliquid perp buy BTC --amount 0.5 --price 95000`
- **Position check fails** → aborts the run (won't hedge a non-existent position)

In dry-run mode, it prints the commands it would execute but doesn't actually trade.

## Log File

Every run appends a JSON record to `hedger_log.jsonl`:

```json
{
  "timestamp": "2026-03-10T19:00:00+00:00",
  "perp": {"exchange": "hyperliquid", "asset": "BTC", "side": "long", "size": 0.5, ...},
  "hedges": [
    {
      "catalyst": "Fed rate hike March 2026",
      "market_price": 0.20,
      "ai_probability": 0.28,
      "ai_drawdown": 0.15,
      "ai_confidence": "medium",
      "ai_reasoning": "Recent hawkish Fed comments and CPI data suggest...",
      "edge": 0.08,
      "shares_target": 21375,
      "shares_delta": 21375,
      "premium_pct": 0.045,
      "action": "buy",
      "commands": [{"command": "polymarket buy ...", "status": "ok"}]
    }
  ]
}
```

Use this for backtesting, auditing, and tracking hedge performance over time.

## Example Output

```
======================================================================
CATALYST HEDGER — 2026-03-10T19:00:00+00:00
======================================================================
Perp: LONG 0.5 BTC @ $95,000 (2.0x) on hyperliquid
Notional: $47,500 | Effective exposure: $95,000
Mode: DRY RUN

--- Perp Position Check ---
  Checking hyperliquid positions...
  ✓ Found existing position: long 0.5 BTC

--- Catalyst: Fed rate hike March 2026 ---
  Fetching Polymarket price for 'will-the-fed-cut-rates-in-march'...
  Market YES price: $0.2000
  Fetching news for 'Federal Reserve rate decision March 2026 FOMC'...
  Found 8 articles
  Estimating probability via OpenAI...
  AI estimate: 28.0% (market: 20.0%) [medium]
  Reasoning: Recent CPI data came in hot, and two Fed governors...
  Edge: +8.0% (CHEAP)
  Target shares: 21,375 (current: 0, delta: +21,375)
  Premium: $4,275 (4.5% of notional)
  Action: BUY
  [DRY RUN] polymarket buy will-the-fed-cut-rates-in-march --outcome yes --amount 21375 --price 0.2000

======================================================================
TOTAL HEDGE PREMIUM: 4.5% of notional
======================================================================
```

## Tips

- **Start with dry run.** Always review the output before enabling `--execute`.
- **Update `current_shares`** in the config after trades execute, or automate it by querying `polymarket positions --json`.
- **Multiple catalysts** are independent — the script hedges each one separately. Total premium is the sum.
- **Premium caps** prevent overspending. If a hedge would cost more than `max_premium_pct`, the position is scaled down.
- **Low-confidence AI estimates** default to market consensus and skip rebalancing to avoid overtrading.

## License

MIT

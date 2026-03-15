# Backtest Strategy Examples

Historical trading simulations with forward PnL analysis. Each script simulates a multi-leg trade at a specific historical date and shows what the PnL would have been at +1, +2, +4, and +7 days.

## Available Scenarios

| Script | Date | Strategy | Assets | Outcome |
|--------|------|----------|--------|---------|
| `covid_crash_hedge.py` | 2020-02-21 | Flight-to-safety | Long GOLD + Short SP500 | SP500 fell 12% in 7 days; short leg dominated |
| `ftx_crypto_contagion.py` | 2022-11-08 | Crypto contagion hedge | Short BTC + Short ETH + Long GOLD | BTC -15%, ETH -18%; crypto shorts drove profit |
| `nvda_earnings_alpha.py` | 2023-05-25 | Sector alpha pair | Long NVDA + Short SP500 | NVDA +5% while SP500 flat; pure alpha |
| `ukraine_oil_shock.py` | 2022-02-24 | Commodity supply shock | Long OIL + Long GOLD + Short SP500 | Oil surged 16% in 7 days on sanctions |
| `svb_crypto_haven.py` | 2023-03-13 | Crypto as digital gold | Long BTC + Long ETH + Long GOLD + Short SP500 | BTC +15%, portfolio +4.8% in 7 days on banking crisis |

## Setup

No API keys or wallet configuration needed. Just build the backtest binary:

```bash
cargo build --release
```

## Usage

```bash
# Run any scenario
python3 examples/backtest/covid_crash_hedge.py
python3 examples/backtest/ftx_crypto_contagion.py
python3 examples/backtest/nvda_earnings_alpha.py
python3 examples/backtest/ukraine_oil_shock.py
python3 examples/backtest/svb_crypto_haven.py

# Override binary path
python3 examples/backtest/covid_crash_hedge.py --backtest /path/to/backtest
```

Each script:
1. Resets the portfolio to a clean state
2. Fetches historical prices at the scenario date
3. Executes simulated trades (spot buy/sell)
4. Displays a PnL table at +1, +2, +4, +7 days for each leg
5. Shows the final portfolio summary (cash balance, positions)
6. Cleans up (resets portfolio)

## How It Works

The `backtest` CLI fetches historical OHLCV data from Yahoo Finance (with CoinGecko fallback for crypto) and computes forward PnL by looking up actual prices at future dates.

- **Auto-pricing**: If `--price` is omitted, the historical close price is used
- **Portfolio state**: Trades are tracked in `~/.fintool/backtest_portfolio.json`
- **Cash balance**: Buying subtracts cost, selling adds proceeds (spot trades only)
- **Perp trades**: Use `perp_buy`/`perp_sell` with leverage for leveraged PnL

## Dependencies

- **Python 3.10+** (no third-party packages — uses only stdlib)
- **backtest** CLI binary (compiled from this repo)

## Writing Your Own

Create a new Python script following this pattern:

```python
#!/usr/bin/env python3
import json, os, subprocess
from pathlib import Path

REPO_DIR = Path(__file__).resolve().parent.parent.parent
BT = os.environ.get("BACKTEST", str(REPO_DIR / "target" / "release" / "backtest"))
DATE = "2024-01-15"

def cli(cmd, date=DATE):
    r = subprocess.run([BT, "--at", date, "--json", json.dumps(cmd)],
                       capture_output=True, text=True, timeout=30)
    return json.loads(r.stdout)

# Reset
cli({"command": "reset"})

# Quote
btc = cli({"command": "quote", "symbol": "BTC"})
print(f"BTC on {DATE}: ${btc['price']}")

# Trade
result = cli({"command": "buy", "symbol": "BTC", "amount": 0.01})
for p in result["pnl"]:
    print(f"  {p['offset']}: ${p['price']} (PnL: {p['pnl']}, {p['pnlPct']}%)")

# Cleanup
cli({"command": "reset"})
```

Supported symbols include all major crypto (BTC, ETH, SOL, ...), stocks (AAPL, NVDA, TSLA, ...), commodities (GOLD, SILVER, OIL), and indices (SP500, NASDAQ, DOW).

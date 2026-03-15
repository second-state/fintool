#!/usr/bin/env python3
"""
COVID-19 Crash — Flight-to-Safety Hedge
========================================

Date: February 21, 2020 (Friday before the crash accelerated)

Thesis: By late February 2020, COVID-19 had spread to Italy and South
Korea. The S&P 500 hit its all-time high on Feb 19. A pandemic-driven
selloff was imminent. The classic hedge: go long gold (safe haven) and
short equities. This is a dollar-neutral pair: ~$5,000 each side.

Legs:
  1. Long GOLD  — 3 oz at ~$1,645/oz  ($4,934 notional)
  2. Short SP500 — 1.5 units at ~$3,337 ($5,006 notional)

What happened: The S&P 500 fell ~12% over the next 7 trading days
(the fastest correction from ATH in history). Gold initially held
steady, then pulled back as margin calls hit.

Result: The short equity leg dominates — this hedge captured the
crash while the gold leg acts as a stabilizer.

Usage: python3 examples/backtest/covid_crash_hedge.py
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_DIR = SCRIPT_DIR.parent.parent

DEFAULTS = {
    "backtest": os.environ.get("BACKTEST", str(REPO_DIR / "target" / "release" / "backtest")),
}

DATE = "2020-02-21"


def cli(cmd: dict, binary: str, date: str) -> dict:
    """Call the backtest CLI in JSON mode. Returns parsed JSON output."""
    try:
        result = subprocess.run(
            [binary, "--at", date, "--json", json.dumps(cmd)],
            capture_output=True, text=True, timeout=30,
        )
        if result.returncode != 0:
            return {"error": result.stderr.strip() or f"exit code {result.returncode}"}
        return json.loads(result.stdout)
    except (json.JSONDecodeError, subprocess.TimeoutExpired) as e:
        return {"error": str(e)}


def run(cfg: dict):
    bt = cfg["backtest"]

    print()
    print("=" * 62)
    print(" COVID-19 Crash Hedge — February 21, 2020")
    print(" Long GOLD + Short S&P 500 (dollar-neutral pair)")
    print("=" * 62)
    print()

    # Reset portfolio
    cli({"command": "reset"}, bt, DATE)

    # Scout prices
    print("-- Scouting prices on", DATE, "--")
    print()

    gold = cli({"command": "quote", "symbol": "GOLD"}, bt, DATE)
    sp = cli({"command": "quote", "symbol": "SP500"}, bt, DATE)
    gold_price = float(gold.get("price", 0))
    sp_price = float(sp.get("price", 0))

    print(f"  GOLD:  ${gold_price:.2f} / oz")
    print(f"  SP500: ${sp_price:.2f}")
    print()

    # Leg 1: Long GOLD
    print("-- Leg 1: Long GOLD (flight to safety) --")
    result = cli({"command": "buy", "symbol": "GOLD", "amount": 3, "price": gold_price}, bt, DATE)
    print_trade(result)

    # Leg 2: Short SP500
    print("-- Leg 2: Short S&P 500 (equity crash) --")
    result = cli({"command": "sell", "symbol": "SP500", "amount": 1.5, "price": sp_price}, bt, DATE)
    print_trade(result)

    # Portfolio summary
    print("=" * 62)
    print(" Portfolio Summary")
    print("=" * 62)
    balance = cli({"command": "balance"}, bt, DATE)
    positions = cli({"command": "positions"}, bt, DATE)
    print_portfolio(balance, positions)

    # Cleanup
    cli({"command": "reset"}, bt, DATE)


def print_trade(result: dict):
    if "error" in result:
        print(f"  ERROR: {result['error']}")
        return
    trade = result.get("trade", {})
    pnl = result.get("pnl", [])
    symbol = trade.get("symbol", "?")
    side = trade.get("side", "?")
    amount = trade.get("amount", 0)
    price = trade.get("price", 0)
    total = amount * price
    print(f"  {side.upper()} {amount} {symbol} @ ${price:,.2f} (${total:,.2f} notional)")
    if pnl:
        print()
        print(f"  {'':>10} {'  +1 day':>14} {'  +2 days':>14} {'  +4 days':>14} {'  +7 days':>14}")
        print(f"  {'':>10} {'':->14} {'':->14} {'':->14} {'':->14}")
        prices = "".join(f"  ${float(p.get('price', 0)):>10,.2f}" for p in pnl)
        pnl_dollars = "".join(
            f"  {'+' if float(p.get('pnl', 0)) >= 0 else ''}{float(p.get('pnl', 0)):>10,.2f}"
            for p in pnl
        )
        pnl_pcts = "".join(
            f"  {'+' if float(p.get('pnlPct', 0)) >= 0 else ''}{float(p.get('pnlPct', 0)):>9,.2f}%"
            for p in pnl
        )
        print(f"  {'Price':>10}{prices}")
        print(f"  {'PnL $':>10}{pnl_dollars}")
        print(f"  {'PnL %':>10}{pnl_pcts}")
    print()

    portfolio = result.get("portfolio", {})
    cash = float(portfolio.get("cashBalance", 0))
    print(f"  [PORTFOLIO] Cash balance: ${cash:,.2f}")
    for pos in portfolio.get("positions", []):
        print(
            f"  [PORTFOLIO] {pos['type']} {pos['side']} {pos['symbol']}: "
            f"{abs(pos['quantity']):.4f} @ avg ${pos['avgEntryPrice']}"
        )
    print()


def print_portfolio(balance: dict, positions: dict):
    cash = float(balance.get("cashBalance", 0))
    total_trades = balance.get("totalTrades", 0)
    pos_list = positions if isinstance(positions, list) else positions.get("positions", [])
    print(f"  Cash balance: ${cash:,.2f}")
    print(f"  Total trades: {total_trades}")
    print(f"  Open positions: {len(pos_list)}")
    if pos_list:
        print()
        print(f"  {'Symbol':<10} {'Type':<6} {'Side':<8} {'Quantity':>12} {'Avg Entry':>14}")
        print(f"  {'-' * 54}")
        for p in pos_list:
            print(
                f"  {p['symbol']:<10} {p['type']:<6} {p['side']:<8} "
                f"{abs(p['quantity']):>12.4f} {float(p['avgEntryPrice']):>14.2f}"
            )
    print()


def main():
    parser = argparse.ArgumentParser(description="COVID-19 crash hedge backtest")
    parser.add_argument("--backtest", default=DEFAULTS["backtest"], help="Path to backtest binary")
    args = parser.parse_args()
    run({"backtest": args.backtest})


if __name__ == "__main__":
    main()

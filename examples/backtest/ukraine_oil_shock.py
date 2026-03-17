#!/usr/bin/env python3
"""
Russia-Ukraine Invasion — Commodity Supply Shock
==================================================

Date: February 24, 2022 (the day Russia invaded Ukraine)

Thesis: Russia is a top-3 global oil producer. A full-scale invasion
means sanctions, supply disruption, and an energy price spike. At the
same time, risk assets sell off on geopolitical uncertainty. This is
a classic macro shock trade: long commodities, short equities.

Legs:
  1. Long OIL  — 35 bbl at ~$92/bbl  ($3,220 notional)
  2. Long GOLD — 2 oz at ~$1,909/oz  ($3,818 notional) — war safe haven
  3. Short SP500 — 1.5 units at ~$4,225 ($6,338 notional)

The short leg is sized larger to roughly balance the two long legs.
Oil is the directional bet; gold is the defensive anchor.

What happened: Oil surged to $115+ within 2 weeks (eventually $130
in March). Gold rallied to $2,050. The S&P 500 dropped ~3% in the
first week before stabilizing.

Result: Both commodity legs profit on the supply shock, while the
equity short captures the initial risk-off move.

Usage: python3 examples/backtest/ukraine_oil_shock.py
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

DATE = "2022-02-24"


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


def run(cfg: dict):
    bt = cfg["backtest"]

    print()
    print("=" * 62)
    print(" Russia-Ukraine Invasion — February 24, 2022")
    print(" Long OIL + Long GOLD + Short S&P 500")
    print("=" * 62)
    print()

    # Reset portfolio
    cli({"command": "reset"}, bt, DATE)

    # Scout prices
    print("-- Scouting prices on", DATE, "--")
    print()

    oil = cli({"command": "quote", "symbol": "OIL"}, bt, DATE)
    gold = cli({"command": "quote", "symbol": "GOLD"}, bt, DATE)
    sp = cli({"command": "quote", "symbol": "SP500"}, bt, DATE)
    oil_price = float(oil.get("price", 0))
    gold_price = float(gold.get("price", 0))
    sp_price = float(sp.get("price", 0))

    print(f"  OIL:   ${oil_price:.2f} / bbl")
    print(f"  GOLD:  ${gold_price:.2f} / oz")
    print(f"  SP500: ${sp_price:.2f}")
    print()

    # Leg 1: Long OIL
    print("-- Leg 1: Long OIL (supply shock from sanctions) --")
    result = cli({"command": "buy", "symbol": "OIL", "amount": 35, "price": oil_price}, bt, DATE)
    print_trade(result)

    # Leg 2: Long GOLD
    print("-- Leg 2: Long GOLD (war premium + safe haven) --")
    result = cli({"command": "buy", "symbol": "GOLD", "amount": 2, "price": gold_price}, bt, DATE)
    print_trade(result)

    # Leg 3: Short SP500
    print("-- Leg 3: Short S&P 500 (risk-off) --")
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


def main():
    parser = argparse.ArgumentParser(description="Ukraine oil shock backtest")
    parser.add_argument("--backtest", default=DEFAULTS["backtest"], help="Path to backtest binary")
    args = parser.parse_args()
    run({"backtest": args.backtest})


if __name__ == "__main__":
    main()

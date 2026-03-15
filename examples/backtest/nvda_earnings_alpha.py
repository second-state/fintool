#!/usr/bin/env python3
"""
NVIDIA Earnings Blowout — AI Sector Alpha
============================================

Date: May 25, 2023 (day after NVDA's historic Q1 FY24 earnings)

Thesis: On May 24, 2023, NVIDIA reported revenue of $7.19B (vs
$6.52B expected) and guided Q2 to $11B — 50% above consensus.
The stock surged 25% after hours. This was the moment the market
realized AI infrastructure demand was real. The play: capture
NVDA's alpha while hedging broad market risk.

Legs:
  1. Long NVDA  — 13 shares at ~$379/share ($4,930 notional)
  2. Short SP500 — 1.2 units at ~$4,151     ($4,981 notional)

This is a classic long/short equity pair: long the winner, short
the index. If the market rallies, NVDA should rally more. If the
market drops, NVDA's AI tailwind should cushion the loss.

What happened: NVDA continued climbing from $379 to $400+ over
the next week as analysts rushed to upgrade. The S&P 500 was
roughly flat. Pure alpha.

Usage: python3 examples/backtest/nvda_earnings_alpha.py
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

DATE = "2023-05-25"


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
    print(" NVIDIA AI Earnings — May 25, 2023")
    print(" Long NVDA + Short S&P 500 (sector alpha pair)")
    print("=" * 62)
    print()

    # Reset portfolio
    cli({"command": "reset"}, bt, DATE)

    # Scout prices
    print("-- Scouting prices on", DATE, "--")
    print()

    nvda = cli({"command": "quote", "symbol": "NVDA"}, bt, DATE)
    sp = cli({"command": "quote", "symbol": "SP500"}, bt, DATE)
    nvda_price = float(nvda.get("price", 0))
    sp_price = float(sp.get("price", 0))

    print(f"  NVDA:  ${nvda_price:.2f}")
    print(f"  SP500: ${sp_price:.2f}")
    print()

    # Leg 1: Long NVDA
    print("-- Leg 1: Long NVDA (AI infrastructure demand) --")
    result = cli({"command": "buy", "symbol": "NVDA", "amount": 13, "price": nvda_price}, bt, DATE)
    print_trade(result)

    # Leg 2: Short SP500
    print("-- Leg 2: Short S&P 500 (hedge broad market risk) --")
    result = cli({"command": "sell", "symbol": "SP500", "amount": 1.2, "price": sp_price}, bt, DATE)
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
    parser = argparse.ArgumentParser(description="NVDA earnings alpha backtest")
    parser.add_argument("--backtest", default=DEFAULTS["backtest"], help="Path to backtest binary")
    args = parser.parse_args()
    run({"backtest": args.backtest})


if __name__ == "__main__":
    main()

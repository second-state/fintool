#!/usr/bin/env python3
"""
SVB Bank Run — Crypto as Digital Gold
=======================================

Date: March 13, 2023 (Monday after Silicon Valley Bank collapsed)

Thesis: On March 10, 2023, Silicon Valley Bank was seized by the FDIC —
the second-largest bank failure in US history. Signature Bank followed
on March 12. Traditional finance was in crisis. The contrarian bet:
crypto isn't just speculative — it's a decentralized alternative to
a fractured banking system. Meanwhile, gold catches safe-haven flows
and equities face contagion risk.

Starting capital: $1,000 split evenly across 4 legs ($250 each):
  1. Long BTC   — digital gold, decentralized store of value ($250)
  2. Long ETH   — DeFi as banking alternative ($250)
  3. Long GOLD  — traditional safe haven ($250)
  4. Short SP500 — banking contagion risk ($250)

The thesis is that all four legs win in a banking crisis: crypto
rallies on "be your own bank" narrative, gold on fear, equities
on contagion. If crypto fails to rally, gold and the short hedge
limit the damage.

Usage: python3 examples/backtest/svb_crypto_haven.py
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

DATE = "2023-03-13"
CAPITAL = 1000.0
NUM_LEGS = 4
PER_LEG = CAPITAL / NUM_LEGS


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
    return pnl


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
    print(" SVB Bank Run — Crypto as Digital Gold")
    print(f" March 13, 2023 | Starting capital: ${CAPITAL:,.0f}")
    print(f" ${PER_LEG:,.0f} per leg x {NUM_LEGS} legs")
    print("=" * 62)
    print()

    # Reset portfolio
    cli({"command": "reset"}, bt, DATE)

    # Scout prices
    print("-- Scouting prices on", DATE, "--")
    print()

    btc = cli({"command": "quote", "symbol": "BTC"}, bt, DATE)
    eth = cli({"command": "quote", "symbol": "ETH"}, bt, DATE)
    gold = cli({"command": "quote", "symbol": "GOLD"}, bt, DATE)
    sp = cli({"command": "quote", "symbol": "SP500"}, bt, DATE)
    btc_price = float(btc.get("price", 0))
    eth_price = float(eth.get("price", 0))
    gold_price = float(gold.get("price", 0))
    sp_price = float(sp.get("price", 0))

    print(f"  BTC:   ${btc_price:,.2f}")
    print(f"  ETH:   ${eth_price:,.2f}")
    print(f"  GOLD:  ${gold_price:,.2f}")
    print(f"  SP500: ${sp_price:,.2f}")
    print()

    # Calculate position sizes for $250 per leg
    btc_amount = round(PER_LEG / btc_price, 6)
    eth_amount = round(PER_LEG / eth_price, 4)
    gold_amount = round(PER_LEG / gold_price, 4)
    sp_amount = round(PER_LEG / sp_price, 4)

    all_pnl = []

    # Leg 1: Long BTC
    print(f"-- Leg 1: Long BTC (${PER_LEG:.0f} — digital gold, decentralized store of value) --")
    pnl = print_trade(cli({"command": "buy", "symbol": "BTC", "amount": btc_amount, "price": btc_price}, bt, DATE))
    all_pnl.append(("BTC long", pnl))

    # Leg 2: Long ETH
    print(f"-- Leg 2: Long ETH (${PER_LEG:.0f} — DeFi as banking alternative) --")
    pnl = print_trade(cli({"command": "buy", "symbol": "ETH", "amount": eth_amount, "price": eth_price}, bt, DATE))
    all_pnl.append(("ETH long", pnl))

    # Leg 3: Long GOLD
    print(f"-- Leg 3: Long GOLD (${PER_LEG:.0f} — traditional safe haven) --")
    pnl = print_trade(cli({"command": "buy", "symbol": "GOLD", "amount": gold_amount, "price": gold_price}, bt, DATE))
    all_pnl.append(("GOLD long", pnl))

    # Leg 4: Short SP500
    print(f"-- Leg 4: Short SP500 (${PER_LEG:.0f} — banking contagion risk) --")
    pnl = print_trade(cli({"command": "sell", "symbol": "SP500", "amount": sp_amount, "price": sp_price}, bt, DATE))
    all_pnl.append(("SP500 short", pnl))

    # Combined PnL summary
    print("=" * 62)
    print(" Combined Portfolio PnL (starting $1,000)")
    print("=" * 62)
    offsets = ["+1 day", "+2 days", "+4 days", "+7 days"]
    print()
    print(f"  {'Leg':<14} {'  +1 day':>10} {'  +2 days':>10} {'  +4 days':>10} {'  +7 days':>10}")
    print(f"  {'-' * 54}")
    totals = [0.0, 0.0, 0.0, 0.0]
    for name, pnl in all_pnl:
        if pnl:
            vals = [float(p.get("pnl", 0)) for p in pnl]
            for i, v in enumerate(vals):
                totals[i] += v
            row = "".join(f"  {'+' if v >= 0 else ''}{v:>8.2f}" for v in vals)
            print(f"  {name:<14}{row}")
    print(f"  {'-' * 54}")
    total_row = "".join(f"  {'+' if t >= 0 else ''}{t:>8.2f}" for t in totals)
    print(f"  {'TOTAL':<14}{total_row}")
    print()

    balance_row = "".join(f"  ${CAPITAL + t:>8.2f}" for t in totals)
    print(f"  {'Balance':<14}{balance_row}")
    pct_row = "".join(f"  {'+' if t >= 0 else ''}{t / CAPITAL * 100:>7.2f}%" for t in totals)
    print(f"  {'Return':<14}{pct_row}")
    print()

    # Portfolio state
    print("=" * 62)
    print(" Final Portfolio State")
    print("=" * 62)
    balance = cli({"command": "balance"}, bt, DATE)
    positions = cli({"command": "positions"}, bt, DATE)
    print_portfolio(balance, positions)

    # Cleanup
    cli({"command": "reset"}, bt, DATE)


def main():
    parser = argparse.ArgumentParser(description="SVB bank run — crypto as digital gold backtest")
    parser.add_argument("--backtest", default=DEFAULTS["backtest"], help="Path to backtest binary")
    args = parser.parse_args()
    run({"backtest": args.backtest})


if __name__ == "__main__":
    main()

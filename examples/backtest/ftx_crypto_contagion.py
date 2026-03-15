#!/usr/bin/env python3
"""
FTX Collapse — Crypto Contagion Hedge
========================================

Date: November 8, 2022 (Binance announced it would sell its FTT holdings)

Thesis: On Nov 6, CoinDesk reported Alameda's balance sheet was full
of FTT tokens. On Nov 8, Binance announced it would liquidate its
~$530M in FTT, triggering a bank run on FTX. Crypto was about to
enter a contagion spiral — but it wasn't clear how far traditional
markets would follow. The trade: short crypto (BTC + ETH) and go
long gold as a safe-haven hedge in case the contagion spread.

Legs:
  1. Short BTC  — 0.15 BTC at ~$18,500  ($2,775 notional)
  2. Short ETH  — 2.0 ETH at ~$1,340    ($2,680 notional)
  3. Long GOLD  — 3 oz at ~$1,712/oz    ($5,136 notional)

Gold is the anchor — roughly sized to match the combined crypto short.
If crypto crashes and gold holds or rises, both sides win. If crypto
recovers, the gold leg limits the damage.

What happened: BTC dropped from $18.5k to $15.7k (-15%) in 7 days.
ETH fell from $1,340 to $1,100 (-18%). Gold was roughly flat,
providing stability. The crypto short was the main profit driver.

Usage: python3 examples/backtest/ftx_crypto_contagion.py
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

DATE = "2022-11-08"


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
    print(" FTX Collapse — November 8, 2022")
    print(" Short BTC + Short ETH + Long GOLD (contagion hedge)")
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
    btc_price = float(btc.get("price", 0))
    eth_price = float(eth.get("price", 0))
    gold_price = float(gold.get("price", 0))

    print(f"  BTC:   ${btc_price:,.2f}")
    print(f"  ETH:   ${eth_price:,.2f}")
    print(f"  GOLD:  ${gold_price:,.2f}")
    print()

    # Leg 1: Short BTC
    print("-- Leg 1: Short BTC (FTX contagion, forced selling) --")
    result = cli({"command": "sell", "symbol": "BTC", "amount": 0.15, "price": btc_price}, bt, DATE)
    print_trade(result)

    # Leg 2: Short ETH
    print("-- Leg 2: Short ETH (correlated crypto drawdown) --")
    result = cli({"command": "sell", "symbol": "ETH", "amount": 2.0, "price": eth_price}, bt, DATE)
    print_trade(result)

    # Leg 3: Long GOLD
    print("-- Leg 3: Long GOLD (safe haven, hedge against reversal) --")
    result = cli({"command": "buy", "symbol": "GOLD", "amount": 3, "price": gold_price}, bt, DATE)
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
    parser = argparse.ArgumentParser(description="FTX crypto contagion hedge backtest")
    parser.add_argument("--backtest", default=DEFAULTS["backtest"], help="Path to backtest binary")
    args = parser.parse_args()
    run({"backtest": args.backtest})


if __name__ == "__main__":
    main()

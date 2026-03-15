#!/usr/bin/env bash
#
# FTX Collapse — Crypto Contagion Hedge
# ========================================
#
# Date: November 8, 2022 (Binance announced it would sell its FTT holdings)
#
# Thesis: On Nov 6, CoinDesk reported Alameda's balance sheet was full
# of FTT tokens. On Nov 8, Binance announced it would liquidate its
# ~$530M in FTT, triggering a bank run on FTX. Crypto was about to
# enter a contagion spiral — but it wasn't clear how far traditional
# markets would follow. The trade: short crypto (BTC + ETH) and go
# long gold as a safe-haven hedge in case the contagion spread.
#
# Legs:
#   1. Short BTC  — 0.15 BTC at ~$18,500  ($2,775 notional)
#   2. Short ETH  — 2.0 ETH at ~$1,340    ($2,680 notional)
#   3. Long GOLD  — 3 oz at ~$1,712/oz    ($5,136 notional)
#
# Gold is the anchor — roughly sized to match the combined crypto short.
# If crypto crashes and gold holds or rises, both sides win. If crypto
# recovers, the gold leg limits the damage.
#
# What happened: BTC dropped from $18.5k to $15.7k (-15%) in 7 days.
# ETH fell from $1,340 to $1,100 (-18%). Gold was roughly flat,
# providing stability. The crypto short was the main profit driver.
#
# Usage: ./examples/backtest/ftx_crypto_contagion.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BT="${BACKTEST:-$SCRIPT_DIR/../../target/release/backtest}"

echo ""
echo "══════════════════════════════════════════════════════════════"
echo " FTX Collapse — November 8, 2022"
echo " Short BTC + Short ETH + Long GOLD (contagion hedge)"
echo "══════════════════════════════════════════════════════════════"
echo ""

# ── Reset portfolio ────────────────────────────────────────────────
$BT --at 2022-11-08 reset 2>/dev/null

# ── Scout: get prices on Nov 8, 2022 ──────────────────────────────
echo "── Scouting prices on 2022-11-08 ──"
echo ""

BTC_PRICE=$($BT --at 2022-11-08 --json '{"command":"quote","symbol":"BTC"}' 2>/dev/null | jq -r '.price')
ETH_PRICE=$($BT --at 2022-11-08 --json '{"command":"quote","symbol":"ETH"}' 2>/dev/null | jq -r '.price')
GOLD_PRICE=$($BT --at 2022-11-08 --json '{"command":"quote","symbol":"GOLD"}' 2>/dev/null | jq -r '.price')

echo "  BTC:   \$$BTC_PRICE"
echo "  ETH:   \$$ETH_PRICE"
echo "  GOLD:  \$$GOLD_PRICE"
echo ""

# ── Leg 1: Short BTC — exchange contagion ─────────────────────────
echo "── Leg 1: Short BTC (FTX contagion, forced selling) ──"
$BT --at 2022-11-08 sell BTC --amount 0.15 --price "$BTC_PRICE"

# ── Leg 2: Short ETH — crypto-wide selloff ────────────────────────
echo "── Leg 2: Short ETH (correlated crypto drawdown) ──"
$BT --at 2022-11-08 sell ETH --amount 2.0 --price "$ETH_PRICE"

# ── Leg 3: Long gold — safe-haven anchor ──────────────────────────
echo "── Leg 3: Long GOLD (safe haven, hedge against reversal) ──"
$BT --at 2022-11-08 buy GOLD --amount 3 --price "$GOLD_PRICE"

# ── Portfolio snapshot ─────────────────────────────────────────────
echo "══════════════════════════════════════════════════════════════"
echo " Portfolio Summary"
echo "══════════════════════════════════════════════════════════════"
$BT --at 2022-11-08 balance
$BT --at 2022-11-08 positions

# ── Cleanup ────────────────────────────────────────────────────────
$BT --at 2022-11-08 reset > /dev/null 2>&1

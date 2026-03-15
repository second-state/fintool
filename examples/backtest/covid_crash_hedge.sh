#!/usr/bin/env bash
#
# COVID-19 Crash — Flight-to-Safety Hedge
# ========================================
#
# Date: February 21, 2020 (Friday before the crash accelerated)
#
# Thesis: By late February 2020, COVID-19 had spread to Italy and South
# Korea. The S&P 500 hit its all-time high on Feb 19. A pandemic-driven
# selloff was imminent. The classic hedge: go long gold (safe haven) and
# short equities. This is a dollar-neutral pair: ~$5,000 each side.
#
# Legs:
#   1. Long GOLD  — 3 oz at ~$1,645/oz  ($4,934 notional)
#   2. Short SP500 — 1.5 units at ~$3,337 ($5,006 notional)
#
# What happened: The S&P 500 fell ~12% over the next 7 trading days
# (the fastest correction from ATH in history). Gold initially held
# steady, then pulled back as margin calls hit.
#
# Result: The short equity leg dominates — this hedge captured the
# crash while the gold leg acts as a stabilizer.
#
# Usage: ./examples/backtest/covid_crash_hedge.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BT="${BACKTEST:-$SCRIPT_DIR/../../target/release/backtest}"

echo ""
echo "══════════════════════════════════════════════════════════════"
echo " COVID-19 Crash Hedge — February 21, 2020"
echo " Long GOLD + Short S&P 500 (dollar-neutral pair)"
echo "══════════════════════════════════════════════════════════════"
echo ""

# ── Reset portfolio ────────────────────────────────────────────────
$BT --at 2020-02-21 reset 2>/dev/null

# ── Scout: get prices on Feb 21, 2020 ─────────────────────────────
echo "── Scouting prices on 2020-02-21 ──"
echo ""

GOLD_PRICE=$($BT --at 2020-02-21 --json '{"command":"quote","symbol":"GOLD"}' 2>/dev/null | jq -r '.price')
SP_PRICE=$($BT --at 2020-02-21 --json '{"command":"quote","symbol":"SP500"}' 2>/dev/null | jq -r '.price')

echo "  GOLD:  \$$GOLD_PRICE / oz"
echo "  SP500: \$$SP_PRICE"
echo ""

# ── Leg 1: Long gold — safe-haven bid ─────────────────────────────
echo "── Leg 1: Long GOLD (flight to safety) ──"
$BT --at 2020-02-21 buy GOLD --amount 3 --price "$GOLD_PRICE"

# ── Leg 2: Short S&P 500 — pandemic selloff ───────────────────────
echo "── Leg 2: Short S&P 500 (equity crash) ──"
$BT --at 2020-02-21 sell SP500 --amount 1.5 --price "$SP_PRICE"

# ── Portfolio snapshot ─────────────────────────────────────────────
echo "══════════════════════════════════════════════════════════════"
echo " Portfolio Summary"
echo "══════════════════════════════════════════════════════════════"
$BT --at 2020-02-21 balance
$BT --at 2020-02-21 positions

# ── Cleanup ────────────────────────────────────────────────────────
$BT --at 2020-02-21 reset > /dev/null 2>&1

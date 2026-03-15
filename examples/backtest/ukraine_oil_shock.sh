#!/usr/bin/env bash
#
# Russia-Ukraine Invasion — Commodity Supply Shock
# ==================================================
#
# Date: February 24, 2022 (the day Russia invaded Ukraine)
#
# Thesis: Russia is a top-3 global oil producer. A full-scale invasion
# means sanctions, supply disruption, and an energy price spike. At the
# same time, risk assets sell off on geopolitical uncertainty. This is
# a classic macro shock trade: long commodities, short equities.
#
# Legs:
#   1. Long OIL  — 35 bbl at ~$92/bbl  ($3,220 notional)
#   2. Long GOLD — 2 oz at ~$1,909/oz  ($3,818 notional) — war safe haven
#   3. Short SP500 — 1.5 units at ~$4,225 ($6,338 notional)
#
# The short leg is sized larger to roughly balance the two long legs.
# Oil is the directional bet; gold is the defensive anchor.
#
# What happened: Oil surged to $115+ within 2 weeks (eventually $130
# in March). Gold rallied to $2,050. The S&P 500 dropped ~3% in the
# first week before stabilizing.
#
# Result: Both commodity legs profit on the supply shock, while the
# equity short captures the initial risk-off move.
#
# Usage: ./examples/backtest/ukraine_oil_shock.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BT="${BACKTEST:-$SCRIPT_DIR/../../target/release/backtest}"

echo ""
echo "══════════════════════════════════════════════════════════════"
echo " Russia-Ukraine Invasion — February 24, 2022"
echo " Long OIL + Long GOLD + Short S&P 500"
echo "══════════════════════════════════════════════════════════════"
echo ""

# ── Reset portfolio ────────────────────────────────────────────────
$BT --at 2022-02-24 reset 2>/dev/null

# ── Scout: get prices on Feb 24, 2022 ─────────────────────────────
echo "── Scouting prices on 2022-02-24 ──"
echo ""

OIL_PRICE=$($BT --at 2022-02-24 --json '{"command":"quote","symbol":"OIL"}' 2>/dev/null | jq -r '.price')
GOLD_PRICE=$($BT --at 2022-02-24 --json '{"command":"quote","symbol":"GOLD"}' 2>/dev/null | jq -r '.price')
SP_PRICE=$($BT --at 2022-02-24 --json '{"command":"quote","symbol":"SP500"}' 2>/dev/null | jq -r '.price')

echo "  OIL:   \$$OIL_PRICE / bbl"
echo "  GOLD:  \$$GOLD_PRICE / oz"
echo "  SP500: \$$SP_PRICE"
echo ""

# ── Leg 1: Long crude oil — supply disruption ─────────────────────
echo "── Leg 1: Long OIL (supply shock from sanctions) ──"
$BT --at 2022-02-24 buy OIL --amount 35 --price "$OIL_PRICE"

# ── Leg 2: Long gold — geopolitical safe haven ────────────────────
echo "── Leg 2: Long GOLD (war premium + safe haven) ──"
$BT --at 2022-02-24 buy GOLD --amount 2 --price "$GOLD_PRICE"

# ── Leg 3: Short S&P 500 — risk-off selloff ──────────────────────
echo "── Leg 3: Short S&P 500 (risk-off) ──"
$BT --at 2022-02-24 sell SP500 --amount 1.5 --price "$SP_PRICE"

# ── Portfolio snapshot ─────────────────────────────────────────────
echo "══════════════════════════════════════════════════════════════"
echo " Portfolio Summary"
echo "══════════════════════════════════════════════════════════════"
$BT --at 2022-02-24 balance
$BT --at 2022-02-24 positions

# ── Cleanup ────────────────────────────────────────────────────────
$BT --at 2022-02-24 reset > /dev/null 2>&1

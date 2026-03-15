#!/usr/bin/env bash
#
# NVIDIA Earnings Blowout — AI Sector Alpha
# ============================================
#
# Date: May 25, 2023 (day after NVDA's historic Q1 FY24 earnings)
#
# Thesis: On May 24, 2023, NVIDIA reported revenue of $7.19B (vs
# $6.52B expected) and guided Q2 to $11B — 50% above consensus.
# The stock surged 25% after hours. This was the moment the market
# realized AI infrastructure demand was real. The play: capture
# NVDA's alpha while hedging broad market risk.
#
# Legs:
#   1. Long NVDA  — 13 shares at ~$379/share ($4,930 notional)
#   2. Short SP500 — 1.2 units at ~$4,151     ($4,981 notional)
#
# This is a classic long/short equity pair: long the winner, short
# the index. If the market rallies, NVDA should rally more. If the
# market drops, NVDA's AI tailwind should cushion the loss.
#
# What happened: NVDA continued climbing from $379 to $400+ over
# the next week as analysts rushed to upgrade. The S&P 500 was
# roughly flat. Pure alpha.
#
# Usage: ./examples/backtest/nvda_earnings_alpha.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BT="${BACKTEST:-$SCRIPT_DIR/../../target/release/backtest}"

echo ""
echo "══════════════════════════════════════════════════════════════"
echo " NVIDIA AI Earnings — May 25, 2023"
echo " Long NVDA + Short S&P 500 (sector alpha pair)"
echo "══════════════════════════════════════════════════════════════"
echo ""

# ── Reset portfolio ────────────────────────────────────────────────
$BT --at 2023-05-25 reset 2>/dev/null

# ── Scout: get prices on May 25, 2023 ─────────────────────────────
echo "── Scouting prices on 2023-05-25 ──"
echo ""

NVDA_PRICE=$($BT --at 2023-05-25 --json '{"command":"quote","symbol":"NVDA"}' 2>/dev/null | jq -r '.price')
SP_PRICE=$($BT --at 2023-05-25 --json '{"command":"quote","symbol":"SP500"}' 2>/dev/null | jq -r '.price')

echo "  NVDA:  \$$NVDA_PRICE"
echo "  SP500: \$$SP_PRICE"
echo ""

# ── Leg 1: Long NVDA — AI momentum ───────────────────────────────
echo "── Leg 1: Long NVDA (AI infrastructure demand) ──"
$BT --at 2023-05-25 buy NVDA --amount 13 --price "$NVDA_PRICE"

# ── Leg 2: Short S&P 500 — market-neutral hedge ──────────────────
echo "── Leg 2: Short S&P 500 (hedge broad market risk) ──"
$BT --at 2023-05-25 sell SP500 --amount 1.2 --price "$SP_PRICE"

# ── Portfolio snapshot ─────────────────────────────────────────────
echo "══════════════════════════════════════════════════════════════"
echo " Portfolio Summary"
echo "══════════════════════════════════════════════════════════════"
$BT --at 2023-05-25 balance
$BT --at 2023-05-25 positions

# ── Cleanup ────────────────────────────────────────────────────────
$BT --at 2023-05-25 reset > /dev/null 2>&1

#!/usr/bin/env bash
#
# Simulated BTC spot buy with forward PnL and portfolio tracking
#
# Uses backtest --json API. Output is always JSON.
#
# Workflow:
#   1. Reset portfolio
#   2. Simulate buying 0.01 BTC on 2025-01-15
#   3. Verify trade details, PnL offsets, and portfolio state
#
# Usage: ./tests/backtest/buy_spot.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

bt() { $BACKTEST --at "$1" --json "$2" 2>/dev/null; }

log "Simulated BTC spot buy on 2025-01-15 (JSON API)"

# ── Reset portfolio ────────────────────────────────────────────────────
bt "2025-01-15" '{"command":"reset"}' > /dev/null

# ── Simulate spot buy ──────────────────────────────────────────────────
info "Buying 0.01 BTC at historical price..."
RESULT=$(bt "2025-01-15" '{"command":"buy","symbol":"BTC","amount":0.01}')

if [[ -z "$RESULT" ]]; then
    fail "BTC spot buy returned empty"
    exit 1
fi

# Verify trade details
SIDE=$(echo "$RESULT" | jq -r '.trade.side // empty')
SYMBOL=$(echo "$RESULT" | jq -r '.trade.symbol // empty')
AMOUNT=$(echo "$RESULT" | jq -r '.trade.amount // empty')
TRADE_TYPE=$(echo "$RESULT" | jq -r '.trade.type // empty')

if [[ "$SIDE" != "buy" ]]; then
    fail "Expected side buy, got: $SIDE"
    echo "$RESULT" | jq .
    exit 1
fi

if [[ "$TRADE_TYPE" != "spot" ]]; then
    fail "Expected type spot, got: $TRADE_TYPE"
    exit 1
fi

# Verify PnL offsets
PNL_COUNT=$(echo "$RESULT" | jq '.pnl | length')
if [[ "$PNL_COUNT" -lt 1 ]]; then
    fail "No PnL data returned"
    echo "$RESULT" | jq .
    exit 1
fi

# Verify portfolio data
CASH=$(echo "$RESULT" | jq -r '.portfolio.cashBalance // empty')
POS_COUNT=$(echo "$RESULT" | jq '.portfolio.positions | length')

if [[ -z "$CASH" ]]; then
    fail "Trade output missing portfolio.cashBalance"
    exit 1
fi

if [[ "$POS_COUNT" -lt 1 ]]; then
    fail "Trade output missing positions"
    exit 1
fi

done_step
ok "BTC spot buy: $AMOUNT BTC (side=$SIDE, type=$TRADE_TYPE)"
ok "PnL offsets returned: $PNL_COUNT"
ok "Cash balance: \$$CASH"
echo "$RESULT" | jq '.pnl[] | {offset, price, pnl, pnlPct}'

# ── Cleanup ────────────────────────────────────────────────────────────
bt "2025-01-15" '{"command":"reset"}' > /dev/null

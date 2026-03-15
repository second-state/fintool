#!/usr/bin/env bash
#
# Simulated ETH spot sell with forward PnL via backtest
#
# Uses backtest --json API. Output is always JSON.
#
# Workflow:
#   1. Simulate selling 0.5 ETH at $3300 on 2025-01-15
#   2. Verify trade details and PnL offsets are returned
#
# Usage: ./tests/backtest/sell_spot.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

bt() { $BACKTEST --at "$1" --json "$2" 2>/dev/null; }

log "Simulated ETH spot sell on 2025-01-15 (JSON API)"

# ── Simulate spot sell ─────────────────────────────────────────────────
info "Selling 0.5 ETH at \$3300..."
RESULT=$(bt "2025-01-15" '{"command":"sell","symbol":"ETH","amount":0.5,"price":3300}')

if [[ -z "$RESULT" ]]; then
    fail "ETH spot sell returned empty"
    exit 1
fi

# Verify trade details
SIDE=$(echo "$RESULT" | jq -r '.trade.side // empty')
SYMBOL=$(echo "$RESULT" | jq -r '.trade.symbol // empty')
TRADE_TYPE=$(echo "$RESULT" | jq -r '.trade.type // empty')
ENTRY_PRICE=$(echo "$RESULT" | jq -r '.trade.price // empty')

if [[ "$SIDE" != "sell" ]]; then
    fail "Expected side sell, got: $SIDE"
    echo "$RESULT" | jq .
    exit 1
fi

if [[ "$TRADE_TYPE" != "spot" ]]; then
    fail "Expected tradeType spot, got: $TRADE_TYPE"
    exit 1
fi

# Verify PnL offsets
PNL_COUNT=$(echo "$RESULT" | jq '.pnl | length')
if [[ "$PNL_COUNT" -lt 1 ]]; then
    fail "No PnL data returned"
    echo "$RESULT" | jq .
    exit 1
fi

done_step
ok "ETH spot sell: 0.5 ETH at \$$ENTRY_PRICE (side=$SIDE, type=$TRADE_TYPE)"
ok "PnL offsets returned: $PNL_COUNT"
echo "$RESULT" | jq '.pnl[] | {offset, price, pnl, pnlPct}'

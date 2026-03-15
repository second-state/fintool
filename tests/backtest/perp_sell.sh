#!/usr/bin/env bash
#
# Simulated BTC perp sell (short) with forward PnL via backtest
#
# Uses backtest --json API. Output is always JSON.
#
# Workflow:
#   1. Simulate perp sell of 0.01 BTC at $100000 on 2025-01-15
#   2. Verify trade details and PnL offsets
#
# Usage: ./tests/backtest/perp_sell.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

bt() { $BACKTEST --at "$1" --json "$2" 2>/dev/null; }

log "Simulated BTC perp short on 2025-01-15 (JSON API)"

# ── Simulate perp sell ─────────────────────────────────────────────────
info "Perp selling 0.01 BTC at \$100000..."
RESULT=$(bt "2025-01-15" '{"command":"perp_sell","symbol":"BTC","amount":0.01,"price":100000}')

if [[ -z "$RESULT" ]]; then
    fail "BTC perp sell returned empty"
    exit 1
fi

# Verify trade details
SIDE=$(echo "$RESULT" | jq -r '.trade.side // empty')
TRADE_TYPE=$(echo "$RESULT" | jq -r '.trade.tradeType // empty')
ENTRY_PRICE=$(echo "$RESULT" | jq -r '.trade.price // empty')

if [[ "$SIDE" != "sell" ]]; then
    fail "Expected side sell, got: $SIDE"
    echo "$RESULT" | jq .
    exit 1
fi

if [[ "$TRADE_TYPE" != "perp" ]]; then
    fail "Expected tradeType perp, got: $TRADE_TYPE"
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
ok "BTC perp sell: 0.01 BTC at \$$ENTRY_PRICE (side=$SIDE, type=$TRADE_TYPE)"
ok "PnL offsets returned: $PNL_COUNT"
echo "$RESULT" | jq '.pnl[] | {offset, price, pnl, pnlPct}'

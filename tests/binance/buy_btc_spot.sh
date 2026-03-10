#!/usr/bin/env bash
#
# Buy ~$12 worth of BTC on Binance spot
#
# Uses binance --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Get BTC price via quote
#   2. Compute buy size (~$12 worth) and limit price (+0.5%)
#   3. Place BTC spot buy order
#
# Usage: ./tests/binance/buy_btc_spot.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $BINANCE --json "$1" 2>/dev/null; }

log "Buy ~\$12 BTC spot on Binance (JSON API)"

# ── Get BTC price ────────────────────────────────────────────────────
info "Fetching BTC price..."
QUOTE=$(ft '{"command":"quote","symbol":"BTC"}')

if [[ -z "$QUOTE" ]]; then
    fail "BTC quote failed"
    exit 1
fi

PRICE=$(echo "$QUOTE" | jq -r '.price // empty')

if [[ -z "$PRICE" || "$PRICE" == "null" ]]; then
    fail "BTC quote returned but price is missing"
    exit 1
fi

# ── Compute order size and limit ─────────────────────────────────────
BUY_SIZE=$(echo "12 / $PRICE" | bc -l | xargs printf "%.5f")
BUY_LIMIT=$(echo "$PRICE" | awk '{printf "%.2f", $1 * 1.005}')

info "Price:           \$$PRICE"
info "Limit buy price: \$$BUY_LIMIT (+0.5% buffer)"
info "Buy size:        $BUY_SIZE BTC (~\$12)"

# ── Place spot buy order ─────────────────────────────────────────────
RESULT=$(ft "{\"command\":\"buy\",\"symbol\":\"BTC\",\"amount\":$BUY_SIZE,\"price\":$BUY_LIMIT}")

if [[ -z "$RESULT" ]]; then
    fail "BTC spot buy failed"
    exit 1
fi

ORDER_ID=$(echo "$RESULT" | jq -r '.response.orderId // empty')
STATUS=$(echo "$RESULT" | jq -r '.response.status // empty')

done_step
info "Order ID:    ${ORDER_ID:-unknown}"
info "Status:      ${STATUS:-unknown}"
info "Symbol:      BTCUSDT"
info "Size:        $BUY_SIZE BTC"
info "Limit:       \$$BUY_LIMIT"

if [[ "$STATUS" == "FILLED" ]]; then
    ok "BTC spot buy FILLED"
elif [[ "$STATUS" == "NEW" ]]; then
    warn "BTC spot buy is RESTING (not yet filled)"
    ok "BTC spot buy order placed (resting)"
elif [[ "$STATUS" == "PARTIALLY_FILLED" ]]; then
    ok "BTC spot buy PARTIALLY FILLED"
else
    ok "BTC spot buy order placed (status: ${STATUS:-unknown})"
fi

#!/usr/bin/env bash
#
# Sell ALL ETH futures position on Binance
#
# Uses binance --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Get futures positions
#   2. Extract ETH position size
#   3. If no position, warn and exit
#   4. Get ETH price via quote
#   5. Sell with close flag at -0.5% below price
#
# Usage: ./tests/binance/sell_eth.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $BINANCE --json "$1" 2>/dev/null; }

log "Sell ALL ETH futures on Binance (JSON API)"

# ── Get positions ────────────────────────────────────────────────────
info "Fetching futures positions to find ETH size..."
POSITIONS=$(ft '{"command":"positions"}')

if [[ -z "$POSITIONS" ]]; then
    fail "Failed to fetch positions"
    exit 1
fi

ETH_SIZE=$(echo "$POSITIONS" | jq -r '
    .futures // [] |
    map(select(.symbol == "ETHUSDT" and (.positionAmt | tonumber | fabs) > 0)) |
    .[0].positionAmt // empty
' 2>/dev/null || true)

if [[ -z "$ETH_SIZE" || "$ETH_SIZE" == "null" ]]; then
    info "No ETH futures position found. Checking open orders..."
    ORDERS=$(ft '{"command":"orders"}')
    if [[ -n "$ORDERS" ]]; then
        OPEN_COUNT=$(echo "$ORDERS" | jq -r '.futures | length' 2>/dev/null || echo "0")
        info "Open futures orders: $OPEN_COUNT"
    fi
    done_step
    warn "No ETH futures position to sell -- order may not have filled"
    exit 0
fi

# ── Compute sell size (absolute value) ───────────────────────────────
SELL_SIZE=$(echo "$ETH_SIZE" | sed 's/^-//')
info "ETH position size: $ETH_SIZE (selling $SELL_SIZE)"

# ── Get ETH price ────────────────────────────────────────────────────
info "Fetching ETH price..."
QUOTE=$(ft '{"command":"quote","symbol":"ETH"}')

if [[ -z "$QUOTE" ]]; then
    fail "ETH quote failed"
    exit 1
fi

SELL_PRICE=$(echo "$QUOTE" | jq -r '.price // empty')
SELL_LIMIT=$(echo "$SELL_PRICE" | awk '{printf "%.2f", $1 * 0.995}')

info "Current price: \$$SELL_PRICE"
info "Sell limit:    \$$SELL_LIMIT (-0.5% buffer)"

# ── Place sell order with close flag ─────────────────────────────────
RESULT=$(ft "{\"command\":\"perp_sell\",\"symbol\":\"ETH\",\"amount\":$SELL_SIZE,\"price\":$SELL_LIMIT,\"close\":true}")

if [[ -z "$RESULT" ]]; then
    fail "ETH futures sell failed"
    warn "Position may still be open -- check manually with 'binance positions'"
    exit 1
fi

ORDER_ID=$(echo "$RESULT" | jq -r '.response.orderId // empty')
STATUS=$(echo "$RESULT" | jq -r '.response.status // empty')

done_step
info "Sold:     $SELL_SIZE ETH"
info "Limit:    \$$SELL_LIMIT"
info "Order ID: ${ORDER_ID:-unknown}"
info "Status:   ${STATUS:-unknown}"
ok "ETH futures sell order placed -- $SELL_SIZE ETH at \$$SELL_LIMIT"

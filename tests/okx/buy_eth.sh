#!/usr/bin/env bash
#
# Buy ETH on OKX (spot limit order)
#
# Uses okx --json API. Places a limit buy below market for safety.
#
# Usage: ./tests/okx/buy_eth.sh [AMOUNT] [DISCOUNT]
#        AMOUNT   = ETH amount (default: 0.005)
#        DISCOUNT = multiplier below market price (default: 0.995)
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $OKX --json "$1" 2>/dev/null; }

AMOUNT="${1:-0.005}"
DISCOUNT="${2:-0.995}"

log "Buy $AMOUNT ETH on OKX (spot limit)"

# ── Get quote ──────────────────────────────────────────────────────
info "Fetching ETH price..."
QUOTE=$(ft '{"command":"quote","symbol":"ETH"}')
ETH_PRICE=$(echo "$QUOTE" | jq -r '.price // empty')

if [[ -z "$ETH_PRICE" || "$ETH_PRICE" == "null" ]]; then
    fail "ETH quote failed"
    exit 1
fi

BUY_PRICE=$(echo "$ETH_PRICE" | awk -v d="$DISCOUNT" '{printf "%.2f", $1 * d}')
TOTAL=$(echo "$AMOUNT * $BUY_PRICE" | bc -l | xargs printf "%.2f")

info "ETH price: \$$ETH_PRICE"
info "Buy price: \$$BUY_PRICE (${DISCOUNT}x)"
info "Amount:    $AMOUNT ETH (~\$$TOTAL)"

# ── Place order ────────────────────────────────────────────────────
RESULT=$(ft "{\"command\":\"buy\",\"symbol\":\"ETH\",\"amount\":$AMOUNT,\"price\":$BUY_PRICE}")

if [[ -z "$RESULT" ]]; then
    fail "Buy order failed"
    exit 1
fi

STATUS=$(echo "$RESULT" | jq -r '.status // "unknown"')
ORDER_ID=$(echo "$RESULT" | jq -r '.orderId // "unknown"')

done_step
info "Status:   $STATUS"
info "Order ID: $ORDER_ID"
ok "ETH spot buy order placed"

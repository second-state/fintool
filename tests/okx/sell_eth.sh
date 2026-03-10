#!/usr/bin/env bash
#
# Sell ETH on OKX (spot limit order)
#
# Uses okx --json API. Places a limit sell above market for safety.
#
# Usage: ./tests/okx/sell_eth.sh [AMOUNT] [PREMIUM]
#        AMOUNT  = ETH amount (default: 0.005)
#        PREMIUM = multiplier above market price (default: 1.005)
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $OKX --json "$1" 2>/dev/null; }

AMOUNT="${1:-0.005}"
PREMIUM="${2:-1.005}"

log "Sell $AMOUNT ETH on OKX (spot limit)"

# ── Get quote ──────────────────────────────────────────────────────
info "Fetching ETH price..."
QUOTE=$(ft '{"command":"quote","symbol":"ETH"}')
ETH_PRICE=$(echo "$QUOTE" | jq -r '.price // empty')

if [[ -z "$ETH_PRICE" || "$ETH_PRICE" == "null" ]]; then
    fail "ETH quote failed"
    exit 1
fi

SELL_PRICE=$(echo "$ETH_PRICE" | awk -v p="$PREMIUM" '{printf "%.2f", $1 * p}')

info "ETH price:  \$$ETH_PRICE"
info "Sell price: \$$SELL_PRICE (${PREMIUM}x)"
info "Amount:     $AMOUNT ETH"

# ── Place order ────────────────────────────────────────────────────
RESULT=$(ft "{\"command\":\"sell\",\"symbol\":\"ETH\",\"amount\":$AMOUNT,\"price\":$SELL_PRICE}")

if [[ -z "$RESULT" ]]; then
    fail "Sell order failed"
    exit 1
fi

STATUS=$(echo "$RESULT" | jq -r '.status // "unknown"')
ORDER_ID=$(echo "$RESULT" | jq -r '.orderId // "unknown"')

done_step
info "Status:   $STATUS"
info "Order ID: $ORDER_ID"
ok "ETH spot sell order placed"

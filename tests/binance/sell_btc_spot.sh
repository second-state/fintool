#!/usr/bin/env bash
#
# Sell ALL BTC on Binance spot
#
# Uses binance --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Get spot balance
#   2. Extract BTC free balance
#   3. If no BTC, warn and exit
#   4. Get BTC price via quote
#   5. Sell at -0.5% below price
#
# Usage: ./tests/binance/sell_btc_spot.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $BINANCE --json "$1" 2>/dev/null; }

log "Sell ALL BTC spot on Binance (JSON API)"

# ── Get balance ──────────────────────────────────────────────────────
info "Fetching spot balance to find BTC..."
BALANCE=$(ft '{"command":"balance"}')

if [[ -z "$BALANCE" ]]; then
    fail "Failed to fetch balance"
    exit 1
fi

BTC_FREE=$(echo "$BALANCE" | jq -r '.spot[]? | select(.asset == "BTC") | .free // "0"' 2>/dev/null || echo "0")
BTC_LOCKED=$(echo "$BALANCE" | jq -r '.spot[]? | select(.asset == "BTC") | .locked // "0"' 2>/dev/null || echo "0")

SELL_SIZE=$(echo "$BTC_FREE" | awk '{v = int($1 * 100000) / 100000; if (v > 0) printf "%.5f", v; else print "0"}')

if [[ "$SELL_SIZE" == "0" || "$SELL_SIZE" == "0.00000" ]]; then
    info "BTC free: $BTC_FREE  locked: $BTC_LOCKED"
    done_step
    warn "No BTC available to sell on spot"
    exit 0
fi

info "BTC free: $BTC_FREE  locked: $BTC_LOCKED  selling: $SELL_SIZE"

# ── Get BTC price ────────────────────────────────────────────────────
info "Fetching BTC price..."
QUOTE=$(ft '{"command":"quote","symbol":"BTC"}')

if [[ -z "$QUOTE" ]]; then
    fail "BTC quote failed"
    exit 1
fi

SELL_PRICE=$(echo "$QUOTE" | jq -r '.price // empty')
SELL_LIMIT=$(echo "$SELL_PRICE" | awk '{printf "%.2f", $1 * 0.995}')

info "Current price: \$$SELL_PRICE"
info "Sell limit:    \$$SELL_LIMIT (-0.5% buffer)"

# ── Place spot sell order ────────────────────────────────────────────
RESULT=$(ft "{\"command\":\"sell\",\"symbol\":\"BTC\",\"amount\":$SELL_SIZE,\"price\":$SELL_LIMIT}")

if [[ -z "$RESULT" ]]; then
    fail "BTC spot sell failed"
    warn "BTC may still be in spot. Sell manually: binance sell BTC --amount $SELL_SIZE --price $SELL_LIMIT"
    exit 1
fi

ORDER_ID=$(echo "$RESULT" | jq -r '.response.orderId // empty')
STATUS=$(echo "$RESULT" | jq -r '.response.status // empty')

done_step
info "Sold:     $SELL_SIZE BTC"
info "Limit:    \$$SELL_LIMIT"
info "Order ID: ${ORDER_ID:-unknown}"
info "Status:   ${STATUS:-unknown}"
ok "BTC spot sell order placed -- $SELL_SIZE BTC at \$$SELL_LIMIT"

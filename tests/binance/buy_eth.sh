#!/usr/bin/env bash
#
# Buy ~$12 worth of ETH USDS-M futures on Binance
#
# Uses binance --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Set ETH leverage to 2x
#   2. Get ETH price via quote
#   3. Compute buy size (~$12 worth) and limit price (+0.5%)
#   4. Place ETH futures buy order
#
# Usage: ./tests/binance/buy_eth.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $BINANCE --json "$1" 2>/dev/null; }

log "Buy ~\$12 ETH futures on Binance (JSON API)"

# ── Set leverage ─────────────────────────────────────────────────────
info "Setting ETHUSDT leverage to 2x..."
RESULT=$(ft '{"command":"perp_leverage","symbol":"ETH","leverage":2}')
if [[ -z "$RESULT" ]]; then
    fail "ETH set leverage failed"
    exit 1
fi
ok "ETH leverage set to 2x"

# ── Get ETH price ────────────────────────────────────────────────────
info "Fetching ETH price..."
QUOTE=$(ft '{"command":"quote","symbol":"ETH"}')

if [[ -z "$QUOTE" ]]; then
    fail "ETH quote failed"
    exit 1
fi

# Quote returns merged data from multiple sources; extract price
MARK_PX=$(echo "$QUOTE" | jq -r '.price // empty')

if [[ -z "$MARK_PX" || "$MARK_PX" == "null" ]]; then
    fail "ETH quote returned but price is missing"
    exit 1
fi

# ── Compute order size and limit ─────────────────────────────────────
BUY_SIZE=$(echo "12 / $MARK_PX" | bc -l | xargs printf "%.3f")
BUY_LIMIT=$(echo "$MARK_PX" | awk '{printf "%.2f", $1 * 1.005}')

info "Price:           \$$MARK_PX"
info "Limit buy price: \$$BUY_LIMIT (+0.5% buffer)"
info "Buy size:        $BUY_SIZE ETH (~\$12)"

# ── Place buy order ──────────────────────────────────────────────────
RESULT=$(ft "{\"command\":\"perp_buy\",\"symbol\":\"ETH\",\"amount\":$BUY_SIZE,\"price\":$BUY_LIMIT,\"close\":false}")

if [[ -z "$RESULT" ]]; then
    fail "ETH futures buy failed"
    exit 1
fi

ORDER_ID=$(echo "$RESULT" | jq -r '.response.orderId // empty')
STATUS=$(echo "$RESULT" | jq -r '.response.status // empty')

done_step
info "Order ID:    ${ORDER_ID:-unknown}"
info "Status:      ${STATUS:-unknown}"
info "Symbol:      ETHUSDT"
info "Size:        $BUY_SIZE ETH"
info "Limit:       \$$BUY_LIMIT"

if [[ "$STATUS" == "FILLED" ]]; then
    ok "ETH futures buy FILLED"
elif [[ "$STATUS" == "NEW" ]]; then
    warn "ETH futures buy is RESTING (not yet filled)"
    ok "ETH futures buy order placed (resting)"
elif [[ "$STATUS" == "PARTIALLY_FILLED" ]]; then
    ok "ETH futures buy PARTIALLY FILLED"
else
    ok "ETH futures buy order placed (status: ${STATUS:-unknown})"
fi

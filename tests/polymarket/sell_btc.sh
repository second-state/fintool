#!/usr/bin/env bash
#
# Sell Yes outcome on a short-term BTC prediction market
#
# Uses fintool --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Search for BTC prediction markets (or use provided slug)
#   2. Get current Yes price
#   3. Sell Yes shares at current price - 0.02 buffer
#
# Usage: ./tests/polymarket/sell_btc.sh [market-slug] [amount]
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $FINTOOL --json "$1" 2>/dev/null; }

MARKET_SLUG="${1:-}"
SELL_AMOUNT="${2:-5}"

log "Sell Yes outcome on BTC prediction market — $SELL_AMOUNT shares (JSON API)"

# ── Find a market if not specified ───────────────────────────────────
if [[ -z "$MARKET_SLUG" ]]; then
    info "Searching for BTC prediction markets..."
    MARKETS=$(ft '{"command":"predict_list","query":"bitcoin","limit":5}')

    if [[ -z "$MARKETS" ]]; then
        fail "Market search returned empty"
        exit 1
    fi

    MARKET_SLUG=$(echo "$MARKETS" | jq -r '.[0].slug // empty')

    if [[ -z "$MARKET_SLUG" ]]; then
        fail "No BTC prediction markets found"
        exit 1
    fi

    MARKET_QUESTION=$(echo "$MARKETS" | jq -r '.[0].question // "?"')
    ok "Found market: $MARKET_QUESTION"
fi

info "Slug: $MARKET_SLUG"

# ── Get current price ────────────────────────────────────────────────
info "Fetching current Yes price..."
QUOTE=$(ft "{\"command\":\"predict_quote\",\"market\":\"$MARKET_SLUG\"}")

if [[ -z "$QUOTE" ]]; then
    fail "Quote returned empty"
    exit 1
fi

YES_PRICE=$(echo "$QUOTE" | jq -r '.outcome_prices[0] // empty')

if [[ -z "$YES_PRICE" || "$YES_PRICE" == "null" ]]; then
    fail "Could not get Yes price"
    exit 1
fi

# ── Place sell order ─────────────────────────────────────────────────
SELL_PRICE=$(echo "$YES_PRICE" | awk '{p = $1 - 0.02; if (p < 0.01) p = 0.01; printf "%.2f", p}')

info "Current Yes price: \$$YES_PRICE"
info "Limit sell price:  \$$SELL_PRICE (-0.02 buffer)"
info "Amount:            $SELL_AMOUNT shares"

RESULT=$(ft "{\"command\":\"predict_sell\",\"market\":\"$MARKET_SLUG\",\"outcome\":\"Yes\",\"amount\":$SELL_AMOUNT,\"price\":$SELL_PRICE}")

if [[ -z "$RESULT" ]]; then
    fail "Sell command returned empty"
    exit 1
fi

ERROR=$(echo "$RESULT" | jq -r '.error // empty')
if [[ -n "$ERROR" ]]; then
    fail "Sell failed: $ERROR"
    exit 1
fi

ORDER_ID=$(echo "$RESULT" | jq -r '.order_id // "?"')
STATUS=$(echo "$RESULT" | jq -r '.status // "?"')

ok "Sell order placed — ID: $ORDER_ID  Status: $STATUS"
done_step
echo "$RESULT" | jq '.'

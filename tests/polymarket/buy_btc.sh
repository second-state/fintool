#!/usr/bin/env bash
#
# Buy Yes outcome on a short-term BTC prediction market
#
# Uses fintool --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Search for BTC prediction markets (or use provided slug)
#   2. Get current Yes price
#   3. Buy Yes shares at current price + 0.02 buffer
#
# Usage: ./tests/polymarket/buy_btc.sh [market-slug] [amount]
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $FINTOOL --json "$1" 2>/dev/null; }

MARKET_SLUG="${1:-}"
BUY_AMOUNT="${2:-5}"

log "Buy Yes outcome on BTC prediction market — \$${BUY_AMOUNT} USDC (JSON API)"

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

# ── Place buy order ──────────────────────────────────────────────────
BUY_PRICE=$(echo "$YES_PRICE" | awk '{p = $1 + 0.02; if (p > 0.99) p = 0.99; printf "%.2f", p}')

info "Current Yes price: \$$YES_PRICE"
info "Limit buy price:   \$$BUY_PRICE (+0.02 buffer)"
info "Amount:            \$$BUY_AMOUNT USDC"

RESULT=$(ft "{\"command\":\"predict_buy\",\"market\":\"$MARKET_SLUG\",\"outcome\":\"Yes\",\"amount\":$BUY_AMOUNT,\"price\":$BUY_PRICE}")

if [[ -z "$RESULT" ]]; then
    fail "Buy command returned empty"
    exit 1
fi

ERROR=$(echo "$RESULT" | jq -r '.error // empty')
if [[ -n "$ERROR" ]]; then
    fail "Buy failed: $ERROR"
    exit 1
fi

ORDER_ID=$(echo "$RESULT" | jq -r '.order_id // "?"')
STATUS=$(echo "$RESULT" | jq -r '.status // "?"')

ok "Buy order placed — ID: $ORDER_ID  Status: $STATUS"
done_step
echo "$RESULT" | jq '.'

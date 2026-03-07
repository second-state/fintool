#!/usr/bin/env bash
#
# Quote a short-term BTC prediction market on Polymarket
#
# Uses fintool --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Search for BTC prediction markets
#   2. Pick the first active market
#   3. Get detailed quote with Yes/No prices
#
# Usage: ./tests/polymarket/quote_btc.sh [market-slug]
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $FINTOOL --json "$1" 2>/dev/null; }

MARKET_SLUG="${1:-}"

log "Quote BTC prediction market on Polymarket (JSON API)"

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
        info "Raw response: $(echo "$MARKETS" | jq -c '.')"
        exit 1
    fi

    MARKET_QUESTION=$(echo "$MARKETS" | jq -r '.[0].question // "?"')
    ok "Found market: $MARKET_QUESTION"
fi

info "Slug: $MARKET_SLUG"

# ── Get quote ────────────────────────────────────────────────────────
info "Fetching quote..."
QUOTE=$(ft "{\"command\":\"predict_quote\",\"market\":\"$MARKET_SLUG\"}")

if [[ -z "$QUOTE" ]]; then
    fail "Quote returned empty"
    exit 1
fi

ERROR=$(echo "$QUOTE" | jq -r '.error // empty')
if [[ -n "$ERROR" ]]; then
    fail "Quote failed: $ERROR"
    exit 1
fi

YES_PRICE=$(echo "$QUOTE" | jq -r '.outcome_prices[0] // "?"')
NO_PRICE=$(echo "$QUOTE" | jq -r '.outcome_prices[1] // "?"')
VOLUME=$(echo "$QUOTE" | jq -r '.volume // "?"')

ok "Yes: \$$YES_PRICE  |  No: \$$NO_PRICE  |  Volume: \$$VOLUME"
done_step
echo "$QUOTE" | jq '.'

#!/usr/bin/env bash
#
# End-to-end Polymarket BTC prediction market test
#
# Uses fintool --json API for all commands. Output is always JSON.
#
# This script illustrates the full deposit -> trade -> exit cycle
# on Polymarket prediction markets. Every fintool call uses --json mode.
#
# Workflow:
#   1. Deposit $10 USDC from Base to Polymarket
#   2. Find a short-term BTC prediction market
#   3. Quote the Yes outcome
#   4. Buy Yes outcome shares ($5 USDC)
#   5. Quote the Yes outcome again (verify)
#   6. Sell the Yes outcome shares
#
# Usage: ./tests/polymarket/predict_btc.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $FINTOOL --json "$1" 2>/dev/null; }

BUY_AMOUNT=5
DEPOSIT_AMOUNT=10

log "Polymarket BTC Prediction — E2E Test (JSON API)"

# ── Step 1: Deposit ─────────────────────────────────────────────────
log "Step 1/6: Deposit \$${DEPOSIT_AMOUNT} USDC to Polymarket"

RESULT=$(ft "{\"command\":\"deposit\",\"asset\":\"USDC\",\"amount\":$DEPOSIT_AMOUNT,\"from\":\"base\",\"exchange\":\"polymarket\"}")

if [[ -z "$RESULT" ]]; then
    fail "Deposit command returned empty"
    exit 1
fi

ERROR=$(echo "$RESULT" | jq -r '.error // empty')
if [[ -n "$ERROR" ]]; then
    fail "Deposit failed: $ERROR"
    exit 1
fi

ok "Deposit info retrieved"

# ── Step 2: Find market ─────────────────────────────────────────────
log "Step 2/6: Find short-term BTC prediction market"

MARKETS=$(ft '{"command":"predict_list","query":"bitcoin","limit":5}')

if [[ -z "$MARKETS" ]]; then
    fail "Market search returned empty"
    exit 1
fi

MARKET_SLUG=$(echo "$MARKETS" | jq -r '.[0].slug // empty')

if [[ -z "$MARKET_SLUG" ]]; then
    fail "No BTC prediction markets found"
    info "Raw: $(echo "$MARKETS" | jq -c '.')"
    exit 1
fi

MARKET_QUESTION=$(echo "$MARKETS" | jq -r '.[0].question // "?"')
ok "Found: $MARKET_QUESTION"
info "Slug: $MARKET_SLUG"

# ── Step 3: Quote Yes ───────────────────────────────────────────────
log "Step 3/6: Quote Yes outcome"

QUOTE=$(ft "{\"command\":\"predict_quote\",\"market\":\"$MARKET_SLUG\"}")

if [[ -z "$QUOTE" ]]; then
    fail "Quote returned empty"
    exit 1
fi

YES_PRICE=$(echo "$QUOTE" | jq -r '.outcome_prices[0] // empty')
NO_PRICE=$(echo "$QUOTE" | jq -r '.outcome_prices[1] // empty')
VOLUME=$(echo "$QUOTE" | jq -r '.volume // "?"')

if [[ -z "$YES_PRICE" || "$YES_PRICE" == "null" ]]; then
    fail "Yes price missing"
    exit 1
fi

ok "Yes: \$$YES_PRICE  |  No: \$$NO_PRICE  |  Volume: \$$VOLUME"

# ── Step 4: Buy Yes ─────────────────────────────────────────────────
log "Step 4/6: Buy Yes — \$${BUY_AMOUNT} USDC"

BUY_PRICE=$(echo "$YES_PRICE" | awk '{p = $1 + 0.02; if (p > 0.99) p = 0.99; printf "%.2f", p}')

info "Limit price: \$$BUY_PRICE (current + 0.02)"

RESULT=$(ft "{\"command\":\"predict_buy\",\"market\":\"$MARKET_SLUG\",\"outcome\":\"Yes\",\"amount\":$BUY_AMOUNT,\"price\":$BUY_PRICE}")

if [[ -z "$RESULT" ]]; then
    fail "Buy returned empty"
    exit 1
fi

ERROR=$(echo "$RESULT" | jq -r '.error // empty')
if [[ -n "$ERROR" ]]; then
    fail "Buy failed: $ERROR"
    exit 1
fi

ok "Buy placed — $(echo "$RESULT" | jq -r '.order_id // "?"') ($(echo "$RESULT" | jq -r '.status // "?"'))"

# ── Step 5: Re-quote ────────────────────────────────────────────────
log "Step 5/6: Re-quote Yes outcome"

sleep 2

QUOTE2=$(ft "{\"command\":\"predict_quote\",\"market\":\"$MARKET_SLUG\"}")

YES_PRICE2=$(echo "$QUOTE2" | jq -r '.outcome_prices[0] // "?"')
NO_PRICE2=$(echo "$QUOTE2" | jq -r '.outcome_prices[1] // "?"')

ok "Yes: \$$YES_PRICE2  |  No: \$$NO_PRICE2"

if [[ "$YES_PRICE" != "$YES_PRICE2" ]]; then
    info "Price moved: $YES_PRICE → $YES_PRICE2"
else
    info "Price unchanged"
fi

# ── Step 6: Sell Yes ────────────────────────────────────────────────
log "Step 6/6: Sell Yes shares"

SELL_PRICE=$(echo "$YES_PRICE2" | awk '{p = $1 - 0.02; if (p < 0.01) p = 0.01; printf "%.2f", p}')

info "Sell price: \$$SELL_PRICE (current - 0.02)"

RESULT=$(ft "{\"command\":\"predict_sell\",\"market\":\"$MARKET_SLUG\",\"outcome\":\"Yes\",\"amount\":$BUY_AMOUNT,\"price\":$SELL_PRICE}")

if [[ -z "$RESULT" ]]; then
    fail "Sell returned empty"
    exit 1
fi

ERROR=$(echo "$RESULT" | jq -r '.error // empty')
if [[ -n "$ERROR" ]]; then
    fail "Sell failed: $ERROR"
    exit 1
fi

ok "Sell placed — $(echo "$RESULT" | jq -r '.order_id // "?"') ($(echo "$RESULT" | jq -r '.status // "?"'))"

# ── Done ─────────────────────────────────────────────────────────────
log "✅ Polymarket BTC prediction e2e test complete"

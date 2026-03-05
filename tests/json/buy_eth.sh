#!/usr/bin/env bash
#
# Buy ~$12 worth of ETH perp on Hyperliquid
#
# Uses fintool --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Set ETH leverage to 2x
#   2. Get ETH mark price via perp_quote
#   3. Compute buy size (~$12 worth) and limit price (+0.5%)
#   4. Place ETH perp buy order
#
# Usage: ./tests/json/buy_eth.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $FINTOOL --json "$1" 2>/dev/null; }

log "Buy ~\$12 ETH perp on Hyperliquid (JSON API)"

# ── Set leverage ─────────────────────────────────────────────────────
info "Setting ETH leverage to 2x..."
RESULT=$(ft '{"command":"perp_leverage","symbol":"ETH","leverage":2}')
if [[ -z "$RESULT" ]]; then
    fail "ETH set leverage failed"
    exit 1
fi
ok "ETH leverage set to 2x"

# ── Get ETH price ────────────────────────────────────────────────────
info "Fetching ETH mark price..."
QUOTE=$(ft '{"command":"perp_quote","symbol":"ETH"}')

if [[ -z "$QUOTE" ]]; then
    fail "ETH perp quote failed"
    exit 1
fi

MARK_PX=$(echo "$QUOTE" | jq -r '.markPx')

if [[ -z "$MARK_PX" || "$MARK_PX" == "null" ]]; then
    fail "ETH quote returned but markPx is missing"
    exit 1
fi

# ── Compute order size and limit ─────────────────────────────────────
BUY_SIZE=$(echo "12 / $MARK_PX" | bc -l | xargs printf "%.4f")
BUY_LIMIT=$(echo "$MARK_PX" | awk '{printf "%.2f", $1 * 1.005}')

info "Mark price:      \$$MARK_PX"
info "Limit buy price: \$$BUY_LIMIT (+0.5% buffer)"
info "Buy size:        $BUY_SIZE ETH (~\$12)"

# ── Place buy order ──────────────────────────────────────────────────
RESULT=$(ft "{\"command\":\"perp_buy\",\"symbol\":\"ETH\",\"amount\":\"$BUY_SIZE\",\"price\":\"$BUY_LIMIT\",\"close\":false}")

if [[ -z "$RESULT" ]]; then
    fail "ETH perp buy failed"
    exit 1
fi

FILL=$(echo "$RESULT" | jq -r '.fillStatus // empty')

done_step
info "Size:        $(echo "$RESULT" | jq -r '.size // empty')"
info "Price:       \$$(echo "$RESULT" | jq -r '.price // empty')"
info "Fill status: $FILL"

if [[ "$FILL" == "filled" ]]; then
    ok "ETH perp buy FILLED"
elif [[ "$FILL" == "resting" ]]; then
    warn "ETH perp buy is RESTING (not yet filled)"
    ok "ETH perp buy order placed (resting)"
elif [[ "$FILL" == error* ]]; then
    fail "ETH perp buy ERROR: $FILL"
    exit 1
else
    ok "ETH perp buy order placed (status: ${FILL:-unknown})"
fi

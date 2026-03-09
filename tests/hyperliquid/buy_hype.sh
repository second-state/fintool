#!/usr/bin/env bash
#
# Buy ~$12 worth of HYPE spot on Hyperliquid
#
# Uses hyperliquid --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Get HYPE spot price via quote
#   2. Compute buy size (~$12 worth) and limit price (+0.5%)
#   3. Place HYPE spot buy order
#
# Usage: ./tests/json/buy_hype.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $HYPERLIQUID --json "$1" 2>/dev/null; }

log "Buy ~\$12 HYPE spot on Hyperliquid (JSON API)"

# ── Get HYPE price ───────────────────────────────────────────────────
info "Fetching HYPE price..."
QUOTE=$(ft '{"command":"quote","symbol":"HYPE"}')

if [[ -z "$QUOTE" ]]; then
    fail "HYPE quote failed"
    exit 1
fi

PRICE=$(echo "$QUOTE" | jq -r '.markPx // empty')

if [[ -z "$PRICE" || "$PRICE" == "null" ]]; then
    fail "HYPE quote returned but price field is missing"
    exit 1
fi

# ── Compute order size and limit ─────────────────────────────────────
BUY_SIZE=$(echo "12 / $PRICE" | bc -l | xargs printf "%.2f")
BUY_LIMIT=$(echo "$PRICE" | awk '{printf "%.4f", $1 * 1.005}')

info "HYPE price:      \$$PRICE"
info "Limit buy price: \$$BUY_LIMIT (+0.5% buffer)"
info "Buy size:        $BUY_SIZE HYPE (~\$12)"

# ── Place buy order ──────────────────────────────────────────────────
RESULT=$(ft "{\"command\":\"buy\",\"symbol\":\"HYPE\",\"amount\":$BUY_SIZE,\"price\":$BUY_LIMIT}")

if [[ -z "$RESULT" ]]; then
    fail "HYPE spot buy failed"
    exit 1
fi

FILL=$(echo "$RESULT" | jq -r '.fillStatus // empty')

done_step
info "Fill status: $FILL"

if [[ "$FILL" == "filled" ]]; then
    ok "HYPE spot buy FILLED"
elif [[ "$FILL" == "resting" ]]; then
    warn "HYPE spot buy is RESTING (not yet filled)"
    ok "HYPE spot buy order placed (resting)"
elif [[ "$FILL" == error* ]]; then
    fail "HYPE spot buy ERROR: $FILL"
    exit 1
else
    ok "HYPE spot buy order placed (status: ${FILL:-unknown})"
fi

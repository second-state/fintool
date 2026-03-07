#!/usr/bin/env bash
#
# Buy ~$1 worth of TSLA on Coinbase
#
# Uses fintool --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Get TSLA spot price via quote
#   2. Compute buy size (~$1 worth) and limit price (+1%)
#   3. Place TSLA spot buy order on Coinbase
#
# Usage: ./tests/json/buy_tsla.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $FINTOOL --json "$1" 2>/dev/null; }

log "Buy ~\$1 TSLA on Coinbase (JSON API)"

# ── Get TSLA price ───────────────────────────────────────────────────
info "Fetching TSLA spot price..."
QUOTE=$(ft '{"command":"quote","symbol":"TSLA"}')

if [[ -z "$QUOTE" ]]; then
    fail "TSLA spot quote failed"
    exit 1
fi

PRICE=$(echo "$QUOTE" | jq -r '.price // empty')

if [[ -z "$PRICE" || "$PRICE" == "null" ]]; then
    fail "TSLA quote returned but price field is missing"
    exit 1
fi

# ── Compute order size and limit ─────────────────────────────────────
BUY_SIZE=$(echo "1 / $PRICE" | bc -l | xargs printf "%.6f")
BUY_LIMIT=$(echo "$PRICE" | awk '{printf "%.2f", $1 * 1.01}')

info "TSLA price:       \$$PRICE"
info "Limit buy price:  \$$BUY_LIMIT (+1%)"
info "Buy size:         $BUY_SIZE shares (~\$1)"

# ── Place buy order on Coinbase ──────────────────────────────────────
RESULT=$(ft "{\"command\":\"order_buy\",\"symbol\":\"TSLA\",\"amount\":$BUY_SIZE,\"price\":$BUY_LIMIT,\"exchange\":\"coinbase\"}")

if [[ -z "$RESULT" ]]; then
    fail "TSLA spot buy on Coinbase failed"
    exit 1
fi

FILL=$(echo "$RESULT" | jq -r '.fillStatus // .status // empty')

done_step
info "Fill status: ${FILL:-unknown}"
ok "TSLA spot buy placed on Coinbase -- ~$BUY_SIZE shares at \$$BUY_LIMIT"

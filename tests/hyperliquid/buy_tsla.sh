#!/usr/bin/env bash
#
# Buy ~$1 worth of TSLA perp on Hyperliquid (HIP-3 stock perp)
#
# Uses hyperliquid --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Get TSLA perp price via perp_quote
#   2. Compute buy size (~$1 worth) and limit price (+1%)
#   3. Place TSLA perp buy order on Hyperliquid
#
# Usage: ./tests/hyperliquid/buy_tsla.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $HYPERLIQUID --json "$1" 2>/dev/null; }

log "Buy ~\$1 TSLA perp on Hyperliquid (JSON API)"

# ── Get TSLA price ───────────────────────────────────────────────────
info "Fetching TSLA perp price..."
QUOTE=$(ft '{"command":"perp_quote","symbol":"TSLA"}')

if [[ -z "$QUOTE" ]]; then
    fail "TSLA perp quote failed"
    exit 1
fi

PRICE=$(echo "$QUOTE" | jq -r '.markPx // empty')

if [[ -z "$PRICE" || "$PRICE" == "null" ]]; then
    fail "TSLA perp quote returned but markPx field is missing"
    exit 1
fi

# ── Compute order size and limit ─────────────────────────────────────
BUY_SIZE=$(echo "1 / $PRICE" | bc -l | xargs printf "%.6f")
BUY_LIMIT=$(echo "$PRICE" | awk '{printf "%.2f", $1 * 1.01}')

info "TSLA price:       \$$PRICE"
info "Limit buy price:  \$$BUY_LIMIT (+1%)"
info "Buy size:         $BUY_SIZE TSLA (~\$1)"

# ── Place perp buy order on Hyperliquid ──────────────────────────────
RESULT=$(ft "{\"command\":\"perp_buy\",\"symbol\":\"TSLA\",\"amount\":$BUY_SIZE,\"price\":$BUY_LIMIT}")

if [[ -z "$RESULT" ]]; then
    fail "TSLA perp buy on Hyperliquid failed"
    exit 1
fi

FILL=$(echo "$RESULT" | jq -r '.fillStatus // .status // empty')

done_step
info "Fill status: ${FILL:-unknown}"
ok "TSLA perp buy placed on Hyperliquid -- ~$BUY_SIZE TSLA at \$$BUY_LIMIT"

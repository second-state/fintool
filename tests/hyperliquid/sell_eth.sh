#!/usr/bin/env bash
#
# Sell ALL ETH perp position on Hyperliquid
#
# Uses fintool --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Get positions via positions command
#   2. Extract ETH position size (szi)
#   3. If no position, warn and exit
#   4. Get ETH price via perp_quote
#   5. Sell with close flag at -0.5% below mark
#
# Usage: ./tests/json/sell_eth.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $FINTOOL --json "$1" 2>/dev/null; }

log "Sell ALL ETH perp on Hyperliquid (JSON API)"

# ── Get positions ────────────────────────────────────────────────────
info "Fetching positions to find ETH size..."
POSITIONS=$(ft '{"command":"positions"}')

if [[ -z "$POSITIONS" ]]; then
    fail "Failed to fetch positions"
    exit 1
fi

ETH_SIZE=$(echo "$POSITIONS" | jq -r '
    [.[] | .position // .] |
    map(select(.coin == "ETH")) |
    .[0].szi // empty
' 2>/dev/null || true)

if [[ -z "$ETH_SIZE" || "$ETH_SIZE" == "null" ]]; then
    info "No ETH position found. Checking open orders..."
    ORDERS=$(ft '{"command":"orders"}')
    if [[ -n "$ORDERS" ]]; then
        OPEN_COUNT=$(echo "$ORDERS" | jq -r 'length' 2>/dev/null || echo "0")
        info "Open orders: $OPEN_COUNT"
    fi
    done_step
    warn "No ETH position to sell -- order may not have filled"
    exit 0
fi

# ── Compute sell size (absolute value) ───────────────────────────────
SELL_SIZE=$(echo "$ETH_SIZE" | sed 's/^-//')
info "ETH position size: $ETH_SIZE (selling $SELL_SIZE)"

# ── Get ETH price ────────────────────────────────────────────────────
info "Fetching ETH mark price..."
QUOTE=$(ft '{"command":"perp_quote","symbol":"ETH"}')

if [[ -z "$QUOTE" ]]; then
    fail "ETH perp quote failed"
    exit 1
fi

SELL_PRICE=$(echo "$QUOTE" | jq -r '.markPx')
SELL_LIMIT=$(echo "$SELL_PRICE" | awk '{printf "%.2f", $1 * 0.995}')

info "Current mark: \$$SELL_PRICE"
info "Sell limit:   \$$SELL_LIMIT (-0.5% buffer)"

# ── Place sell order with close flag ─────────────────────────────────
RESULT=$(ft "{\"command\":\"perp_sell\",\"symbol\":\"ETH\",\"amount\":$SELL_SIZE,\"price\":$SELL_LIMIT,\"close\":true}")

if [[ -z "$RESULT" ]]; then
    fail "ETH perp sell failed"
    warn "Position may still be open -- check manually with 'fintool positions'"
    exit 1
fi

FILL=$(echo "$RESULT" | jq -r '.fillStatus // empty')

done_step
info "Sold:        $SELL_SIZE ETH"
info "Limit:       \$$SELL_LIMIT"
info "Fill status: $FILL"
ok "ETH perp sell order placed -- $SELL_SIZE ETH at \$$SELL_LIMIT"

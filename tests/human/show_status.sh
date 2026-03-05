#!/usr/bin/env bash
#
# Display positions, orders, and balance on Hyperliquid
#
# Uses the human CLI API — fintool commands produce human-readable output.
# Displays fintool's output directly, and also queries the HL API for counts.
#
# Usage: ./tests/human/show_status.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

USER_ADDR=$($FINTOOL address 2>/dev/null)
hl_api() {
  curl -s https://api.hyperliquid.xyz/info -H 'Content-Type: application/json' -d "$1"
}

log "Show positions, orders, and balances on Hyperliquid"

# ── Positions ────────────────────────────────────────────────────────
info "-- Perp Positions (including HIP-3) --"
run_fintool positions
if [[ $LAST_EXIT -eq 0 ]]; then
    echo "$LAST_STDOUT"
else
    warn "Could not fetch positions"
fi

# Query HL API for position count
POS_JSON=$(hl_api "{\"type\":\"clearinghouseState\",\"user\":\"$USER_ADDR\"}")
POS_COUNT=$(echo "$POS_JSON" | jq -r '.assetPositions | length' 2>/dev/null || echo "?")
info "Open positions (from HL API): $POS_COUNT"

echo ""

# ── Orders ───────────────────────────────────────────────────────────
info "-- Open Orders --"
run_fintool orders
if [[ $LAST_EXIT -eq 0 ]]; then
    echo "$LAST_STDOUT"
else
    warn "Could not fetch orders"
fi

# Query HL API for order count
ORD_JSON=$(hl_api "{\"type\":\"openOrders\",\"user\":\"$USER_ADDR\"}")
ORD_COUNT=$(echo "$ORD_JSON" | jq -r 'length' 2>/dev/null || echo "?")
info "Open orders (from HL API): $ORD_COUNT"

echo ""

# ── Balance ──────────────────────────────────────────────────────────
info "-- Balances (Perp + Spot) --"
run_fintool balance
if [[ $LAST_EXIT -eq 0 ]]; then
    echo "$LAST_STDOUT"
else
    warn "Could not fetch balance"
fi

done_step
ok "Status displayed"

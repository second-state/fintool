#!/usr/bin/env bash
#
# Display all positions, orders, and balance in JSON format
#
# Uses fintool --json API for all commands. Output is always JSON.
#
# Shows:
#   - Perp positions (including HIP-3)
#   - Open orders
#   - Balances (perp + spot)
#
# Usage: ./tests/json/show_status.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
ensure_built

ft() { $FINTOOL --json "$1" 2>/dev/null; }

log "Show positions, orders, and balances on Hyperliquid (JSON API)"

# ── Positions ────────────────────────────────────────────────────────
info "-- Perp Positions (including HIP-3) --"
POSITIONS=$(ft '{"command":"positions"}')

if [[ -n "$POSITIONS" ]]; then
    POS_COUNT=$(echo "$POSITIONS" | jq -r 'length' 2>/dev/null || echo "?")
    info "Open positions: $POS_COUNT"
    if [[ "$POS_COUNT" != "0" && "$POS_COUNT" != "?" ]]; then
        echo "$POSITIONS" | jq -r '.[] | .position // . | "    \(.coin): size=\(.szi) entryPx=$\(.entryPx // "?") unrealizedPnl=$\(.unrealizedPnl // "?")"' 2>/dev/null || echo "$POSITIONS" | jq '.' 2>/dev/null
    fi
else
    warn "Could not fetch positions"
fi

# ── Orders ───────────────────────────────────────────────────────────
echo ""
info "-- Open Orders --"
ORDERS=$(ft '{"command":"orders"}')

if [[ -n "$ORDERS" ]]; then
    ORD_COUNT=$(echo "$ORDERS" | jq -r 'length' 2>/dev/null || echo "?")
    info "Open orders: $ORD_COUNT"
    if [[ "$ORD_COUNT" != "0" && "$ORD_COUNT" != "?" ]]; then
        echo "$ORDERS" | jq '.' 2>/dev/null || echo "$ORDERS"
    fi
else
    warn "Could not fetch orders"
fi

# ── Balances ─────────────────────────────────────────────────────────
echo ""
info "-- Balances (Perp + Spot) --"
BALANCE=$(ft '{"command":"balance"}')

if [[ -n "$BALANCE" ]]; then
    # Perp balance
    PERP_VALUE=$(echo "$BALANCE" | jq -r '.perp.marginSummary.accountValue // .perp.crossMarginSummary.accountValue // empty' 2>/dev/null || true)
    PERP_WITHDRAWABLE=$(echo "$BALANCE" | jq -r '.perp.withdrawable // empty' 2>/dev/null || true)
    info "Perp account value: \$${PERP_VALUE:-unknown}"
    info "Perp withdrawable:  \$${PERP_WITHDRAWABLE:-unknown}"

    # Spot balances
    SPOT_COINS=$(echo "$BALANCE" | jq -r '.spot.balances[]? | "\(.coin): \(.total) (hold: \(.hold))"' 2>/dev/null || true)
    if [[ -n "$SPOT_COINS" ]]; then
        info "Spot balances:"
        echo "$SPOT_COINS" | while read -r line; do
            info "  $line"
        done
    else
        info "Spot balances: (none)"
    fi
else
    warn "Could not fetch balance"
fi

done_step
ok "Status displayed"

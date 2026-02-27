#!/usr/bin/env bash
#
# Show all positions, pending orders, and balances (perp + spot) on Hyperliquid
#
# Usage: ./tests/show_status.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
ensure_built

log "Show positions, orders, and balances on Hyperliquid"

info "── Perp Positions (including HIP-3) ──"
run_fintool positions
if [[ $LAST_EXIT -eq 0 ]]; then
    POS_COUNT=$(echo "$LAST_STDOUT" | jq -r 'length' 2>/dev/null || echo "?")
    info "Open positions: $POS_COUNT"
    if [[ "$POS_COUNT" != "0" && "$POS_COUNT" != "?" ]]; then
        echo "$LAST_STDOUT" | jq -r '.[] | .position // . | "    \(.coin): size=\(.szi) entryPx=$\(.entryPx // "?") unrealizedPnl=$\(.unrealizedPnl // "?")"' 2>/dev/null || echo "$LAST_STDOUT" | jq '.' 2>/dev/null
    fi
else
    warn "Could not fetch positions"
fi

echo ""
info "── Open Orders ──"
run_fintool orders
if [[ $LAST_EXIT -eq 0 ]]; then
    ORD_COUNT=$(echo "$LAST_STDOUT" | jq -r 'length' 2>/dev/null || echo "?")
    info "Open orders: $ORD_COUNT"
    if [[ "$ORD_COUNT" != "0" && "$ORD_COUNT" != "?" ]]; then
        echo "$LAST_STDOUT" | jq '.' 2>/dev/null || echo "$LAST_STDOUT"
    fi
else
    warn "Could not fetch orders"
fi

echo ""
info "── Balances (Perp + Spot) ──"
run_fintool balance
if [[ $LAST_EXIT -eq 0 ]]; then
    # Perp balance
    PERP_VALUE=$(echo "$LAST_STDOUT" | jq -r '.perp.marginSummary.accountValue // .perp.crossMarginSummary.accountValue // empty' 2>/dev/null || true)
    PERP_WITHDRAWABLE=$(echo "$LAST_STDOUT" | jq -r '.perp.withdrawable // empty' 2>/dev/null || true)
    info "Perp account value: \$${PERP_VALUE:-unknown}"
    info "Perp withdrawable:  \$${PERP_WITHDRAWABLE:-unknown}"

    # Spot balances
    SPOT_COINS=$(echo "$LAST_STDOUT" | jq -r '.spot.balances[]? | "\(.coin): \(.total) (hold: \(.hold))"' 2>/dev/null || true)
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

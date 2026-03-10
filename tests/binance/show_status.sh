#!/usr/bin/env bash
#
# Display all positions, orders, and balances on Binance in JSON format
#
# Uses binance --json API for all commands. Output is always JSON.
#
# Shows:
#   - Futures positions
#   - Open orders (spot + futures)
#   - Balances (spot + futures)
#
# Usage: ./tests/binance/show_status.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $BINANCE --json "$1" 2>/dev/null; }

log "Show positions, orders, and balances on Binance (JSON API)"

# ── Positions ────────────────────────────────────────────────────────
info "-- Futures Positions --"
POSITIONS=$(ft '{"command":"positions"}')

if [[ -n "$POSITIONS" ]]; then
    POS_COUNT=$(echo "$POSITIONS" | jq -r '.futures | length' 2>/dev/null || echo "?")
    info "Open futures positions: $POS_COUNT"
    if [[ "$POS_COUNT" != "0" && "$POS_COUNT" != "?" ]]; then
        echo "$POSITIONS" | jq -r '.futures[]? | "    \(.symbol): size=\(.positionAmt) entryPx=$\(.entryPrice) unrealizedPnl=$\(.unRealizedProfit) leverage=\(.leverage)x"' 2>/dev/null || echo "$POSITIONS" | jq '.' 2>/dev/null
    fi
else
    warn "Could not fetch positions"
fi

# ── Orders ───────────────────────────────────────────────────────────
echo ""
info "-- Open Orders (Spot + Futures) --"
ORDERS=$(ft '{"command":"orders"}')

if [[ -n "$ORDERS" ]]; then
    SPOT_COUNT=$(echo "$ORDERS" | jq -r '.spot | length' 2>/dev/null || echo "0")
    FUTURES_COUNT=$(echo "$ORDERS" | jq -r '.futures | length' 2>/dev/null || echo "0")
    info "Open spot orders:    $SPOT_COUNT"
    info "Open futures orders: $FUTURES_COUNT"
    if [[ "$SPOT_COUNT" != "0" || "$FUTURES_COUNT" != "0" ]]; then
        echo "$ORDERS" | jq '.' 2>/dev/null || echo "$ORDERS"
    fi
else
    warn "Could not fetch orders"
fi

# ── Balances ─────────────────────────────────────────────────────────
echo ""
info "-- Balances (Spot + Futures) --"
BALANCE=$(ft '{"command":"balance"}')

if [[ -n "$BALANCE" ]]; then
    # Spot balances
    SPOT_COINS=$(echo "$BALANCE" | jq -r '.spot[]? | select((.free | tonumber) > 0 or (.locked | tonumber) > 0) | "\(.asset): free=\(.free) locked=\(.locked)"' 2>/dev/null || true)
    if [[ -n "$SPOT_COINS" ]]; then
        info "Spot balances:"
        echo "$SPOT_COINS" | while read -r line; do
            info "  $line"
        done
    else
        info "Spot balances: (none with balance)"
    fi

    # Futures balances
    FUTURES_COINS=$(echo "$BALANCE" | jq -r '.futures[]? | select((.balance | tonumber) > 0) | "\(.asset): balance=\(.balance) available=\(.availableBalance)"' 2>/dev/null || true)
    if [[ -n "$FUTURES_COINS" ]]; then
        info "Futures balances:"
        echo "$FUTURES_COINS" | while read -r line; do
            info "  $line"
        done
    else
        info "Futures balances: (none with balance)"
    fi
else
    warn "Could not fetch balance"
fi

done_step
ok "Status displayed"

#!/usr/bin/env bash
#
# Show OKX account status: balance, positions, orders
#
# Uses okx --json API for all commands. Output is always JSON.
#
# Usage: ./tests/okx/show_status.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $OKX --json "$1" 2>/dev/null; }

log "Show OKX account status (JSON API)"

# ── Balance ────────────────────────────────────────────────────────
log "Balance"
BALANCE=$(ft '{"command":"balance"}')

if [[ -n "$BALANCE" ]]; then
    info "Trading account:"
    echo "$BALANCE" | jq -r '.trading // [] | .[]? | .details // [] | .[]? | select((.eq // "0") | tonumber > 0) | "    \(.ccy): \(.eq) (avail: \(.availBal))"' 2>/dev/null || true
    info "Funding account:"
    echo "$BALANCE" | jq -r '.funding // [] | .[]? | select((.bal // "0") | tonumber > 0) | "    \(.ccy): \(.bal) (avail: \(.availBal))"' 2>/dev/null || true
    ok "Balance displayed"
else
    fail "Could not fetch balance"
fi

# ── Positions ──────────────────────────────────────────────────────
echo ""
log "Positions"
POSITIONS=$(ft '{"command":"positions"}')

if [[ -n "$POSITIONS" ]]; then
    POS_COUNT=$(echo "$POSITIONS" | jq -r '.positions | length' 2>/dev/null || echo "0")
    info "Open positions: $POS_COUNT"
    if [[ "$POS_COUNT" -gt 0 ]]; then
        echo "$POSITIONS" | jq -r '.positions[]? | "    \(.instId) | \(.posSide) \(.pos) | entry: \(.avgPx) | PnL: \(.upl)"' 2>/dev/null || true
    fi
    ok "Positions displayed"
else
    fail "Could not fetch positions"
fi

# ── Orders ─────────────────────────────────────────────────────────
echo ""
log "Open Orders"
ORDERS=$(ft '{"command":"orders"}')

if [[ -n "$ORDERS" ]]; then
    ORD_COUNT=$(echo "$ORDERS" | jq -r '.orders | length' 2>/dev/null || echo "0")
    info "Open orders: $ORD_COUNT"
    if [[ "$ORD_COUNT" -gt 0 ]]; then
        echo "$ORDERS" | jq -r '.orders[]? | "    \(.instId) | \(.side) \(.sz) @ $\(.px) | \(.ordType) | \(.ordId)"' 2>/dev/null || true
    fi
    ok "Orders displayed"
else
    fail "Could not fetch orders"
fi

done_step
ok "OKX status check completed"

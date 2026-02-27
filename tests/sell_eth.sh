#!/usr/bin/env bash
#
# Sell ALL ETH perp on Hyperliquid
#
# Usage: ./tests/sell_eth.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
ensure_built

log "Sell ALL ETH perp on Hyperliquid"
info "Fetching positions to find ETH size, then selling all."

run_fintool positions

if check_fail "Failed to fetch positions"; then
    exit 1
fi

ETH_SIZE=$(echo "$LAST_STDOUT" | jq -r '
    [.[] | .position // .] |
    map(select(.coin == "ETH")) |
    .[0].szi // empty
' 2>/dev/null || true)

if [[ -z "$ETH_SIZE" || "$ETH_SIZE" == "null" ]]; then
    info "No ETH position found. Checking open orders..."
    run_fintool orders
    if [[ $LAST_EXIT -eq 0 ]]; then
        OPEN_COUNT=$(echo "$LAST_STDOUT" | jq -r 'length' 2>/dev/null || echo "0")
        info "Open orders: $OPEN_COUNT"
    fi
    done_step
    warn "No ETH position to sell — order may not have filled"
    exit 0
fi

SELL_SIZE=$(echo "$ETH_SIZE" | sed 's/^-//')
info "ETH position size: $ETH_SIZE (selling $SELL_SIZE)"

run_fintool perp quote ETH
if check_fail "ETH perp quote failed"; then
    exit 1
fi
SELL_PRICE=$(echo "$LAST_STDOUT" | jq -r '.markPx' 2>/dev/null)
SELL_LIMIT=$(echo "$SELL_PRICE" | awk '{printf "%.2f", $1 * 0.995}')

info "Current mark: \$$SELL_PRICE"
info "Sell limit:   \$$SELL_LIMIT (-0.5% buffer)"

run_fintool perp sell ETH "$SELL_SIZE" "$SELL_LIMIT" --close

if check_fail "ETH perp sell failed"; then
    warn "Position may still be open — check manually with 'fintool positions'"
    exit 1
fi

SELL_JSON="$LAST_STDOUT"
SELL_FILL=$(echo "$SELL_JSON" | jq -r '.fillStatus // empty' 2>/dev/null || true)

done_step
info "Sold:        $SELL_SIZE ETH"
info "Limit:       \$$SELL_LIMIT"
info "Fill status: $SELL_FILL"
ok "ETH perp sell order placed — $SELL_SIZE ETH at \$$SELL_LIMIT"

#!/usr/bin/env bash
#
# Buy 0.006 ETH perp on Hyperliquid (~$12 at $2000/ETH)
#
# Usage: ./tests/buy_eth.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
ensure_built

log "Buy ~\$12 ETH perp on Hyperliquid"

info "Setting ETH leverage to 2x..."
run_fintool perp leverage ETH --leverage 2
if check_fail "ETH set leverage failed"; then
    exit 1
fi

info "Fetching ETH mark price, then placing a limit buy at +0.5% above mark."

run_fintool perp quote ETH

if check_fail "ETH perp quote failed"; then
    exit 1
fi

ETH_PRICE=$(echo "$LAST_STDOUT" | jq -r '.markPx' 2>/dev/null)

if [[ -z "$ETH_PRICE" || "$ETH_PRICE" == "null" ]]; then
    fail "ETH quote returned but markPx is missing"
    exit 1
fi

BUY_LIMIT=$(echo "$ETH_PRICE" | awk '{printf "%.2f", $1 * 1.005}')
BUY_SIZE=$(echo "$ETH_PRICE" | awk '{printf "%.6f", 12.0 / $1}')

info "Mark price:      \$$ETH_PRICE"
info "Limit buy price: \$$BUY_LIMIT (+0.5% buffer)"
info "Buy size:        $BUY_SIZE ETH"

run_fintool perp buy ETH --amount "$BUY_SIZE" --price "$BUY_LIMIT"

if check_fail "ETH perp buy failed"; then
    exit 1
fi

BUY_JSON="$LAST_STDOUT"
BUY_FILL=$(echo "$BUY_JSON" | jq -r '.fillStatus // empty' 2>/dev/null || true)

done_step
info "Size:        $(echo "$BUY_JSON" | jq -r '.size // empty' 2>/dev/null || true)"
info "Price:       \$$(echo "$BUY_JSON" | jq -r '.price // empty' 2>/dev/null || true)"
info "Fill status: $BUY_FILL"

if [[ "$BUY_FILL" == "filled" ]]; then
    ok "ETH perp buy FILLED"
elif [[ "$BUY_FILL" == "resting" ]]; then
    warn "ETH perp buy is RESTING (not yet filled)"
    ok "ETH perp buy order placed (resting)"
elif [[ "$BUY_FILL" == error* ]]; then
    fail "ETH perp buy ERROR: $BUY_FILL"
    exit 1
else
    ok "ETH perp buy order placed (status: ${BUY_FILL:-unknown})"
fi

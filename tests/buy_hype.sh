#!/usr/bin/env bash
#
# Buy $12 HYPE spot on Hyperliquid
#
# Usage: ./tests/buy_hype.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
ensure_built

log "Buy \$12 HYPE spot on Hyperliquid"
info "Fetching HYPE spot price, then placing a limit buy at +0.5% above mark."

run_fintool quote HYPE

if check_fail "HYPE spot quote failed"; then
    exit 1
fi

HYPE_PRICE=$(echo "$LAST_STDOUT" | jq -r '.price // .markPx // empty' 2>/dev/null)

if [[ -z "$HYPE_PRICE" || "$HYPE_PRICE" == "null" ]]; then
    fail "HYPE quote returned but price field is missing"
    exit 1
fi

BUY_LIMIT=$(echo "$HYPE_PRICE" | awk '{printf "%.4f", $1 * 1.005}')
BUY_SIZE=$(echo "$BUY_LIMIT" | awk '{printf "%.4f", 12.0 / $1}')

info "HYPE price:      \$$HYPE_PRICE"
info "Limit buy price: \$$BUY_LIMIT (+0.5% buffer)"
info "Estimated size:  $BUY_SIZE HYPE"

run_fintool order buy HYPE 12 "$BUY_LIMIT"

if check_fail "HYPE spot buy failed"; then
    exit 1
fi

BUY_JSON="$LAST_STDOUT"
BUY_FILL=$(echo "$BUY_JSON" | jq -r '.fillStatus // empty' 2>/dev/null || true)

done_step
info "Fill status: $BUY_FILL"

if [[ "$BUY_FILL" == "filled" ]]; then
    ok "HYPE spot buy FILLED"
elif [[ "$BUY_FILL" == "resting" ]]; then
    warn "HYPE spot buy is RESTING (not yet filled)"
    ok "HYPE spot buy order placed (resting)"
elif [[ "$BUY_FILL" == error* ]]; then
    fail "HYPE spot buy ERROR: $BUY_FILL"
    exit 1
else
    ok "HYPE spot buy order placed (status: ${BUY_FILL:-unknown})"
fi

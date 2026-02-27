#!/usr/bin/env bash
#
# Buy $1 TSLA on Coinbase
#
# Usage: ./tests/buy_tsla.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
ensure_built

log "Buy \$1 TSLA on Coinbase"
info "Fetching TSLA price, then placing a spot limit buy on Coinbase."

run_fintool quote TSLA

if check_fail "TSLA spot quote failed"; then
    exit 1
fi

TSLA_PRICE=$(echo "$LAST_STDOUT" | jq -r '.price // empty' 2>/dev/null)

if [[ -z "$TSLA_PRICE" || "$TSLA_PRICE" == "null" ]]; then
    fail "TSLA quote returned but price field is missing"
    exit 1
fi

TSLA_LIMIT=$(echo "$TSLA_PRICE" | awk '{printf "%.2f", $1 * 1.01}')
TSLA_SIZE=$(echo "$TSLA_PRICE" | awk '{printf "%.6f", 1.0 / $1}')

info "TSLA price:       \$$TSLA_PRICE"
info "Limit buy price:  \$$TSLA_LIMIT (+1%)"
info "Estimated size:   $TSLA_SIZE shares"

run_fintool order buy TSLA 1 "$TSLA_LIMIT" --exchange coinbase

if check_fail "TSLA spot buy on Coinbase failed"; then
    exit 1
fi

done_step
ok "TSLA spot buy placed on Coinbase — ~$TSLA_SIZE shares at \$$TSLA_LIMIT"

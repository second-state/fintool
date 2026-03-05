#!/usr/bin/env bash
#
# Buy ~$12 worth of ETH perp on Hyperliquid
#
# Uses the human CLI API — fintool commands produce human-readable output.
# Data extraction (prices, positions) is done via the Hyperliquid API directly.
#
# Usage: ./tests/human/buy_eth.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

USER_ADDR=$($FINTOOL address 2>/dev/null)
hl_api() {
  curl -s https://api.hyperliquid.xyz/info -H 'Content-Type: application/json' -d "$1"
}

log "Buy ~\$12 ETH perp on Hyperliquid"

# ── Set leverage ─────────────────────────────────────────────────────
info "Setting ETH leverage to 2x..."
run_fintool perp leverage ETH --leverage 2
if check_fail "ETH set leverage failed"; then
    exit 1
fi
ok "ETH leverage set to 2x"

# ── Get ETH price from HL API ────────────────────────────────────────
info "Fetching ETH mark price from Hyperliquid API..."
META_JSON=$(hl_api '{"type":"metaAndAssetCtxs"}')
ETH_PRICE=$(echo "$META_JSON" | jq -r '.[1][] | select(.coin == "ETH") | .markPx // empty' 2>/dev/null)

if [[ -z "$ETH_PRICE" || "$ETH_PRICE" == "null" ]]; then
    # Fallback: search in the array by index
    ETH_PRICE=$(echo "$META_JSON" | jq -r '
        . as $root |
        ($root[0].universe | to_entries[] | select(.value.name == "ETH") | .key) as $idx |
        $root[1][$idx].markPx
    ' 2>/dev/null)
fi

if [[ -z "$ETH_PRICE" || "$ETH_PRICE" == "null" ]]; then
    fail "Could not fetch ETH mark price from HL API"
    exit 1
fi

# ── Compute order size and limit ─────────────────────────────────────
BUY_LIMIT=$(echo "$ETH_PRICE" | awk '{printf "%.2f", $1 * 1.005}')
BUY_SIZE=$(echo "$ETH_PRICE" | awk '{printf "%.6f", 12.0 / $1}')

info "Mark price:      \$$ETH_PRICE"
info "Limit buy price: \$$BUY_LIMIT (+0.5% buffer)"
info "Buy size:        $BUY_SIZE ETH (~\$12)"

# ── Place buy order ──────────────────────────────────────────────────
run_fintool perp buy ETH --amount "$BUY_SIZE" --price "$BUY_LIMIT"

if check_fail "ETH perp buy failed"; then
    exit 1
fi

done_step
info "Output: $LAST_STDOUT"
ok "ETH perp buy order placed"

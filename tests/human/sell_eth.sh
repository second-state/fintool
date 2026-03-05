#!/usr/bin/env bash
#
# Sell ALL ETH perp position on Hyperliquid
#
# Uses the human CLI API — fintool commands produce human-readable output.
# Data extraction (positions, prices) is done via the Hyperliquid API directly.
#
# Usage: ./tests/human/sell_eth.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

USER_ADDR=$($FINTOOL address 2>/dev/null)
hl_api() {
  curl -s https://api.hyperliquid.xyz/info -H 'Content-Type: application/json' -d "$1"
}

log "Sell ALL ETH perp on Hyperliquid"

# ── Get ETH position from HL API ─────────────────────────────────────
info "Fetching positions from Hyperliquid API..."
POS_JSON=$(hl_api "{\"type\":\"clearinghouseState\",\"user\":\"$USER_ADDR\"}")
ETH_SIZE=$(echo "$POS_JSON" | jq -r '
    .assetPositions[]? |
    .position | select(.coin == "ETH") | .szi // empty
' 2>/dev/null || true)

if [[ -z "$ETH_SIZE" || "$ETH_SIZE" == "null" || "$ETH_SIZE" == "0" ]]; then
    done_step
    warn "No ETH position found -- order may not have filled"
    exit 0
fi

SELL_SIZE=$(echo "$ETH_SIZE" | sed 's/^-//')
info "ETH position size: $ETH_SIZE (selling $SELL_SIZE)"

# ── Get ETH price from HL API ────────────────────────────────────────
info "Fetching ETH mark price from Hyperliquid API..."
META_JSON=$(hl_api '{"type":"metaAndAssetCtxs"}')
ETH_PRICE=$(echo "$META_JSON" | jq -r '
    . as $root |
    ($root[0].universe | to_entries[] | select(.value.name == "ETH") | .key) as $idx |
    $root[1][$idx].markPx
' 2>/dev/null)

if [[ -z "$ETH_PRICE" || "$ETH_PRICE" == "null" ]]; then
    fail "Could not fetch ETH mark price from HL API"
    exit 1
fi

SELL_LIMIT=$(echo "$ETH_PRICE" | awk '{printf "%.2f", $1 * 0.995}')

info "Current mark: \$$ETH_PRICE"
info "Sell limit:   \$$SELL_LIMIT (-0.5% buffer)"

# ── Place sell order ─────────────────────────────────────────────────
run_fintool perp sell ETH --amount "$SELL_SIZE" --price "$SELL_LIMIT" --close

if check_fail "ETH perp sell failed"; then
    warn "Position may still be open -- check manually with 'fintool positions'"
    exit 1
fi

done_step
info "Sold:   $SELL_SIZE ETH"
info "Limit:  \$$SELL_LIMIT"
info "Output: $LAST_STDOUT"
ok "ETH perp sell order placed -- $SELL_SIZE ETH at \$$SELL_LIMIT"

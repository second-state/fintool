#!/usr/bin/env bash
#
# Sell ALL SILVER perp on Hyperliquid (HIP-3 cash dex)
#
# Uses the human CLI API — fintool commands produce human-readable output.
# Data extraction (positions, balances, prices) is done via the Hyperliquid API directly.
#
# After selling, transfers USDT0 from cash dex back to spot,
# then swaps USDT0 -> USDC on spot.
#
# Usage: ./tests/human/sell_silver.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

USER_ADDR=$($FINTOOL address 2>/dev/null)
hl_api() {
  curl -s https://api.hyperliquid.xyz/info -H 'Content-Type: application/json' -d "$1"
}

log "Sell ALL SILVER perp on Hyperliquid (cash dex)"

# ── Get SILVER position from HL API (cash dex) ──────────────────────
info "Fetching cash dex positions from Hyperliquid API..."
CASH_POS=$(hl_api "{\"type\":\"clearinghouseState\",\"user\":\"$USER_ADDR\",\"dex\":\"cash\"}")
SILVER_SIZE=$(echo "$CASH_POS" | jq -r '
    .assetPositions[]? |
    .position | select(.coin == "SILVER") | .szi // empty
' 2>/dev/null || true)

if [[ -z "$SILVER_SIZE" || "$SILVER_SIZE" == "null" || "$SILVER_SIZE" == "0" ]]; then
    done_step
    warn "No SILVER position found -- order may not have filled"
    exit 0
fi

SELL_SIZE=$(echo "$SILVER_SIZE" | sed 's/^-//')
info "SILVER position size: $SILVER_SIZE oz (selling $SELL_SIZE)"

# ── Get SILVER price from HL API (cash dex) ──────────────────────────
info "Fetching SILVER mark price from Hyperliquid cash dex API..."
CASH_META=$(hl_api '{"type":"metaAndAssetCtxs","dex":"cash"}')
SILVER_PRICE=$(echo "$CASH_META" | jq -r '
    . as $root |
    ($root[0].universe | to_entries[] | select(.value.name == "SILVER") | .key) as $idx |
    $root[1][$idx].markPx
' 2>/dev/null)

if [[ -z "$SILVER_PRICE" || "$SILVER_PRICE" == "null" ]]; then
    fail "Could not fetch SILVER mark price from HL API"
    exit 1
fi

SELL_LIMIT=$(echo "$SILVER_PRICE" | awk '{printf "%.4f", $1 * 0.995}')

info "Current mark: \$$SILVER_PRICE"
info "Sell limit:   \$$SELL_LIMIT (-0.5% buffer)"

# ── Sell SILVER perp ─────────────────────────────────────────────────
run_fintool perp sell SILVER --amount "$SELL_SIZE" --price "$SELL_LIMIT" --close

if check_fail "SILVER perp sell failed"; then
    warn "Position may still be open -- check manually with 'fintool positions'"
    exit 1
fi

done_step
info "Sold:   $SELL_SIZE oz"
info "Limit:  \$$SELL_LIMIT"
info "Output: $LAST_STDOUT"
ok "SILVER perp sell order placed -- $SELL_SIZE oz at \$$SELL_LIMIT"

# ── Transfer USDT0 from cash dex back to spot ────────────────────────
info "Transferring USDT0 from cash dex back to spot..."
sleep 2

info "Fetching cash dex withdrawable from HL API..."
CASH_STATE=$(hl_api "{\"type\":\"clearinghouseState\",\"user\":\"$USER_ADDR\",\"dex\":\"cash\"}")
CASH_WITHDRAWABLE=$(echo "$CASH_STATE" | jq -r '.withdrawable // "0"' 2>/dev/null || echo "0")
info "Cash dex withdrawable: $CASH_WITHDRAWABLE USDT0"

# Round down to avoid rounding issues
TRANSFER_AMT=$(echo "$CASH_WITHDRAWABLE" | awk '{v = int($1 * 100) / 100; if (v > 0) printf "%.2f", v; else print "0"}')

if [[ "$TRANSFER_AMT" != "0" && "$TRANSFER_AMT" != "0.00" ]]; then
    info "Transferring $TRANSFER_AMT USDT0 from cash dex to spot..."
    run_fintool transfer USDT0 --amount "$TRANSFER_AMT" --from cash --to spot
    if check_fail "USDT0 transfer from cash dex failed"; then
        warn "USDT0 may still be in cash dex. Use: fintool transfer USDT0 --amount <amount> --from cash --to spot"
    else
        ok "Transferred $TRANSFER_AMT USDT0 from cash dex to spot"
    fi
    sleep 1
else
    info "No withdrawable USDT0 in cash dex"
fi

# ── Swap spot USDT0 -> USDC ──────────────────────────────────────────
info "Checking spot USDT0 balance from HL API..."
SPOT_STATE=$(hl_api "{\"type\":\"spotClearinghouseState\",\"user\":\"$USER_ADDR\"}")
SPOT_USDT0=$(echo "$SPOT_STATE" | jq -r '.balances[]? | select(.coin == "USDT0") | .total // "0"' 2>/dev/null || echo "0")
USDT0_HOLD=$(echo "$SPOT_STATE" | jq -r '.balances[]? | select(.coin == "USDT0") | .hold // "0"' 2>/dev/null || echo "0")

# Subtract hold and round down
SELL_AMT=$(echo "$SPOT_USDT0 $USDT0_HOLD" | awk '{v = int(($1 - $2) * 100) / 100; if (v > 0) printf "%.2f", v; else print "0"}')

if [[ "$SELL_AMT" != "0" && "$SELL_AMT" != "0.00" ]]; then
    info "Swapping $SELL_AMT USDT0 -> USDC on spot..."
    run_fintool order sell USDT0 --amount "$SELL_AMT" --price 0.998
    if check_fail "USDT0 -> USDC spot swap failed"; then
        warn "USDT0 still in spot. Sell manually: fintool order sell USDT0 --amount $SELL_AMT --price 0.998"
    else
        ok "Swapped $SELL_AMT USDT0 -> USDC"
    fi
else
    info "No USDT0 available to swap back to USDC"
fi

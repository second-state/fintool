#!/usr/bin/env bash
#
# Buy ~$12 worth of SILVER perp on Hyperliquid (HIP-3 cash dex)
#
# Uses the human CLI API — fintool commands produce human-readable output.
# Data extraction (prices, balances) is done via the Hyperliquid API directly.
#
# The cash dex uses USDT0 as collateral, so the workflow is:
# 1. Check USDC balance and swap USDC -> USDT0 on spot
# 2. Transfer USDT0 from spot to cash dex
# 3. Set leverage and place SILVER perp buy
#
# Usage: ./tests/human/buy_silver.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

USER_ADDR=$($FINTOOL address 2>/dev/null)
hl_api() {
  curl -s https://api.hyperliquid.xyz/info -H 'Content-Type: application/json' -d "$1"
}

log "Buy ~\$12 SILVER perp on Hyperliquid (cash dex)"

# ── Set leverage ─────────────────────────────────────────────────────
info "Setting SILVER leverage to 2x..."
run_fintool perp leverage SILVER --leverage 2
if check_fail "SILVER set leverage failed"; then
    exit 1
fi
ok "SILVER leverage set to 2x"

# ── Step 1: Check USDC balance and swap to USDT0 ────────────────────
info "Checking USDC balance from HL API..."
SPOT_STATE=$(hl_api "{\"type\":\"spotClearinghouseState\",\"user\":\"$USER_ADDR\"}")
SPOT_USDC=$(echo "$SPOT_STATE" | jq -r '.balances[]? | select(.coin == "USDC") | .total // "0"' 2>/dev/null || echo "0")
info "Spot USDC balance: \$$SPOT_USDC"

# Buy USDT0 with available USDC (leave a small buffer)
BUY_USDT0=$(echo "$SPOT_USDC" | awk '{v = int($1 * 100) / 100; if (v > 0.5) printf "%.2f", v - 0.50; else print "0"}')

if [[ "$BUY_USDT0" != "0" ]] && (( $(echo "$BUY_USDT0 > 0" | bc -l) )); then
    info "Swapping \$$BUY_USDT0 USDC -> USDT0 on spot (cash dex collateral)..."
    run_fintool order buy USDT0 --amount "$BUY_USDT0" --price 1.002
    if check_fail "USDC -> USDT0 spot swap failed"; then
        exit 1
    fi
    ok "USDT0 swap submitted"
    sleep 2
else
    warn "Insufficient USDC for USDT0 swap (balance: \$$SPOT_USDC)"
fi

# ── Step 2: Check USDT0 balance and transfer to cash dex ────────────
info "Checking spot USDT0 balance from HL API..."
SPOT_STATE=$(hl_api "{\"type\":\"spotClearinghouseState\",\"user\":\"$USER_ADDR\"}")
SPOT_USDT0=$(echo "$SPOT_STATE" | jq -r '.balances[]? | select(.coin == "USDT0") | .total // "0"' 2>/dev/null || echo "0")
info "Spot USDT0 balance: $SPOT_USDT0"

TRANSFER_AMT=$(echo "$SPOT_USDT0" | awk '{v = int($1 * 100) / 100; if (v > 0) printf "%.2f", v; else print "0"}')

if [[ "$TRANSFER_AMT" != "0" && "$TRANSFER_AMT" != "0.00" ]]; then
    info "Transferring $TRANSFER_AMT USDT0 from spot to cash dex..."
    run_fintool transfer USDT0 --amount "$TRANSFER_AMT" --from spot --to cash
    if check_fail "USDT0 transfer to cash dex failed"; then
        exit 1
    fi
    ok "Transferred $TRANSFER_AMT USDT0 to cash dex"
    sleep 1
else
    warn "No USDT0 available to transfer to cash dex"
fi

# ── Step 3: Get SILVER price and buy ─────────────────────────────────
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

BUY_LIMIT=$(echo "$SILVER_PRICE" | awk '{printf "%.4f", $1 * 1.005}')
BUY_SIZE=$(echo "$SILVER_PRICE" | awk '{printf "%.4f", 12.0 / $1}')

info "Mark price:      \$$SILVER_PRICE"
info "Limit buy price: \$$BUY_LIMIT (+0.5% buffer)"
info "Buy size:        $BUY_SIZE oz (~\$12)"

run_fintool perp buy SILVER --amount "$BUY_SIZE" --price "$BUY_LIMIT"

if check_fail "SILVER perp buy failed"; then
    exit 1
fi

done_step
info "Output: $LAST_STDOUT"
ok "SILVER perp buy order placed"

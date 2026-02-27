#!/usr/bin/env bash
#
# Sell ALL SILVER perp on Hyperliquid
#
# After selling, transfers USDT0 from cash dex back to spot,
# then swaps USDT0 → USDC on spot.
#
# Usage: ./tests/sell_silver.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
ensure_built

log "Sell ALL SILVER perp on Hyperliquid"
info "Fetching positions to find SILVER size, then selling all."

run_fintool positions

if check_fail "Failed to fetch positions"; then
    exit 1
fi

SILVER_SIZE=$(echo "$LAST_STDOUT" | jq -r '
    [.[] | .position // .] |
    map(select(.coin == "SILVER" or .coin == "cash:SILVER")) |
    .[0].szi // empty
' 2>/dev/null || true)

if [[ -z "$SILVER_SIZE" || "$SILVER_SIZE" == "null" ]]; then
    info "No SILVER position found. Checking open orders..."
    run_fintool orders
    if [[ $LAST_EXIT -eq 0 ]]; then
        OPEN_COUNT=$(echo "$LAST_STDOUT" | jq -r 'length' 2>/dev/null || echo "0")
        info "Open orders: $OPEN_COUNT"
    fi
    done_step
    warn "No SILVER position to sell — order may not have filled"
    exit 0
fi

SELL_SIZE=$(echo "$SILVER_SIZE" | sed 's/^-//')
info "SILVER position size: $SILVER_SIZE oz (selling $SELL_SIZE)"

run_fintool perp quote SILVER
if check_fail "SILVER perp quote failed"; then
    exit 1
fi
SELL_PRICE=$(echo "$LAST_STDOUT" | jq -r '.markPx' 2>/dev/null)
SELL_LIMIT=$(echo "$SELL_PRICE" | awk '{printf "%.4f", $1 * 0.995}')

info "Current mark: \$$SELL_PRICE"
info "Sell limit:   \$$SELL_LIMIT (-0.5% buffer)"

run_fintool perp sell SILVER "$SELL_SIZE" "$SELL_LIMIT" --close

if check_fail "SILVER perp sell failed"; then
    warn "Position may still be open — check manually with 'fintool positions'"
    exit 1
fi

SELL_JSON="$LAST_STDOUT"
SELL_FILL=$(echo "$SELL_JSON" | jq -r '.fillStatus // empty' 2>/dev/null || true)

done_step
info "Sold:        $SELL_SIZE oz"
info "Limit:       \$$SELL_LIMIT"
info "Fill status: $SELL_FILL"
ok "SILVER perp sell order placed — $SELL_SIZE oz at \$$SELL_LIMIT"

# ── Transfer USDT0 from cash dex back to spot ──────────────────────
info "Transferring USDT0 from cash dex back to spot..."
sleep 2

# Get wallet address and query cash dex withdrawable balance
run_fintool address
USER_ADDR=$(echo "$LAST_STDOUT" | jq -r '.address // empty' 2>/dev/null || true)

CASH_WITHDRAWABLE="0"
if [[ -n "$USER_ADDR" ]]; then
    CASH_STATE=$(curl -s -X POST https://api.hyperliquid.xyz/info \
        -H 'Content-Type: application/json' \
        -d "{\"type\":\"clearinghouseState\",\"user\":\"$USER_ADDR\",\"dex\":\"cash\"}" 2>/dev/null)
    CASH_WITHDRAWABLE=$(echo "$CASH_STATE" | jq -r '.withdrawable // "0"' 2>/dev/null || echo "0")
    info "Cash dex withdrawable: $CASH_WITHDRAWABLE USDT0"
fi

# Round down to avoid rounding issues
TRANSFER_AMT=$(echo "$CASH_WITHDRAWABLE" | awk '{v = int($1 * 100) / 100; if (v > 0) printf "%.2f", v; else print "0"}')

if [[ "$TRANSFER_AMT" != "0" && "$TRANSFER_AMT" != "0.00" ]]; then
    info "Transferring $TRANSFER_AMT USDT0 from cash dex to spot..."
    run_fintool transfer "$TRANSFER_AMT" from-dex --dex cash
    if check_fail "USDT0 transfer from cash dex failed"; then
        warn "USDT0 may still be in cash dex. Use: fintool transfer <amount> from-dex --dex cash"
    else
        ok "Transferred $TRANSFER_AMT USDT0 from cash dex to spot"
    fi
    sleep 1
else
    info "No withdrawable USDT0 in cash dex"
fi

# ── Swap ALL spot USDT0 → USDC ─────────────────────────────────────
# USDT0 may be in spot from the cash dex transfer above, or leftover from
# the buy step. Swap everything back to USDC so the user can withdraw.
info "Checking spot USDT0 balance..."
run_fintool balance
SPOT_USDT0=$(echo "$LAST_STDOUT" | jq -r '.spot.balances[]? | select(.coin == "USDT0") | .total // "0"' 2>/dev/null || echo "0")
USDT0_HOLD=$(echo "$LAST_STDOUT" | jq -r '.spot.balances[]? | select(.coin == "USDT0") | .hold // "0"' 2>/dev/null || echo "0")

# Subtract hold amount (used as cash dex margin) and round down
SELL_AMT=$(echo "$SPOT_USDT0 $USDT0_HOLD" | awk '{v = int(($1 - $2) * 100) / 100; if (v > 0) printf "%.2f", v; else print "0"}')

if [[ "$SELL_AMT" != "0" && "$SELL_AMT" != "0.00" ]]; then
    info "Swapping $SELL_AMT USDT0 → USDC on spot..."
    run_fintool order sell USDT0 "$SELL_AMT" 0.998
    if check_fail "USDT0 → USDC spot swap failed"; then
        warn "USDT0 still in spot. Sell manually: fintool order sell USDT0 $SELL_AMT 0.998"
    else
        SWAP_FILL=$(echo "$LAST_STDOUT" | jq -r '.fillStatus // empty' 2>/dev/null || true)
        info "USDT0→USDC swap fill: $SWAP_FILL"
        ok "Swapped $SELL_AMT USDT0 → USDC"
    fi
else
    info "No USDT0 available to swap back to USDC"
fi

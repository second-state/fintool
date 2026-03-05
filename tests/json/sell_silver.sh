#!/usr/bin/env bash
#
# Sell ALL SILVER perp on Hyperliquid + transfer USDT0 back + swap to USDC
#
# Uses fintool --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Get positions, find SILVER
#   2. Get SILVER price from perp_quote
#   3. Sell SILVER perp with close flag
#   4. Get wallet address via address command
#   5. Query cash dex withdrawable via HL API (curl)
#   6. Transfer USDT0 from cash dex to spot
#   7. Check spot USDT0 balance
#   8. Sell USDT0 for USDC on spot
#
# Usage: ./tests/json/sell_silver.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $FINTOOL --json "$1" 2>/dev/null; }

log "Sell ALL SILVER perp on Hyperliquid (JSON API)"

# ── Step 1: Get positions ────────────────────────────────────────────
info "Fetching positions to find SILVER size..."
POSITIONS=$(ft '{"command":"positions"}')

if [[ -z "$POSITIONS" ]]; then
    fail "Failed to fetch positions"
    exit 1
fi

SILVER_SIZE=$(echo "$POSITIONS" | jq -r '
    [.[] | .position // .] |
    map(select(.coin == "SILVER" or .coin == "cash:SILVER")) |
    .[0].szi // empty
' 2>/dev/null || true)

if [[ -z "$SILVER_SIZE" || "$SILVER_SIZE" == "null" ]]; then
    info "No SILVER position found. Checking open orders..."
    ORDERS=$(ft '{"command":"orders"}')
    if [[ -n "$ORDERS" ]]; then
        OPEN_COUNT=$(echo "$ORDERS" | jq -r 'length' 2>/dev/null || echo "0")
        info "Open orders: $OPEN_COUNT"
    fi
    done_step
    warn "No SILVER position to sell -- order may not have filled"
    exit 0
fi

SELL_SIZE=$(echo "$SILVER_SIZE" | sed 's/^-//')
info "SILVER position size: $SILVER_SIZE oz (selling $SELL_SIZE)"

# ── Step 2: Get SILVER price ─────────────────────────────────────────
info "Fetching SILVER mark price..."
QUOTE=$(ft '{"command":"perp_quote","symbol":"SILVER"}')

if [[ -z "$QUOTE" ]]; then
    fail "SILVER perp quote failed"
    exit 1
fi

SELL_PRICE=$(echo "$QUOTE" | jq -r '.markPx')
SELL_LIMIT=$(echo "$SELL_PRICE" | awk '{printf "%.4f", $1 * 0.995}')

info "Current mark: \$$SELL_PRICE"
info "Sell limit:   \$$SELL_LIMIT (-0.5% buffer)"

# ── Step 3: Sell SILVER perp with close flag ─────────────────────────
RESULT=$(ft "{\"command\":\"perp_sell\",\"symbol\":\"SILVER\",\"amount\":\"$SELL_SIZE\",\"price\":\"$SELL_LIMIT\",\"close\":true}")

if [[ -z "$RESULT" ]]; then
    fail "SILVER perp sell failed"
    warn "Position may still be open -- check manually with 'fintool positions'"
    exit 1
fi

FILL=$(echo "$RESULT" | jq -r '.fillStatus // empty')

done_step
info "Sold:        $SELL_SIZE oz"
info "Limit:       \$$SELL_LIMIT"
info "Fill status: $FILL"
ok "SILVER perp sell order placed -- $SELL_SIZE oz at \$$SELL_LIMIT"

# ── Step 4: Get wallet address ───────────────────────────────────────
info "Transferring USDT0 from cash dex back to spot..."
sleep 2

ADDR_JSON=$(ft '{"command":"address"}')
USER_ADDR=$(echo "$ADDR_JSON" | jq -r '.address // empty')

# ── Step 5: Query cash dex withdrawable via HL API ───────────────────
CASH_WITHDRAWABLE="0"
if [[ -n "$USER_ADDR" ]]; then
    CASH_STATE=$(curl -s -X POST https://api.hyperliquid.xyz/info \
        -H 'Content-Type: application/json' \
        -d "{\"type\":\"clearinghouseState\",\"user\":\"$USER_ADDR\",\"dex\":\"cash\"}" 2>/dev/null)
    CASH_WITHDRAWABLE=$(echo "$CASH_STATE" | jq -r '.withdrawable // "0"' 2>/dev/null || echo "0")
    info "Cash dex withdrawable: $CASH_WITHDRAWABLE USDT0"
fi

# ── Step 6: Transfer USDT0 from cash to spot ─────────────────────────
TRANSFER_AMT=$(echo "$CASH_WITHDRAWABLE" | awk '{v = int($1 * 100) / 100; if (v > 0) printf "%.2f", v; else print "0"}')

if [[ "$TRANSFER_AMT" != "0" && "$TRANSFER_AMT" != "0.00" ]]; then
    info "Transferring $TRANSFER_AMT USDT0 from cash dex to spot..."
    RESULT=$(ft "{\"command\":\"transfer\",\"asset\":\"USDT0\",\"amount\":\"$TRANSFER_AMT\",\"from\":\"cash\",\"to\":\"spot\"}")
    if [[ -z "$RESULT" ]]; then
        warn "USDT0 transfer from cash dex failed"
        warn "USDT0 may still be in cash dex. Use: fintool transfer USDT0 --amount <amount> --from cash --to spot"
    else
        ok "Transferred $TRANSFER_AMT USDT0 from cash dex to spot"
    fi
    sleep 1
else
    info "No withdrawable USDT0 in cash dex"
fi

# ── Step 7: Check spot USDT0 balance ─────────────────────────────────
info "Checking spot USDT0 balance..."
BALANCE=$(ft '{"command":"balance"}')
SPOT_USDT0=$(echo "$BALANCE" | jq -r '.spot.balances[]? | select(.coin == "USDT0") | .total // "0"' 2>/dev/null || echo "0")
USDT0_HOLD=$(echo "$BALANCE" | jq -r '.spot.balances[]? | select(.coin == "USDT0") | .hold // "0"' 2>/dev/null || echo "0")

# ── Step 8: Sell USDT0 for USDC ──────────────────────────────────────
SELL_AMT=$(echo "$SPOT_USDT0 $USDT0_HOLD" | awk '{v = int(($1 - $2) * 100) / 100; if (v > 0) printf "%.2f", v; else print "0"}')

if [[ "$SELL_AMT" != "0" && "$SELL_AMT" != "0.00" ]]; then
    info "Swapping $SELL_AMT USDT0 -> USDC on spot..."
    RESULT=$(ft "{\"command\":\"order_sell\",\"symbol\":\"USDT0\",\"amount\":\"$SELL_AMT\",\"price\":\"0.998\"}")
    if [[ -z "$RESULT" ]]; then
        warn "USDT0 -> USDC spot swap failed"
        warn "USDT0 still in spot. Sell manually: fintool order sell USDT0 --amount $SELL_AMT --price 0.998"
    else
        SWAP_FILL=$(echo "$RESULT" | jq -r '.fillStatus // empty')
        info "USDT0->USDC swap fill: $SWAP_FILL"
        ok "Swapped $SELL_AMT USDT0 -> USDC"
    fi
else
    info "No USDT0 available to swap back to USDC"
fi

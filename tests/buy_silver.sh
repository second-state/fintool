#!/usr/bin/env bash
#
# Buy ~$12 SILVER perp on Hyperliquid
#
# The cash dex uses USDT0 as collateral, so we need to:
# 1. Swap USDC → USDT0 on spot
# 2. Transfer USDT0 from spot to cash dex
# 3. Place SILVER perp buy
#
# Usage: ./tests/buy_silver.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
ensure_built

log "Buy ~\$12 SILVER perp on Hyperliquid"

info "Setting SILVER leverage to 2x..."
run_fintool perp leverage SILVER --leverage 2
if check_fail "SILVER set leverage failed"; then
    exit 1
fi

# ── Step 1: Swap USDC → USDT0 on spot ──────────────────────────────
# The cash dex (SILVER, GOLD, stocks) uses USDT0 as collateral, not USDC.
# We buy USDT0 with USDC at a slight premium to ensure fill.
info "Checking spot USDC balance..."
run_fintool balance
SPOT_USDC=$(echo "$LAST_STDOUT" | jq -r '.spot.balances[]? | select(.coin == "USDC") | .total // "0"' 2>/dev/null || echo "0")
info "Spot USDC: \$$SPOT_USDC"

# Round down to leave a small buffer
SWAP_AMT=$(echo "$SPOT_USDC" | awk '{v = int($1 * 100) / 100; if (v > 0.5) printf "%.2f", v - 0.50; else print "0"}')

if [[ "$SWAP_AMT" != "0" ]] && (( $(echo "$SWAP_AMT > 0" | bc -l) )); then
    info "Swapping \$$SWAP_AMT USDC → USDT0 on spot (cash dex collateral)..."
    run_fintool order buy USDT0 --amount "$SWAP_AMT" --price 1.002
    if check_fail "USDC → USDT0 spot swap failed"; then
        exit 1
    fi
    SWAP_FILL=$(echo "$LAST_STDOUT" | jq -r '.fillStatus // empty' 2>/dev/null || true)
    info "Swap fill status: $SWAP_FILL"
    sleep 1
fi

# ── Step 2: Transfer USDT0 from spot to cash dex ───────────────────
info "Checking spot USDT0 balance..."
run_fintool balance
SPOT_USDT0=$(echo "$LAST_STDOUT" | jq -r '.spot.balances[]? | select(.coin == "USDT0") | .total // "0"' 2>/dev/null || echo "0")
info "Spot USDT0: $SPOT_USDT0"

TRANSFER_AMT=$(echo "$SPOT_USDT0" | awk '{v = int($1 * 100) / 100; if (v > 0) printf "%.2f", v; else print "0"}')

if [[ "$TRANSFER_AMT" != "0" && "$TRANSFER_AMT" != "0.00" ]]; then
    info "Transferring $TRANSFER_AMT USDT0 from spot to cash dex..."
    run_fintool transfer USDT0 --amount "$TRANSFER_AMT" --from spot --to cash
    if check_fail "USDT0 transfer to cash dex failed"; then
        exit 1
    fi
    sleep 1
else
    warn "No USDT0 available to transfer to cash dex"
fi

# ── Step 3: Place SILVER perp buy ──────────────────────────────────
info "Fetching SILVER mark price, then placing a limit buy at +0.5% above mark."

run_fintool perp quote SILVER

if check_fail "SILVER perp quote failed"; then
    exit 1
fi

SILVER_PRICE=$(echo "$LAST_STDOUT" | jq -r '.markPx' 2>/dev/null)

if [[ -z "$SILVER_PRICE" || "$SILVER_PRICE" == "null" ]]; then
    fail "SILVER quote returned but markPx is missing"
    exit 1
fi

BUY_LIMIT=$(echo "$SILVER_PRICE" | awk '{printf "%.4f", $1 * 1.005}')
BUY_SIZE=$(echo "$SILVER_PRICE" | awk '{printf "%.4f", 12.0 / $1}')

info "Mark price:      \$$SILVER_PRICE"
info "Limit buy price: \$$BUY_LIMIT (+0.5% buffer)"
info "Buy size:        $BUY_SIZE oz"

run_fintool perp buy SILVER --amount "$BUY_SIZE" --price "$BUY_LIMIT"

if check_fail "SILVER perp buy failed"; then
    exit 1
fi

BUY_JSON="$LAST_STDOUT"
BUY_FILL=$(echo "$BUY_JSON" | jq -r '.fillStatus // empty' 2>/dev/null || true)

done_step
info "Size:        $(echo "$BUY_JSON" | jq -r '.size // empty' 2>/dev/null || true)"
info "Price:       \$$(echo "$BUY_JSON" | jq -r '.price // empty' 2>/dev/null || true)"
info "Fill status: $BUY_FILL"

if [[ "$BUY_FILL" == "filled" ]]; then
    ok "SILVER perp buy FILLED"
elif [[ "$BUY_FILL" == "resting" ]]; then
    warn "SILVER perp buy is RESTING (not yet filled)"
    ok "SILVER perp buy order placed (resting)"
elif [[ "$BUY_FILL" == error* ]]; then
    fail "SILVER perp buy ERROR: $BUY_FILL"
    exit 1
else
    ok "SILVER perp buy order placed (status: ${BUY_FILL:-unknown})"
fi

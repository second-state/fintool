#!/usr/bin/env bash
#
# Buy ~$12 worth of TSLA perp on Hyperliquid (HIP-3 stock perp)
#
# Uses hyperliquid --json API for all commands. Output is always JSON.
#
# The HIP-3 cash dex uses USDT0 as collateral (not USDC), so the workflow is:
#   1. Set TSLA leverage to 2x
#   2. Check USDC balance
#   3. Swap USDC -> USDT0 on spot (buy USDT0)
#   4. Wait for settlement, check USDT0 balance
#   5. Transfer USDT0 from spot to cash dex
#   6. Get TSLA price via perp_quote
#   7. Place TSLA perp buy order
#
# Usage: ./tests/hyperliquid/buy_tsla.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $HYPERLIQUID --json "$1" 2>/dev/null; }

log "Buy ~\$12 TSLA perp on Hyperliquid (JSON API)"

# ── Step 1: Set leverage ─────────────────────────────────────────────
info "Setting TSLA leverage to 2x..."
RESULT=$(ft '{"command":"perp_leverage","symbol":"TSLA","leverage":2}')
if [[ -z "$RESULT" ]]; then
    fail "TSLA set leverage failed"
    exit 1
fi
ok "TSLA leverage set to 2x"

# ── Step 2: Check USDC balance ───────────────────────────────────────
info "Checking spot USDC balance..."
BALANCE=$(ft '{"command":"balance"}')
SPOT_USDC=$(echo "$BALANCE" | jq -r '.spot.balances[]? | select(.coin == "USDC") | .total // "0"' 2>/dev/null || echo "0")
info "Spot USDC: \$$SPOT_USDC"

# Round down and leave a small buffer
SWAP_AMT=$(echo "$SPOT_USDC" | awk '{v = int($1 * 100) / 100; if (v > 0.5) printf "%.2f", v - 0.50; else print "0"}')

# ── Step 3: Swap USDC -> USDT0 on spot ───────────────────────────────
if [[ "$SWAP_AMT" != "0" ]] && (( $(echo "$SWAP_AMT > 0" | bc -l) )); then
    info "Swapping \$$SWAP_AMT USDC -> USDT0 on spot (cash dex collateral)..."
    RESULT=$(ft "{\"command\":\"buy\",\"symbol\":\"USDT0\",\"amount\":$SWAP_AMT,\"price\":1.002}")
    if [[ -z "$RESULT" ]]; then
        fail "USDC -> USDT0 spot swap failed"
        exit 1
    fi
    SWAP_FILL=$(echo "$RESULT" | jq -r '.fillStatus // empty')
    info "Swap fill status: $SWAP_FILL"
    sleep 1
fi

# ── Step 4: Wait and check USDT0 balance ─────────────────────────────
info "Waiting 5 seconds for settlement..."
sleep 5

info "Checking spot USDT0 balance..."
BALANCE=$(ft '{"command":"balance"}')
SPOT_USDT0=$(echo "$BALANCE" | jq -r '.spot.balances[]? | select(.coin == "USDT0") | .total // "0"' 2>/dev/null || echo "0")
info "Spot USDT0: $SPOT_USDT0"

# ── Step 5: Transfer USDT0 from spot to cash dex ─────────────────────
TRANSFER_AMT=$(echo "$SPOT_USDT0" | awk '{v = int($1 * 100) / 100; if (v > 0) printf "%.2f", v; else print "0"}')

if [[ "$TRANSFER_AMT" != "0" && "$TRANSFER_AMT" != "0.00" ]]; then
    info "Transferring $TRANSFER_AMT USDT0 from spot to cash dex..."
    RESULT=$(ft "{\"command\":\"transfer\",\"asset\":\"USDT0\",\"amount\":$TRANSFER_AMT,\"from\":\"spot\",\"to\":\"cash\"}")
    if [[ -z "$RESULT" ]]; then
        fail "USDT0 transfer to cash dex failed"
        exit 1
    fi
    ok "Transferred $TRANSFER_AMT USDT0 to cash dex"
    sleep 1
else
    warn "No USDT0 available to transfer to cash dex"
fi

# ── Step 6: Get TSLA price ───────────────────────────────────────────
info "Fetching TSLA perp price..."
QUOTE=$(ft '{"command":"perp_quote","symbol":"TSLA"}')

if [[ -z "$QUOTE" ]]; then
    fail "TSLA perp quote failed"
    exit 1
fi

PRICE=$(echo "$QUOTE" | jq -r '.markPx // empty')

if [[ -z "$PRICE" || "$PRICE" == "null" ]]; then
    fail "TSLA perp quote returned but markPx is missing"
    exit 1
fi

# ── Step 7: Place TSLA perp buy ──────────────────────────────────────
BUY_LIMIT=$(echo "$PRICE" | awk '{printf "%.2f", $1 * 1.005}')
BUY_SIZE=$(echo "$PRICE" | awk '{printf "%.6f", 12.0 / $1}')

info "Mark price:       \$$PRICE"
info "Limit buy price:  \$$BUY_LIMIT (+0.5% buffer)"
info "Buy size:         $BUY_SIZE TSLA (~\$12)"

RESULT=$(ft "{\"command\":\"perp_buy\",\"symbol\":\"TSLA\",\"amount\":$BUY_SIZE,\"price\":$BUY_LIMIT,\"close\":false}")

if [[ -z "$RESULT" ]]; then
    fail "TSLA perp buy failed"
    exit 1
fi

FILL=$(echo "$RESULT" | jq -r '.fillStatus // empty')

done_step
info "Size:        $(echo "$RESULT" | jq -r '.size // empty')"
info "Price:       \$$(echo "$RESULT" | jq -r '.price // empty')"
info "Fill status: $FILL"

if [[ "$FILL" == "filled" ]]; then
    ok "TSLA perp buy FILLED"
elif [[ "$FILL" == "resting" ]]; then
    warn "TSLA perp buy is RESTING (not yet filled)"
    ok "TSLA perp buy order placed (resting)"
elif [[ "$FILL" == error* ]]; then
    fail "TSLA perp buy ERROR: $FILL"
    exit 1
else
    ok "TSLA perp buy order placed (status: ${FILL:-unknown})"
fi

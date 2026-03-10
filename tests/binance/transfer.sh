#!/usr/bin/env bash
#
# Transfer USDT between spot and futures wallets on Binance
#
# Uses binance --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Transfer USDT from spot to futures (or vice versa)
#
# Usage: ./tests/binance/transfer.sh [AMOUNT] [FROM] [TO]
#        ./tests/binance/transfer.sh 10 spot futures
#        ./tests/binance/transfer.sh 10 futures spot
#        ./tests/binance/transfer.sh           # defaults: $10 USDT spot->futures
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $BINANCE --json "$1" 2>/dev/null; }

AMOUNT="${1:-10}"
FROM="${2:-spot}"
TO="${3:-futures}"

log "Transfer $AMOUNT USDT from $FROM to $TO on Binance (JSON API)"

# ── Check balance before ─────────────────────────────────────────────
info "Checking balance before transfer..."
BALANCE=$(ft '{"command":"balance"}')

if [[ -n "$BALANCE" ]]; then
    if [[ "$FROM" == "spot" ]]; then
        USDT_FREE=$(echo "$BALANCE" | jq -r '.spot[]? | select(.asset == "USDT") | .free // "0"' 2>/dev/null || echo "0")
        info "Spot USDT free: $USDT_FREE"
    else
        USDT_AVAIL=$(echo "$BALANCE" | jq -r '.futures[]? | select(.asset == "USDT") | .availableBalance // "0"' 2>/dev/null || echo "0")
        info "Futures USDT available: $USDT_AVAIL"
    fi
fi

# ── Transfer ─────────────────────────────────────────────────────────
info "Transferring $AMOUNT USDT from $FROM to $TO..."
RESULT=$(ft "{\"command\":\"transfer\",\"asset\":\"USDT\",\"amount\":$AMOUNT,\"from\":\"$FROM\",\"to\":\"$TO\"}")

if [[ -z "$RESULT" ]]; then
    fail "Transfer failed"
    exit 1
fi

# Check for error
ERROR=$(echo "$RESULT" | jq -r '.error // empty' 2>/dev/null)
if [[ -n "$ERROR" ]]; then
    fail "Transfer error: $ERROR"
    exit 1
fi

TXN_ID=$(echo "$RESULT" | jq -r '.txn_id // empty')
STATUS=$(echo "$RESULT" | jq -r '.status // empty')

done_step
info "Transaction ID: ${TXN_ID:-unknown}"
info "Status:         ${STATUS:-unknown}"
ok "Transferred $AMOUNT USDT from $FROM to $TO"

# ── Check balance after ──────────────────────────────────────────────
sleep 1
info "Checking balance after transfer..."
BALANCE=$(ft '{"command":"balance"}')

if [[ -n "$BALANCE" ]]; then
    SPOT_USDT=$(echo "$BALANCE" | jq -r '.spot[]? | select(.asset == "USDT") | .free // "0"' 2>/dev/null || echo "0")
    FUTURES_USDT=$(echo "$BALANCE" | jq -r '.futures[]? | select(.asset == "USDT") | .availableBalance // "0"' 2>/dev/null || echo "0")
    info "Spot USDT free:         $SPOT_USDT"
    info "Futures USDT available: $FUTURES_USDT"
fi

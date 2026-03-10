#!/usr/bin/env bash
#
# Transfer USDT between funding and trading accounts on OKX
#
# Uses okx --json API for all commands. Output is always JSON.
#
# Usage: ./tests/okx/transfer.sh [AMOUNT] [FROM] [TO]
#        ./tests/okx/transfer.sh 10 funding trading
#        ./tests/okx/transfer.sh 10 trading funding
#        ./tests/okx/transfer.sh           # defaults: $10 USDT funding->trading
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $OKX --json "$1" 2>/dev/null; }

AMOUNT="${1:-10}"
FROM="${2:-funding}"
TO="${3:-trading}"

log "Transfer $AMOUNT USDT from $FROM to $TO on OKX (JSON API)"

# ── Check balance before ─────────────────────────────────────────
info "Checking balance before transfer..."
BALANCE=$(ft '{"command":"balance"}')

if [[ -n "$BALANCE" ]]; then
    info "Trading account balances:"
    echo "$BALANCE" | jq -r '.trading // [] | .[]? | .details // [] | .[]? | select(.ccy == "USDT") | "    USDT eq: \(.eq) avail: \(.availBal)"' 2>/dev/null || true
    info "Funding account balances:"
    echo "$BALANCE" | jq -r '.funding // [] | .[]? | select(.ccy == "USDT") | "    USDT bal: \(.bal) avail: \(.availBal)"' 2>/dev/null || true
fi

# ── Transfer ─────────────────────────────────────────────────────
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

TXN_ID=$(echo "$RESULT" | jq -r '.transactionId // empty')

done_step
info "Transaction ID: ${TXN_ID:-unknown}"
ok "Transferred $AMOUNT USDT from $FROM to $TO"

# ── Check balance after ──────────────────────────────────────────
sleep 1
info "Checking balance after transfer..."
BALANCE=$(ft '{"command":"balance"}')

if [[ -n "$BALANCE" ]]; then
    info "Trading account:"
    echo "$BALANCE" | jq -r '.trading // [] | .[]? | .details // [] | .[]? | select(.ccy == "USDT") | "    USDT eq: \(.eq) avail: \(.availBal)"' 2>/dev/null || true
    info "Funding account:"
    echo "$BALANCE" | jq -r '.funding // [] | .[]? | select(.ccy == "USDT") | "    USDT bal: \(.bal) avail: \(.availBal)"' 2>/dev/null || true
fi

#!/usr/bin/env bash
#
# Withdraw USDC from OKX to Base or Ethereum
#
# Uses okx --json API for all commands. Output is always JSON.
#
# Usage: ./tests/okx/withdraw.sh [AMOUNT] [NETWORK]
#        ./tests/okx/withdraw.sh 10 base
#        ./tests/okx/withdraw.sh 10 ethereum
#        ./tests/okx/withdraw.sh           # defaults: $10 USDC to base
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $OKX --json "$1"; }

USDC_AMOUNT="${1:-10}"
NETWORK="${2:-base}"

log "Withdraw $USDC_AMOUNT USDC from OKX to $NETWORK (JSON API)"
info "Network: $NETWORK"
info "Amount:  $USDC_AMOUNT USDC"

# ── Withdraw USDC ────────────────────────────────────────────────
info "Submitting withdrawal..."
RESULT=$(ft "{\"command\":\"withdraw\",\"asset\":\"USDC\",\"amount\":$USDC_AMOUNT,\"network\":\"$NETWORK\"}")

if [[ -z "$RESULT" ]]; then
    fail "USDC withdrawal to $NETWORK failed"
    warn "Funds remain on OKX."
    exit 1
fi

# Check for error
ERROR=$(echo "$RESULT" | jq -r '.error // empty' 2>/dev/null)
if [[ -n "$ERROR" ]]; then
    fail "Withdrawal error: $ERROR"
    exit 1
fi

WITHDRAW_ID=$(echo "$RESULT" | jq -r '.withdrawalId // empty')

done_step
info "Withdrawal ID: ${WITHDRAW_ID:-unknown}"
ok "USDC withdrawal submitted -- $USDC_AMOUNT USDC to $NETWORK"

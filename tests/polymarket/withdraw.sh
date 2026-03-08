#!/usr/bin/env bash
#
# Withdraw USDC from Polymarket to Base chain
#
# Uses fintool --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Call Polymarket bridge API to get withdrawal address
#   2. Display withdrawal instructions (send USDC.e on Polygon to the address)
#
# Usage: ./tests/polymarket/withdraw.sh [amount] [chain]
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $FINTOOL --json "$1" 2>/dev/null; }

WITHDRAW_AMOUNT=${1:-10}
DEST_CHAIN=${2:-base}

log "Withdraw \$${WITHDRAW_AMOUNT} USDC from Polymarket to ${DEST_CHAIN} (JSON API)"

info "Requesting Polymarket withdrawal address..."

RESULT=$(ft "{\"command\":\"withdraw\",\"asset\":\"USDC\",\"amount\":$WITHDRAW_AMOUNT,\"to\":\"$DEST_CHAIN\",\"exchange\":\"polymarket\"}")

if [[ -z "$RESULT" ]]; then
    fail "Withdraw command returned empty"
    exit 1
fi

ERROR=$(echo "$RESULT" | jq -r '.error // empty')
if [[ -n "$ERROR" ]]; then
    fail "Withdraw failed: $ERROR"
    exit 1
fi

WITHDRAW_ADDR=$(echo "$RESULT" | jq -r '.withdrawal_address_evm // empty')
DEST=$(echo "$RESULT" | jq -r '.destination_chain // empty')

ok "Withdrawal address retrieved"
info "Chain:   $DEST"
info "Address: $WITHDRAW_ADDR"
info "Send $WITHDRAW_AMOUNT USDC.e on Polygon to the address above"
done_step
echo "$RESULT" | jq '.'

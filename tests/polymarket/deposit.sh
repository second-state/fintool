#!/usr/bin/env bash
#
# Deposit $10 USDC from Base to Polymarket
#
# Uses polymarket --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Get Polymarket deposit address
#   2. Display deposit info
#
# Usage: ./tests/polymarket/deposit.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $POLYMARKET --json "$1" 2>/dev/null; }

DEPOSIT_AMOUNT=${1:-10}

log "Deposit \$${DEPOSIT_AMOUNT} USDC from Base to Polymarket (JSON API)"

info "Requesting Polymarket deposit address..."

RESULT=$(ft "{\"command\":\"deposit\",\"asset\":\"USDC\",\"amount\":$DEPOSIT_AMOUNT,\"from\":\"base\"}")

if [[ -z "$RESULT" ]]; then
    fail "Deposit command returned empty"
    exit 1
fi

ERROR=$(echo "$RESULT" | jq -r '.error // empty')
if [[ -n "$ERROR" ]]; then
    fail "Deposit failed: $ERROR"
    exit 1
fi

ok "Deposit info retrieved"
done_step
echo "$RESULT" | jq '.'

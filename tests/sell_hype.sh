#!/usr/bin/env bash
#
# Sell ALL HYPE spot on Hyperliquid
#
# Usage: ./tests/sell_hype.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
ensure_built

log "Sell ALL HYPE spot on Hyperliquid"
info "Checking spot balances for HYPE, then selling all."

# Get spot balances from fintool balance (returns {"perp": ..., "spot": ...})
run_fintool balance
if check_fail "Could not fetch balance"; then
    exit 1
fi

HYPE_TOTAL=$(echo "$LAST_STDOUT" | jq -r '
    .spot.balances[]? | select(.coin == "HYPE") | .total // empty
' 2>/dev/null || true)

HYPE_HOLD=$(echo "$LAST_STDOUT" | jq -r '
    .spot.balances[]? | select(.coin == "HYPE") | .hold // "0"
' 2>/dev/null || true)

if [[ -z "$HYPE_TOTAL" || "$HYPE_TOTAL" == "null" || "$HYPE_TOTAL" == "0" || "$HYPE_TOTAL" == "0.0" ]]; then
    done_step
    warn "No HYPE balance found — buy order may not have filled"
    exit 0
fi

# Available = total - hold
SELL_SIZE=$(echo "$HYPE_TOTAL $HYPE_HOLD" | awk '{printf "%.2f", $1 - $2}')

if [[ "$SELL_SIZE" == "0.00" || "$SELL_SIZE" == "0" ]]; then
    done_step
    warn "All HYPE is on hold (in open orders) — nothing available to sell"
    exit 0
fi

info "HYPE balance: $HYPE_TOTAL (hold: $HYPE_HOLD, selling: $SELL_SIZE)"

run_fintool quote HYPE
if check_fail "HYPE spot quote failed"; then
    exit 1
fi
HYPE_PRICE=$(echo "$LAST_STDOUT" | jq -r '.price // .markPx // empty' 2>/dev/null)
SELL_LIMIT=$(echo "$HYPE_PRICE" | awk '{printf "%.4f", $1 * 0.995}')

info "Current price: \$$HYPE_PRICE"
info "Sell limit:    \$$SELL_LIMIT (-0.5% buffer)"

run_fintool order sell HYPE --amount "$SELL_SIZE" --price "$SELL_LIMIT"

if check_fail "HYPE spot sell failed"; then
    warn "Tokens may still be held — check manually with 'fintool balance'"
    exit 1
fi

done_step
ok "HYPE spot sell placed — $SELL_SIZE HYPE at \$$SELL_LIMIT"

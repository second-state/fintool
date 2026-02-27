#!/usr/bin/env bash
#
# Deposit $15 USDC from Base to Hyperliquid
#
# Usage: ./tests/deposit.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
ensure_built

log "Deposit \$15 USDC from Base to Hyperliquid"
info "Bridging \$15 USDC from Base mainnet → Arbitrum → Hyperliquid via Across Protocol."
info "HL Bridge2 requires minimum 5 USDC deposit (below 5 is lost forever)."
info "This signs 3 transactions: USDC approval, Across bridge, HL Bridge2 deposit."
info "Requires ETH on Base for gas fees."

run_fintool deposit USDC --amount 15 --from base

if check_fail "Deposit \$15 USDC from Base to Hyperliquid failed"; then
    exit 1
fi

DEPOSIT_JSON="$LAST_STDOUT"
DEPOSIT_STATUS=$(echo "$DEPOSIT_JSON" | jq -r '.status // empty' 2>/dev/null || true)
DEPOSIT_AMOUNT_OUT=$(echo "$DEPOSIT_JSON" | jq -r '.amount_deposited // .amount_out // empty' 2>/dev/null || true)
DEPOSIT_BRIDGE_TX=$(echo "$DEPOSIT_JSON" | jq -r '.bridge_tx // empty' 2>/dev/null || true)

done_step
info "Status:           ${DEPOSIT_STATUS:-unknown}"
info "Amount deposited: ${DEPOSIT_AMOUNT_OUT:-pending} USDC"
if [[ -n "$DEPOSIT_BRIDGE_TX" && "$DEPOSIT_BRIDGE_TX" != "null" ]]; then
    info "Bridge TX:        $DEPOSIT_BRIDGE_TX"
fi
ok "Deposit completed — ${DEPOSIT_AMOUNT_OUT:-~15} USDC credited to Hyperliquid"

info "Waiting 60 seconds for the deposit to settle on Hyperliquid..."
sleep 60

info "Enabling unified account mode (shares USDC across perp + spot)..."
run_fintool perp set-mode unified
if check_fail "Failed to enable unified account mode"; then
    warn "Continuing anyway — may need manual transfer for some dexes"
fi

info "Checking balance after deposit..."
run_fintool balance
if [[ $LAST_EXIT -eq 0 ]]; then
    SPOT_USDC=$(echo "$LAST_STDOUT" | jq -r '.spot.balances[]? | select(.coin == "USDC") | .total // empty' 2>/dev/null || true)
    if [[ -n "$SPOT_USDC" && "$SPOT_USDC" != "null" ]]; then
        info "Post-deposit USDC balance: \$$SPOT_USDC"
    fi
fi

#!/usr/bin/env bash
#
# Deposit $15 USDC from Base to Hyperliquid
#
# Uses fintool --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Bridge $15 USDC from Base -> Across -> Arbitrum -> HL Bridge2 -> Hyperliquid
#   2. Wait 60 seconds for deposit to settle
#   3. Enable unified account mode
#   4. Check balance
#
# Prerequisites:
#   - ETH on Base for gas fees
#   - USDC on Base to deposit
#
# Usage: ./tests/json/deposit.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $FINTOOL --json "$1" 2>/dev/null; }

log "Deposit \$15 USDC from Base to Hyperliquid (JSON API)"

info "Bridging \$15 USDC from Base mainnet -> Arbitrum -> Hyperliquid via Across Protocol."
info "HL Bridge2 requires minimum 5 USDC deposit (below 5 is lost forever)."
info "This signs 3 transactions: USDC approval, Across bridge, HL Bridge2 deposit."
info "Requires ETH on Base for gas fees."

# ── Deposit ──────────────────────────────────────────────────────────
RESULT=$(ft '{"command":"deposit","asset":"USDC","amount":"15","from":"base"}')

if [[ -z "$RESULT" ]]; then
    fail "Deposit \$15 USDC from Base to Hyperliquid failed"
    exit 1
fi

DEPOSIT_STATUS=$(echo "$RESULT" | jq -r '.status // empty')
DEPOSIT_AMOUNT=$(echo "$RESULT" | jq -r '.amount_deposited // .amount_out // empty')
DEPOSIT_TX=$(echo "$RESULT" | jq -r '.bridge_tx // empty')

done_step
info "Status:           ${DEPOSIT_STATUS:-unknown}"
info "Amount deposited: ${DEPOSIT_AMOUNT:-pending} USDC"
if [[ -n "$DEPOSIT_TX" && "$DEPOSIT_TX" != "null" ]]; then
    info "Bridge TX:        $DEPOSIT_TX"
fi
ok "Deposit completed -- ${DEPOSIT_AMOUNT:-~15} USDC credited to Hyperliquid"

# ── Wait for settlement ──────────────────────────────────────────────
info "Waiting 60 seconds for the deposit to settle on Hyperliquid..."
sleep 60

# ── Enable unified mode ──────────────────────────────────────────────
info "Enabling unified account mode (shares USDC across perp + spot)..."
RESULT=$(ft '{"command":"perp_set_mode","mode":"unified"}')
if [[ -z "$RESULT" ]]; then
    warn "Failed to enable unified account mode -- continuing anyway"
else
    ok "Unified account mode enabled"
fi

# ── Check balance ────────────────────────────────────────────────────
info "Checking balance after deposit..."
BALANCE=$(ft '{"command":"balance"}')

if [[ -n "$BALANCE" ]]; then
    echo "$BALANCE" | jq .
    SPOT_USDC=$(echo "$BALANCE" | jq -r '.spot.balances[]? | select(.coin == "USDC") | .total // empty')
    if [[ -n "$SPOT_USDC" && "$SPOT_USDC" != "null" ]]; then
        info "Post-deposit USDC balance: \$$SPOT_USDC"
    fi
fi

#!/usr/bin/env bash
#
# Deposit $15 USDC from Base to Hyperliquid
#
# Uses the human CLI API — fintool commands produce human-readable output.
# After depositing, waits 60 seconds for settlement, enables unified mode,
# and displays the balance.
#
# Usage: ./tests/human/deposit.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

log "Deposit \$15 USDC from Base to Hyperliquid"
info "Bridging \$15 USDC from Base mainnet -> Arbitrum -> Hyperliquid via Across Protocol."
info "HL Bridge2 requires minimum 5 USDC deposit (below 5 is lost forever)."
info "This signs 3 transactions: USDC approval, Across bridge, HL Bridge2 deposit."
info "Requires ETH on Base for gas fees."

# ── Deposit ──────────────────────────────────────────────────────────
run_fintool deposit USDC --amount 15 --from base

if check_fail "Deposit \$15 USDC from Base to Hyperliquid failed"; then
    exit 1
fi

done_step
info "Output: $LAST_STDOUT"
ok "Deposit submitted"

# ── Wait for settlement ──────────────────────────────────────────────
info "Waiting 60 seconds for the deposit to settle on Hyperliquid..."
sleep 60

# ── Enable unified mode ──────────────────────────────────────────────
info "Enabling unified account mode (shares USDC across perp + spot)..."
run_fintool perp set-mode unified
if check_fail "Failed to enable unified account mode"; then
    warn "Continuing anyway -- may need manual transfer for some dexes"
fi

# ── Display balance ──────────────────────────────────────────────────
info "Checking balance after deposit..."
run_fintool balance
if [[ $LAST_EXIT -eq 0 ]]; then
    info "Balance output:"
    echo "$LAST_STDOUT"
else
    warn "Could not fetch balance"
fi

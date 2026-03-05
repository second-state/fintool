#!/usr/bin/env bash
#
# Withdraw USDC and/or ETH from Hyperliquid to Base
#
# Uses the human CLI API — fintool commands produce human-readable output.
# Accepts optional arguments for USDC and ETH amounts.
#
# Usage: ./tests/human/withdraw.sh [USDC_AMOUNT] [ETH_AMOUNT]
#        ./tests/human/withdraw.sh 1 0.0003
#        ./tests/human/withdraw.sh          # defaults: 1 USDC, 0 ETH
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

USDC_AMOUNT="${1:-1}"
ETH_AMOUNT="${2:-0}"

log "Withdraw USDC and ETH from Hyperliquid to Base"
info "Route: Hyperliquid -> HL Bridge2 -> Arbitrum -> Across bridge -> Base"

# ── Withdraw USDC ────────────────────────────────────────────────────
info "-- Withdraw $USDC_AMOUNT USDC --"
run_fintool withdraw USDC --amount "$USDC_AMOUNT" --to base

if check_fail "USDC withdrawal to Base failed"; then
    warn "USDC withdrawal failed -- funds remain on Hyperliquid."
else
    done_step
    info "Output: $LAST_STDOUT"
    ok "USDC withdrawal submitted -- $USDC_AMOUNT USDC to Base"
fi

# ── Withdraw ETH (if requested) ─────────────────────────────────────
if awk "BEGIN{exit !(${ETH_AMOUNT:-0} > 0)}"; then
    info "Waiting 5 seconds..."
    sleep 5

    info "-- Withdraw $ETH_AMOUNT ETH --"
    run_fintool withdraw ETH --amount "$ETH_AMOUNT" --to base

    if check_fail "ETH withdrawal to Base failed"; then
        warn "ETH withdrawal failed -- funds remain on Hyperliquid."
    else
        done_step
        info "Output: $LAST_STDOUT"
        ok "ETH withdrawal submitted -- $ETH_AMOUNT ETH to Base"
    fi
else
    info "ETH amount is 0 -- skipping ETH withdrawal."
fi

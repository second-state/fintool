#!/usr/bin/env bash
#
# Withdraw USDC and ETH from Hyperliquid to Base
#
# Usage: ./tests/withdraw.sh [USDC_AMOUNT] [ETH_AMOUNT]
#        ./tests/withdraw.sh 1 0.0003
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/helpers.sh"
ensure_built

USDC_AMOUNT="${1:-1}"
ETH_AMOUNT="${2:-0}"

log "Withdraw USDC and ETH from Hyperliquid to Base"
info "Route: Hyperliquid → HL Bridge2 → Arbitrum → Across bridge → Base"

info "── Withdraw $USDC_AMOUNT USDC ──"
run_fintool withdraw USDC --amount "$USDC_AMOUNT" --to base

if check_fail "USDC withdrawal to Base failed"; then
    warn "USDC withdrawal failed — funds remain on Hyperliquid."
else
    USDC_WD_STATUS=$(echo "$LAST_STDOUT" | jq -r '.status // empty' 2>/dev/null || true)
    done_step
    info "Status: ${USDC_WD_STATUS:-unknown}"
    ok "USDC withdrawal submitted — $USDC_AMOUNT USDC to Base"
fi

if awk "BEGIN{exit !(${ETH_AMOUNT:-0} > 0)}"; then
    info "Waiting 5 seconds..."
    sleep 5

    info "── Withdraw $ETH_AMOUNT ETH ──"
    run_fintool withdraw ETH --amount "$ETH_AMOUNT" --to base

    if check_fail "ETH withdrawal to Base failed"; then
        warn "ETH withdrawal failed — funds remain on Hyperliquid."
    else
        ETH_WD_STATUS=$(echo "$LAST_STDOUT" | jq -r '.status // empty' 2>/dev/null || true)
        done_step
        info "Status: ${ETH_WD_STATUS:-unknown}"
        ok "ETH withdrawal submitted — $ETH_AMOUNT ETH to Base"
    fi
else
    info "ETH amount is 0 — skipping ETH withdrawal."
fi

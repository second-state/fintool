#!/usr/bin/env bash
#
# Withdraw USDC and ETH from Hyperliquid to Base
#
# Uses hyperliquid --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Withdraw USDC to Base (default $1)
#   2. If ETH amount > 0, also withdraw ETH to Base
#
# Route: Hyperliquid -> HL Bridge2 -> Arbitrum -> Across bridge -> Base
#
# Usage: ./tests/json/withdraw.sh [USDC_AMOUNT] [ETH_AMOUNT]
#        ./tests/json/withdraw.sh 1 0.0003
#        ./tests/json/withdraw.sh           # defaults: $1 USDC, 0 ETH
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $HYPERLIQUID --json "$1"; }

USDC_AMOUNT="${1:-1}"
ETH_AMOUNT="${2:-0}"

log "Withdraw USDC and ETH from Hyperliquid to Base (JSON API)"
info "Route: Hyperliquid -> HL Bridge2 -> Arbitrum -> Across bridge -> Base"

# ── Withdraw USDC ────────────────────────────────────────────────────
info "-- Withdraw $USDC_AMOUNT USDC --"
RESULT=$(ft "{\"command\":\"withdraw\",\"asset\":\"USDC\",\"amount\":$USDC_AMOUNT,\"to\":\"base\"}")

if [[ -z "$RESULT" ]]; then
    fail "USDC withdrawal to Base failed"
    warn "USDC withdrawal failed -- funds remain on Hyperliquid."
else
    STATUS=$(echo "$RESULT" | jq -r '.status // empty')
    done_step
    info "Status: ${STATUS:-unknown}"
    ok "USDC withdrawal submitted -- $USDC_AMOUNT USDC to Base"
fi

# ── Withdraw ETH (if amount > 0) ─────────────────────────────────────
if awk "BEGIN{exit !(${ETH_AMOUNT:-0} > 0)}"; then
    info "Waiting 5 seconds..."
    sleep 5

    info "-- Withdraw $ETH_AMOUNT ETH --"
    RESULT=$(ft "{\"command\":\"withdraw\",\"asset\":\"ETH\",\"amount\":$ETH_AMOUNT,\"to\":\"base\"}")

    if [[ -z "$RESULT" ]]; then
        fail "ETH withdrawal to Base failed"
        warn "ETH withdrawal failed -- funds remain on Hyperliquid."
    else
        STATUS=$(echo "$RESULT" | jq -r '.status // empty')
        done_step
        info "Status: ${STATUS:-unknown}"
        ok "ETH withdrawal submitted -- $ETH_AMOUNT ETH to Base"
    fi
else
    info "ETH amount is 0 -- skipping ETH withdrawal."
fi

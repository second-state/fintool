#!/usr/bin/env bash
#
# Get deposit address for USDC on Binance (Base and Ethereum networks)
#
# Uses binance --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Get USDC deposit address for Base network
#   2. Get USDC deposit address for Ethereum network
#   3. Get ETH deposit address for Ethereum network
#
# Usage: ./tests/binance/deposit.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $BINANCE --json "$1" 2>/dev/null; }

log "Get deposit addresses on Binance (JSON API)"

# ── USDC on Base ─────────────────────────────────────────────────────
info "-- USDC deposit address (Base network) --"
RESULT=$(ft '{"command":"deposit","asset":"USDC","from":"base"}')

if [[ -z "$RESULT" ]]; then
    warn "Failed to get USDC deposit address for Base"
else
    ADDRESS=$(echo "$RESULT" | jq -r '.address // empty')
    NETWORK=$(echo "$RESULT" | jq -r '.network // empty')
    info "Address: ${ADDRESS:-unknown}"
    info "Network: ${NETWORK:-unknown}"
    ok "USDC deposit address (Base): $ADDRESS"
fi

# ── USDC on Ethereum ─────────────────────────────────────────────────
echo ""
info "-- USDC deposit address (Ethereum network) --"
RESULT=$(ft '{"command":"deposit","asset":"USDC","from":"ethereum"}')

if [[ -z "$RESULT" ]]; then
    warn "Failed to get USDC deposit address for Ethereum"
else
    ADDRESS=$(echo "$RESULT" | jq -r '.address // empty')
    NETWORK=$(echo "$RESULT" | jq -r '.network // empty')
    info "Address: ${ADDRESS:-unknown}"
    info "Network: ${NETWORK:-unknown}"
    ok "USDC deposit address (Ethereum): $ADDRESS"
fi

# ── ETH on Ethereum ──────────────────────────────────────────────────
echo ""
info "-- ETH deposit address (Ethereum network) --"
RESULT=$(ft '{"command":"deposit","asset":"ETH","from":"ethereum"}')

if [[ -z "$RESULT" ]]; then
    warn "Failed to get ETH deposit address for Ethereum"
else
    ADDRESS=$(echo "$RESULT" | jq -r '.address // empty')
    NETWORK=$(echo "$RESULT" | jq -r '.network // empty')
    info "Address: ${ADDRESS:-unknown}"
    info "Network: ${NETWORK:-unknown}"
    ok "ETH deposit address (Ethereum): $ADDRESS"
fi

done_step
ok "Deposit addresses displayed"

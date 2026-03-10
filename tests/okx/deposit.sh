#!/usr/bin/env bash
#
# Get deposit addresses on OKX
#
# Uses okx --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Get USDC deposit address for Base network
#   2. Get USDC deposit address for Ethereum network
#   3. Get ETH deposit address for Ethereum network
#
# Usage: ./tests/okx/deposit.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $OKX --json "$1" 2>/dev/null; }

log "Get deposit addresses on OKX (JSON API)"

# ── USDC on Base ─────────────────────────────────────────────────
info "-- USDC deposit address (Base network) --"
RESULT=$(ft '{"command":"deposit","asset":"USDC","network":"base"}')

if [[ -z "$RESULT" ]]; then
    warn "Failed to get USDC deposit address for Base"
else
    echo "$RESULT" | jq -r '.addresses[]? | "    Chain: \(.chain)\n    Address: \(.address)\n    Min deposit: \(.minDeposit)"' 2>/dev/null || true
    ok "USDC deposit address (Base) displayed"
fi

# ── USDC on Ethereum ─────────────────────────────────────────────
echo ""
info "-- USDC deposit address (Ethereum network) --"
RESULT=$(ft '{"command":"deposit","asset":"USDC","network":"ethereum"}')

if [[ -z "$RESULT" ]]; then
    warn "Failed to get USDC deposit address for Ethereum"
else
    echo "$RESULT" | jq -r '.addresses[]? | "    Chain: \(.chain)\n    Address: \(.address)\n    Min deposit: \(.minDeposit)"' 2>/dev/null || true
    ok "USDC deposit address (Ethereum) displayed"
fi

# ── ETH on Ethereum ──────────────────────────────────────────────
echo ""
info "-- ETH deposit address (Ethereum network) --"
RESULT=$(ft '{"command":"deposit","asset":"ETH","network":"ethereum"}')

if [[ -z "$RESULT" ]]; then
    warn "Failed to get ETH deposit address for Ethereum"
else
    echo "$RESULT" | jq -r '.addresses[]? | "    Chain: \(.chain)\n    Address: \(.address)\n    Min deposit: \(.minDeposit)"' 2>/dev/null || true
    ok "ETH deposit address (Ethereum) displayed"
fi

done_step
ok "Deposit addresses displayed"

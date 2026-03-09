#!/usr/bin/env bash
#
# Sell ALL HYPE spot on Hyperliquid
#
# Uses hyperliquid --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Get balance via balance command
#   2. Extract HYPE total and hold from .spot.balances[]
#   3. If no HYPE balance, warn and exit
#   4. Get HYPE price via quote
#   5. Sell at -0.5% below market
#
# Usage: ./tests/json/sell_hype.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $HYPERLIQUID --json "$1" 2>/dev/null; }

log "Sell ALL HYPE spot on Hyperliquid (JSON API)"

# ── Get balance ──────────────────────────────────────────────────────
info "Checking spot balances for HYPE..."
BALANCE=$(ft '{"command":"balance"}')

if [[ -z "$BALANCE" ]]; then
    fail "Could not fetch balance"
    exit 1
fi

HYPE_TOTAL=$(echo "$BALANCE" | jq -r '
    .spot.balances[]? | select(.coin == "HYPE") | .total // empty
' 2>/dev/null || true)

HYPE_HOLD=$(echo "$BALANCE" | jq -r '
    .spot.balances[]? | select(.coin == "HYPE") | .hold // "0"
' 2>/dev/null || true)

if [[ -z "$HYPE_TOTAL" || "$HYPE_TOTAL" == "null" || "$HYPE_TOTAL" == "0" || "$HYPE_TOTAL" == "0.0" ]]; then
    done_step
    warn "No HYPE balance found -- buy order may not have filled"
    exit 0
fi

# ── Compute sell size (total - hold) ─────────────────────────────────
SELL_SIZE=$(echo "$HYPE_TOTAL $HYPE_HOLD" | awk '{printf "%.2f", $1 - $2}')

if [[ "$SELL_SIZE" == "0.00" || "$SELL_SIZE" == "0" ]]; then
    done_step
    warn "All HYPE is on hold (in open orders) -- nothing available to sell"
    exit 0
fi

info "HYPE balance: $HYPE_TOTAL (hold: $HYPE_HOLD, selling: $SELL_SIZE)"

# ── Get HYPE price ───────────────────────────────────────────────────
info "Fetching HYPE price..."
QUOTE=$(ft '{"command":"perp_quote","symbol":"HYPE"}')

if [[ -z "$QUOTE" ]]; then
    fail "HYPE quote failed"
    exit 1
fi

HYPE_PRICE=$(echo "$QUOTE" | jq -r '.markPx // empty')
SELL_LIMIT=$(echo "$HYPE_PRICE" | awk '{printf "%.4f", $1 * 0.995}')

info "Current price: \$$HYPE_PRICE"
info "Sell limit:    \$$SELL_LIMIT (-0.5% buffer)"

# ── Place sell order ─────────────────────────────────────────────────
RESULT=$(ft "{\"command\":\"sell\",\"symbol\":\"HYPE\",\"amount\":$SELL_SIZE,\"price\":$SELL_LIMIT}")

if [[ -z "$RESULT" ]]; then
    fail "HYPE spot sell failed"
    warn "Tokens may still be held -- check manually with 'hyperliquid balance'"
    exit 1
fi

FILL=$(echo "$RESULT" | jq -r '.fillStatus // empty')

done_step
info "Fill status: $FILL"
ok "HYPE spot sell placed -- $SELL_SIZE HYPE at \$$SELL_LIMIT"

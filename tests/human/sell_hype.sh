#!/usr/bin/env bash
#
# Sell ALL HYPE spot on Hyperliquid
#
# Uses the human CLI API — fintool commands produce human-readable output.
# Data extraction (balances, prices) is done via the Hyperliquid API directly.
#
# Usage: ./tests/human/sell_hype.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

USER_ADDR=$($FINTOOL address 2>/dev/null)
hl_api() {
  curl -s https://api.hyperliquid.xyz/info -H 'Content-Type: application/json' -d "$1"
}

log "Sell ALL HYPE spot on Hyperliquid"

# ── Get HYPE spot balance from HL API ────────────────────────────────
info "Fetching spot balances from Hyperliquid API..."
SPOT_STATE=$(hl_api "{\"type\":\"spotClearinghouseState\",\"user\":\"$USER_ADDR\"}")

HYPE_TOTAL=$(echo "$SPOT_STATE" | jq -r '.balances[]? | select(.coin == "HYPE") | .total // empty' 2>/dev/null || true)
HYPE_HOLD=$(echo "$SPOT_STATE" | jq -r '.balances[]? | select(.coin == "HYPE") | .hold // "0"' 2>/dev/null || true)

if [[ -z "$HYPE_TOTAL" || "$HYPE_TOTAL" == "null" || "$HYPE_TOTAL" == "0" || "$HYPE_TOTAL" == "0.0" ]]; then
    done_step
    warn "No HYPE balance found -- buy order may not have filled"
    exit 0
fi

# Available = total - hold
SELL_SIZE=$(echo "$HYPE_TOTAL $HYPE_HOLD" | awk '{printf "%.2f", $1 - $2}')

if [[ "$SELL_SIZE" == "0.00" || "$SELL_SIZE" == "0" ]]; then
    done_step
    warn "All HYPE is on hold (in open orders) -- nothing available to sell"
    exit 0
fi

info "HYPE balance: $HYPE_TOTAL (hold: $HYPE_HOLD, selling: $SELL_SIZE)"

# ── Get HYPE price from HL API ───────────────────────────────────────
info "Fetching HYPE spot price from Hyperliquid API..."
HYPE_PRICE=$(hl_api '{"type":"allMids"}' | jq -r '.["@HYPE"]' 2>/dev/null)

if [[ -z "$HYPE_PRICE" || "$HYPE_PRICE" == "null" ]]; then
    fail "Could not fetch HYPE spot price from HL API"
    exit 1
fi

SELL_LIMIT=$(echo "$HYPE_PRICE" | awk '{printf "%.4f", $1 * 0.995}')

info "Current price: \$$HYPE_PRICE"
info "Sell limit:    \$$SELL_LIMIT (-0.5% buffer)"

# ── Place sell order ─────────────────────────────────────────────────
run_fintool order sell HYPE --amount "$SELL_SIZE" --price "$SELL_LIMIT"

if check_fail "HYPE spot sell failed"; then
    warn "Tokens may still be held -- check manually with 'fintool balance'"
    exit 1
fi

done_step
info "Output: $LAST_STDOUT"
ok "HYPE spot sell placed -- $SELL_SIZE HYPE at \$$SELL_LIMIT"

#!/usr/bin/env bash
#
# Buy ~$12 worth of HYPE spot on Hyperliquid
#
# Uses the human CLI API — fintool commands produce human-readable output.
# Data extraction (prices) is done via the Hyperliquid API directly.
#
# Usage: ./tests/human/buy_hype.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

USER_ADDR=$($FINTOOL address 2>/dev/null)
hl_api() {
  curl -s https://api.hyperliquid.xyz/info -H 'Content-Type: application/json' -d "$1"
}

log "Buy ~\$12 HYPE spot on Hyperliquid"

# ── Get HYPE price from HL API ───────────────────────────────────────
info "Fetching HYPE spot price from Hyperliquid API..."
HYPE_PRICE=$(hl_api '{"type":"allMids"}' | jq -r '.["@HYPE"]' 2>/dev/null)

if [[ -z "$HYPE_PRICE" || "$HYPE_PRICE" == "null" ]]; then
    fail "Could not fetch HYPE spot price from HL API"
    exit 1
fi

# ── Compute order size and limit ─────────────────────────────────────
BUY_LIMIT=$(echo "$HYPE_PRICE" | awk '{printf "%.4f", $1 * 1.005}')
BUY_SIZE=$(echo "$HYPE_PRICE" | awk '{printf "%.4f", 12.0 / $1}')

info "HYPE price:      \$$HYPE_PRICE"
info "Limit buy price: \$$BUY_LIMIT (+0.5% buffer)"
info "Buy size:        $BUY_SIZE HYPE (~\$12)"

# ── Place buy order ──────────────────────────────────────────────────
run_fintool order buy HYPE --amount "$BUY_SIZE" --price "$BUY_LIMIT"

if check_fail "HYPE spot buy failed"; then
    exit 1
fi

done_step
info "Output: $LAST_STDOUT"
ok "HYPE spot buy order placed"

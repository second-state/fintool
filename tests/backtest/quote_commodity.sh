#!/usr/bin/env bash
#
# Historical commodity (GOLD) quote via backtest
#
# Uses backtest --json API. Output is always JSON.
#
# Workflow:
#   1. Fetch GOLD historical price on 2025-01-15
#   2. Verify price is returned
#
# Usage: ./tests/backtest/quote_commodity.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

bt() { $BACKTEST --at "$1" --json "$2" 2>/dev/null; }

log "Historical GOLD quote on 2025-01-15 (JSON API)"

# ── Fetch GOLD price ──────────────────────────────────────────────────
info "Fetching GOLD price on 2025-01-15..."
RESULT=$(bt "2025-01-15" '{"command":"quote","symbol":"GOLD"}')

if [[ -z "$RESULT" ]]; then
    fail "GOLD historical quote returned empty"
    exit 1
fi

PRICE=$(echo "$RESULT" | jq -r '.price // empty')
SYMBOL=$(echo "$RESULT" | jq -r '.symbol // empty')

if [[ -z "$PRICE" ]]; then
    fail "GOLD quote returned but price is missing"
    echo "$RESULT" | jq .
    exit 1
fi

if [[ "$SYMBOL" != "GOLD" ]]; then
    fail "Expected symbol GOLD, got: $SYMBOL"
    exit 1
fi

done_step
ok "GOLD price on 2025-01-15: \$$PRICE"
echo "$RESULT" | jq .

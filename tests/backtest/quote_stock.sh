#!/usr/bin/env bash
#
# Historical stock (AAPL) quote via backtest
#
# Uses backtest --json API. Output is always JSON.
#
# Workflow:
#   1. Fetch AAPL historical price on 2025-01-15
#   2. Verify price is returned
#
# Usage: ./tests/backtest/quote_stock.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

bt() { $BACKTEST --at "$1" --json "$2" 2>/dev/null; }

log "Historical AAPL quote on 2025-01-15 (JSON API)"

# ── Fetch AAPL price ───────────────────────────────────────────────────
info "Fetching AAPL price on 2025-01-15..."
RESULT=$(bt "2025-01-15" '{"command":"quote","symbol":"AAPL"}')

if [[ -z "$RESULT" ]]; then
    fail "AAPL historical quote returned empty"
    exit 1
fi

PRICE=$(echo "$RESULT" | jq -r '.price // empty')
SYMBOL=$(echo "$RESULT" | jq -r '.symbol // empty')

if [[ -z "$PRICE" ]]; then
    fail "AAPL quote returned but price is missing"
    echo "$RESULT" | jq .
    exit 1
fi

if [[ "$SYMBOL" != "AAPL" ]]; then
    fail "Expected symbol AAPL, got: $SYMBOL"
    exit 1
fi

done_step
ok "AAPL price on 2025-01-15: \$$PRICE"
echo "$RESULT" | jq .

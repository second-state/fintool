#!/usr/bin/env bash
#
# Historical BTC quote via backtest
#
# Uses backtest --json API. Output is always JSON.
#
# Workflow:
#   1. Fetch BTC historical price on 2025-01-15
#   2. Verify price is returned and reasonable
#
# Usage: ./tests/backtest/quote_btc.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

bt() { $BACKTEST --at "$1" --json "$2" 2>/dev/null; }

log "Historical BTC quote on 2025-01-15 (JSON API)"

# ── Fetch BTC price ────────────────────────────────────────────────────
info "Fetching BTC price on 2025-01-15..."
RESULT=$(bt "2025-01-15" '{"command":"quote","symbol":"BTC"}')

if [[ -z "$RESULT" ]]; then
    fail "BTC historical quote returned empty"
    exit 1
fi

PRICE=$(echo "$RESULT" | jq -r '.price // empty')
SYMBOL=$(echo "$RESULT" | jq -r '.symbol // empty')
DATE=$(echo "$RESULT" | jq -r '.date // empty')

if [[ -z "$PRICE" ]]; then
    fail "BTC quote returned but price is missing"
    echo "$RESULT" | jq .
    exit 1
fi

if [[ "$SYMBOL" != "BTC" ]]; then
    fail "Expected symbol BTC, got: $SYMBOL"
    exit 1
fi

if [[ "$DATE" != "2025-01-15" ]]; then
    fail "Expected date 2025-01-15, got: $DATE"
    exit 1
fi

done_step
ok "BTC price on 2025-01-15: \$$PRICE"
echo "$RESULT" | jq .

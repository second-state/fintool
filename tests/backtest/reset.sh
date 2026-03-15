#!/usr/bin/env bash
#
# Test portfolio reset via backtest
#
# Uses backtest --json API. Output is always JSON.
#
# Workflow:
#   1. Add some trades to build up portfolio state
#   2. Verify state exists
#   3. Reset and verify clean state
#
# Usage: ./tests/backtest/reset.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

bt() { $BACKTEST --at "$1" --json "$2" 2>/dev/null; }

log "Portfolio reset test (JSON API)"

# ── Add some trades ────────────────────────────────────────────────────
info "Adding trades..."
bt "2025-01-15" '{"command":"reset"}' > /dev/null
bt "2025-01-15" '{"command":"buy","symbol":"BTC","amount":0.01}' > /dev/null
bt "2025-01-15" '{"command":"perp_buy","symbol":"ETH","amount":0.5,"price":3300}' > /dev/null

# Verify state exists
RESULT=$(bt "2025-01-15" '{"command":"balance"}')
TRADES=$(echo "$RESULT" | jq -r '.totalTrades // "0"')
if [[ "$TRADES" -ge 2 ]]; then
    ok "State has $TRADES trades"
else
    fail "Expected at least 2 trades, got $TRADES"
    exit 1
fi

# ── Reset ──────────────────────────────────────────────────────────────
info "Resetting portfolio..."
RESULT=$(bt "2025-01-15" '{"command":"reset"}')
STATUS=$(echo "$RESULT" | jq -r '.status // empty')

if [[ "$STATUS" != "ok" ]]; then
    fail "Reset did not return status:ok"
    echo "$RESULT" | jq .
    exit 1
fi
ok "Reset returned status: ok"

# ── Verify clean state ─────────────────────────────────────────────────
RESULT=$(bt "2025-01-15" '{"command":"balance"}')
TRADES=$(echo "$RESULT" | jq -r '.totalTrades // empty')
CASH=$(echo "$RESULT" | jq -r '.cashBalance // empty')
POS_COUNT=$(echo "$RESULT" | jq '.positions | length')

if [[ "$TRADES" != "0" ]]; then
    fail "Expected 0 trades after reset, got: $TRADES"
    exit 1
fi

if [[ "$POS_COUNT" != "0" ]]; then
    fail "Expected 0 positions after reset, got: $POS_COUNT"
    exit 1
fi

done_step
ok "Reset verified: $TRADES trades, $POS_COUNT positions, cash \$$CASH"
echo "$RESULT" | jq .

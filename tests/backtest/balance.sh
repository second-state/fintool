#!/usr/bin/env bash
#
# Test portfolio balance tracking via backtest
#
# Uses backtest --json API. Output is always JSON.
#
# Workflow:
#   1. Reset portfolio
#   2. Buy BTC — verify cash goes negative
#   3. Sell BTC at higher price — verify cash becomes positive
#
# Usage: ./tests/backtest/balance.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

bt() { $BACKTEST --at "$1" --json "$2" 2>/dev/null; }

log "Portfolio balance tracking (JSON API)"

# ── Reset ──────────────────────────────────────────────────────────────
info "Resetting portfolio..."
bt "2025-01-15" '{"command":"reset"}' > /dev/null

RESULT=$(bt "2025-01-15" '{"command":"balance"}')
CASH=$(echo "$RESULT" | jq -r '.cashBalance // empty')
TRADES=$(echo "$RESULT" | jq -r '.totalTrades // empty')

if [[ "$TRADES" != "0" ]]; then
    fail "Reset failed: $TRADES trades remain"
    exit 1
fi
ok "Start: cash \$$CASH, $TRADES trades"

# ── Buy BTC ────────────────────────────────────────────────────────────
info "Buying 0.01 BTC..."
RESULT=$(bt "2025-01-15" '{"command":"buy","symbol":"BTC","amount":0.01}')

CASH=$(echo "$RESULT" | jq -r '.portfolio.cashBalance // empty')
if echo "$CASH" | grep -q '^-'; then
    ok "After buy: cash \$$CASH (negative — correct)"
else
    fail "Expected negative cash after buy, got: \$$CASH"
    exit 1
fi

# ── Sell BTC at profit ─────────────────────────────────────────────────
info "Selling 0.01 BTC at \$105000 (above entry)..."
RESULT=$(bt "2025-02-15" '{"command":"sell","symbol":"BTC","amount":0.01,"price":105000}')

CASH=$(echo "$RESULT" | jq -r '.portfolio.cashBalance // empty')
if echo "$CASH" | grep -qv '^-'; then
    ok "After sell: cash \$$CASH (positive — profit!)"
else
    fail "Expected positive cash after profitable sell, got: \$$CASH"
    exit 1
fi

# ── Verify via balance command ─────────────────────────────────────────
RESULT=$(bt "2025-02-15" '{"command":"balance"}')
CASH=$(echo "$RESULT" | jq -r '.cashBalance // empty')
TRADES=$(echo "$RESULT" | jq -r '.totalTrades // empty')
POS_COUNT=$(echo "$RESULT" | jq '.positions | length')

done_step
ok "Final: cash \$$CASH, $TRADES trades, $POS_COUNT open positions"
echo "$RESULT" | jq .

# ── Cleanup ────────────────────────────────────────────────────────────
bt "2025-01-15" '{"command":"reset"}' > /dev/null

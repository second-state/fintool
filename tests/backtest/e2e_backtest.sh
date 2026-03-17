#!/usr/bin/env bash
#
# End-to-end backtest CLI tests
#
# Tests historical quotes, simulated trades, PnL output, persistent
# portfolio balance and positions.
#
# Usage: ./tests/backtest/e2e_backtest.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

bt() { $BACKTEST --at "$1" --json "$2" 2>/dev/null; }

# ══════════════════════════════════════════════════════════════════════
# 0. Reset portfolio
# ══════════════════════════════════════════════════════════════════════
log "Step 0: Reset portfolio"

RESULT=$(bt "2025-01-15" '{"command":"reset"}')
STATUS=$(echo "$RESULT" | jq -r '.status // empty')
if [[ "$STATUS" == "ok" ]]; then
    ok "Portfolio reset"
else
    fail "Portfolio reset failed"
    echo "$RESULT" | jq .
    exit 1
fi

# ══════════════════════════════════════════════════════════════════════
# 1. Historical quote — BTC
# ══════════════════════════════════════════════════════════════════════
log "Step 1: Historical BTC quote on 2025-01-15"

RESULT=$(bt "2025-01-15" '{"command":"quote","symbol":"BTC"}')
if [[ -z "$RESULT" ]]; then
    fail "BTC historical quote failed"
    exit 1
fi

PRICE=$(echo "$RESULT" | jq -r '.price // empty')
if [[ -z "$PRICE" ]]; then
    fail "BTC quote returned but price is missing"
    echo "$RESULT" | jq .
    exit 1
fi

ok "BTC price on 2025-01-15: \$$PRICE"

# ══════════════════════════════════════════════════════════════════════
# 2. Historical quote — AAPL (stock)
# ══════════════════════════════════════════════════════════════════════
log "Step 2: Historical AAPL quote on 2025-01-15"

RESULT=$(bt "2025-01-15" '{"command":"quote","symbol":"AAPL"}')
PRICE=$(echo "$RESULT" | jq -r '.price // empty')

if [[ -n "$PRICE" ]]; then
    ok "AAPL price on 2025-01-15: \$$PRICE"
else
    warn "AAPL quote returned no price"
    echo "$RESULT" | jq .
fi

# ══════════════════════════════════════════════════════════════════════
# 3. Historical quote — GOLD (commodity alias)
# ══════════════════════════════════════════════════════════════════════
log "Step 3: Historical GOLD quote on 2025-01-15"

RESULT=$(bt "2025-01-15" '{"command":"quote","symbol":"GOLD"}')
PRICE=$(echo "$RESULT" | jq -r '.price // empty')

if [[ -n "$PRICE" ]]; then
    ok "GOLD price on 2025-01-15: \$$PRICE"
else
    warn "GOLD quote returned no price"
fi

# ══════════════════════════════════════════════════════════════════════
# 4. Simulated spot buy with PnL + portfolio
# ══════════════════════════════════════════════════════════════════════
log "Step 4: Simulated BTC spot buy"

RESULT=$(bt "2025-01-15" '{"command":"buy","symbol":"BTC","amount":0.01}')
if [[ -z "$RESULT" ]]; then
    fail "BTC simulated buy failed"
    exit 1
fi

PNL_COUNT=$(echo "$RESULT" | jq '.pnl | length')
if [[ "$PNL_COUNT" -ge 1 ]]; then
    ok "BTC buy returned PnL with $PNL_COUNT offsets"
else
    warn "BTC buy returned no PnL data"
    echo "$RESULT" | jq .
fi

# Verify portfolio is included in trade output
CASH=$(echo "$RESULT" | jq -r '.portfolio.cashBalance // empty')
if [[ -n "$CASH" ]]; then
    ok "Portfolio cash balance after buy: \$$CASH"
else
    fail "Trade output missing portfolio.cashBalance"
fi

POS_COUNT=$(echo "$RESULT" | jq '.portfolio.positions | length')
if [[ "$POS_COUNT" -ge 1 ]]; then
    ok "Portfolio shows $POS_COUNT position(s)"
else
    fail "No positions after buy"
fi

# ══════════════════════════════════════════════════════════════════════
# 5. Check balance (should be negative)
# ══════════════════════════════════════════════════════════════════════
log "Step 5: Check balance (should be negative after buy)"

RESULT=$(bt "2025-01-15" '{"command":"balance"}')
CASH=$(echo "$RESULT" | jq -r '.cashBalance // empty')
TRADES=$(echo "$RESULT" | jq -r '.totalTrades // empty')

# Cash should be negative (we bought)
if echo "$CASH" | grep -q '^-'; then
    ok "Cash balance is negative: \$$CASH (correct after buy)"
else
    warn "Expected negative cash balance, got: \$$CASH"
fi
ok "Total trades: $TRADES"

# ══════════════════════════════════════════════════════════════════════
# 6. Simulated perp sell (short) with PnL
# ══════════════════════════════════════════════════════════════════════
log "Step 6: Simulated ETH perp short"

RESULT=$(bt "2025-01-15" '{"command":"perp_sell","symbol":"ETH","amount":0.1,"price":3300}')
if [[ -z "$RESULT" ]]; then
    fail "ETH perp short failed"
    exit 1
fi

SIDE=$(echo "$RESULT" | jq -r '.trade.side // empty')
if [[ "$SIDE" == "sell" ]]; then
    ok "ETH perp short recorded (side=$SIDE)"
else
    warn "Unexpected trade side: $SIDE"
fi

# ══════════════════════════════════════════════════════════════════════
# 7. Check positions
# ══════════════════════════════════════════════════════════════════════
log "Step 7: Check positions"

RESULT=$(bt "2025-01-15" '{"command":"positions"}')
POS_COUNT=$(echo "$RESULT" | jq '.positions | length')
if [[ "$POS_COUNT" -ge 2 ]]; then
    ok "Found $POS_COUNT positions (BTC spot + ETH perp)"
    echo "$RESULT" | jq '.positions[] | {symbol, type, side, quantity}'
else
    warn "Expected 2 positions, got $POS_COUNT"
    echo "$RESULT" | jq .
fi

# ══════════════════════════════════════════════════════════════════════
# 8. News stub
# ══════════════════════════════════════════════════════════════════════
log "Step 8: News stub"

RESULT=$(bt "2025-01-15" '{"command":"news","symbol":"BTC"}')
MSG=$(echo "$RESULT" | jq -r '.message // empty')
if [[ "$MSG" == *"not available"* ]]; then
    ok "News stub returned expected message"
else
    warn "Unexpected news response"
    echo "$RESULT" | jq .
fi

# ══════════════════════════════════════════════════════════════════════
# 9. SEC report list with date filter
# ══════════════════════════════════════════════════════════════════════
log "Step 9: SEC filings for AAPL before 2024-06-01"

RESULT=$(bt "2024-06-01" '{"command":"report_list","symbol":"AAPL","limit":3}')
if [[ -z "$RESULT" ]]; then
    fail "Report list failed"
    exit 1
fi

COUNT=$(echo "$RESULT" | jq 'length')
if [[ "$COUNT" -ge 1 ]]; then
    ok "Found $COUNT filings for AAPL before 2024-06-01"
    echo "$RESULT" | jq '.[] | {form, filingDate}'
else
    warn "No filings returned"
fi

# ══════════════════════════════════════════════════════════════════════
# 10. Reset and verify clean state
# ══════════════════════════════════════════════════════════════════════
log "Step 10: Reset and verify clean state"

bt "2025-01-15" '{"command":"reset"}' > /dev/null
RESULT=$(bt "2025-01-15" '{"command":"balance"}')
CASH=$(echo "$RESULT" | jq -r '.cashBalance // empty')
TRADES=$(echo "$RESULT" | jq -r '.totalTrades // empty')

if [[ "$TRADES" == "0" ]]; then
    ok "Portfolio reset: 0 trades, cash \$$CASH"
else
    fail "Reset failed: $TRADES trades remain"
fi

done_step
ok "End-to-end backtest workflow complete"

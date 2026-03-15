#!/usr/bin/env bash
#
# Simulated ETH perp buy (long) with forward PnL via backtest
#
# Uses backtest --json API. Output is always JSON.
#
# Workflow:
#   1. Set ETH leverage to 3x
#   2. Simulate perp buy of 0.1 ETH at $3300 on 2025-01-15
#   3. Verify trade details and leveraged PnL
#
# Usage: ./tests/backtest/perp_buy.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

bt() { $BACKTEST --at "$1" --json "$2" 2>/dev/null; }

log "Simulated ETH perp long on 2025-01-15 (JSON API)"

# ── Set leverage ───────────────────────────────────────────────────────
info "Setting ETH leverage to 3x..."
RESULT=$(bt "2025-01-15" '{"command":"perp_leverage","symbol":"ETH","leverage":3}')

if [[ -z "$RESULT" ]]; then
    fail "ETH set leverage failed"
    exit 1
fi

LEVERAGE=$(echo "$RESULT" | jq -r '.leverage // empty')
ok "ETH leverage set to ${LEVERAGE}x"

# ── Simulate perp buy ─────────────────────────────────────────────────
info "Perp buying 0.1 ETH at \$3300..."
RESULT=$(bt "2025-01-15" '{"command":"perp_buy","symbol":"ETH","amount":0.1,"price":3300}')

if [[ -z "$RESULT" ]]; then
    fail "ETH perp buy returned empty"
    exit 1
fi

# Verify trade details
SIDE=$(echo "$RESULT" | jq -r '.trade.side // empty')
TRADE_TYPE=$(echo "$RESULT" | jq -r '.trade.tradeType // empty')

if [[ "$SIDE" != "buy" ]]; then
    fail "Expected side buy, got: $SIDE"
    echo "$RESULT" | jq .
    exit 1
fi

if [[ "$TRADE_TYPE" != "perp" ]]; then
    fail "Expected tradeType perp, got: $TRADE_TYPE"
    exit 1
fi

# Verify PnL offsets
PNL_COUNT=$(echo "$RESULT" | jq '.pnl | length')
if [[ "$PNL_COUNT" -lt 1 ]]; then
    fail "No PnL data returned"
    echo "$RESULT" | jq .
    exit 1
fi

done_step
ok "ETH perp buy: 0.1 ETH (side=$SIDE, type=$TRADE_TYPE)"
ok "PnL offsets returned: $PNL_COUNT"
echo "$RESULT" | jq '.pnl[] | {offset, price, pnl, pnlPct}'

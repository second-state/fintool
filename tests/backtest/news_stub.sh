#!/usr/bin/env bash
#
# News stub test via backtest
#
# Uses backtest --json API. Output is always JSON.
#
# Workflow:
#   1. Request historical news for BTC
#   2. Verify stub message is returned
#
# Usage: ./tests/backtest/news_stub.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

bt() { $BACKTEST --at "$1" --json "$2" 2>/dev/null; }

log "News stub for BTC on 2025-01-15 (JSON API)"

# ── Test news stub ─────────────────────────────────────────────────────
info "Requesting BTC news..."
RESULT=$(bt "2025-01-15" '{"command":"news","symbol":"BTC"}')

if [[ -z "$RESULT" ]]; then
    fail "News command returned empty"
    exit 1
fi

MSG=$(echo "$RESULT" | jq -r '.message // empty')

if [[ "$MSG" == *"not available"* ]]; then
    done_step
    ok "News stub returned expected message"
    echo "$RESULT" | jq .
else
    fail "Unexpected news response"
    echo "$RESULT" | jq .
    exit 1
fi

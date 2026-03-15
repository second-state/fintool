#!/usr/bin/env bash
#
# SEC filings with date filter via backtest
#
# Uses backtest --json API. Output is always JSON.
#
# Workflow:
#   1. List AAPL SEC filings before 2024-06-01
#   2. Verify filings are returned and dates are correct
#
# Usage: ./tests/backtest/report_list.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

bt() { $BACKTEST --at "$1" --json "$2" 2>/dev/null; }

log "SEC filings for AAPL before 2024-06-01 (JSON API)"

# ── Fetch SEC filings ─────────────────────────────────────────────────
info "Listing AAPL filings before 2024-06-01..."
RESULT=$(bt "2024-06-01" '{"command":"report_list","symbol":"AAPL","limit":3}')

if [[ -z "$RESULT" ]]; then
    fail "Report list returned empty"
    exit 1
fi

COUNT=$(echo "$RESULT" | jq 'length')

if [[ "$COUNT" -lt 1 ]]; then
    fail "No filings returned"
    echo "$RESULT" | jq .
    exit 1
fi

# Verify all filing dates are before or on 2024-06-01
FUTURE=$(echo "$RESULT" | jq '[.[] | select(.filingDate > "2024-06-01")] | length')
if [[ "$FUTURE" -gt 0 ]]; then
    fail "Found $FUTURE filing(s) after 2024-06-01 — date filter broken"
    echo "$RESULT" | jq .
    exit 1
fi

done_step
ok "Found $COUNT filings for AAPL before 2024-06-01"
echo "$RESULT" | jq '.[] | {form: .form, filingDate: .filingDate, reportDate: .reportDate}'

#!/usr/bin/env bash
#
# Buy ~$1 of TSLA on Coinbase
#
# Uses the human CLI API — fintool commands produce human-readable output.
# Price data is fetched from the Yahoo Finance API.
#
# Usage: ./tests/human/buy_tsla.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

log "Buy ~\$1 TSLA on Coinbase"

# ── Get TSLA price from Yahoo Finance API ────────────────────────────
info "Fetching TSLA price from Yahoo Finance API..."
TSLA_PRICE=$(curl -s "https://query1.finance.yahoo.com/v8/finance/chart/TSLA" | jq -r '.chart.result[0].meta.regularMarketPrice' 2>/dev/null)

if [[ -z "$TSLA_PRICE" || "$TSLA_PRICE" == "null" ]]; then
    fail "Could not fetch TSLA price from Yahoo Finance API"
    exit 1
fi

# ── Compute order size and limit ─────────────────────────────────────
BUY_SIZE=$(echo "$TSLA_PRICE" | awk '{printf "%.6f", 1.0 / $1}')
BUY_LIMIT=$(echo "$TSLA_PRICE" | awk '{printf "%.2f", $1 * 1.01}')

info "TSLA price:       \$$TSLA_PRICE"
info "Limit buy price:  \$$BUY_LIMIT (+1%)"
info "Buy size:         $BUY_SIZE shares (~\$1)"

# ── Place buy order on Coinbase ──────────────────────────────────────
run_fintool order buy TSLA --amount "$BUY_SIZE" --price "$BUY_LIMIT" --exchange coinbase

if check_fail "TSLA spot buy on Coinbase failed"; then
    exit 1
fi

done_step
info "Output: $LAST_STDOUT"
ok "TSLA spot buy placed on Coinbase -- ~$BUY_SIZE shares at \$$BUY_LIMIT"

#!/usr/bin/env bash
#
# End-to-end trading test on OKX
#
# Uses okx --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Show initial status (balance, positions, orders)
#   2. BTC spot: buy ~$12 worth, then sell
#   3. Transfer USDT from funding to trading
#   4. ETH perp: set leverage, buy ~$12, then sell
#   5. Transfer USDT back from trading to funding
#   6. Show final status
#
# Prerequisites:
#   - OKX API key, secret, and passphrase in ~/.fintool/config.toml
#   - USDT in funding or trading account (at least ~$25)
#
# Usage: ./tests/okx/e2e_trading.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $OKX --json "$1" 2>/dev/null; }

log "End-to-end trading test on OKX (JSON API)"

# ═══════════════════════════════════════════════════════════════════════
# Step 1: Show initial status
# ═══════════════════════════════════════════════════════════════════════
log "Step 1: Initial balance check"
BALANCE=$(ft '{"command":"balance"}')

if [[ -n "$BALANCE" ]]; then
    info "Trading account USDT:"
    echo "$BALANCE" | jq -r '.trading // [] | .[]? | .details // [] | .[]? | select(.ccy == "USDT") | "    eq: \(.eq) avail: \(.availBal)"' 2>/dev/null || echo "    (none)"
    info "Funding account USDT:"
    echo "$BALANCE" | jq -r '.funding // [] | .[]? | select(.ccy == "USDT") | "    bal: \(.bal) avail: \(.availBal)"' 2>/dev/null || echo "    (none)"
else
    fail "Could not fetch balance"
    exit 1
fi
ok "Initial balance checked"

# ═══════════════════════════════════════════════════════════════════════
# Step 2: BTC spot buy
# ═══════════════════════════════════════════════════════════════════════
log "Step 2: BTC spot buy (~\$12)"

info "Fetching BTC price..."
QUOTE=$(ft '{"command":"quote","symbol":"BTC"}')
BTC_PRICE=$(echo "$QUOTE" | jq -r '.price // empty')

if [[ -z "$BTC_PRICE" || "$BTC_PRICE" == "null" ]]; then
    fail "BTC quote failed"
    exit 1
fi

BTC_SIZE=$(echo "12 / $BTC_PRICE" | bc -l | xargs printf "%.5f")
BTC_BUY_LIMIT=$(echo "$BTC_PRICE" | awk '{printf "%.2f", $1 * 1.005}')

info "BTC price: \$$BTC_PRICE"
info "Buy size:  $BTC_SIZE BTC at \$$BTC_BUY_LIMIT"

RESULT=$(ft "{\"command\":\"buy\",\"symbol\":\"BTC\",\"amount\":$BTC_SIZE,\"price\":$BTC_BUY_LIMIT}")
BTC_BUY_STATUS=$(echo "$RESULT" | jq -r '.status // "unknown"')
info "Buy status: $BTC_BUY_STATUS"
ok "BTC spot buy order placed"

sleep 2

# ═══════════════════════════════════════════════════════════════════════
# Step 3: BTC spot sell
# ═══════════════════════════════════════════════════════════════════════
log "Step 3: BTC spot sell"

BTC_SELL_LIMIT=$(echo "$BTC_PRICE" | awk '{printf "%.2f", $1 * 0.995}')
info "Selling $BTC_SIZE BTC at \$$BTC_SELL_LIMIT"

RESULT=$(ft "{\"command\":\"sell\",\"symbol\":\"BTC\",\"amount\":$BTC_SIZE,\"price\":$BTC_SELL_LIMIT}")
BTC_SELL_STATUS=$(echo "$RESULT" | jq -r '.status // "unknown"')
info "Sell status: $BTC_SELL_STATUS"
ok "BTC spot sell order placed"

sleep 2

# ═══════════════════════════════════════════════════════════════════════
# Step 4: Transfer USDT to trading
# ═══════════════════════════════════════════════════════════════════════
log "Step 4: Transfer \$12 USDT from funding to trading"

RESULT=$(ft '{"command":"transfer","asset":"USDT","amount":12,"from":"funding","to":"trading"}')
if [[ -z "$RESULT" ]]; then
    warn "Transfer to trading failed -- may not have enough USDT in funding"
else
    TXN_ID=$(echo "$RESULT" | jq -r '.transactionId // empty')
    info "Transfer txn ID: ${TXN_ID:-unknown}"
    ok "Transferred \$12 USDT to trading"
fi

sleep 1

# ═══════════════════════════════════════════════════════════════════════
# Step 5: ETH perp buy
# ═══════════════════════════════════════════════════════════════════════
log "Step 5: ETH perp buy (~\$12)"

info "Setting ETH-USDT-SWAP leverage to 2x..."
RESULT=$(ft '{"command":"perp_leverage","symbol":"ETH","leverage":2,"cross":true}')
if [[ -z "$RESULT" ]]; then
    warn "Set leverage failed"
else
    ok "ETH leverage set to 2x"
fi

info "Fetching ETH price..."
QUOTE=$(ft '{"command":"quote","symbol":"ETH"}')
ETH_PRICE=$(echo "$QUOTE" | jq -r '.price // empty')

if [[ -z "$ETH_PRICE" || "$ETH_PRICE" == "null" ]]; then
    fail "ETH quote failed"
    exit 1
fi

ETH_SIZE=$(echo "12 / $ETH_PRICE" | bc -l | xargs printf "%.3f")
ETH_BUY_LIMIT=$(echo "$ETH_PRICE" | awk '{printf "%.2f", $1 * 1.005}')

info "ETH price: \$$ETH_PRICE"
info "Buy size:  $ETH_SIZE ETH at \$$ETH_BUY_LIMIT"

RESULT=$(ft "{\"command\":\"perp_buy\",\"symbol\":\"ETH\",\"amount\":$ETH_SIZE,\"price\":$ETH_BUY_LIMIT,\"close\":false}")
ETH_BUY_STATUS=$(echo "$RESULT" | jq -r '.status // "unknown"')
info "Buy status: $ETH_BUY_STATUS"
ok "ETH perp buy order placed"

sleep 2

# ═══════════════════════════════════════════════════════════════════════
# Step 6: ETH perp sell (close)
# ═══════════════════════════════════════════════════════════════════════
log "Step 6: ETH perp sell (close position)"

POSITIONS=$(ft '{"command":"positions"}')
ETH_POS=$(echo "$POSITIONS" | jq -r '
    .positions // [] |
    map(select(.instId == "ETH-USDT-SWAP" and (.pos | tonumber | fabs) > 0)) |
    .[0].pos // empty
' 2>/dev/null || true)

if [[ -z "$ETH_POS" || "$ETH_POS" == "null" ]]; then
    warn "No ETH perp position -- buy may not have filled"
else
    ETH_SELL_SIZE=$(echo "$ETH_POS" | sed 's/^-//')
    ETH_SELL_LIMIT=$(echo "$ETH_PRICE" | awk '{printf "%.2f", $1 * 0.995}')
    info "Selling $ETH_SELL_SIZE ETH at \$$ETH_SELL_LIMIT (close)"

    RESULT=$(ft "{\"command\":\"perp_sell\",\"symbol\":\"ETH\",\"amount\":$ETH_SELL_SIZE,\"price\":$ETH_SELL_LIMIT,\"close\":true}")
    ETH_SELL_STATUS=$(echo "$RESULT" | jq -r '.status // "unknown"')
    info "Sell status: $ETH_SELL_STATUS"
    ok "ETH perp sell order placed"
fi

sleep 2

# ═══════════════════════════════════════════════════════════════════════
# Step 7: Transfer USDT back to funding
# ═══════════════════════════════════════════════════════════════════════
log "Step 7: Transfer USDT back from trading to funding"

BALANCE=$(ft '{"command":"balance"}')
TRADING_USDT=$(echo "$BALANCE" | jq -r '.trading // [] | .[]? | .details // [] | .[]? | select(.ccy == "USDT") | .availBal // "0"' 2>/dev/null || echo "0")
TRANSFER_BACK=$(echo "$TRADING_USDT" | awk '{v = int($1 * 100) / 100; if (v > 0) printf "%.2f", v; else print "0"}')

if [[ "$TRANSFER_BACK" != "0" && "$TRANSFER_BACK" != "0.00" ]]; then
    info "Transferring $TRANSFER_BACK USDT from trading to funding..."
    RESULT=$(ft "{\"command\":\"transfer\",\"asset\":\"USDT\",\"amount\":$TRANSFER_BACK,\"from\":\"trading\",\"to\":\"funding\"}")
    if [[ -z "$RESULT" ]]; then
        warn "Transfer back to funding failed"
    else
        ok "Transferred $TRANSFER_BACK USDT back to funding"
    fi
else
    info "No available USDT in trading to transfer back"
fi

# ═══════════════════════════════════════════════════════════════════════
# Step 8: Final status
# ═══════════════════════════════════════════════════════════════════════
log "Step 8: Final status"

POSITIONS=$(ft '{"command":"positions"}')
ORDERS=$(ft '{"command":"orders"}')
BALANCE=$(ft '{"command":"balance"}')

# Positions
POS_COUNT=$(echo "$POSITIONS" | jq -r '.positions | length' 2>/dev/null || echo "?")
info "Open positions: $POS_COUNT"

# Orders
ORD_COUNT=$(echo "$ORDERS" | jq -r '.orders | length' 2>/dev/null || echo "0")
info "Open orders: $ORD_COUNT"

# Balances
info "Trading USDT:"
echo "$BALANCE" | jq -r '.trading // [] | .[]? | .details // [] | .[]? | select(.ccy == "USDT") | "    eq: \(.eq) avail: \(.availBal)"' 2>/dev/null || echo "    (none)"
info "Funding USDT:"
echo "$BALANCE" | jq -r '.funding // [] | .[]? | select(.ccy == "USDT") | "    bal: \(.bal) avail: \(.availBal)"' 2>/dev/null || echo "    (none)"

done_step
ok "End-to-end trading test completed"

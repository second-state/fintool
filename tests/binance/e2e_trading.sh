#!/usr/bin/env bash
#
# End-to-end trading test on Binance
#
# Uses binance --json API for all commands. Output is always JSON.
#
# Workflow:
#   1. Show initial status (balance, positions, orders)
#   2. BTC spot: buy ~$12 worth, then sell
#   3. Transfer USDT from spot to futures
#   4. ETH futures: set leverage, buy ~$12, then sell
#   5. Transfer USDT back from futures to spot
#   6. Show final status
#
# Prerequisites:
#   - Binance API key and secret in ~/.fintool/config.toml
#   - USDT in spot wallet (at least ~$25)
#
# Usage: ./tests/binance/e2e_trading.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $BINANCE --json "$1" 2>/dev/null; }

log "End-to-end trading test on Binance (JSON API)"

# ═══════════════════════════════════════════════════════════════════════
# Step 1: Show initial status
# ═══════════════════════════════════════════════════════════════════════
log "Step 1: Initial balance check"
BALANCE=$(ft '{"command":"balance"}')

if [[ -n "$BALANCE" ]]; then
    SPOT_USDT=$(echo "$BALANCE" | jq -r '.spot[]? | select(.asset == "USDT") | .free // "0"' 2>/dev/null || echo "0")
    FUTURES_USDT=$(echo "$BALANCE" | jq -r '.futures[]? | select(.asset == "USDT") | .availableBalance // "0"' 2>/dev/null || echo "0")
    info "Spot USDT free:         $SPOT_USDT"
    info "Futures USDT available: $FUTURES_USDT"
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
BTC_BUY_STATUS=$(echo "$RESULT" | jq -r '.response.status // "unknown"')
info "Buy status: $BTC_BUY_STATUS"
ok "BTC spot buy order placed"

sleep 2

# ═══════════════════════════════════════════════════════════════════════
# Step 3: BTC spot sell
# ═══════════════════════════════════════════════════════════════════════
log "Step 3: BTC spot sell"

# Check BTC balance
BALANCE=$(ft '{"command":"balance"}')
BTC_FREE=$(echo "$BALANCE" | jq -r '.spot[]? | select(.asset == "BTC") | .free // "0"' 2>/dev/null || echo "0")
BTC_SELL_SIZE=$(echo "$BTC_FREE" | awk '{v = int($1 * 100000) / 100000; if (v > 0) printf "%.5f", v; else print "0"}')

if [[ "$BTC_SELL_SIZE" == "0" || "$BTC_SELL_SIZE" == "0.00000" ]]; then
    warn "No BTC to sell -- buy may not have filled"
else
    BTC_SELL_LIMIT=$(echo "$BTC_PRICE" | awk '{printf "%.2f", $1 * 0.995}')
    info "Selling $BTC_SELL_SIZE BTC at \$$BTC_SELL_LIMIT"

    RESULT=$(ft "{\"command\":\"sell\",\"symbol\":\"BTC\",\"amount\":$BTC_SELL_SIZE,\"price\":$BTC_SELL_LIMIT}")
    BTC_SELL_STATUS=$(echo "$RESULT" | jq -r '.response.status // "unknown"')
    info "Sell status: $BTC_SELL_STATUS"
    ok "BTC spot sell order placed"
fi

sleep 2

# ═══════════════════════════════════════════════════════════════════════
# Step 4: Transfer USDT to futures
# ═══════════════════════════════════════════════════════════════════════
log "Step 4: Transfer \$12 USDT from spot to futures"

RESULT=$(ft '{"command":"transfer","asset":"USDT","amount":12,"from":"spot","to":"futures"}')
if [[ -z "$RESULT" ]]; then
    warn "Transfer to futures failed -- may not have enough USDT"
else
    TXN_ID=$(echo "$RESULT" | jq -r '.txn_id // empty')
    info "Transfer txn ID: ${TXN_ID:-unknown}"
    ok "Transferred \$12 USDT to futures"
fi

sleep 1

# ═══════════════════════════════════════════════════════════════════════
# Step 5: ETH futures buy
# ═══════════════════════════════════════════════════════════════════════
log "Step 5: ETH futures buy (~\$12)"

info "Setting ETHUSDT leverage to 2x..."
RESULT=$(ft '{"command":"perp_leverage","symbol":"ETH","leverage":2}')
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
ETH_BUY_STATUS=$(echo "$RESULT" | jq -r '.response.status // "unknown"')
info "Buy status: $ETH_BUY_STATUS"
ok "ETH futures buy order placed"

sleep 2

# ═══════════════════════════════════════════════════════════════════════
# Step 6: ETH futures sell (close)
# ═══════════════════════════════════════════════════════════════════════
log "Step 6: ETH futures sell (close position)"

POSITIONS=$(ft '{"command":"positions"}')
ETH_POS=$(echo "$POSITIONS" | jq -r '
    .futures // [] |
    map(select(.symbol == "ETHUSDT" and (.positionAmt | tonumber | fabs) > 0)) |
    .[0].positionAmt // empty
' 2>/dev/null || true)

if [[ -z "$ETH_POS" || "$ETH_POS" == "null" ]]; then
    warn "No ETH futures position -- buy may not have filled"
else
    ETH_SELL_SIZE=$(echo "$ETH_POS" | sed 's/^-//')
    ETH_SELL_LIMIT=$(echo "$ETH_PRICE" | awk '{printf "%.2f", $1 * 0.995}')
    info "Selling $ETH_SELL_SIZE ETH at \$$ETH_SELL_LIMIT (close)"

    RESULT=$(ft "{\"command\":\"perp_sell\",\"symbol\":\"ETH\",\"amount\":$ETH_SELL_SIZE,\"price\":$ETH_SELL_LIMIT,\"close\":true}")
    ETH_SELL_STATUS=$(echo "$RESULT" | jq -r '.response.status // "unknown"')
    info "Sell status: $ETH_SELL_STATUS"
    ok "ETH futures sell order placed"
fi

sleep 2

# ═══════════════════════════════════════════════════════════════════════
# Step 7: Transfer USDT back to spot
# ═══════════════════════════════════════════════════════════════════════
log "Step 7: Transfer USDT back from futures to spot"

BALANCE=$(ft '{"command":"balance"}')
FUTURES_USDT=$(echo "$BALANCE" | jq -r '.futures[]? | select(.asset == "USDT") | .availableBalance // "0"' 2>/dev/null || echo "0")
TRANSFER_BACK=$(echo "$FUTURES_USDT" | awk '{v = int($1 * 100) / 100; if (v > 0) printf "%.2f", v; else print "0"}')

if [[ "$TRANSFER_BACK" != "0" && "$TRANSFER_BACK" != "0.00" ]]; then
    info "Transferring $TRANSFER_BACK USDT from futures to spot..."
    RESULT=$(ft "{\"command\":\"transfer\",\"asset\":\"USDT\",\"amount\":$TRANSFER_BACK,\"from\":\"futures\",\"to\":\"spot\"}")
    if [[ -z "$RESULT" ]]; then
        warn "Transfer back to spot failed"
    else
        ok "Transferred $TRANSFER_BACK USDT back to spot"
    fi
else
    info "No available USDT in futures to transfer back"
fi

# ═══════════════════════════════════════════════════════════════════════
# Step 8: Final status
# ═══════════════════════════════════════════════════════════════════════
log "Step 8: Final status"

POSITIONS=$(ft '{"command":"positions"}')
ORDERS=$(ft '{"command":"orders"}')
BALANCE=$(ft '{"command":"balance"}')

# Positions
POS_COUNT=$(echo "$POSITIONS" | jq -r '.futures | length' 2>/dev/null || echo "?")
info "Open futures positions: $POS_COUNT"

# Orders
SPOT_ORD=$(echo "$ORDERS" | jq -r '.spot | length' 2>/dev/null || echo "0")
FUTURES_ORD=$(echo "$ORDERS" | jq -r '.futures | length' 2>/dev/null || echo "0")
info "Open spot orders:    $SPOT_ORD"
info "Open futures orders: $FUTURES_ORD"

# Balances
SPOT_USDT=$(echo "$BALANCE" | jq -r '.spot[]? | select(.asset == "USDT") | .free // "0"' 2>/dev/null || echo "0")
FUTURES_USDT=$(echo "$BALANCE" | jq -r '.futures[]? | select(.asset == "USDT") | .availableBalance // "0"' 2>/dev/null || echo "0")
info "Spot USDT free:         $SPOT_USDT"
info "Futures USDT available: $FUTURES_USDT"

done_step
ok "End-to-end trading test completed"

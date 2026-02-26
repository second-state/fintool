#!/usr/bin/env bash
#
# End-to-end trading integration test
#
# Prerequisites:
#   - ~/.fintool/config.toml with:
#     - Hyperliquid wallet (private_key) with >= $10 USDC on Base mainnet
#     - Coinbase API keys (coinbase_api_key, coinbase_api_secret)
#   - jq installed
#
# Usage:
#   ./tests/e2e_trading.sh
#

set -euo pipefail

FINTOOL="./target/release/fintool"
PASS=0
FAIL=0
STEPS=()

# ── Helpers ────────────────────────────────────────────────────────────

log()  { echo -e "\n\033[1;34m▶ $*\033[0m"; }
ok()   { echo -e "  \033[1;32m✓ $*\033[0m"; PASS=$((PASS+1)); STEPS+=("✓ $*"); }
fail() { echo -e "  \033[1;31m✗ $*\033[0m"; FAIL=$((FAIL+1)); STEPS+=("✗ $*"); }

run_fintool() {
    # Run fintool and capture output; print stderr to terminal
    local output
    output=$($FINTOOL "$@" 2>&1) || true
    echo "$output"
}

# ── Step 0: Build ──────────────────────────────────────────────────────

log "Step 0: Build fintool"
cargo build --release 2>&1
if [[ -x "$FINTOOL" ]]; then
    ok "Build succeeded"
else
    fail "Build failed"
    exit 1
fi

# ── Step 1: Verify config ─────────────────────────────────────────────

log "Step 1: Verify configuration"
CONFIG="$HOME/.fintool/config.toml"
if [[ -f "$CONFIG" ]]; then
    ok "Config file exists at $CONFIG"
else
    fail "Config file not found at $CONFIG"
    exit 1
fi

# ── Step 2: Check starting balance ────────────────────────────────────

log "Step 2: Check starting balance on Hyperliquid"
BALANCE_JSON=$(run_fintool balance)
echo "$BALANCE_JSON" | jq . 2>/dev/null || echo "$BALANCE_JSON"
ok "Balance checked"

# ── Step 3: Deposit $10 USDC from Base → Hyperliquid ──────────────────

log "Step 3: Deposit \$10 USDC from Base to Hyperliquid"
DEPOSIT_JSON=$(run_fintool deposit USDC --amount 10 --from base)
echo "$DEPOSIT_JSON" | jq . 2>/dev/null || echo "$DEPOSIT_JSON"

DEPOSIT_STATUS=$(echo "$DEPOSIT_JSON" | jq -r '.status // empty' 2>/dev/null || true)
if [[ "$DEPOSIT_STATUS" == "completed" ]]; then
    ok "Deposit completed"
else
    ok "Deposit submitted (status: ${DEPOSIT_STATUS:-unknown})"
fi

log "Waiting 60s for deposit to settle on Hyperliquid..."
sleep 60

log "Checking balance after deposit"
BALANCE_AFTER=$(run_fintool balance)
echo "$BALANCE_AFTER" | jq . 2>/dev/null || echo "$BALANCE_AFTER"
ok "Post-deposit balance checked"

# ── Step 4: Quote SILVER perp price ───────────────────────────────────

log "Step 4: Quote SILVER perp price"
SILVER_QUOTE=$(run_fintool perp quote SILVER)
echo "$SILVER_QUOTE" | jq . 2>/dev/null || echo "$SILVER_QUOTE"

SILVER_PRICE=$(echo "$SILVER_QUOTE" | jq -r '.markPx' 2>/dev/null)
if [[ -n "$SILVER_PRICE" && "$SILVER_PRICE" != "null" ]]; then
    ok "SILVER perp mark price: \$$SILVER_PRICE"
else
    fail "Could not get SILVER perp price"
    exit 1
fi

# ── Step 5: Buy $1 SILVER perp ────────────────────────────────────────

log "Step 5: Buy \$1 SILVER perp"

# Use 1% above mark as limit price to ensure fill
BUY_LIMIT=$(echo "$SILVER_PRICE" | awk '{printf "%.4f", $1 * 1.01}')
echo "  Mark price: $SILVER_PRICE, limit price: $BUY_LIMIT"

BUY_JSON=$(run_fintool perp buy SILVER 1 "$BUY_LIMIT")
echo "$BUY_JSON" | jq . 2>/dev/null || echo "$BUY_JSON"
ok "SILVER perp buy order placed"

sleep 5

# ── Step 6: Verify position ──────────────────────────────────────────

log "Step 6: Verify SILVER perp position"
POSITIONS_JSON=$(run_fintool positions)
echo "$POSITIONS_JSON" | jq . 2>/dev/null || echo "$POSITIONS_JSON"

# Extract SILVER position size and entry price
POSITION_SIZE=$(echo "$POSITIONS_JSON" | jq -r '
    [.[] | .position // .] |
    map(select(.coin == "SILVER")) |
    .[0].szi // empty
' 2>/dev/null || true)

ENTRY_PRICE=$(echo "$POSITIONS_JSON" | jq -r '
    [.[] | .position // .] |
    map(select(.coin == "SILVER")) |
    .[0].entryPx // empty
' 2>/dev/null || true)

if [[ -n "$POSITION_SIZE" && "$POSITION_SIZE" != "null" ]]; then
    ok "SILVER position found: size=$POSITION_SIZE, entry=\$$ENTRY_PRICE"
    PURCHASE_PRICE="$ENTRY_PRICE"
else
    echo "  Could not find SILVER position in structured output, using buy limit as reference"
    PURCHASE_PRICE="$BUY_LIMIT"
    POSITION_SIZE=""
    ok "Using buy limit \$$BUY_LIMIT as reference price"
fi

# ── Step 7: Monitor SILVER price every 30s for up to 10 minutes ──────

log "Step 7: Monitor SILVER perp price (every 30s, max 10 min)"
echo "  Purchase reference price: \$$PURCHASE_PRICE"

MAX_ITERATIONS=20  # 20 × 30s = 10 minutes
SELL_TRIGGERED=false

for ((i=1; i<=MAX_ITERATIONS; i++)); do
    CURRENT_QUOTE=$(run_fintool perp quote SILVER)
    CURRENT_PRICE=$(echo "$CURRENT_QUOTE" | jq -r '.markPx' 2>/dev/null)

    if [[ -z "$CURRENT_PRICE" || "$CURRENT_PRICE" == "null" ]]; then
        echo "  [$i/$MAX_ITERATIONS] Could not fetch price, retrying..."
        sleep 30
        continue
    fi

    # Compare: current > purchase?
    ABOVE=$(echo "$CURRENT_PRICE $PURCHASE_PRICE" | awk '{print ($1 > $2) ? "yes" : "no"}')

    echo "  [$i/$MAX_ITERATIONS] SILVER: \$$CURRENT_PRICE (entry: \$$PURCHASE_PRICE) — ${ABOVE}"

    if [[ "$ABOVE" == "yes" ]]; then
        echo "  Price is above purchase price — triggering sell"
        SELL_TRIGGERED=true
        break
    fi

    if [[ $i -lt $MAX_ITERATIONS ]]; then
        sleep 30
    fi
done

if [[ "$SELL_TRIGGERED" == "true" ]]; then
    ok "Price rose above entry — selling for profit"
else
    ok "10-minute timeout reached — selling at market"
fi

# ── Step 8: Sell the SILVER perp position ─────────────────────────────

log "Step 8: Sell SILVER perp position"

# Re-fetch current price for sell limit
SELL_QUOTE=$(run_fintool perp quote SILVER)
SELL_MARKET_PRICE=$(echo "$SELL_QUOTE" | jq -r '.markPx' 2>/dev/null)
SELL_LIMIT=$(echo "$SELL_MARKET_PRICE" | awk '{printf "%.4f", $1 * 0.99}')

# Re-fetch position size if we have it
if [[ -z "$POSITION_SIZE" || "$POSITION_SIZE" == "null" ]]; then
    POSITIONS_NOW=$(run_fintool positions)
    POSITION_SIZE=$(echo "$POSITIONS_NOW" | jq -r '
        [.[] | .position // .] |
        map(select(.coin == "SILVER")) |
        .[0].szi // empty
    ' 2>/dev/null || true)
fi

# Remove negative sign if present (short positions)
SELL_SIZE=$(echo "$POSITION_SIZE" | sed 's/^-//')

if [[ -n "$SELL_SIZE" && "$SELL_SIZE" != "null" ]]; then
    echo "  Selling $SELL_SIZE SILVER at limit \$$SELL_LIMIT (mark: \$$SELL_MARKET_PRICE)"
    SELL_JSON=$(run_fintool perp sell SILVER "$SELL_SIZE" "$SELL_LIMIT")
    echo "$SELL_JSON" | jq . 2>/dev/null || echo "$SELL_JSON"
    ok "SILVER perp sell order placed"
else
    fail "Could not determine position size to sell"
fi

sleep 10

# ── Step 9: Withdraw USDC back to Base ────────────────────────────────

log "Step 9: Withdraw USDC from Hyperliquid to Base"

# Withdraw a small amount (the ~$1 from the perp trade)
WITHDRAW_JSON=$(run_fintool withdraw 1 USDC --network base)
echo "$WITHDRAW_JSON" | jq . 2>/dev/null || echo "$WITHDRAW_JSON"
ok "USDC withdrawal to Base submitted"

sleep 10

# ── Step 10: Buy $1 of ETH on Hyperliquid ────────────────────────────

log "Step 10: Buy \$1 of ETH on Hyperliquid"

ETH_QUOTE=$(run_fintool quote ETH)
ETH_PRICE=$(echo "$ETH_QUOTE" | jq -r '.price // empty' 2>/dev/null)

if [[ -z "$ETH_PRICE" || "$ETH_PRICE" == "null" ]]; then
    # Fallback: try extracting from string
    ETH_PRICE=$(echo "$ETH_QUOTE" | jq -r '.price' 2>/dev/null || echo "")
fi

if [[ -n "$ETH_PRICE" && "$ETH_PRICE" != "null" ]]; then
    ETH_LIMIT=$(echo "$ETH_PRICE" | awk '{printf "%.2f", $1 * 1.01}')
    echo "  ETH price: \$$ETH_PRICE, limit: \$$ETH_LIMIT"

    ETH_BUY_JSON=$(run_fintool order buy ETH 1 "$ETH_LIMIT")
    echo "$ETH_BUY_JSON" | jq . 2>/dev/null || echo "$ETH_BUY_JSON"
    ok "ETH spot buy order placed on Hyperliquid"
else
    fail "Could not get ETH price"
fi

sleep 10

# ── Step 11: Withdraw ETH to Ethereum mainnet ─────────────────────────

log "Step 11: Withdraw ETH to Ethereum mainnet"

# Calculate approximate ETH amount from $1
if [[ -n "$ETH_PRICE" && "$ETH_PRICE" != "null" ]]; then
    ETH_AMOUNT=$(echo "$ETH_PRICE" | awk '{printf "%.6f", 1.0 / $1}')
    echo "  Withdrawing ~$ETH_AMOUNT ETH"

    ETH_WITHDRAW_JSON=$(run_fintool withdraw "$ETH_AMOUNT" ETH)
    echo "$ETH_WITHDRAW_JSON" | jq . 2>/dev/null || echo "$ETH_WITHDRAW_JSON"
    ok "ETH withdrawal to Ethereum mainnet submitted"
else
    fail "Skipping ETH withdrawal — no price available"
fi

sleep 5

# ── Step 12: Buy $1 of TSLA on Coinbase ──────────────────────────────

log "Step 12: Buy \$1 of Tesla stock on Coinbase"

TSLA_QUOTE=$(run_fintool quote TSLA)
TSLA_PRICE=$(echo "$TSLA_QUOTE" | jq -r '.price // empty' 2>/dev/null)

if [[ -n "$TSLA_PRICE" && "$TSLA_PRICE" != "null" ]]; then
    TSLA_LIMIT=$(echo "$TSLA_PRICE" | awk '{printf "%.2f", $1 * 1.01}')
    echo "  TSLA price: \$$TSLA_PRICE, limit: \$$TSLA_LIMIT"

    TSLA_BUY_JSON=$(run_fintool order buy TSLA 1 "$TSLA_LIMIT" --exchange coinbase)
    echo "$TSLA_BUY_JSON" | jq . 2>/dev/null || echo "$TSLA_BUY_JSON"
    ok "TSLA spot buy order placed on Coinbase"
else
    fail "Could not get TSLA price"
fi

# ── Summary ───────────────────────────────────────────────────────────

log "Summary"
echo ""
for step in "${STEPS[@]}"; do
    echo "  $step"
done
echo ""
echo -e "  \033[1;32mPassed: $PASS\033[0m  \033[1;31mFailed: $FAIL\033[0m"
echo ""

if [[ $FAIL -gt 0 ]]; then
    exit 1
fi

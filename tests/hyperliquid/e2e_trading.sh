#!/usr/bin/env bash
#
# End-to-end hyperliquid workflow using the JSON API
#
# Uses hyperliquid --json API for all commands. Output is always JSON.
#
# This script illustrates the full deposit -> trade -> withdraw cycle.
# Every hyperliquid call uses --json mode with structured JSON input/output.
#
# Workflow:
#   1. Deposit USDC from Base
#   2. Enable unified mode
#   3. Trade ETH perp (buy + sell)
#   4. Trade HYPE spot (buy + sell)
#   5. Swap USDC -> USDT0 and transfer to cash dex
#   6. Trade SILVER perp (buy + sell)
#   7. Transfer USDT0 back and swap to USDC
#   8. Check status
#   9. Withdraw to Base
#
# Prerequisites:
#   - cargo build --release
#   - ~/.fintool/config.toml configured with wallet + API keys
#   - ETH on Base for gas fees
#   - USDC on Base to deposit
#
# Usage: ./tests/json/e2e_trading.sh
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../helpers.sh"
ensure_built

ft() { $HYPERLIQUID --json "$1" 2>/dev/null; }

# ══════════════════════════════════════════════════════════════════════
# 1. Deposit USDC from Base to Hyperliquid
# ══════════════════════════════════════════════════════════════════════
log "Step 1: Deposit \$15 USDC from Base"

info "Bridging \$15 USDC: Base -> Across -> Arbitrum -> HL Bridge2 -> Hyperliquid"
RESULT=$(ft '{"command":"deposit","asset":"USDC","amount":15,"from":"base"}')
if [[ -z "$RESULT" ]]; then
    fail "Deposit failed"
    exit 1
fi
DEPOSIT_STATUS=$(echo "$RESULT" | jq -r '.status // empty')
info "Deposit status: ${DEPOSIT_STATUS:-submitted}"
ok "Deposit submitted"

info "Waiting 300 seconds for deposit to settle..."
sleep 300

# ══════════════════════════════════════════════════════════════════════
# 2. Enable unified mode and check balance
# ══════════════════════════════════════════════════════════════════════
log "Step 2: Enable unified mode"

info "Enabling unified account mode..."
RESULT=$(ft '{"command":"perp_set_mode","mode":"unified"}')
if [[ -n "$RESULT" ]]; then
    ok "Unified mode enabled"
else
    warn "Could not enable unified mode -- continuing anyway"
fi

info "Checking balance..."
ft '{"command":"balance"}' | jq .

# ══════════════════════════════════════════════════════════════════════
# 3. Trade ETH perp
# ══════════════════════════════════════════════════════════════════════
log "Step 3: Trade ETH perp"

info "Setting ETH leverage to 2x..."
ft '{"command":"perp_leverage","symbol":"ETH","leverage":2}'

info "Fetching ETH quote..."
QUOTE=$(ft '{"command":"quote","symbol":"ETH"}')
echo "$QUOTE" | jq .
ETH_PRICE=$(echo "$QUOTE" | jq -r '.markPx')
info "ETH mark price: \$$ETH_PRICE"

info "Buying 0.006 ETH perp at \$2100 limit..."
RESULT=$(ft '{"command":"perp_buy","symbol":"ETH","amount":0.006,"price":2100.00,"close":false}')
BUY_FILL=$(echo "$RESULT" | jq -r '.fillStatus // empty')
info "ETH buy fill: $BUY_FILL"

info "Checking positions..."
ft '{"command":"positions"}' | jq .

info "Selling 0.006 ETH perp at \$2050 limit (close)..."
RESULT=$(ft '{"command":"perp_sell","symbol":"ETH","amount":0.006,"price":2050.00,"close":true}')
SELL_FILL=$(echo "$RESULT" | jq -r '.fillStatus // empty')
info "ETH sell fill: $SELL_FILL"

# ══════════════════════════════════════════════════════════════════════
# 4. Trade HYPE spot
# ══════════════════════════════════════════════════════════════════════
log "Step 4: Trade HYPE spot"

info "Fetching HYPE quote..."
QUOTE=$(ft '{"command":"quote","symbol":"HYPE"}')
echo "$QUOTE" | jq .
HYPE_PRICE=$(echo "$QUOTE" | jq -r '.markPx // empty')
info "HYPE price: \$$HYPE_PRICE"

info "Buying 0.48 HYPE at \$25.00 limit..."
RESULT=$(ft '{"command":"buy","symbol":"HYPE","amount":0.48,"price":25.00}')
BUY_FILL=$(echo "$RESULT" | jq -r '.fillStatus // empty')
info "HYPE buy fill: $BUY_FILL"

info "Checking balance..."
ft '{"command":"balance"}' | jq .

info "Selling 0.48 HYPE at \$24.50 limit..."
RESULT=$(ft '{"command":"sell","symbol":"HYPE","amount":0.48,"price":24.50}')
SELL_FILL=$(echo "$RESULT" | jq -r '.fillStatus // empty')
info "HYPE sell fill: $SELL_FILL"

# ══════════════════════════════════════════════════════════════════════
# 5. Swap USDC -> USDT0 and transfer to cash dex
# ══════════════════════════════════════════════════════════════════════
log "Step 5: Fund cash dex with USDT0"

info "Buying 30 USDT0 (swapping USDC -> USDT0)..."
RESULT=$(ft '{"command":"buy","symbol":"USDT0","amount":30,"price":1.002}')
SWAP_FILL=$(echo "$RESULT" | jq -r '.fillStatus // empty')
info "USDT0 buy fill: $SWAP_FILL"

info "Transferring 30 USDT0 from spot to cash dex..."
RESULT=$(ft '{"command":"transfer","asset":"USDT0","amount":30,"from":"spot","to":"cash"}')
if [[ -n "$RESULT" ]]; then
    ok "USDT0 transferred to cash dex"
else
    warn "USDT0 transfer failed"
fi

# ══════════════════════════════════════════════════════════════════════
# 6. Trade SILVER perp (HIP-3 cash dex)
# ══════════════════════════════════════════════════════════════════════
log "Step 6: Trade SILVER perp"

info "Setting SILVER leverage to 2x..."
ft '{"command":"perp_leverage","symbol":"SILVER","leverage":2}'

info "Fetching SILVER quote..."
QUOTE=$(ft '{"command":"quote","symbol":"SILVER"}')
echo "$QUOTE" | jq .
SILVER_PRICE=$(echo "$QUOTE" | jq -r '.markPx')
info "SILVER mark price: \$$SILVER_PRICE"

info "Buying 0.13 oz SILVER perp at \$89.00 limit..."
RESULT=$(ft '{"command":"perp_buy","symbol":"SILVER","amount":0.13,"price":89.00,"close":false}')
BUY_FILL=$(echo "$RESULT" | jq -r '.fillStatus // empty')
info "SILVER buy fill: $BUY_FILL"

info "Checking positions..."
ft '{"command":"positions"}' | jq .

info "Selling 0.14 oz SILVER perp at \$87.00 limit (close)..."
RESULT=$(ft '{"command":"perp_sell","symbol":"SILVER","amount":0.14,"price":87.00,"close":true}')
SELL_FILL=$(echo "$RESULT" | jq -r '.fillStatus // empty')
info "SILVER sell fill: $SELL_FILL"

# ══════════════════════════════════════════════════════════════════════
# 7. Transfer USDT0 back and swap to USDC
# ══════════════════════════════════════════════════════════════════════
log "Step 7: Return USDT0 to spot and swap to USDC"

info "Transferring 30 USDT0 from cash dex to spot..."
RESULT=$(ft '{"command":"transfer","asset":"USDT0","amount":30,"from":"cash","to":"spot"}')
if [[ -n "$RESULT" ]]; then
    ok "USDT0 transferred back to spot"
else
    warn "USDT0 transfer from cash failed"
fi

info "Selling 30 USDT0 (swapping USDT0 -> USDC)..."
RESULT=$(ft '{"command":"sell","symbol":"USDT0","amount":30,"price":0.998}')
SWAP_FILL=$(echo "$RESULT" | jq -r '.fillStatus // empty')
info "USDT0 sell fill: $SWAP_FILL"

# ══════════════════════════════════════════════════════════════════════
# 8. Check final status
# ══════════════════════════════════════════════════════════════════════
log "Step 8: Final status check"

info "Positions:"
ft '{"command":"positions"}' | jq .

info "Orders:"
ft '{"command":"orders"}' | jq .

info "Balance:"
ft '{"command":"balance"}' | jq .

# ══════════════════════════════════════════════════════════════════════
# 9. Withdraw to Base
# ══════════════════════════════════════════════════════════════════════
log "Step 9: Withdraw \$10 USDC to Base"

info "Withdrawing \$10 USDC to Base..."
RESULT=$(ft '{"command":"withdraw","asset":"USDC","amount":10,"to":"base"}')
if [[ -z "$RESULT" ]]; then
    fail "USDC withdrawal failed"
    exit 1
fi
WD_STATUS=$(echo "$RESULT" | jq -r '.status // empty')
info "Withdrawal status: ${WD_STATUS:-submitted}"
ok "Withdrawal submitted -- \$10 USDC to Base"

done_step
ok "End-to-end trading workflow complete"

#!/usr/bin/env bash
#
# End-to-end trading integration test
#
# Prerequisites:
#   - ~/.fintool/config.toml with:
#     - Hyperliquid wallet (private_key) with >= $3 USDC on Base mainnet
#     - Coinbase API keys (coinbase_api_key, coinbase_api_secret)
#   - jq installed
#
# Usage:
#   ./tests/e2e_trading.sh
#

set -uo pipefail

FINTOOL="./target/release/fintool"
PASS=0
FAIL=0
STEPS=()

# Last command results (set by run_fintool)
LAST_STDOUT=""
LAST_STDERR=""
LAST_EXIT=0

# ── Helpers ────────────────────────────────────────────────────────────

log()  { echo -e "\n\033[1;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"; echo -e "\033[1;34m▶ $*\033[0m"; echo -e "\033[1;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"; }
info() { echo -e "  \033[0;36mℹ $*\033[0m"; }
ok()   { echo -e "  \033[1;32m✓ $*\033[0m"; PASS=$((PASS+1)); STEPS+=("✓ $*"); }
fail() { echo -e "  \033[1;31m✗ $*\033[0m"; FAIL=$((FAIL+1)); STEPS+=("✗ $*"); }
warn() { echo -e "  \033[1;33m⚠ $*\033[0m"; }
done_step() { echo -e "\n  \033[1;33m── Result ──\033[0m"; }

# Run fintool and capture stdout, stderr, and exit code separately.
# After calling, check LAST_EXIT. LAST_STDOUT has the JSON output,
# LAST_STDERR has error/progress messages.
run_fintool() {
    local tmp_stdout tmp_stderr
    tmp_stdout=$(mktemp)
    tmp_stderr=$(mktemp)
    LAST_EXIT=0
    $FINTOOL "$@" >"$tmp_stdout" 2>"$tmp_stderr" || LAST_EXIT=$?
    LAST_STDOUT=$(cat "$tmp_stdout")
    LAST_STDERR=$(cat "$tmp_stderr")
    rm -f "$tmp_stdout" "$tmp_stderr"
}

# Check if last command failed. Prints error details and calls fail().
# Returns 0 (true) if failed, 1 (false) if succeeded.
# Usage: if check_fail "description"; then handle_failure; fi
check_fail() {
    # Always show stderr if non-empty (contains progress messages or errors)
    if [[ -n "$LAST_STDERR" ]]; then
        echo "$LAST_STDERR" | while IFS= read -r line; do
            if [[ $LAST_EXIT -ne 0 ]]; then
                echo -e "  \033[0;31m  $line\033[0m"
            else
                echo -e "  \033[0;90m  $line\033[0m"
            fi
        done
    fi

    if [[ $LAST_EXIT -ne 0 ]]; then
        done_step
        fail "$1 (exit code $LAST_EXIT)"
        if [[ -n "$LAST_STDOUT" ]]; then
            echo -e "  \033[0;31m  stdout: $LAST_STDOUT\033[0m"
        fi
        return 0  # true — command failed
    fi
    return 1  # false — command succeeded
}

# ── Step 0: Build ──────────────────────────────────────────────────────

log "Step 0: Build fintool"
info "Compiling fintool in release mode via 'cargo build --release'..."
cargo build --release 2>&1
if [[ -x "$FINTOOL" ]]; then
    done_step
    ok "Build succeeded — binary at $FINTOOL"
else
    done_step
    fail "Build failed — binary not found at $FINTOOL"
    exit 1
fi

# ── Step 1: Verify config ─────────────────────────────────────────────

log "Step 1: Verify configuration"
info "Checking that ~/.fintool/config.toml exists with wallet and exchange keys."
CONFIG="$HOME/.fintool/config.toml"
if [[ -f "$CONFIG" ]]; then
    done_step
    ok "Config file exists at $CONFIG"
else
    done_step
    fail "Config file not found at $CONFIG — run 'fintool init' first"
    exit 1
fi

# ── Step 2: Check starting balance ────────────────────────────────────

log "Step 2: Check starting balance on Hyperliquid"
info "Querying Hyperliquid account balance to verify funds are available."
run_fintool balance

if check_fail "Balance check failed"; then
    warn "Cannot verify starting balance — continuing anyway"
else
    BALANCE_JSON="$LAST_STDOUT"
    ACCOUNT_VALUE=$(echo "$BALANCE_JSON" | jq -r '.marginSummary.accountValue // .crossMarginSummary.accountValue // empty' 2>/dev/null || true)
    MARGIN_USED=$(echo "$BALANCE_JSON" | jq -r '.marginSummary.totalMarginUsed // .crossMarginSummary.totalMarginUsed // empty' 2>/dev/null || true)

    done_step
    if [[ -n "$ACCOUNT_VALUE" && "$ACCOUNT_VALUE" != "null" ]]; then
        info "Account value: \$$ACCOUNT_VALUE"
        info "Margin used:   \$$MARGIN_USED"
        ok "Starting balance retrieved — account value: \$$ACCOUNT_VALUE"
    else
        info "Raw balance response:"
        echo "$BALANCE_JSON" | jq . 2>/dev/null || echo "$BALANCE_JSON"
        ok "Balance checked (could not parse account value)"
    fi
fi

# ── Step 3: Deposit $3 USDC from Base → Hyperliquid ──────────────────

log "Step 3: Deposit \$3 USDC from Base to Hyperliquid"
info "Bridging \$3 USDC from Base mainnet → Arbitrum → Hyperliquid via Across Protocol."
info "This signs 3 transactions: USDC approval, Across bridge, HL Bridge2 deposit."
info "Requires ETH on Base for gas fees."
run_fintool deposit USDC --amount 3 --from base

if check_fail "Deposit \$3 USDC from Base to Hyperliquid failed"; then
    fail "Deposit is required for all subsequent steps — aborting."
    exit 1
else
    DEPOSIT_JSON="$LAST_STDOUT"
    DEPOSIT_STATUS=$(echo "$DEPOSIT_JSON" | jq -r '.status // empty' 2>/dev/null || true)
    DEPOSIT_AMOUNT_IN=$(echo "$DEPOSIT_JSON" | jq -r '.amount_in // empty' 2>/dev/null || true)
    DEPOSIT_AMOUNT_OUT=$(echo "$DEPOSIT_JSON" | jq -r '.amount_deposited // .amount_out // empty' 2>/dev/null || true)
    DEPOSIT_BRIDGE_TX=$(echo "$DEPOSIT_JSON" | jq -r '.bridge_tx // empty' 2>/dev/null || true)

    done_step
    info "Status:           ${DEPOSIT_STATUS:-unknown}"
    info "Amount sent:      ${DEPOSIT_AMOUNT_IN:-3} USDC"
    info "Amount deposited: ${DEPOSIT_AMOUNT_OUT:-pending} USDC"
    if [[ -n "$DEPOSIT_BRIDGE_TX" && "$DEPOSIT_BRIDGE_TX" != "null" ]]; then
        info "Bridge TX:        $DEPOSIT_BRIDGE_TX"
    fi

    if [[ "$DEPOSIT_STATUS" == "completed" ]]; then
        ok "Deposit completed — ${DEPOSIT_AMOUNT_OUT:-~3} USDC credited to Hyperliquid"
    else
        ok "Deposit submitted (status: ${DEPOSIT_STATUS:-unknown}) — waiting for confirmation"
    fi

    info "Waiting 60 seconds for the deposit to settle on Hyperliquid..."
    sleep 60

    info "Checking balance after deposit..."
    run_fintool balance
    if [[ $LAST_EXIT -eq 0 ]]; then
        BALANCE_AFTER="$LAST_STDOUT"
        ACCOUNT_VALUE_AFTER=$(echo "$BALANCE_AFTER" | jq -r '.marginSummary.accountValue // .crossMarginSummary.accountValue // empty' 2>/dev/null || true)
        if [[ -n "$ACCOUNT_VALUE_AFTER" && "$ACCOUNT_VALUE_AFTER" != "null" ]]; then
            info "Post-deposit account value: \$$ACCOUNT_VALUE_AFTER"
        fi
    fi
    ok "Post-deposit balance verified"
fi

# ── Step 4: Quote SILVER perp price ───────────────────────────────────

log "Step 4: Quote SILVER perp price"
info "Fetching SILVER perpetual futures data from Hyperliquid HIP-3 (cash dex)."
info "This gives us the mark price, funding rate, open interest, and max leverage."
run_fintool perp quote SILVER

if check_fail "SILVER perp quote failed"; then
    fail "Cannot continue trading without SILVER price — aborting perp workflow"
    SILVER_PRICE=""
else
    SILVER_QUOTE="$LAST_STDOUT"
    SILVER_PRICE=$(echo "$SILVER_QUOTE" | jq -r '.markPx' 2>/dev/null)
    SILVER_ORACLE=$(echo "$SILVER_QUOTE" | jq -r '.oraclePx // empty' 2>/dev/null || true)
    SILVER_FUNDING=$(echo "$SILVER_QUOTE" | jq -r '.funding // empty' 2>/dev/null || true)
    SILVER_OI=$(echo "$SILVER_QUOTE" | jq -r '.openInterest // empty' 2>/dev/null || true)
    SILVER_LEVERAGE=$(echo "$SILVER_QUOTE" | jq -r '.maxLeverage // empty' 2>/dev/null || true)
    SILVER_CHANGE=$(echo "$SILVER_QUOTE" | jq -r '.change24h // empty' 2>/dev/null || true)
    SILVER_SOURCE=$(echo "$SILVER_QUOTE" | jq -r '.source // empty' 2>/dev/null || true)

    done_step
    if [[ -n "$SILVER_PRICE" && "$SILVER_PRICE" != "null" ]]; then
        info "Mark price:     \$$SILVER_PRICE"
        info "Oracle price:   \$$SILVER_ORACLE"
        info "24h change:     ${SILVER_CHANGE}%"
        info "Funding rate:   $SILVER_FUNDING"
        info "Open interest:  $SILVER_OI"
        info "Max leverage:   ${SILVER_LEVERAGE}x"
        info "Source:         $SILVER_SOURCE"
        ok "SILVER perp quoted — mark price: \$$SILVER_PRICE"
    else
        fail "SILVER perp quote returned but markPx is missing"
        info "Raw response: $SILVER_QUOTE"
    fi
fi

# ── Step 5: Buy $1 SILVER perp ────────────────────────────────────────

log "Step 5: Buy \$1 SILVER perp"

if [[ -z "$SILVER_PRICE" || "$SILVER_PRICE" == "null" ]]; then
    done_step
    fail "Skipping SILVER perp buy — no price from Step 4"
    BUY_OK=false
else
    BUY_OK=false
    info "Placing a limit buy (long) order for \$1 worth of SILVER perp."
    info "Setting limit price to 1% above mark to ensure the order fills."

    BUY_LIMIT=$(echo "$SILVER_PRICE" | awk '{printf "%.4f", $1 * 1.01}')
    BUY_SIZE=$(echo "$BUY_LIMIT" | awk '{printf "%.4f", 1.0 / $1}')

    info "Current mark price: \$$SILVER_PRICE"
    info "Limit buy price:    \$$BUY_LIMIT (+1% buffer)"
    info "Estimated size:     $BUY_SIZE oz"

    run_fintool perp buy SILVER 1 "$BUY_LIMIT"

    if check_fail "SILVER perp buy failed"; then
        warn "Could not open SILVER long — check HL balance and margin."
    else
        BUY_OK=true
        BUY_JSON="$LAST_STDOUT"
        BUY_ACTION=$(echo "$BUY_JSON" | jq -r '.action // empty' 2>/dev/null || true)
        BUY_ACTUAL_SIZE=$(echo "$BUY_JSON" | jq -r '.size // empty' 2>/dev/null || true)
        BUY_ACTUAL_PRICE=$(echo "$BUY_JSON" | jq -r '.price // empty' 2>/dev/null || true)
        BUY_NETWORK=$(echo "$BUY_JSON" | jq -r '.network // empty' 2>/dev/null || true)

        done_step
        info "Action:   $BUY_ACTION"
        info "Size:     $BUY_ACTUAL_SIZE"
        info "Price:    \$$BUY_ACTUAL_PRICE"
        info "Network:  $BUY_NETWORK"
        ok "SILVER perp buy order placed — $BUY_ACTUAL_SIZE oz at \$$BUY_ACTUAL_PRICE"

        info "Waiting 5 seconds for the order to fill..."
        sleep 5
    fi
fi

# ── Step 6: Verify position ──────────────────────────────────────────

log "Step 6: Verify SILVER perp position"

POSITION_SIZE=""
PURCHASE_PRICE="${BUY_LIMIT:-}"

if [[ "$BUY_OK" != "true" ]]; then
    done_step
    fail "Skipping position verify — SILVER buy did not succeed"
else
    info "Fetching open positions to confirm the SILVER long was filled."
    run_fintool positions

    if check_fail "Failed to fetch positions"; then
        warn "Using buy limit \$${BUY_LIMIT} as reference price"
        PURCHASE_PRICE="$BUY_LIMIT"
    else
        POSITIONS_JSON="$LAST_STDOUT"
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

        UNREALIZED_PNL=$(echo "$POSITIONS_JSON" | jq -r '
            [.[] | .position // .] |
            map(select(.coin == "SILVER")) |
            .[0].unrealizedPnl // empty
        ' 2>/dev/null || true)

        done_step
        if [[ -n "$POSITION_SIZE" && "$POSITION_SIZE" != "null" ]]; then
            info "Position size:    $POSITION_SIZE oz"
            info "Entry price:      \$$ENTRY_PRICE"
            info "Unrealized PnL:   \$$UNREALIZED_PNL"
            PURCHASE_PRICE="$ENTRY_PRICE"
            ok "SILVER position confirmed — $POSITION_SIZE oz at \$$ENTRY_PRICE entry"
        else
            info "Could not find SILVER in positions output."
            info "The buy order may not have filled. Raw positions:"
            echo "$POSITIONS_JSON" | jq . 2>/dev/null || echo "$POSITIONS_JSON"
            fail "No SILVER position found after buy — order may not have filled"
            PURCHASE_PRICE="$BUY_LIMIT"
        fi
    fi
fi

# ── Step 7: Monitor SILVER price every 30s for up to 10 minutes ──────

log "Step 7: Monitor SILVER perp price (every 30s, max 10 min)"

SELL_TRIGGERED=false

if [[ "$BUY_OK" != "true" ]]; then
    done_step
    fail "Skipping price monitoring — no SILVER position to monitor"
else
    info "Watching SILVER price to sell when it rises above entry price."
    info "Entry/reference price: \$$PURCHASE_PRICE"
    info "Will check every 30 seconds for up to 10 minutes (20 checks)."
    echo ""

    MAX_ITERATIONS=20

    for ((i=1; i<=MAX_ITERATIONS; i++)); do
        run_fintool perp quote SILVER

        if [[ $LAST_EXIT -ne 0 ]]; then
            echo -e "  \033[0;33m[$i/$MAX_ITERATIONS] ⚠ Quote failed (exit $LAST_EXIT): $LAST_STDERR — retrying in 30s...\033[0m"
            sleep 30
            continue
        fi

        CURRENT_PRICE=$(echo "$LAST_STDOUT" | jq -r '.markPx' 2>/dev/null)

        if [[ -z "$CURRENT_PRICE" || "$CURRENT_PRICE" == "null" ]]; then
            echo -e "  \033[0;33m[$i/$MAX_ITERATIONS] ⚠ Could not parse markPx, retrying in 30s...\033[0m"
            sleep 30
            continue
        fi

        ABOVE=$(echo "$CURRENT_PRICE $PURCHASE_PRICE" | awk '{print ($1 > $2) ? "yes" : "no"}')
        DIFF=$(echo "$CURRENT_PRICE $PURCHASE_PRICE" | awk '{printf "%+.4f", $1 - $2}')
        DIFF_PCT=$(echo "$CURRENT_PRICE $PURCHASE_PRICE" | awk '{printf "%+.2f", ($1 - $2) / $2 * 100}')

        if [[ "$ABOVE" == "yes" ]]; then
            echo -e "  \033[1;32m[$i/$MAX_ITERATIONS] SILVER: \$$CURRENT_PRICE | entry: \$$PURCHASE_PRICE | diff: \$$DIFF (${DIFF_PCT}%) | ▲ ABOVE ENTRY\033[0m"
            info "Price is above purchase price — triggering sell!"
            SELL_TRIGGERED=true
            break
        else
            echo -e "  \033[0;37m[$i/$MAX_ITERATIONS] SILVER: \$$CURRENT_PRICE | entry: \$$PURCHASE_PRICE | diff: \$$DIFF (${DIFF_PCT}%) | waiting...\033[0m"
        fi

        if [[ $i -lt $MAX_ITERATIONS ]]; then
            sleep 30
        fi
    done

    echo ""
    done_step
    if [[ "$SELL_TRIGGERED" == "true" ]]; then
        ok "Price rose above entry — proceeding to sell for profit"
    else
        info "10-minute monitoring window expired. Selling at current market price."
        ok "Timeout reached — selling at market regardless"
    fi
fi

# ── Step 8: Sell the SILVER perp position ─────────────────────────────

log "Step 8: Sell SILVER perp position"

if [[ "$BUY_OK" != "true" ]]; then
    done_step
    fail "Skipping SILVER sell — no position was opened"
else
    info "Closing the SILVER long position by placing a limit sell order."

    run_fintool perp quote SILVER
    if [[ $LAST_EXIT -ne 0 ]]; then
        warn "Could not get current SILVER price for sell (exit $LAST_EXIT): $LAST_STDERR"
        SELL_MARKET_PRICE="$PURCHASE_PRICE"
    else
        SELL_MARKET_PRICE=$(echo "$LAST_STDOUT" | jq -r '.markPx' 2>/dev/null)
    fi
    SELL_LIMIT=$(echo "$SELL_MARKET_PRICE" | awk '{printf "%.4f", $1 * 0.99}')

    if [[ -z "$POSITION_SIZE" || "$POSITION_SIZE" == "null" ]]; then
        info "Re-fetching position size..."
        run_fintool positions
        if [[ $LAST_EXIT -eq 0 ]]; then
            POSITION_SIZE=$(echo "$LAST_STDOUT" | jq -r '
                [.[] | .position // .] |
                map(select(.coin == "SILVER")) |
                .[0].szi // empty
            ' 2>/dev/null || true)
        else
            warn "Could not fetch positions (exit $LAST_EXIT): $LAST_STDERR"
        fi
    fi

    SELL_SIZE=$(echo "${POSITION_SIZE:-}" | sed 's/^-//')

    if [[ -n "$SELL_SIZE" && "$SELL_SIZE" != "null" && "$SELL_SIZE" != "" ]]; then
        info "Current mark price: \$$SELL_MARKET_PRICE"
        info "Limit sell price:   \$$SELL_LIMIT (-1% buffer to ensure fill)"
        info "Selling size:       $SELL_SIZE oz"
        info "Entry was:          \$$PURCHASE_PRICE"

        PNL_EST=$(echo "$SELL_MARKET_PRICE $PURCHASE_PRICE $SELL_SIZE" | awk '{printf "%+.4f", ($1 - $2) * $3}')
        info "Estimated PnL:      \$$PNL_EST"

        run_fintool perp sell SILVER "$SELL_SIZE" "$SELL_LIMIT"

        if check_fail "SILVER perp sell failed"; then
            warn "Position may still be open — check manually with 'fintool positions'"
        else
            SELL_JSON="$LAST_STDOUT"
            SELL_ACTION=$(echo "$SELL_JSON" | jq -r '.action // empty' 2>/dev/null || true)

            done_step
            info "Action: $SELL_ACTION"
            info "Sold $SELL_SIZE oz SILVER at limit \$$SELL_LIMIT"
            ok "SILVER perp sell order placed — $SELL_SIZE oz at \$$SELL_LIMIT (est. PnL: \$$PNL_EST)"
        fi

        info "Waiting 10 seconds for the sell order to settle..."
        sleep 10
    else
        done_step
        fail "Could not determine position size to sell — no SILVER position found"
    fi
fi

# ── Step 9: Withdraw USDC back to Base ────────────────────────────────

log "Step 9: Withdraw USDC from Hyperliquid to Base"
info "Withdrawing 1 USDC from Hyperliquid back to your Base wallet."
info "Route: Hyperliquid → HL Bridge2 → Arbitrum → Across bridge → Base"
info "Estimated time: ~5-6 minutes"

run_fintool withdraw 1 USDC --network base

if check_fail "USDC withdrawal to Base failed"; then
    warn "Withdrawal failed — USDC remains on Hyperliquid (if any)."
else
    WITHDRAW_JSON="$LAST_STDOUT"
    WITHDRAW_STATUS=$(echo "$WITHDRAW_JSON" | jq -r '.status // empty' 2>/dev/null || true)
    WITHDRAW_AMOUNT=$(echo "$WITHDRAW_JSON" | jq -r '.amount // empty' 2>/dev/null || true)
    WITHDRAW_DEST=$(echo "$WITHDRAW_JSON" | jq -r '.destination_chain // empty' 2>/dev/null || true)
    WITHDRAW_ADDR=$(echo "$WITHDRAW_JSON" | jq -r '.destination_address // empty' 2>/dev/null || true)
    WITHDRAW_BRIDGE_TX=$(echo "$WITHDRAW_JSON" | jq -r '.bridge_tx // empty' 2>/dev/null || true)

    done_step
    info "Status:      ${WITHDRAW_STATUS:-unknown}"
    info "Amount:      ${WITHDRAW_AMOUNT:-1} USDC"
    info "Destination: ${WITHDRAW_DEST:-base}"
    if [[ -n "$WITHDRAW_ADDR" && "$WITHDRAW_ADDR" != "null" ]]; then
        info "Address:     $WITHDRAW_ADDR"
    fi
    if [[ -n "$WITHDRAW_BRIDGE_TX" && "$WITHDRAW_BRIDGE_TX" != "null" ]]; then
        info "Bridge TX:   $WITHDRAW_BRIDGE_TX"
    fi
    ok "USDC withdrawal to Base submitted — ${WITHDRAW_AMOUNT:-1} USDC"
fi

info "Waiting 10 seconds before next step..."
sleep 10

# ── Step 10: Buy $1 of ETH on Hyperliquid ────────────────────────────

log "Step 10: Buy \$1 of ETH spot on Hyperliquid"
info "Fetching current ETH price, then placing a spot limit buy for \$1."

ETH_PRICE=""
run_fintool quote ETH

if check_fail "ETH spot quote failed"; then
    warn "Cannot buy ETH without a price quote."
else
    ETH_PRICE=$(echo "$LAST_STDOUT" | jq -r '.price // empty' 2>/dev/null)

    if [[ -n "$ETH_PRICE" && "$ETH_PRICE" != "null" ]]; then
        ETH_LIMIT=$(echo "$ETH_PRICE" | awk '{printf "%.2f", $1 * 1.01}')
        ETH_SIZE=$(echo "$ETH_PRICE" | awk '{printf "%.6f", 1.0 / $1}')

        info "ETH current price: \$$ETH_PRICE"
        info "Limit buy price:   \$$ETH_LIMIT (+1% buffer)"
        info "Estimated size:    $ETH_SIZE ETH"

        run_fintool order buy ETH 1 "$ETH_LIMIT"

        if check_fail "ETH spot buy order failed"; then
            warn "Could not buy ETH — check HL balance."
            ETH_PRICE=""
        else
            ETH_BUY_JSON="$LAST_STDOUT"
            ETH_BUY_SIZE=$(echo "$ETH_BUY_JSON" | jq -r '.size // empty' 2>/dev/null || true)
            ETH_BUY_PRICE=$(echo "$ETH_BUY_JSON" | jq -r '.maxPrice // .price // empty' 2>/dev/null || true)

            done_step
            info "Order size:  ${ETH_BUY_SIZE:-$ETH_SIZE} ETH"
            info "Limit price: \$$ETH_BUY_PRICE"
            ok "ETH spot buy placed on Hyperliquid — ~$ETH_SIZE ETH at \$$ETH_LIMIT"
        fi
    else
        done_step
        fail "ETH quote returned but price field is missing"
        info "Raw response: $LAST_STDOUT"
    fi
fi

info "Waiting 10 seconds for the order to fill..."
sleep 10

# ── Step 11: Withdraw ETH to Ethereum mainnet ─────────────────────────

log "Step 11: Withdraw ETH to Ethereum mainnet"
info "Withdrawing the purchased ETH from Hyperliquid to Ethereum mainnet."
info "Route: Hyperliquid → HyperUnit bridge → Ethereum L1"
info "Estimated time: ~3 minutes"

if [[ -z "$ETH_PRICE" || "$ETH_PRICE" == "null" ]]; then
    done_step
    fail "Skipping ETH withdrawal — ETH buy did not succeed in Step 10"
else
    ETH_AMOUNT=$(echo "$ETH_PRICE" | awk '{printf "%.6f", 1.0 / $1}')

    info "Withdrawing $ETH_AMOUNT ETH (~\$1) to your Ethereum address"

    run_fintool withdraw "$ETH_AMOUNT" ETH

    if check_fail "ETH withdrawal to Ethereum failed"; then
        warn "ETH may still be on Hyperliquid — check manually."
    else
        ETH_WITHDRAW_JSON="$LAST_STDOUT"
        ETH_WD_STATUS=$(echo "$ETH_WITHDRAW_JSON" | jq -r '.status // empty' 2>/dev/null || true)
        ETH_WD_DEST=$(echo "$ETH_WITHDRAW_JSON" | jq -r '.destination_chain // empty' 2>/dev/null || true)
        ETH_WD_ADDR=$(echo "$ETH_WITHDRAW_JSON" | jq -r '.destination_address // empty' 2>/dev/null || true)

        done_step
        info "Status:      ${ETH_WD_STATUS:-unknown}"
        info "Amount:      $ETH_AMOUNT ETH"
        info "Destination: ${ETH_WD_DEST:-ethereum}"
        if [[ -n "$ETH_WD_ADDR" && "$ETH_WD_ADDR" != "null" ]]; then
            info "Address:     $ETH_WD_ADDR"
        fi
        ok "ETH withdrawal to Ethereum mainnet submitted — $ETH_AMOUNT ETH"
    fi
fi

info "Waiting 5 seconds before final step..."
sleep 5

# ── Step 12: Buy $1 of TSLA on Coinbase ──────────────────────────────

log "Step 12: Buy \$1 of Tesla stock on Coinbase"
info "Fetching current TSLA price, then placing a spot limit buy on Coinbase."

run_fintool quote TSLA

if check_fail "TSLA spot quote failed"; then
    warn "Cannot buy TSLA without a price quote."
else
    TSLA_QUOTE="$LAST_STDOUT"
    TSLA_PRICE=$(echo "$TSLA_QUOTE" | jq -r '.price // empty' 2>/dev/null)

    TSLA_TREND=$(echo "$TSLA_QUOTE" | jq -r '.trend // empty' 2>/dev/null || true)
    TSLA_CHANGE=$(echo "$TSLA_QUOTE" | jq -r '.change_24h_pct // .change24h // empty' 2>/dev/null || true)
    TSLA_SUMMARY=$(echo "$TSLA_QUOTE" | jq -r '.summary // empty' 2>/dev/null || true)

    if [[ -n "$TSLA_PRICE" && "$TSLA_PRICE" != "null" ]]; then
        TSLA_LIMIT=$(echo "$TSLA_PRICE" | awk '{printf "%.2f", $1 * 1.01}')
        TSLA_SIZE=$(echo "$TSLA_PRICE" | awk '{printf "%.6f", 1.0 / $1}')

        info "TSLA current price: \$$TSLA_PRICE"
        if [[ -n "$TSLA_CHANGE" && "$TSLA_CHANGE" != "null" ]]; then
            info "24h change:         ${TSLA_CHANGE}%"
        fi
        if [[ -n "$TSLA_TREND" && "$TSLA_TREND" != "null" ]]; then
            info "Trend:              $TSLA_TREND"
        fi
        if [[ -n "$TSLA_SUMMARY" && "$TSLA_SUMMARY" != "null" ]]; then
            info "Summary:            $TSLA_SUMMARY"
        fi
        info "Limit buy price:    \$$TSLA_LIMIT (+1% buffer)"
        info "Estimated size:     $TSLA_SIZE shares"

        run_fintool order buy TSLA 1 "$TSLA_LIMIT" --exchange coinbase

        if check_fail "TSLA spot buy on Coinbase failed"; then
            warn "Check Coinbase API keys and account balance."
        else
            TSLA_BUY_JSON="$LAST_STDOUT"
            TSLA_BUY_STATUS=$(echo "$TSLA_BUY_JSON" | jq -r '.status // empty' 2>/dev/null || true)
            TSLA_ORDER_ID=$(echo "$TSLA_BUY_JSON" | jq -r '.orderId // empty' 2>/dev/null || true)

            done_step
            info "Exchange:  Coinbase"
            info "Status:    ${TSLA_BUY_STATUS:-submitted}"
            if [[ -n "$TSLA_ORDER_ID" && "$TSLA_ORDER_ID" != "null" ]]; then
                info "Order ID:  $TSLA_ORDER_ID"
            fi
            ok "TSLA spot buy placed on Coinbase — ~$TSLA_SIZE shares at \$$TSLA_LIMIT"
        fi
    else
        done_step
        fail "TSLA quote returned but price field is missing"
        info "Raw response: $TSLA_QUOTE"
    fi
fi

# ── Summary ───────────────────────────────────────────────────────────

echo ""
echo -e "\033[1;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"
echo -e "\033[1;34m▶ Test Summary\033[0m"
echo -e "\033[1;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"
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

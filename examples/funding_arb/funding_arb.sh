#!/usr/bin/env bash
#
# Funding Rate Arbitrage Bot
#
# Strategy: Buy spot + short perp on the asset with the highest positive
# funding rate among liquid overlapping pairs. Collect hourly funding.
# If funding turns negative, unwind and wait for the next opportunity.
#
# Usage: ./tests/funding_arb.sh [--dry-run] [--interval 3600]
#
# Requires: fintool CLI, jq, curl, OPENAI_API_KEY env var
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$REPO_DIR/tests/helpers.sh"
ensure_built

# ── Config ─────────────────────────────────────────────────────────────

DRY_RUN=false
CHECK_INTERVAL=3600  # 1 hour (matches Hyperliquid funding interval)
SLIPPAGE_BPS=50      # 0.5% slippage tolerance for limit orders
MIN_FUNDING=0.0001   # Minimum funding rate to enter (0.01% per hour)
MIN_VOLUME=1000000   # Minimum 24h perp volume in USD
LEVERAGE=1           # 1x leverage for perp short (delta neutral)
POSITION_PCT=90      # Use 90% of available USDC (keep 10% buffer)
LOG_FILE="/tmp/funding_arb.log"

# Assets available on both spot and perp (spot ticker -> perp ticker)
# These are the liquid overlapping pairs from our research
declare -A SPOT_TO_PERP=(
    ["HYPE"]="HYPE"
    ["PURR"]="PURR"
    ["TRUMP"]="TRUMP"
    ["PUMP"]="PUMP"
    ["BERA"]="BERA"
    ["MON"]="MON"
    ["ANIME"]="ANIME"
    ["LINK0"]="LINK"
    ["AVAX0"]="AVAX"
    ["AAVE0"]="AAVE"
    ["XMR1"]="XMR"
    ["BNB0"]="BNB"
    ["XRP1"]="XRP"
)

# ── Parse args ─────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run) DRY_RUN=true; shift ;;
        --interval) CHECK_INTERVAL="$2"; shift 2 ;;
        *) echo "Unknown arg: $1"; exit 1 ;;
    esac
done

# ── Logging ────────────────────────────────────────────────────────────

ts() { date '+%Y-%m-%d %H:%M:%S'; }
log_msg() { echo "[$(ts)] $*" | tee -a "$LOG_FILE"; }

# ── Hyperliquid API helpers ────────────────────────────────────────────

# Fetch funding rates and volume for all perps via Hyperliquid API
fetch_all_funding() {
    curl -s https://api.hyperliquid.xyz/info \
        -X POST \
        -H 'Content-Type: application/json' \
        -d '{"type": "metaAndAssetCtxs"}'
}

# Fetch spot orderbook depth for a given spot pair
# Returns best bid/ask and depth
fetch_spot_book() {
    local coin="$1"
    curl -s https://api.hyperliquid.xyz/info \
        -X POST \
        -H 'Content-Type: application/json' \
        -d "{\"type\": \"l2Book\", \"coin\": \"${coin}\"}"
}

# ── OpenAI analysis ───────────────────────────────────────────────────

# Ask OpenAI to analyze candidates and pick the best one
analyze_with_openai() {
    local candidates_json="$1"

    local prompt="You are a quantitative trading analyst. Analyze these Hyperliquid assets for a funding rate arbitrage trade (buy spot + short perp to collect positive funding).

For each candidate, I'm providing: symbol, funding rate (hourly), 24h perp volume, open interest, spot bid/ask spread, and spot depth.

Candidates:
${candidates_json}

Pick the SINGLE best asset to trade. Consider:
1. Funding rate magnitude (higher = more profit)
2. Spot liquidity (tight spread, good depth = lower entry/exit cost)
3. Perp volume and OI (higher = more liquid, easier to short)
4. Risk (avoid very new or volatile meme tokens if possible)

Respond in EXACTLY this JSON format, nothing else:
{\"pick\": \"SYMBOL\", \"reason\": \"one sentence reason\", \"confidence\": \"high|medium|low\"}"

    local response
    response=$(curl -s https://api.openai.com/v1/chat/completions \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer ${OPENAI_API_KEY}" \
        -d "$(jq -n \
            --arg prompt "$prompt" \
            '{
                "model": "gpt-4o-mini",
                "messages": [{"role": "user", "content": $prompt}],
                "temperature": 0.1,
                "max_tokens": 200
            }')")

    echo "$response" | jq -r '.choices[0].message.content' 2>/dev/null
}

# ── Core logic ─────────────────────────────────────────────────────────

# Check if we currently have any positions
get_current_state() {
    run_fintool positions
    local positions="$LAST_STDOUT"

    run_fintool balance
    local balance="$LAST_STDOUT"

    # Check if we have any perp positions
    local has_perp
    has_perp=$(echo "$positions" | jq -r 'if type == "array" then (map(select(.size != "0" and .size != "0.0" and .size != null)) | length) else 0 end' 2>/dev/null || echo "0")

    # Get USDC balance
    local usdc
    usdc=$(echo "$balance" | jq -r '
        if type == "array" then
            map(select(.coin == "USDC" or .asset == "USDC")) | .[0] | (.available // .free // .balance // "0")
        elif type == "object" then
            .USDC // .usdc // .available // "0"
        else "0" end
    ' 2>/dev/null || echo "0")

    echo "{\"has_positions\": $has_perp, \"usdc_available\": \"$usdc\", \"positions\": $positions, \"balance\": $balance}"
}

# Gather funding data for all candidate assets
gather_candidates() {
    local all_data
    all_data=$(fetch_all_funding)

    if [[ -z "$all_data" ]]; then
        log_msg "ERROR: Failed to fetch funding data"
        return 1
    fi

    local candidates="[]"

    for spot_ticker in "${!SPOT_TO_PERP[@]}"; do
        local perp_ticker="${SPOT_TO_PERP[$spot_ticker]}"

        # Extract perp data
        local perp_data
        perp_data=$(echo "$all_data" | python3 -c "
import json, sys
data = json.load(sys.stdin)
universe = data[0]['universe']
ctxs = data[1]
target = '$perp_ticker'
for u, c in zip(universe, ctxs):
    if u['name'] == target:
        print(json.dumps({
            'perp_ticker': target,
            'spot_ticker': '$spot_ticker',
            'funding': float(c['funding']),
            'markPx': float(c['markPx']),
            'volume24h': float(c['dayNtlVlm']),
            'openInterest': float(c['openInterest']) * float(c['markPx'])
        }))
        break
" 2>/dev/null)

        if [[ -z "$perp_data" ]]; then
            continue
        fi

        local funding vol
        funding=$(echo "$perp_data" | jq -r '.funding')
        vol=$(echo "$perp_data" | jq -r '.volume24h')

        # Skip if funding is not positive or volume too low
        if (( $(echo "$funding <= 0" | bc -l) )); then
            continue
        fi
        if (( $(echo "$vol < $MIN_VOLUME" | bc -l) )); then
            continue
        fi

        # Fetch spot book for spread analysis
        local book spread_data
        book=$(fetch_spot_book "@${spot_ticker}")
        # If spot ticker doesn't have @, try the raw name
        if echo "$book" | jq -e '.levels' >/dev/null 2>&1; then
            spread_data=$(echo "$book" | python3 -c "
import json, sys
data = json.load(sys.stdin)
bids = data.get('levels', [[],[]])[0]
asks = data.get('levels', [[],[]])[1]
if bids and asks:
    best_bid = float(bids[0]['px'])
    best_ask = float(asks[0]['px'])
    spread_pct = (best_ask - best_bid) / best_bid * 100
    bid_depth = sum(float(b['sz']) * float(b['px']) for b in bids[:5])
    ask_depth = sum(float(a['sz']) * float(a['px']) for a in asks[:5])
    print(json.dumps({'spread_pct': round(spread_pct, 4), 'bid_depth_usd': round(bid_depth, 2), 'ask_depth_usd': round(ask_depth, 2)}))
else:
    print(json.dumps({'spread_pct': 99, 'bid_depth_usd': 0, 'ask_depth_usd': 0}))
" 2>/dev/null)
        else
            spread_data='{"spread_pct": 99, "bid_depth_usd": 0, "ask_depth_usd": 0}'
        fi

        # Merge perp + spot data
        local merged
        merged=$(echo "$perp_data" | jq --argjson spread "$spread_data" '. + $spread')

        candidates=$(echo "$candidates" | jq --argjson c "$merged" '. + [$c]')
    done

    # Sort by funding rate descending
    candidates=$(echo "$candidates" | jq 'sort_by(-.funding)')
    echo "$candidates"
}

# Open a position: buy spot + short perp
open_position() {
    local spot_ticker="$1"
    local perp_ticker="$2"
    local usdc_amount="$3"

    # Split USDC roughly 50/50 between spot and perp margin
    local half
    half=$(echo "$usdc_amount" | awk '{printf "%.2f", $1 / 2}')

    log_msg "Opening position: spot=$spot_ticker perp=$perp_ticker amount=\$${usdc_amount} (${half} each side)"

    # 1. Get current prices
    run_fintool perp quote "$perp_ticker"
    if check_fail "Perp quote for $perp_ticker failed"; then return 1; fi
    local perp_price
    perp_price=$(echo "$LAST_STDOUT" | jq -r '.markPx')

    run_fintool quote "$spot_ticker"
    if check_fail "Spot quote for $spot_ticker failed"; then return 1; fi
    local spot_price
    spot_price=$(echo "$LAST_STDOUT" | jq -r '.price // .markPx')

    # 2. Calculate limit prices with slippage
    local spot_limit perp_limit spot_size
    spot_limit=$(echo "$spot_price" | awk -v s="$SLIPPAGE_BPS" '{printf "%.6f", $1 * (1 + s/10000)}')
    perp_limit=$(echo "$perp_price" | awk -v s="$SLIPPAGE_BPS" '{printf "%.6f", $1 * (1 - s/10000)}')
    spot_size=$(echo "$half $spot_price" | awk '{printf "%.6f", $1 / $2}')

    log_msg "  Spot: buy \$$half of $spot_ticker at limit \$$spot_limit"
    log_msg "  Perp: sell (short) $spot_size $perp_ticker at limit \$$perp_limit"

    if $DRY_RUN; then
        log_msg "  [DRY RUN] Skipping actual trades"
        return 0
    fi

    # 3. Set leverage to 1x for the perp
    run_fintool perp leverage "$perp_ticker" "$LEVERAGE" --cross
    if check_fail "Set leverage failed"; then
        warn "Leverage setting failed, continuing anyway..."
    fi

    # 4. Buy spot
    run_fintool order buy "$spot_ticker" "$half" "$spot_limit"
    if check_fail "Spot buy failed"; then return 1; fi
    local spot_fill
    spot_fill=$(echo "$LAST_STDOUT" | jq -r '.fillStatus // "unknown"')
    log_msg "  Spot buy status: $spot_fill"

    # 5. Short perp (sell without --close opens a short)
    run_fintool perp sell "$perp_ticker" "$spot_size" "$perp_limit"
    if check_fail "Perp short failed"; then return 1; fi
    local perp_fill
    perp_fill=$(echo "$LAST_STDOUT" | jq -r '.fillStatus // "unknown"')
    log_msg "  Perp short status: $perp_fill"

    log_msg "  Position opened successfully"
    return 0
}

# Close all positions: sell spot + close perp short
close_all_positions() {
    log_msg "Closing all positions..."

    run_fintool positions
    if check_fail "Failed to get positions"; then return 1; fi

    local positions="$LAST_STDOUT"

    # Close each perp position
    echo "$positions" | jq -c '.[] | select(.size != "0" and .size != null)' 2>/dev/null | while read -r pos; do
        local symbol size entry_price
        symbol=$(echo "$pos" | jq -r '.coin // .symbol')
        size=$(echo "$pos" | jq -r '.size // .positionSize')
        entry_price=$(echo "$pos" | jq -r '.entryPx // .entryPrice')

        # Determine if long or short based on size sign
        local abs_size is_short
        abs_size=$(echo "$size" | awk '{print ($1 < 0) ? -$1 : $1}')
        is_short=$(echo "$size" | awk '{print ($1 < 0) ? "true" : "false"}')

        if [[ -z "$symbol" || "$symbol" == "null" ]]; then
            continue
        fi

        log_msg "  Closing perp: $symbol size=$size"

        # Get current price for limit
        run_fintool perp quote "$symbol"
        local current_price
        current_price=$(echo "$LAST_STDOUT" | jq -r '.markPx')

        if $DRY_RUN; then
            log_msg "  [DRY RUN] Would close $symbol perp"
            continue
        fi

        if [[ "$is_short" == "true" ]]; then
            # Short position -> buy to close
            local close_limit
            close_limit=$(echo "$current_price" | awk -v s="$SLIPPAGE_BPS" '{printf "%.6f", $1 * (1 + s/10000)}')
            run_fintool perp buy "$symbol" "$(echo "$abs_size $current_price" | awk '{printf "%.2f", $1 * $2}')" "$close_limit" --close
        else
            # Long position -> sell to close
            local close_limit
            close_limit=$(echo "$current_price" | awk -v s="$SLIPPAGE_BPS" '{printf "%.6f", $1 * (1 - s/10000)}')
            run_fintool perp sell "$symbol" "$abs_size" "$close_limit" --close
        fi

        if check_fail "Failed to close perp $symbol"; then
            warn "Could not close $symbol perp — manual intervention may be needed"
        else
            log_msg "  Closed perp $symbol"
        fi
    done

    # Sell all spot holdings (except USDC)
    run_fintool balance
    echo "$LAST_STDOUT" | jq -c '.[] | select(.coin != "USDC" and .asset != "USDC" and (.coin // .asset) != null)' 2>/dev/null | while read -r holding; do
        local coin amount
        coin=$(echo "$holding" | jq -r '.coin // .asset')
        amount=$(echo "$holding" | jq -r '.total // .balance // .available // "0"')

        if [[ -z "$coin" || "$coin" == "null" || "$amount" == "0" ]]; then
            continue
        fi

        log_msg "  Selling spot: $coin amount=$amount"

        # Get price for limit
        run_fintool quote "$coin"
        local price
        price=$(echo "$LAST_STDOUT" | jq -r '.price // .markPx // "0"')

        if [[ "$price" == "0" || "$price" == "null" ]]; then
            warn "Cannot get price for $coin, skipping"
            continue
        fi

        local sell_limit
        sell_limit=$(echo "$price" | awk -v s="$SLIPPAGE_BPS" '{printf "%.6f", $1 * (1 - s/10000)}')

        if $DRY_RUN; then
            log_msg "  [DRY RUN] Would sell $amount $coin at \$$sell_limit"
            continue
        fi

        run_fintool order sell "$coin" "$amount" "$sell_limit"
        if check_fail "Failed to sell $coin spot"; then
            warn "Could not sell $coin — manual intervention may be needed"
        else
            log_msg "  Sold spot $coin"
        fi
    done

    log_msg "All positions closed"
}

# Check if current position's funding has turned negative
check_current_funding() {
    run_fintool positions
    local positions="$LAST_STDOUT"

    # Get the perp symbol we're short on
    local short_symbol
    short_symbol=$(echo "$positions" | jq -r '
        [.[] | select((.size // "0") != "0")] | .[0] | .coin // .symbol // empty
    ' 2>/dev/null)

    if [[ -z "$short_symbol" || "$short_symbol" == "null" ]]; then
        echo "none"
        return
    fi

    # Get current funding for that symbol
    run_fintool perp quote "$short_symbol"
    local funding
    funding=$(echo "$LAST_STDOUT" | jq -r '.funding // "0"')

    log_msg "Current position: $short_symbol | Funding rate: $funding"

    if (( $(echo "$funding < 0" | bc -l) )); then
        echo "negative"
    else
        echo "positive"
    fi
}

# ── Main loop ──────────────────────────────────────────────────────────

main() {
    log_msg "═══════════════════════════════════════════════════"
    log_msg "  Funding Rate Arbitrage Bot"
    log_msg "  Dry run: $DRY_RUN | Interval: ${CHECK_INTERVAL}s"
    log_msg "  Min funding: $MIN_FUNDING | Min volume: \$$MIN_VOLUME"
    log_msg "═══════════════════════════════════════════════════"

    while true; do
        log_msg "────── Check cycle at $(ts) ──────"

        # 1. Get current account state
        local state
        state=$(get_current_state)
        local has_positions usdc
        has_positions=$(echo "$state" | jq -r '.has_positions')
        usdc=$(echo "$state" | jq -r '.usdc_available')

        log_msg "Account: positions=$has_positions USDC=$usdc"

        if [[ "$has_positions" -gt 0 ]]; then
            # ── We have positions: check if funding is still positive ──
            local funding_status
            funding_status=$(check_current_funding)

            if [[ "$funding_status" == "negative" ]]; then
                log_msg "⚠️  Funding turned NEGATIVE — closing all positions"
                close_all_positions
            elif [[ "$funding_status" == "none" ]]; then
                log_msg "⚠️  No perp position found but expected one — resetting"
                close_all_positions
            else
                log_msg "✅ Funding still positive — holding position"
            fi
        else
            # ── No positions: look for opportunities ──
            log_msg "Scanning for funding opportunities..."

            local candidates
            candidates=$(gather_candidates)
            local count
            count=$(echo "$candidates" | jq 'length')

            if [[ "$count" -eq 0 ]]; then
                log_msg "No assets with positive funding above threshold. Waiting..."
            else
                log_msg "Found $count candidates with positive funding:"
                echo "$candidates" | jq -r '.[] | "  \(.perp_ticker): funding=\(.funding) vol=$\(.volume24h|round) spread=\(.spread_pct)% depth=$\(.bid_depth_usd)"'

                # Use OpenAI to pick the best one
                if [[ -n "${OPENAI_API_KEY:-}" ]]; then
                    log_msg "Asking OpenAI to analyze candidates..."
                    local analysis
                    analysis=$(analyze_with_openai "$(echo "$candidates" | jq -c '.')")
                    log_msg "OpenAI analysis: $analysis"

                    local pick
                    pick=$(echo "$analysis" | jq -r '.pick // empty' 2>/dev/null)

                    if [[ -n "$pick" ]]; then
                        # Find the spot ticker for this pick
                        local spot_ticker=""
                        for st in "${!SPOT_TO_PERP[@]}"; do
                            if [[ "${SPOT_TO_PERP[$st]}" == "$pick" ]]; then
                                spot_ticker="$st"
                                break
                            fi
                        done

                        if [[ -n "$spot_ticker" ]]; then
                            local trade_amount
                            trade_amount=$(echo "$usdc" | awk -v p="$POSITION_PCT" '{printf "%.2f", $1 * p / 100}')
                            log_msg "Selected: $pick (spot: $spot_ticker) — deploying \$$trade_amount"
                            open_position "$spot_ticker" "$pick" "$trade_amount"
                        else
                            log_msg "ERROR: Could not find spot ticker for $pick"
                        fi
                    else
                        log_msg "OpenAI did not return a valid pick, skipping this cycle"
                    fi
                else
                    # No OpenAI key — just pick the highest funding rate
                    local top
                    top=$(echo "$candidates" | jq -r '.[0]')
                    local pick_perp pick_spot
                    pick_perp=$(echo "$top" | jq -r '.perp_ticker')
                    pick_spot=$(echo "$top" | jq -r '.spot_ticker')

                    local trade_amount
                    trade_amount=$(echo "$usdc" | awk -v p="$POSITION_PCT" '{printf "%.2f", $1 * p / 100}')
                    log_msg "No OPENAI_API_KEY — picking highest funding: $pick_perp"
                    open_position "$pick_spot" "$pick_perp" "$trade_amount"
                fi
            fi
        fi

        log_msg "Sleeping ${CHECK_INTERVAL}s until next check..."
        sleep "$CHECK_INTERVAL"
    done
}

# ── Entry point ────────────────────────────────────────────────────────

main

#!/usr/bin/env bash
#
# Funding Rate Arbitrage Bot (JSON API)
#
# Same strategy as bot.sh but uses fintool's --json input mode for all calls.
# All fintool output is JSON, parsed with jq.
#
# For the human CLI version, see bot.sh.
#
# Strategy: Buy spot + short perp on the asset with the highest positive
# funding rate among liquid overlapping pairs. Collect hourly funding.
# If funding turns negative, unwind and wait for the next opportunity.
#
# Usage: ./bot_json.sh [--dry-run] [--interval 3600]
#
# Requires: fintool CLI, jq, curl, python3, OPENAI_API_KEY env var
#
set -uo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

# ── Config ─────────────────────────────────────────────────────────────

FINTOOL="${FINTOOL:-${REPO_DIR}/target/release/fintool}"
DRY_RUN=false
CHECK_INTERVAL=3600  # 1 hour (matches Hyperliquid funding interval)
SLIPPAGE_BPS=50      # 0.5% slippage tolerance for limit orders
MIN_FUNDING=0.0001   # Minimum funding rate to enter (0.01% per hour)
MIN_VOLUME=1000000   # Minimum 24h perp volume in USD
LEVERAGE=1           # 1x leverage for perp short (delta neutral)
POSITION_PCT=90      # Use 90% of available USDC (keep 10% buffer)
LOG_FILE="/tmp/funding_arb.log"

# Ensure fintool is built
if [[ ! -x "$FINTOOL" ]]; then
    echo "Building fintool..."
    (cd "$REPO_DIR" && cargo build --release 2>&1)
    if [[ ! -x "$FINTOOL" ]]; then
        echo "ERROR: Build failed — binary not found at $FINTOOL"
        exit 1
    fi
fi

# Assets available on both spot and perp (spot ticker -> perp ticker)
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

# ── Fintool JSON helper ──────────────────────────────────────────────

# Call fintool in JSON mode. Returns JSON output.
ft() {
    $FINTOOL --json "$1" 2>/dev/null
}

# Call fintool in JSON mode, exit on failure
ft_or_fail() {
    local result
    result=$(ft "$1")
    if [[ $? -ne 0 ]] || echo "$result" | jq -e '.error' >/dev/null 2>&1; then
        log_msg "ERROR: fintool --json failed: $result"
        return 1
    fi
    echo "$result"
}

# ── Hyperliquid API helpers (for market data scanning) ───────────────

hl_api() {
    curl -s https://api.hyperliquid.xyz/info \
        -X POST \
        -H 'Content-Type: application/json' \
        -d "$1"
}

fetch_all_funding() {
    hl_api '{"type": "metaAndAssetCtxs"}'
}

fetch_spot_book() {
    local coin="$1"
    hl_api "{\"type\": \"l2Book\", \"coin\": \"${coin}\"}"
}

# ── OpenAI analysis ───────────────────────────────────────────────────

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

get_current_state() {
    local positions balance
    positions=$(ft '{"command":"positions"}')
    balance=$(ft '{"command":"balance"}')

    local has_perp
    has_perp=$(echo "$positions" | jq -r 'if type == "array" then (map(select(.size != "0" and .size != "0.0" and .size != null)) | length) else 0 end' 2>/dev/null || echo "0")

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

        if [[ -z "$perp_data" ]]; then continue; fi

        local funding vol
        funding=$(echo "$perp_data" | jq -r '.funding')
        vol=$(echo "$perp_data" | jq -r '.volume24h')

        if (( $(echo "$funding <= 0" | bc -l) )); then continue; fi
        if (( $(echo "$vol < $MIN_VOLUME" | bc -l) )); then continue; fi

        local book spread_data
        book=$(fetch_spot_book "@${spot_ticker}")
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

        local merged
        merged=$(echo "$perp_data" | jq --argjson spread "$spread_data" '. + $spread')
        candidates=$(echo "$candidates" | jq --argjson c "$merged" '. + [$c]')
    done

    candidates=$(echo "$candidates" | jq 'sort_by(-.funding)')
    echo "$candidates"
}

open_position() {
    local spot_ticker="$1"
    local perp_ticker="$2"
    local usdc_amount="$3"

    local half
    half=$(echo "$usdc_amount" | awk '{printf "%.2f", $1 / 2}')

    log_msg "Opening position: spot=$spot_ticker perp=$perp_ticker amount=\$${usdc_amount} (${half} each side)"

    # 1. Get current prices via fintool JSON
    local perp_quote spot_quote perp_price spot_price
    perp_quote=$(ft "{\"command\":\"perp_quote\",\"symbol\":\"$perp_ticker\"}")
    spot_quote=$(ft "{\"command\":\"quote\",\"symbol\":\"$spot_ticker\"}")

    perp_price=$(echo "$perp_quote" | jq -r '.markPx')
    spot_price=$(echo "$spot_quote" | jq -r '.price // .markPx')

    if [[ -z "$perp_price" || "$perp_price" == "null" || -z "$spot_price" || "$spot_price" == "null" ]]; then
        log_msg "ERROR: Could not fetch prices"
        return 1
    fi

    # 2. Calculate limit prices with slippage
    local spot_limit perp_limit spot_size
    spot_limit=$(echo "$spot_price" | awk -v s="$SLIPPAGE_BPS" '{printf "%.6f", $1 * (1 + s/10000)}')
    perp_limit=$(echo "$perp_price" | awk -v s="$SLIPPAGE_BPS" '{printf "%.6f", $1 * (1 - s/10000)}')
    spot_size=$(echo "$half $spot_price" | awk '{printf "%.6f", $1 / $2}')

    log_msg "  Spot: buy $spot_size $spot_ticker at limit \$$spot_limit"
    log_msg "  Perp: sell (short) $spot_size $perp_ticker at limit \$$perp_limit"

    if $DRY_RUN; then
        log_msg "  [DRY RUN] Skipping actual trades"
        return 0
    fi

    # 3. Set leverage to 1x
    ft "{\"command\":\"perp_leverage\",\"symbol\":\"$perp_ticker\",\"leverage\":$LEVERAGE,\"cross\":true}" | tee -a "$LOG_FILE"

    # 4. Buy spot (amount is in symbol units)
    local spot_result
    spot_result=$(ft "{\"command\":\"order_buy\",\"symbol\":\"$spot_ticker\",\"amount\":\"$spot_size\",\"price\":\"$spot_limit\"}")
    local spot_fill
    spot_fill=$(echo "$spot_result" | jq -r '.fillStatus // "unknown"')
    log_msg "  Spot buy status: $spot_fill"
    echo "$spot_result" | tee -a "$LOG_FILE"

    # 5. Short perp
    local perp_result
    perp_result=$(ft "{\"command\":\"perp_sell\",\"symbol\":\"$perp_ticker\",\"amount\":\"$spot_size\",\"price\":\"$perp_limit\"}")
    local perp_fill
    perp_fill=$(echo "$perp_result" | jq -r '.fillStatus // "unknown"')
    log_msg "  Perp short status: $perp_fill"
    echo "$perp_result" | tee -a "$LOG_FILE"

    log_msg "  Position opened successfully"
    return 0
}

close_all_positions() {
    log_msg "Closing all positions..."

    local positions
    positions=$(ft '{"command":"positions"}')

    # Close each perp position
    echo "$positions" | jq -c '.[] | select(.size != "0" and .size != null)' 2>/dev/null | while read -r pos; do
        local symbol size
        symbol=$(echo "$pos" | jq -r '.coin // .symbol')
        size=$(echo "$pos" | jq -r '.size // .positionSize')

        local abs_size is_short
        abs_size=$(echo "$size" | awk '{print ($1 < 0) ? -$1 : $1}')
        is_short=$(echo "$size" | awk '{print ($1 < 0) ? "true" : "false"}')

        if [[ -z "$symbol" || "$symbol" == "null" ]]; then continue; fi

        log_msg "  Closing perp: $symbol size=$size"

        local current_quote current_price
        current_quote=$(ft "{\"command\":\"perp_quote\",\"symbol\":\"$symbol\"}")
        current_price=$(echo "$current_quote" | jq -r '.markPx')

        if $DRY_RUN; then
            log_msg "  [DRY RUN] Would close $symbol perp"
            continue
        fi

        if [[ "$is_short" == "true" ]]; then
            local close_limit
            close_limit=$(echo "$current_price" | awk -v s="$SLIPPAGE_BPS" '{printf "%.6f", $1 * (1 + s/10000)}')
            ft "{\"command\":\"perp_buy\",\"symbol\":\"$symbol\",\"amount\":\"$abs_size\",\"price\":\"$close_limit\",\"close\":true}" | tee -a "$LOG_FILE"
        else
            local close_limit
            close_limit=$(echo "$current_price" | awk -v s="$SLIPPAGE_BPS" '{printf "%.6f", $1 * (1 - s/10000)}')
            ft "{\"command\":\"perp_sell\",\"symbol\":\"$symbol\",\"amount\":\"$abs_size\",\"price\":\"$close_limit\",\"close\":true}" | tee -a "$LOG_FILE"
        fi

        log_msg "  Closed perp $symbol"
    done

    # Sell all spot holdings (except USDC)
    local balance
    balance=$(ft '{"command":"balance"}')

    echo "$balance" | jq -c '.[] | select(.coin != "USDC" and .asset != "USDC" and (.coin // .asset) != null)' 2>/dev/null | while read -r holding; do
        local coin amount
        coin=$(echo "$holding" | jq -r '.coin // .asset')
        amount=$(echo "$holding" | jq -r '.total // .balance // .available // "0"')

        if [[ -z "$coin" || "$coin" == "null" || "$amount" == "0" ]]; then continue; fi

        log_msg "  Selling spot: $coin amount=$amount"

        local quote price
        quote=$(ft "{\"command\":\"quote\",\"symbol\":\"$coin\"}")
        price=$(echo "$quote" | jq -r '.price // .markPx // "0"')

        if [[ "$price" == "0" || "$price" == "null" ]]; then
            log_msg "  WARNING: Cannot get price for $coin, skipping"
            continue
        fi

        local sell_limit
        sell_limit=$(echo "$price" | awk -v s="$SLIPPAGE_BPS" '{printf "%.6f", $1 * (1 - s/10000)}')

        if $DRY_RUN; then
            log_msg "  [DRY RUN] Would sell $amount $coin at \$$sell_limit"
            continue
        fi

        ft "{\"command\":\"order_sell\",\"symbol\":\"$coin\",\"amount\":\"$amount\",\"price\":\"$sell_limit\"}" | tee -a "$LOG_FILE"
        log_msg "  Sold spot $coin"
    done

    log_msg "All positions closed"
}

check_current_funding() {
    local positions
    positions=$(ft '{"command":"positions"}')

    local short_symbol
    short_symbol=$(echo "$positions" | jq -r '
        [.[] | select((.size // "0") != "0")] | .[0] | .coin // .symbol // empty
    ' 2>/dev/null)

    if [[ -z "$short_symbol" || "$short_symbol" == "null" ]]; then
        echo "none"
        return
    fi

    local quote funding
    quote=$(ft "{\"command\":\"perp_quote\",\"symbol\":\"$short_symbol\"}")
    funding=$(echo "$quote" | jq -r '.funding // "0"')

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
    log_msg "  Funding Rate Arbitrage Bot (JSON API)"
    log_msg "  Dry run: $DRY_RUN | Interval: ${CHECK_INTERVAL}s"
    log_msg "  Min funding: $MIN_FUNDING | Min volume: \$$MIN_VOLUME"
    log_msg "═══════════════════════════════════════════════════"

    while true; do
        log_msg "────── Check cycle at $(ts) ──────"

        local state
        state=$(get_current_state)
        local has_positions usdc
        has_positions=$(echo "$state" | jq -r '.has_positions')
        usdc=$(echo "$state" | jq -r '.usdc_available')

        log_msg "Account: positions=$has_positions USDC=$usdc"

        if [[ "$has_positions" -gt 0 ]]; then
            local funding_status
            funding_status=$(check_current_funding)

            if [[ "$funding_status" == "negative" ]]; then
                log_msg "Funding turned NEGATIVE — closing all positions"
                close_all_positions
            elif [[ "$funding_status" == "none" ]]; then
                log_msg "No perp position found but expected one — resetting"
                close_all_positions
            else
                log_msg "Funding still positive — holding position"
            fi
        else
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

                if [[ -n "${OPENAI_API_KEY:-}" ]]; then
                    log_msg "Asking OpenAI to analyze candidates..."
                    local analysis
                    analysis=$(analyze_with_openai "$(echo "$candidates" | jq -c '.')")
                    log_msg "OpenAI analysis: $analysis"

                    local pick
                    pick=$(echo "$analysis" | jq -r '.pick // empty' 2>/dev/null)

                    if [[ -n "$pick" ]]; then
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

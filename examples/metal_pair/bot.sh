#!/usr/bin/env bash
#
# Metal Pairs Trading Bot — GOLD vs SILVER
#
# Strategy: Long one metal, short the other (equal notional) based on:
#   1. News sentiment (which metal is more talked about and in what direction)
#   2. 24h price momentum (which is trending harder)
#   3. Funding rates (pay vs receive)
#
# Runs daily via cron. Uses fintool CLI for Hyperliquid HIP-3 perps.
#
# Prerequisites:
#   - fintool binary (set FINTOOL below)
#   - ~/.fintool/config.toml with wallet private_key + openai_api_key
#   - USDC balance on Hyperliquid (bot auto-converts to USDT0 as needed)
#   - Brave Search API key (BRAVE_API_KEY env var)
#   - OpenAI API key (OPENAI_API_KEY env var)
#
set -euo pipefail

###############################################################################
# Config
###############################################################################
FINTOOL="${FINTOOL:-/Users/michaelyuan/clawd/fintool-bin}"
TARGET_USDT0="${TARGET_USDT0:-50}"            # target USDT0 balance (margin for both legs)
POSITION_SIZE_USD="${POSITION_SIZE_USD:-50}"   # notional per leg ($50 each @ 2x = $25 margin each = $50 total)
LEVERAGE="${LEVERAGE:-2}"
LOG_DIR="/Users/michaelyuan/clawd/metal-pairs-bot/logs"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/$(date +%Y-%m-%d).log"

# API keys — fallback to config or env
BRAVE_API_KEY="${BRAVE_API_KEY:-BSAX4sEVelg1WgBIgcuvaSN_Exr2Z8t}"
OPENAI_API_KEY="${OPENAI_API_KEY:-$(grep openai_api_key ~/.fintool/config.toml 2>/dev/null | sed 's/.*= *"\(.*\)"/\1/')}"

log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"; }

###############################################################################
# Step 1: Search news for GOLD and SILVER
###############################################################################
search_news() {
  local query="$1"
  curl -s "https://api.search.brave.com/res/v1/news/search?q=${query}&count=10&freshness=pd" \
    -H "Accept: application/json" \
    -H "X-Subscription-Token: ${BRAVE_API_KEY}" \
    | jq -r '.results[] | "\(.title) — \(.description // "")"' 2>/dev/null || echo "No results"
}

log "=== Metal Pairs Bot Starting ==="

log "Fetching GOLD news..."
GOLD_NEWS=$(search_news "gold+commodity+price+market")
log "Fetching SILVER news..."
SILVER_NEWS=$(search_news "silver+commodity+price+market")

###############################################################################
# Step 2: LLM sentiment analysis
###############################################################################
log "Analyzing sentiment with LLM..."

SENTIMENT_PROMPT=$(cat <<'PROMPT_END'
You are a commodities trading analyst. Below are today's news headlines for GOLD and SILVER.

GOLD NEWS:
__GOLD_NEWS__

SILVER NEWS:
__SILVER_NEWS__

Analyze and respond in EXACTLY this JSON format (no markdown, no explanation):
{
  "more_talked_about": "GOLD" or "SILVER",
  "gold_sentiment": number from -1.0 (very bearish) to 1.0 (very bullish),
  "silver_sentiment": number from -1.0 (very bearish) to 1.0 (very bullish),
  "gold_headline_count": number,
  "silver_headline_count": number,
  "reasoning": "one sentence"
}
PROMPT_END
)

# Substitute news into prompt
SENTIMENT_PROMPT="${SENTIMENT_PROMPT/__GOLD_NEWS__/$GOLD_NEWS}"
SENTIMENT_PROMPT="${SENTIMENT_PROMPT/__SILVER_NEWS__/$SILVER_NEWS}"

SENTIMENT_JSON=$(curl -s https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${OPENAI_API_KEY}" \
  -d "$(jq -n --arg prompt "$SENTIMENT_PROMPT" '{
    model: "gpt-4.1-mini",
    temperature: 0.1,
    messages: [{role: "user", content: $prompt}]
  }')" \
  | jq -r '.choices[0].message.content' 2>/dev/null)

log "Sentiment: $SENTIMENT_JSON"

GOLD_SENTIMENT=$(echo "$SENTIMENT_JSON" | jq -r '.gold_sentiment // 0')
SILVER_SENTIMENT=$(echo "$SENTIMENT_JSON" | jq -r '.silver_sentiment // 0')

###############################################################################
# Step 3: Get 24h pricing data and trends
###############################################################################
log "Fetching price quotes..."

GOLD_QUOTE=$($FINTOOL quote GOLD 2>/dev/null)
SILVER_QUOTE=$($FINTOOL quote SILVER 2>/dev/null)

GOLD_CHANGE=$(echo "$GOLD_QUOTE" | jq -r '.change_24h_pct // 0')
SILVER_CHANGE=$(echo "$SILVER_QUOTE" | jq -r '.change_24h_pct // 0')
GOLD_PRICE=$(echo "$GOLD_QUOTE" | jq -r '.price // 0')
SILVER_PRICE=$(echo "$SILVER_QUOTE" | jq -r '.price // 0')

log "GOLD:   price=$GOLD_PRICE  24h_change=${GOLD_CHANGE}%"
log "SILVER: price=$SILVER_PRICE  24h_change=${SILVER_CHANGE}%"

###############################################################################
# Step 4: Get funding rates
###############################################################################
log "Fetching perp funding rates..."

GOLD_PERP=$($FINTOOL perp quote GOLD 2>/dev/null)
SILVER_PERP=$($FINTOOL perp quote SILVER 2>/dev/null)

GOLD_FUNDING=$(echo "$GOLD_PERP" | jq -r '.funding_rate // 0')
SILVER_FUNDING=$(echo "$SILVER_PERP" | jq -r '.funding_rate // 0')
GOLD_PERP_PRICE=$(echo "$GOLD_PERP" | jq -r '.mark_price // .price // 0')
SILVER_PERP_PRICE=$(echo "$SILVER_PERP" | jq -r '.mark_price // .price // 0')

log "GOLD   funding_rate=$GOLD_FUNDING  perp_price=$GOLD_PERP_PRICE"
log "SILVER funding_rate=$SILVER_FUNDING  perp_price=$SILVER_PERP_PRICE"

###############################################################################
# Step 5: Decision — which to long, which to short
###############################################################################
log "Computing trading decision..."

DECISION_PROMPT=$(cat <<DECISION_END
You are a quantitative trading system deciding a pairs trade between GOLD and SILVER perps.

DATA:
- GOLD:   24h_change=${GOLD_CHANGE}%, sentiment=${GOLD_SENTIMENT}, funding_rate=${GOLD_FUNDING}, price=${GOLD_PERP_PRICE}
- SILVER: 24h_change=${SILVER_CHANGE}%, sentiment=${SILVER_SENTIMENT}, funding_rate=${SILVER_FUNDING}, price=${SILVER_PERP_PRICE}

RULES:
1. Long the metal with stronger bullish momentum + sentiment, short the other
2. Prefer longing the one with negative/lower funding (you get paid)
3. If signals conflict, weight: momentum 40%, sentiment 35%, funding 25%
4. If both are nearly identical (within 0.5% change, similar sentiment), output "HOLD"

Respond in EXACTLY this JSON (no markdown):
{
  "action": "TRADE" or "HOLD",
  "long": "GOLD" or "SILVER",
  "short": "GOLD" or "SILVER",
  "confidence": number 0-1,
  "reasoning": "one sentence"
}
DECISION_END
)

DECISION_JSON=$(curl -s https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${OPENAI_API_KEY}" \
  -d "$(jq -n --arg prompt "$DECISION_PROMPT" '{
    model: "gpt-4.1-mini",
    temperature: 0.0,
    messages: [{role: "user", content: $prompt}]
  }')" \
  | jq -r '.choices[0].message.content' 2>/dev/null)

log "Decision: $DECISION_JSON"

ACTION=$(echo "$DECISION_JSON" | jq -r '.action // "HOLD"')
LONG_METAL=$(echo "$DECISION_JSON" | jq -r '.long // "GOLD"')
SHORT_METAL=$(echo "$DECISION_JSON" | jq -r '.short // "SILVER"')
CONFIDENCE=$(echo "$DECISION_JSON" | jq -r '.confidence // 0')
REASONING=$(echo "$DECISION_JSON" | jq -r '.reasoning // "unknown"')

if [[ "$ACTION" == "HOLD" ]]; then
  log "Decision: HOLD — no trade today. Reason: $REASONING"
  log "=== Bot Complete (no trade) ==="
  exit 0
fi

log "Decision: LONG $LONG_METAL / SHORT $SHORT_METAL (confidence: $CONFIDENCE)"

###############################################################################
# Step 6: Close ALL existing positions — return to clean USDT0 state
###############################################################################
log "Closing all existing positions..."

POSITIONS=$($FINTOOL positions 2>/dev/null || echo '[]')

close_position() {
  local symbol="$1"
  local pos_size

  pos_size=$(echo "$POSITIONS" | jq -r --arg s "$symbol" \
    '.[] | select(.symbol == $s or .symbol == ("cash:" + $s)) | .size // 0' 2>/dev/null | head -1)

  if [[ -n "$pos_size" && "$pos_size" != "0" && "$pos_size" != "null" ]]; then
    local abs_size=$(echo "$pos_size" | sed 's/^-//')
    local pos_price=$(echo "$POSITIONS" | jq -r --arg s "$symbol" \
      '.[] | select(.symbol == $s or .symbol == ("cash:" + $s)) | .entry_price // 0' 2>/dev/null | head -1)

    if (( $(echo "$pos_size > 0" | bc -l) )); then
      local close_price=$(echo "$pos_price * 0.95" | bc -l | xargs printf "%.2f")
      log "Closing LONG $symbol: sell $abs_size @ $close_price --close"
      $FINTOOL perp sell "$symbol" "$abs_size" "$close_price" --close 2>&1 | tee -a "$LOG_FILE"
    else
      local close_price=$(echo "$pos_price * 1.05" | bc -l | xargs printf "%.2f")
      log "Closing SHORT $symbol: buy $abs_size @ $close_price --close"
      $FINTOOL perp buy "$symbol" "$abs_size" "$close_price" --close 2>&1 | tee -a "$LOG_FILE"
    fi
    sleep 3
  else
    log "No existing position for $symbol"
  fi
}

close_position "GOLD"
close_position "SILVER"

# Wait for settlements
sleep 5

###############################################################################
# Step 7: Normalize USDT0 balance to exactly $TARGET_USDT0
#
# After closing positions, all margin is freed as USDT0 in the HIP-3 dex.
# We transfer everything back to spot, then adjust to hit the target:
#   - Excess USDT0 → sell for USDC
#   - Deficit USDT0 → buy with USDC (bridge from Base if USDC insufficient)
# Finally, transfer exactly $TARGET_USDT0 to HIP-3 dex for the new positions.
###############################################################################
log "Normalizing USDT0 balance to \$${TARGET_USDT0}..."

# Transfer all USDT0 from HIP-3 dex back to spot first
# (Get dex balance — may show as cash dex balance)
DEX_BALANCE=$($FINTOOL balance 2>/dev/null)
DEX_USDT0=$(echo "$DEX_BALANCE" | jq -r '
  .dex_balances[]? | select(.dex == "cash") | .total // 0
' 2>/dev/null || echo "0")

if (( $(echo "${DEX_USDT0:-0} > 0.01" | bc -l) )); then
  log "Transferring $DEX_USDT0 USDT0 from HIP-3 dex back to spot..."
  $FINTOOL transfer "$DEX_USDT0" from-dex --dex cash 2>&1 | tee -a "$LOG_FILE"
  sleep 3
fi

# Now check spot USDT0 and USDC balances
BALANCE=$($FINTOOL balance 2>/dev/null)
USDT0_BALANCE=$(echo "$BALANCE" | jq -r '
  .spot_balances[]? | select(.token == "USDT0") | .total // 0
' 2>/dev/null || echo "0")
USDC_BALANCE=$(echo "$BALANCE" | jq -r '
  .perp_balance.total // .usdc_balance // 0
' 2>/dev/null || echo "0")

log "Current balances — USDT0: $USDT0_BALANCE, USDC: $USDC_BALANCE"

# Calculate difference from target
USDT0_DIFF=$(echo "$USDT0_BALANCE - $TARGET_USDT0" | bc -l)

if (( $(echo "$USDT0_DIFF > 1" | bc -l) )); then
  # Too much USDT0 — sell excess for USDC
  SELL_AMOUNT=$(printf "%.0f" "$(echo "$USDT0_DIFF" | bc -l)")
  log "Excess USDT0: selling $SELL_AMOUNT USDT0 → USDC"
  $FINTOOL order sell USDT0 "$SELL_AMOUNT" 0.998 2>&1 | tee -a "$LOG_FILE"
  sleep 5

elif (( $(echo "$USDT0_DIFF < -1" | bc -l) )); then
  # Not enough USDT0 — buy more with USDC
  BUY_AMOUNT=$(printf "%.0f" "$(echo "$USDT0_DIFF * -1" | bc -l)")

  # Check if we have enough USDC; if not, bridge from Base
  if (( $(echo "$USDC_BALANCE < $BUY_AMOUNT" | bc -l) )); then
    BRIDGE_AMOUNT=$(printf "%.0f" "$(echo "$BUY_AMOUNT - $USDC_BALANCE + 10" | bc -l)")
    log "Insufficient USDC ($USDC_BALANCE). Bridging $BRIDGE_AMOUNT USDC from Base..."
    $FINTOOL deposit USDC --amount "$BRIDGE_AMOUNT" --from base 2>&1 | tee -a "$LOG_FILE"
    sleep 10
  fi

  log "Buying $BUY_AMOUNT USDT0 with USDC"
  $FINTOOL order buy USDT0 "$BUY_AMOUNT" 1.003 2>&1 | tee -a "$LOG_FILE"
  sleep 5

else
  log "USDT0 balance is within target range (diff: $USDT0_DIFF)"
fi

# Transfer exactly $TARGET_USDT0 to HIP-3 dex for trading
log "Transferring \$${TARGET_USDT0} USDT0 to HIP-3 dex..."
$FINTOOL transfer "$TARGET_USDT0" to-dex --dex cash 2>&1 | tee -a "$LOG_FILE"
sleep 3

###############################################################################
# Step 8: Set leverage and open positions
###############################################################################
log "Setting leverage to ${LEVERAGE}x..."
$FINTOOL perp leverage "$LONG_METAL" "$LEVERAGE" 2>&1 | tee -a "$LOG_FILE"
$FINTOOL perp leverage "$SHORT_METAL" "$LEVERAGE" 2>&1 | tee -a "$LOG_FILE"

# Get current prices for limit orders (use aggressive limits to fill quickly)
if [[ "$LONG_METAL" == "GOLD" ]]; then
  LONG_PRICE=$GOLD_PERP_PRICE
  SHORT_PRICE=$SILVER_PERP_PRICE
else
  LONG_PRICE=$SILVER_PERP_PRICE
  SHORT_PRICE=$GOLD_PERP_PRICE
fi

# Long: buy at slightly above market (aggressive fill)
LONG_LIMIT=$(echo "$LONG_PRICE * 1.005" | bc -l | xargs printf "%.2f")
# Short: sell at slightly below market (aggressive fill)
SHORT_LIMIT=$(echo "$SHORT_PRICE * 0.995" | bc -l | xargs printf "%.2f")

# Each leg: $50 notional @ 2x leverage = $25 margin. Two legs = $50 total margin = all USDT0.
log "Opening LONG $LONG_METAL: \$${POSITION_SIZE_USD} notional @ limit $LONG_LIMIT (margin: \$$(echo "$POSITION_SIZE_USD / $LEVERAGE" | bc))"
$FINTOOL perp buy "$LONG_METAL" "$POSITION_SIZE_USD" "$LONG_LIMIT" 2>&1 | tee -a "$LOG_FILE"

log "Opening SHORT $SHORT_METAL: \$${POSITION_SIZE_USD} notional @ limit $SHORT_LIMIT (margin: \$$(echo "$POSITION_SIZE_USD / $LEVERAGE" | bc))"
$FINTOOL perp sell "$SHORT_METAL" "$POSITION_SIZE_USD" "$SHORT_LIMIT" 2>&1 | tee -a "$LOG_FILE"

sleep 5

###############################################################################
# Step 9: Verify positions
###############################################################################
log "Verifying positions..."
$FINTOOL positions --human 2>&1 | tee -a "$LOG_FILE"
$FINTOOL balance --human 2>&1 | tee -a "$LOG_FILE"

log "=== Bot Complete ==="
log "Summary: LONG $LONG_METAL / SHORT $SHORT_METAL | \$${POSITION_SIZE_USD}/leg | ${LEVERAGE}x leverage"
log "Reasoning: $REASONING"

# Output summary JSON for programmatic consumption
cat <<EOF
{
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "action": "$ACTION",
  "long": "$LONG_METAL",
  "short": "$SHORT_METAL",
  "position_size_usd": $POSITION_SIZE_USD,
  "leverage": $LEVERAGE,
  "confidence": $CONFIDENCE,
  "gold_24h_change": $GOLD_CHANGE,
  "silver_24h_change": $SILVER_CHANGE,
  "gold_sentiment": $GOLD_SENTIMENT,
  "silver_sentiment": $SILVER_SENTIMENT,
  "gold_funding": $GOLD_FUNDING,
  "silver_funding": $SILVER_FUNDING,
  "reasoning": "$REASONING"
}
EOF

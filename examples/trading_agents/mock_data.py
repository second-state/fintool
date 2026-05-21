"""
Mock market data and pre-written LLM responses for simulate.py.

Three scenarios covering the full decision space:
  btc   — BTC spot on Hyperliquid  → BUY  (clear bullish setup)
  eth   — ETH perp on Binance      → HOLD (risk team vetoes on divergence)
  hype  — HYPE spot on Hyperliquid → SELL (overbought, momentum fading)
"""

import json

# ──────────────────────────────────────────────────────────────
# Market data fixtures
# ──────────────────────────────────────────────────────────────

BTC_FUNDAMENTAL = {
    "pe_ratio": None,
    "forward_pe": None,
    "roe": None,
    "roa": None,
    "debt_to_equity": None,
    "current_ratio": None,
    "quick_ratio": None,
    "profit_margin": None,
    "revenue_growth": None,
    "earnings_growth": None,
    "market_cap": 1_271_000_000_000,
    "price_to_book": None,
    "beta": 1.48,
    "52w_high": 73_780.0,
    "52w_low": 38_505.0,
    "sector": "Cryptocurrency",
    "industry": "Digital Assets",
}

BTC_TECHNICAL = {
    "current_price":   64_180.0,
    "rsi_14":          54.2,
    "macd":            312.4,
    "macd_signal":     274.1,
    "macd_histogram":  38.3,
    "bb_upper":        68_410.0,
    "bb_mid":          63_100.0,
    "bb_lower":        57_790.0,
    "cci_20":          88.4,
    "sma_50":          61_340.0,
    "sma_200":         52_800.0,
    "vol_avg_20":      28_400_000_000,
    "price_vs_sma50":  "above",
    "price_vs_sma200": "above",
}

BTC_SENTIMENT = {
    "ticker": "BTC",
    "headlines": [
        "BlackRock IBIT sees record $520M single-day inflow as institutions pile in",
        "Bitcoin holds $63K support as ETF demand continues to outpace new supply",
        "El Salvador adds 80 BTC to national treasury following IMF deal approval",
        "On-chain data: Long-term holders accumulating at fastest pace since 2020",
        "Fed dovish pivot expectations boost risk assets; BTC leads crypto gains",
        "Sovereign wealth funds eye Bitcoin allocations after Norway fund report",
        "Bitcoin hashrate hits all-time high of 680 EH/s — network security peaks",
        "Options market shows growing demand for BTC $70K calls expiring June",
    ],
    "count": 8,
}

BTC_MACRO = {
    "SPY": [
        "S&P 500 hits fresh all-time high on strong earnings beats across tech sector",
        "Fed minutes signal two rate cuts in 2025; markets rally on dovish tone",
        "Risk-on sentiment dominates as VIX drops to 12.4, lowest since 2019",
    ],
    "GOLD": [
        "Gold consolidates at $2,340 as dollar weakens on rate cut expectations",
        "Central bank gold buying reaches 5-year high — emerging markets leading",
    ],
    "DXY": [
        "Dollar index slips to 103.2 as euro strengthens on ECB hawkish hold",
        "DXY weekly decline fourth in a row — dollar bears take control",
    ],
}

ETH_FUNDAMENTAL = {
    "pe_ratio": None,
    "forward_pe": None,
    "roe": None,
    "roa": None,
    "debt_to_equity": None,
    "current_ratio": None,
    "quick_ratio": None,
    "profit_margin": None,
    "revenue_growth": None,
    "earnings_growth": None,
    "market_cap": 287_000_000_000,
    "price_to_book": None,
    "beta": 1.62,
    "52w_high": 4_094.0,
    "52w_low": 1_506.0,
    "sector": "Cryptocurrency",
    "industry": "Smart Contract Platform",
}

ETH_TECHNICAL = {
    "current_price":   2_381.0,
    "rsi_14":          67.8,
    "macd":            42.1,
    "macd_signal":     48.6,
    "macd_histogram":  -6.5,
    "bb_upper":        2_520.0,
    "bb_mid":          2_280.0,
    "bb_lower":        2_040.0,
    "cci_20":          118.2,
    "sma_50":          2_185.0,
    "sma_200":         2_620.0,
    "vol_avg_20":      12_100_000_000,
    "price_vs_sma50":  "above",
    "price_vs_sma200": "below",
}

ETH_SENTIMENT = {
    "ticker": "ETH",
    "headlines": [
        "Ethereum ETF inflows disappoint vs Bitcoin as institutional appetite diverges",
        "L2 fee revenue cannibalizing Ethereum base layer — EIP-4844 impact deepens",
        "Solana DEX volume surpasses Ethereum for third consecutive week",
        "Vitalik: 'We need to solve the UX problem before mass adoption is possible'",
        "Ethereum staking yield drops to 3.2% as validator count hits 1M milestone",
        "Spot ETH ETF sees $84M outflow — largest single-day redemption since launch",
    ],
    "count": 6,
}

ETH_MACRO = {
    "SPY": [
        "Equities mixed ahead of CPI print — tech sector sells off on valuation concerns",
        "Fed officials signal patience on rate cuts; 'higher for longer' back in play",
    ],
    "GOLD": [
        "Gold surges 1.4% as geopolitical tensions in Middle East escalate",
    ],
    "DXY": [
        "Dollar strengthens on jobs data beat — DXY breaks above 104.5 resistance",
        "Treasury yields spike to 4.72% on hot PPI reading — risk assets under pressure",
    ],
}

HYPE_FUNDAMENTAL = {
    "pe_ratio": None,
    "forward_pe": None,
    "roe": None,
    "roa": None,
    "debt_to_equity": None,
    "current_ratio": None,
    "quick_ratio": None,
    "profit_margin": None,
    "revenue_growth": None,
    "earnings_growth": None,
    "market_cap": 8_400_000_000,
    "price_to_book": None,
    "beta": 2.41,
    "52w_high": 34.90,
    "52w_low": 2.80,
    "sector": "Cryptocurrency",
    "industry": "DEX / Derivatives",
}

HYPE_TECHNICAL = {
    "current_price":   22.40,
    "rsi_14":          73.6,
    "macd":            0.82,
    "macd_signal":     1.14,
    "macd_histogram":  -0.32,
    "bb_upper":        24.90,
    "bb_mid":          20.80,
    "bb_lower":        16.70,
    "cci_20":          142.8,
    "sma_50":          18.60,
    "sma_200":         11.30,
    "vol_avg_20":      380_000_000,
    "price_vs_sma50":  "above",
    "price_vs_sma200": "above",
}

HYPE_SENTIMENT = {
    "ticker": "HYPE",
    "headlines": [
        "HYPE prints new all-time high at $34.90 — up 11x since launch",
        "Hyperliquid daily volume hits $8B — challenging Binance perp dominance",
        "Early HYPE airdrop recipients begin distribution, sell pressure concerns mount",
        "Hyperliquid suffers $4M exploit via oracle manipulation — team covers losses",
        "HYPE tokenomics face criticism: 76% team/investor allocation unlocking in Q3",
        "Retail FOMO drives HYPE to overbought territory — funding rate at +0.12%/8h",
    ],
    "count": 6,
}

HYPE_MACRO = BTC_MACRO  # same macro backdrop for simplicity


# ──────────────────────────────────────────────────────────────
# Pre-written LLM responses  (keyed by agent role)
# ──────────────────────────────────────────────────────────────

BTC_LLM = {

"Fundamental": """\
Reasoning: Bitcoin lacks traditional fundamental metrics (P/E, ROE), so I evaluate it through \
monetary asset proxies — supply dynamics, network security, and institutional demand flow.

(1) VALUATION: Market cap of $1.27T places BTC at $64,180. The NVT ratio has normalized post-halving \
to ~45x, historically a fair-value zone rather than a speculative peak. ETF AUM of ~$58B represents \
persistent buy pressure that the halving-reduced supply (3.125 BTC/block) cannot offset.

(2) PROFITABILITY: Miner revenue ~$52M/day at current hashrate of 620 EH/s. Break-even for the \
median mining operation is ~$38,000 — current price offers a $26K cushion. This floor support is \
structurally meaningful.

(3) LIQUIDITY RISKS: The ETF bid is the key liquidity variable. BlackRock IBIT holds 287,000 BTC. \
A coordinated redemption cycle would overwhelm spot liquidity. Concentration in 5 custodians is a \
systemic tail risk worth tracking. On the upside, each new allocation (sovereign wealth funds, \
corporate treasuries) expands the demand curve structurally.

(4) GROWTH: Post-halving supply squeeze + institutional on-ramp expansion + potential BTC strategic \
reserve policy signals = genuine multi-year demand catalysts. Revenue growth analog: hash rate is up \
40% YoY, reflecting growing conviction from miners who vote with capital.

Verdict: Moderately bullish — the supply/demand setup is constructive, current price reflects much \
of the ETF narrative but not the sovereign/SWF adoption wave that is still early stage.\
""",

"Technical": """\
Reasoning: Let me work through each indicator systematically for BTC at $64,180.

(1) MOMENTUM — RSI/MACD: RSI-14 at 54.2 is constructively positioned. Above 50 signals bullish \
bias, and the distance from 70 (overbought) means there is headroom before exhaustion. More \
importantly, the MACD just produced a bullish crossover: MACD line (+$312.4) crossed above the \
signal line (+$274.1), with the histogram turning positive (+$38.3). This is a fresh, unextended \
momentum trigger — not a stale signal.

(2) VOLATILITY — Bollinger Bands: Price at $64,180 sits in the upper-mid zone between the midband \
($63,100) and upper band ($68,410). Bandwidth has been contracting for 12 sessions, indicating coiled \
energy. Contracting bands followed by a MACD cross historically precede directional breakouts.

(3) TREND — SMAs/CCI: Price is $2,840 above SMA-50 ($61,340) and $11,380 above SMA-200 ($52,800). \
Golden cross intact. CCI-20 at +88.4 is in the bullish momentum zone (above 0, below the overbought \
+100 threshold). The price is not technically overbought by any measure.

(4) OVERBOUGHT/OVERSOLD: Not overbought. RSI has ~16 points before the 70 threshold. CCI is 12 \
points from the +100 signal level. This setup is early-mid move, not a top.

Directional bias: BULLISH with 70% confidence. The MACD crossover with RSI headroom is the \
cleanest signal. Key invalidation: a close below SMA-20 at $62,500 would cancel the thesis.\
""",

"Sentiment": """\
Reasoning: Reading the news cluster to identify the dominant narrative and investor psychology.

(1) SENTIMENT SCORE: +0.52 (moderately bullish). Institutional flow headlines dominate, with strong \
positive framing around ETF inflows and sovereign adoption. No panic-sell language detected.

(2) KEY NARRATIVE THEMES:
   • ETF institutional demand as a structural floor (BlackRock $520M inflow)
   • Supply shock narrative post-halving (macro bullish framing)
   • Sovereign adoption wave (El Salvador, SWF signals)
   • Hash rate ATH = security premium + miner confidence signal

(3) CONTRARIAN SIGNALS: The ETF concentration risk deserves attention — if BlackRock IBIT redemptions \
spike, the forced selling would overwhelm spot depth. Also: the $70K call options demand may be \
crowded positioning that creates a squeeze rather than genuine upside conviction.

(4) INVESTOR PSYCHOLOGY: Risk appetite is firmly on. Long-term holders accumulating at the fastest \
pace since 2020 is a significant signal — these cohorts have historically been correct. The options \
market skew suggests confident (not panicked) bullishness. Overall: greed is present but not extreme.\
""",

"Macro": """\
Reasoning: The macro environment must be filtered through the lens of BTC as a risk asset with a \
monetary premium — it responds to both macro sentiment AND its own supply/demand mechanics.

(1) KEY MACRO RISKS: Fed rate cut expectations driving the risk-on tape. The dovish pivot narrative \
(two cuts priced for 2025) is the primary tailwind for speculative assets. However, any hot CPI/PPI \
print could reverse this rapidly. BTC's 30-day correlation to the Nasdaq is currently +0.62.

(2) SECTOR TAILWINDS: Dollar weakness (DXY -4th consecutive weekly decline) is a direct BTC \
tailwind — hard asset / scarce supply narrative strengthens as fiat purchasing power degrades.

(3) CORRELATION: Risk-on positioning (VIX at 12.4, SPY at ATH) benefits BTC disproportionately \
given its beta of 1.48 vs equities. In periods of synchronized risk-on, BTC typically outperforms \
its beta vs traditional risk assets.

(4) NEAR-TERM PRICE IMPACT: Favorable. The macro setup (dovish Fed + weak dollar + risk-on equities) \
is the ideal BTC entry environment. The only near-term risk is a geopolitical shock reversing the VIX \
compression, which would trigger a correlated sell-off across risk assets including BTC.\
""",

"Bull_R1": """\
Round 1 — Bullish Case for BTC:

The data presents a textbook accumulation-phase entry. Let me be specific:

SUPPLY SHOCK IS REAL: At 3.125 BTC/block post-halving, approximately 450 BTC enters supply daily. \
ETF inflows averaged +$180M/day over the past 30 days — at $64,180/BTC, that represents demand for \
~2,800 BTC/day. The structural supply/demand imbalance is not priced in at current levels.

TECHNICAL MOMENTUM IS UNEXTENDED: RSI-54 with a fresh MACD crossover at non-overbought levels is \
the entry pattern technical traders have historically front-run. The histogram turning positive after \
a 12-day BB compression is a coiled spring setup. We are not buying a top — we are entering at the \
beginning of a momentum leg.

INSTITUTIONAL DEMAND IS ACCELERATING: BlackRock's $520M single-day inflow is not noise — it \
reflects allocation mandates from endowments and pension funds that were previously unable to hold \
BTC. This is a demand cohort that does not sell on volatility. Their cost basis is being established \
between $58,000–$65,000.

SOVEREIGN ADOPTION CURVE: El Salvador demonstrated proof of concept. The Norway SWF BTC exposure \
report — even if indirect — signals that the world's largest sovereign wealth fund is watching. When \
that cohort allocates even 0.5%, it represents $600B in demand against a $1.27T market.

The bear case will focus on concentration risk and ETF redemption scenarios. These are real tail \
risks, not base cases. We size appropriately and manage risk, but avoiding the trade entirely is the \
wrong response to a bullish primary signal with unextended momentum.\
""",

"Bear_R1": """\
Round 1 — Bearish Rebuttal:

The Bull presents a compelling narrative. Let me stress-test each claim and explain why the \
technical MACD crossover is likely a bull trap.

THE TECHNICAL BULLISH SIGNAL IS A TRAP: The MACD crossover at RSI-54 sounds constructive. But look \
at the context: this crossover is occurring in a 12-day consolidation range between $61,000–$66,000. \
Historically, MACD crossovers within consolidation rather than at the start of breakouts have a 58% \
failure rate. The BB compression could resolve downward just as easily as upward — and a break below \
$62,500 (SMA-20) would accelerate selling from momentum traders who set their invalidation there.

ETF CONCENTRATION IS NOT A TAIL RISK — IT IS THE DOMINANT RISK: The Bull's own data is damning. \
Five custodians hold the majority of ETF BTC. If even ONE large ETF sees 10% redemptions in a \
risk-off event, we are looking at 28,700+ BTC hitting market depth in hours. The $64K price already \
reflects peak institutional optimism — there is limited incremental buyer left in the ETF cohort.

SUPPLY SHOCK ARITHMETIC IS INCOMPLETE: Yes, ETF demand outpaces new supply. But that math ignores \
long-term holder distribution — these wallets have been reducing exposure in the $60K–$70K range, \
which is where they accumulated in 2021. They are the natural sellers at these levels.

SOVEREIGN ADOPTION IS SPECULATION: The Norway SWF "report" is an indirect, passive holding through \
equity exposure to Coinbase — not a direct BTC allocation. Pricing in sovereign buying that has not \
been announced is narrative extrapolation, not fundamental analysis.

The Bull's case requires multiple optimistic assumptions to materialize simultaneously. In risk \
management terms, this is not asymmetric upside — it is symmetric risk masquerading as an obvious \
long entry.\
""",

"Bull_R2": """\
Round 2 — Bullish Rebuttal:

The Bear raises the right concerns but overstates the probability of each risk materializing.

ON THE MACD TRAP ARGUMENT: The Bear cites a 58% failure rate for MACD crossovers in consolidation — \
but this is the population average. When you condition on (a) RSI below 60, (b) price above both \
SMA-50 and SMA-200, and (c) BB bandwidth expanding after compression, the success rate of MACD \
crossovers as breakout precursors rises above 70%. All three conditions are met here.

ON ETF CONCENTRATION RISK: A 10% single-fund redemption event would require institutional investors \
to simultaneously exit a position they built over 6+ months. The actual redemption mechanism requires \
T+1 processing and AP arbitrage — not the instant liquidation cascade the Bear implies. Moreover, \
ETF outflows in this range have historically triggered accumulators from other cohorts (exchanges, \
OTC desks) to absorb at a discount.

ON LONG-TERM HOLDER DISTRIBUTION: The Bear correctly identifies LTH selling between $60K–$70K. But \
on-chain data shows the magnitude: approximately 180,000 BTC moved from LTH to new buyers in the \
past 45 days. That cohort transition is nearly complete — the overhang is smaller than it was in \
February. We are closer to the end of distribution, not the beginning.

The Bear has not offered an alternative direction — only reasons to be cautious. Caution is \
appropriate; that is why we are sizing at 12–15%, not 40%. The asymmetric argument remains: \
risk-managed entry on a constructive technical setup with structural tailwinds.\
""",

"Bear_R2": """\
Round 2 — Bearish Rebuttal:

The Bull makes a credible statistical case but misses the meta-risk.

ON CONDITIONAL MACD STATISTICS: The Bull's conditional success rate adjustment is valid methodology. \
However, it assumes the current setup matches historical data distribution — which it may not. The \
2024-2025 ETF era has fundamentally changed the BTC volatility structure. Implied vol has been \
suppressed by ETF market-making, which means that when volatility does re-emerge, it will be larger \
and faster than historical analogs. The indicators are calibrated to a different regime.

ON REDEMPTION MECHANICS: T+1 processing is correct for ETF shares, but the Bear's point was about \
MARKET IMPACT, not processing speed. When APs hedge ETF redemptions, they short BTC futures — which \
hits price in real-time, before the physical redemption settles. This futures pressure is what \
creates the cascade, not the physical delivery.

ON LTH DISTRIBUTION COMPLETION: The claim that LTH selling is "nearly complete" is unfalsifiable \
without the actual wallet data. CoinGlass estimates only 40% of the typical LTH distribution cycle \
is complete at current prices. We cannot rely on this claim as a bull catalyst.

FINAL ASSESSMENT: The Bear does not argue for a short position. The Bull's case has merit for a \
small, risk-managed long. But "size at 12-15%" still represents meaningful capital at risk in a \
market where the primary risk — correlated ETF redemption during a risk-off event — could produce \
a $15,000–$20,000 drawdown within 48 hours. The Fund Manager should cap size at 10% and set a clear \
stop below $59,000.\
""",

"Trader": json.dumps({
    "action": "buy",
    "size_pct": 14,
    "price_offset_pct": -0.5,
    "rationale": "Fresh MACD crossover with RSI at 54 (headroom to 70), BB compression resolving upward, and price above both SMAs. Structural supply/demand imbalance from ETF inflows (2,800 BTC/day demand vs 450 BTC/day supply) provides fundamental support. The Bear's strongest concern — ETF redemption cascade — is a tail risk, not the base case. Sizing at 14% balances conviction with the identified downside.",
    "key_risk": "ETF concentration — a correlated institutional risk-off redemption event could produce a $12,000–$18,000 drawdown faster than typical stop-losses can execute."
}),

"Aggressive": json.dumps({
    "approved": True,
    "recommended_size_pct": 22,
    "key_concern": "ETF redemption cascade is a valid tail risk but probability-weighted impact is manageable",
    "assessment": "The setup is clean: fresh MACD cross, RSI headroom, price above all major SMAs, structural supply deficit. The Bear's arguments are cautionary, not directionally bearish. At 22% we capture meaningful upside while staying within a single-day drawdown tolerance of ~$1,400 on a $5,000 portfolio. I'd go higher but respect the Fund Manager's process."
}),

"Neutral": json.dumps({
    "approved": True,
    "recommended_size_pct": 14,
    "key_concern": "MACD crossover within consolidation has elevated failure rate; need a confirmed breakout above $66,000 to add",
    "assessment": "The trade has merit but requires disciplined sizing. RSI at 54 and MACD cross provide entry signal with clear invalidation ($62,500 / SMA-20). ETF supply/demand math supports the thesis but LTH distribution could extend. 14% is the appropriate risk-adjusted size — captures meaningful upside while limiting damage if the MACD cross fails. Recommend setting a stop at $62,000."
}),

"Conservative": json.dumps({
    "approved": True,
    "recommended_size_pct": 8,
    "key_concern": "ETF concentration risk is structural, not tail — five custodians holding >$50B represents single-point-of-failure systemic exposure",
    "assessment": "Capital preservation mandate leads me to cap at 8%. The Bear's strongest point — that we are buying into an environment where a single redemption event could cascade — remains insufficiently addressed by the Bull. The price already reflects significant institutional optimism. I approve a small position at 8% with a hard stop at $61,000 (SMA-50). Any position larger than this violates my risk mandate given the unquantified ETF concentration risk."
}),

"FundManager": json.dumps({
    "self_reflection": "The Bear's strongest point is ETF concentration risk — five custodians hold >$50B in BTC on behalf of institutional investors who have never navigated a full BTC bear market with this product structure. Does our proposed size of 14% adequately account for this? The Conservative voice capped at 8% citing exactly this concern. The weighted average of 12.5% is a reasonable compromise — it is meaningful exposure without overcommitting to a single-point-of-failure scenario. The $62,000 stop (SMA-50 proximity) limits maximum loss to ~3.4% of the total portfolio. This is acceptable.",
    "action": "buy",
    "final_size_pct": 12,
    "price_offset_pct": -0.5,
    "confidence": 0.71,
    "reasoning": "Weighted vote (Conservative 50% × 8% + Neutral 30% × 14% + Aggressive 20% × 22%) yields 12.4%, rounded to 12%. The technical setup is constructive (fresh MACD cross, RSI headroom, BB compression), fundamental supply/demand is supportive, and macro environment (dovish Fed, weak dollar, risk-on) is favorable. Sizing at 12% with a defined stop at $61,000 creates an asymmetric risk/reward of approximately 1:3.2. Self-reflection confirms the trade size is appropriate — the Conservative voice's concern is accounted for by capping below the Neutral and Aggressive recommendations.",
    "no_trade_reason": None
}),
}

ETH_LLM = {

"Fundamental": """\
Reasoning: Ethereum is a cash-flow-generating smart contract platform. Unlike BTC, it can be \
evaluated on fundamental metrics: fee revenue, validator yield, and protocol utilization.

(1) VALUATION: Market cap of $287B. Daily fee revenue ~$8.2M, annualized ~$3B. Implied P/S \
multiple of ~96x. Post-EIP-4844, fees dropped 85% as blobs took L2 activity off-chain. The protocol \
is effectively subsidizing L2 growth at the cost of its own fee generation engine. This is a \
valuation problem.

(2) PROFITABILITY: Staking yield compressed to 3.2% — below US risk-free rate of 4.72%. Economic \
validators now earn negative real yield. When staking yield competes unfavorably with Treasuries, \
the marginal validator rationale weakens.

(3) LIQUIDITY RISKS: ETH ETF outflows of $84M in a single day signal institutional differentiation \
— managers are rotating INTO BTC ETFs and OUT of ETH ETFs. This is a structural preference shift, \
not a temporary blip. Spot ETH ETF underperformance vs BTC ETF is the key liquidity risk.

(4) GROWTH: ETH faces genuine competitive pressure. Solana's DEX volume surpassing Ethereum for \
three consecutive weeks is not a narrative — it is market share data. The L2 ecosystem is thriving, \
but value accrual to ETH L1 is structurally weakened by EIP-4844 design choices.

Verdict: Bearish fundamental outlook. The protocol's fee revenue engine is impaired, staking yield \
is uncompetitive with risk-free rates, and institutional money is preferring BTC. Not a short \
at current levels, but fundamentals do not support a long either.\
""",

"Technical": """\
Reasoning: ETH at $2,381 shows a critical divergence that demands attention.

(1) MOMENTUM — RSI/MACD: RSI at 67.8 is approaching overbought territory (70). More importantly, \
the MACD is printing a BEARISH DIVERGENCE: price has made a higher high ($2,381 vs prior $2,290) \
while the MACD histogram has turned NEGATIVE (-6.5). This is a textbook bearish divergence — \
momentum is weakening while price is still elevated. This is the primary bearish signal.

(2) VOLATILITY — Bollinger Bands: Price at $2,381 is approaching the upper band ($2,520). The \
upper band is dynamic resistance. With momentum weakening (MACD divergence) and price near upper \
band, the risk/reward for new longs is poor. A mean-reversion to the midband ($2,280) would \
represent a -4.2% move.

(3) TREND — SMAs/CCI: Critical negative: ETH is BELOW its SMA-200 at $2,620. This is the key trend \
indicator — price is in a long-term downtrend relative to the 200-day average. CCI at +118.2 is \
in overbought territory (above +100). The combination of below-SMA-200 and overbought CCI is a \
high-probability reversal setup.

(4) OVERBOUGHT/OVERSOLD: OVERBOUGHT on CCI (118.2 > 100) and near-overbought on RSI (67.8). Both \
signals align: this is not an entry point for a long position.

Directional bias: BEARISH-NEUTRAL with 65% confidence. The MACD divergence plus below-SMA-200 plus \
CCI overbought is a three-indicator warning cluster. A breakout above $2,520 (BB upper) would \
invalidate the bearish thesis; below $2,280 would confirm it.\
""",

"Sentiment": """\
Reasoning: The ETH news cluster is notably more mixed than BTC, with genuine fundamental concerns \
surfacing alongside typical price-action FOMO.

(1) SENTIMENT SCORE: -0.12 (mildly bearish). While not panic territory, the negative data points \
are substantive — not just retail fear. The Solana DEX volume surpassing ETH headline is market \
structure data, not FUD.

(2) KEY NARRATIVE THEMES:
   • ETF underperformance vs BTC — institutional differentiation is occurring in real-time
   • L2 fee cannibalization — EIP-4844 "blob" design reducing L1 revenue structurally
   • Validator yield uncompetitive with risk-free rate — economic argument for staking weakening
   • Competitive pressure from Solana validated by volume data

(3) CONTRARIAN SIGNALS: The bearish sentiment may be excessive — ETH at these levels has historically \
been a buy when sentiment turns negative. However, unlike prior cycles, the fundamental headwinds \
(fee revenue model disruption) are new and not priced as temporary.

(4) INVESTOR PSYCHOLOGY: Cautious. The $84M ETF outflow is the key psychological marker — \
sophisticated institutional money is differentiating. Retail momentum chasers may be bidding ETH \
to near-overbought, but smart money is rotating toward BTC. This divergence between retail and \
institutional behavior is a caution flag.\
""",

"Macro": """\
Reasoning: The macro environment has shifted since our BTC analysis — recent data is more hawkish.

(1) KEY MACRO RISKS: Dollar strengthening (DXY breaking above 104.5 resistance) and Treasury yields \
spiking to 4.72% on hot PPI data represent a meaningful headwind for risk assets. ETH, with its \
equity-like beta (1.62 vs BTC's 1.48), is more sensitive to risk-off repricing.

(2) SECTOR HEADWINDS: The broader DeFi/smart contract platform sector is losing narrative momentum \
to Bitcoin's "digital gold / reserve asset" framing. In a risk-off macro environment, the market \
consolidates into the highest-conviction assets first. ETH is second-tier in this allocation \
preference order.

(3) CORRELATION: ETH historically underperforms BTC by 15–30% during risk-off periods driven by \
dollar strength and rising yields. If DXY sustains above 104.5, expect ETH/BTC ratio to compress.

(4) NEAR-TERM PRICE IMPACT: Negative. The combination of hot PPI data, dollar strength, and rising \
yields is the worst macro cocktail for ETH perp. A risk-off event in this environment could see ETH \
test $2,100 (prior support) before the technical picture improves.\
""",

"Bull_R1": """\
Round 1 — Bullish Case for ETH Perp:

While the concerns raised in the analyst reports are valid, the market is forward-looking and may \
be mispricing ETH's medium-term potential.

VALUATION INFLECTION: Yes, fee revenue dropped post-EIP-4844. But this was a DELIBERATE design \
choice to capture L2 ecosystem growth. The bet is that L2 adoption grows the total addressable \
market of Ethereum computation, and sequencer fees eventually flow back through protocol upgrades. \
Ethereum has a governance roadmap (Pectra, Osaka upgrades) that will address value accrual.

RSI AND PRICE STRENGTH: ETH is holding above $2,380 despite macro headwinds. This relative strength \
against a rising dollar and higher yields is a positive signal — buyers are absorbing the macro \
pressure. If macro reverses (which it could quickly given the Fed's dual mandate), ETH could rapidly \
mean-revert to $2,800+ where it traded before this correction.

PERP SETUP: We are entering a perp, not a spot long. Funding rates are currently near neutral, \
which means the long-side is not paying an excessive carry. This is a better environment for a \
perp long than when funding was +0.1%/8h.

POSITIONING: The ETF outflow narrative may create a sentiment bottom. When institutional money \
appears to be exiting, contrarian longs at support levels often produce the best risk/reward. \
$2,380 is the entry; $2,180 is the stop (SMA-50 with buffer); $2,750 is the target.\
""",

"Bear_R1": """\
Round 1 — Bearish Rebuttal:

The Bull is constructing a contrarian case on thin evidence while ignoring three convergent \
warning signals that historically precede sharp corrections.

THE MACD DIVERGENCE IS THE SINGLE MOST IMPORTANT INDICATOR HERE: Price making higher highs while \
MACD histogram goes negative is a textbook reversal pattern. The Bull did not address this. When RSI \
is simultaneously approaching overbought (67.8) and MACD is diverging bearishly, historical success \
rate for longs initiated here is under 35%. This is not an opinion — this is pattern frequency data.

THE "DELIBERATE DESIGN CHOICE" ARGUMENT IS POST-HOC RATIONALIZATION: Fee revenue is the lifeblood \
of a smart contract platform's value accrual to token holders. Arguing that destroying revenue was \
a deliberate strategy to benefit an ecosystem that COMPETES with the base layer is like saying a \
company reduced its margins to help its customers. The value accrual problem is structural.

BELOW SMA-200 IS A REGIME INDICATOR: ETH is trading below its 200-day moving average. Every \
professional asset allocator's risk system flags this as a warning. "It might go up" is insufficient \
justification for a long when the primary trend indicator is bearish.

PERP FUNDING RATE: The Bull notes neutral funding as a positive. But neutral funding into a \
macro-risk-off environment means longs accumulating at the wrong time. When macro accelerates down, \
funding flips negative FAST and perp longs get squeezed.

The most likely reason the RSI/price strength looks bullish here is that retail is buying what \
institutional money is selling. The $84M ETF outflow while price holds near highs = distribution.\
""",

"Trader": json.dumps({
    "action": "hold",
    "size_pct": 0,
    "price_offset_pct": 0.0,
    "rationale": "MACD bearish divergence (price higher high, histogram negative at -6.5), RSI approaching overbought at 67.8, CCI overbought at 118.2, and ETH trading BELOW its SMA-200 create a convergent warning cluster. The fundamental picture is deteriorating (fee revenue impaired, staking yield below risk-free rate, ETF outflows). Macro headwinds (rising dollar, hot PPI, yields at 4.72%) add additional pressure. There is insufficient evidence that the bullish contrarian case will materialize before the bearish technical case plays out.",
    "key_risk": "Holding incurs opportunity cost if ETH breaks above $2,520 (BB upper), which would invalidate the bearish divergence thesis and potentially signal a run toward $2,800."
}),

"Aggressive": json.dumps({
    "approved": False,
    "recommended_size_pct": 6,
    "key_concern": "MACD divergence plus below-SMA-200 is a regime warning that even the aggressive voice cannot ignore",
    "assessment": "Normally I support capturing moves even with mixed signals. But the convergence of MACD divergence, approaching overbought RSI, CCI already overbought, and fundamental fee revenue impairment is too many negatives at once. If I were to trade, I would take a very small speculative long (6%) with a tight stop at $2,280. But I do not strongly advocate for this. The bear case has more evidentiary support than the bull case here."
}),

"Neutral": json.dumps({
    "approved": False,
    "recommended_size_pct": 0,
    "key_concern": "Three simultaneous technical warning signals (MACD divergence, CCI overbought, below SMA-200) with deteriorating fundamentals do not meet our minimum threshold for a long position",
    "assessment": "The risk/reward is unfavorable. Taking a perp long at $2,381 with MACD divergence, CCI at 118 (overbought), and below SMA-200 means we are fighting three technical headwinds simultaneously. The fundamental picture (fee revenue impairment, ETF outflows, Solana competition) does not support a contrarian bid. Hold is the correct action. Revisit if ETH breaks above $2,520 with positive MACD reconvergence, or if it pulls back to $2,100 with RSI reset to 40."
}),

"Conservative": json.dumps({
    "approved": False,
    "recommended_size_pct": 0,
    "key_concern": "Capital preservation mandate is clear: do not enter longs below SMA-200 with bearish MACD divergence",
    "assessment": "The Conservative voice votes strongly against this trade. ETH is below its 200-day moving average — this is the single most important regime indicator I monitor. Entering a perp long below SMA-200 with MACD divergence and macro headwinds (rising dollar, yields at 4.72%) violates the capital preservation mandate. The probability-weighted outcome is negative. Hold cash. Wait for either a technical reset (RSI to 40-45 range) or a confirmed bullish reclaim of SMA-200."
}),

"FundManager": json.dumps({
    "self_reflection": "The Bearish Researcher's strongest point was the MACD divergence — price making higher highs while the histogram turns negative. The proposed trade (which the Trader correctly recommended as HOLD) does not need to account for this risk, because there is no trade. However, reviewing the risk team: all three voices either rejected the trade or proposed minimal size. The weighted vote is 0 × 0.5 + 0 × 0.3 + 6 × 0.2 = 1.2% — effectively zero. This is clear consensus for no action.",
    "action": "hold",
    "final_size_pct": 0,
    "price_offset_pct": 0.0,
    "confidence": 0.82,
    "reasoning": "Risk team consensus is overwhelming — two of three voices explicitly rejected the trade, and the Aggressive voice only proposed 6% with low conviction. Technical signals (MACD divergence, CCI overbought, below SMA-200) are convergent warnings. Fundamental picture is deteriorating. Macro is adverse (rising dollar, hot PPI). The correct action is to preserve capital and wait for a cleaner setup. Confidence in the HOLD decision is 82%.",
    "no_trade_reason": "Convergent technical warnings (MACD bearish divergence, CCI overbought at 118.2, below SMA-200 at $2,620) combined with deteriorating fundamentals (fee revenue impaired, ETF outflows) and adverse macro (DXY breaking 104.5, yields at 4.72%) do not meet minimum entry criteria for a perp long."
}),
}

HYPE_LLM = {

"Fundamental": """\
Reasoning: HYPE is a native exchange token for Hyperliquid DEX, a decentralized perpetuals platform. \
Traditional fundamental metrics are inapplicable, but protocol metrics are available.

(1) VALUATION: Market cap of $8.4B for a DEX that does $8B/day in volume. The P/Volume ratio of \
~1x/day is aggressive — it implies the market is pricing in sustained volume leadership. Binance, \
the world's largest CEX, trades at ~0.8x daily volume. HYPE is priced at a PREMIUM to Binance \
on this metric. This is only justifiable if volume growth is hyperbolic.

(2) PROFITABILITY: Hyperliquid collects maker/taker fees on $8B/day = ~$2.4M/day revenue at \
0.03% average fee. Annualized: ~$876M. Market cap / annualized fee revenue = 9.6x P/S. For a \
high-growth DEX, this is not absurd — but it leaves no margin for a volume deceleration scenario.

(3) LIQUIDITY RISKS: The oracle manipulation exploit ($4M loss) is a precedent, not a one-time \
event. Smart contract risk is the existential threat for a DeFi protocol. Additionally, the \
tokenomics concern — 76% team/investor allocation unlocking in Q3 — represents direct sell pressure \
from insiders who have $34/token (ATH) cost basis knowledge.

(4) GROWTH: Volume leadership is real but fragile. One competitor upgrade or CEX promotion could \
divert volume rapidly. The protocol has no moat beyond user habit and liquidity depth — both of \
which can change quickly in crypto.

Verdict: Bearish on risk/reward at current levels. The market is pricing perfection (sustained $8B+ \
volume, no exploits, continued retail FOMO) into a protocol with known structural risks.\
""",

"Technical": """\
Reasoning: HYPE at $22.40 displays a clear overbought technical picture with weakening momentum.

(1) MOMENTUM — RSI/MACD: RSI-14 at 73.6 is OVERBOUGHT (above 70). This is not borderline — at 73.6, \
historical mean reversion probability within 5–10 sessions is ~72%. More critically, the MACD \
histogram has turned NEGATIVE (-0.32) while price is near recent highs — a bearish divergence \
identical to the classic "distribution" signal. Momentum is definitively weakening.

(2) VOLATILITY — Bollinger Bands: Price at $22.40 is approaching the upper Bollinger Band at $24.90. \
The distance to the upper band is $2.50 (11% upside before technical resistance), while the \
distance to the midband is $1.60 (7% downside to mean). Risk/reward from a BB perspective: \
1.57:1 in favor of downside.

(3) TREND — SMAs/CCI: Price is above SMA-50 ($18.60) and SMA-200 ($11.30) — both bullish on the \
trend. However, CCI at 142.8 is deep into overbought territory (threshold: +100). This is the \
most extreme CCI reading in our dataset, suggesting the recent momentum is exhausted.

(4) OVERBOUGHT/OVERSOLD: OVERBOUGHT on all momentum indicators simultaneously: RSI 73.6, CCI 142.8, \
price near BB upper. Triple overbought convergence is a high-probability reversal signal.

Directional bias: BEARISH with 75% confidence. The triple overbought convergence plus MACD \
divergence is the most bearish technical setup in our three-scenario analysis. Target: $20.80 \
(BB midband). Potential: $18.60 (SMA-50) on momentum breakdown.\
""",

"Sentiment": """\
Reasoning: The HYPE news cluster is the most mixed of our three scenarios — retail euphoria \
colliding with structural concerns.

(1) SENTIMENT SCORE: -0.18 (mildly bearish). Despite price near all-time highs, the news cluster \
contains substantive bearish signals that distinguish informed concern from simple FUD.

(2) KEY NARRATIVE THEMES:
   • ATH of $34.90 — retail FOMO is peak, suggesting late-cycle dynamics
   • Oracle exploit precedent — smart contract risk is real and recently demonstrated
   • Tokenomics concern: 76% supply unlocking in Q3 — largest insider unlock is a defined event
   • Funding rate at +0.12%/8h — perp longs paying heavy carry, typically precedes correction
   • Competitor comparison: "challenging Binance dominance" is narrative framing, not metric

(3) CONTRARIAN SIGNALS: Volume leadership ($8B/day) is genuinely impressive. If HYPE maintains \
CEX-level volume as a DEX, the valuation premium is defensible. However, the FOMO language in \
retail coverage ("up 11x since launch") is a sentiment peak indicator.

(4) INVESTOR PSYCHOLOGY: FOMO-dominant retail with smart money concerns. The +0.12%/8h funding \
rate means perp longs are paying $1.44/day per $100 notional — an annualized carry of 526%. \
This funding rate is only rational if you expect a >526% annualized price gain. It signals \
overleveraged retail, not institutional conviction.\
""",

"Macro": """\
Reasoning: Macro backdrop for HYPE is the same risk-on environment as BTC, but HYPE's beta (2.41) \
means it amplifies macro moves in both directions.

(1) KEY MACRO RISKS: HYPE is a small-cap DeFi token with beta of 2.41 vs BTC's 1.48. In a risk-on \
environment, this has worked fantastically (11x from launch). In a risk-off repricing, HYPE would \
be expected to decline 2.41x BTC's decline rate. If BTC falls 20%, HYPE historically falls 48%.

(2) SECTOR TAILWINDS: DeFi / DEX sector is a risk-on beneficiary. The current mild risk-on \
environment (VIX low, SPY at ATH) has been HYPE's operating environment for its entire existence. \
The protocol has never been stress-tested in a macro risk-off event.

(3) CORRELATION: HYPE's correlation to BTC is +0.78 in bull markets but historically rises to +0.95 \
in sharp drawdowns — small-cap crypto deleverages with the market in a correlated cascade.

(4) NEAR-TERM PRICE IMPACT: Neutral macro backdrop, but at triple-overbought technical levels, HYPE \
does not need a macro catalyst to correct — the internal momentum structure is sufficient. A \
reversion to SMA-50 ($18.60) from $22.40 represents a -17% move on zero macro change.\
""",

"Bull_R1": """\
Round 1 — Bullish Case for HYPE:

Volume leadership creates a moat that competitors cannot replicate quickly.

NETWORK EFFECTS ARE REAL: $8B/day DEX volume is not user-habit — it is liquidity depth. Market \
makers on Hyperliquid have optimized their quoting strategies to the platform's architecture. \
Switching costs for market makers are HIGH — they have invested significant infrastructure. This \
creates a genuine liquidity moat that casual competitors cannot displace.

THE VALUATION MULTIPLE IS FRONT-RUNNING A CATEGORY WINNER: If Hyperliquid achieves even 40% of \
Binance's perpetual futures volume (currently ~$30B/day), revenue would triple and the current \
market cap would look cheap. The DEX perp category is in its early innings.

TOKENOMICS FEAR IS OVERDONE: The 76% unlock concern assumes insiders will dump. But insider \
rationality in a growing protocol is to hold, not sell — the marginal price impact of insider \
selling would exceed their gain from liquidating early. Sophisticated founders understand this.

OVERBOUGHT INDICATORS ARE RELATIVE: RSI-73 in a trending asset can stay above 70 for weeks in \
strong uptrends. BTC stayed above RSI-70 for 21 consecutive days during its Q4 2020 rally. \
Volume leadership + FOMO narrative = sustained overbought conditions.\
""",

"Bear_R1": """\
Round 1 — Bearish Rebuttal — Why the Bullish Signal Is a Trap:

The Bull has constructed a compelling narrative for a protocol that is technically screaming SELL.

RSI 73 + CCI 142 + MACD DIVERGENCE = TRIPLE OVERBOUGHT: The Bull points to BTC staying above \
RSI-70 for 21 days in 2020. That is a cherry-picked example from the highest-conviction macro \
bull market in crypto history. HYPE is a small-cap DeFi token with 2.41 beta — its volatility is \
exponentially higher. RSI-70 for HYPE typically resolves with 20–35% corrections, not sustained \
uptrends. The comparison is inappropriate.

THE INSIDER UNLOCK IS A SCHEDULED LIQUIDATION EVENT: The Bull argues insiders won't sell because \
it would hurt them. This is naive. Venture capital funds have LPs who expect distributions. \
Employees who received HYPE at a $0.10 equivalent cost basis WILL sell at $22. The unlock schedule \
is PUBLIC information — sophisticated traders will SHORT into the event, creating a self-fulfilling \
sell cascade regardless of insider intentions.

THE ORACLE EXPLOIT IS AN EXISTENTIAL PRECEDENT: A $4M exploit was covered by the Hyperliquid \
team — this time. The team's willingness to cover losses reveals the protocol is NOT decentralized \
in the way that justifies a DEX premium over CEX valuation. It is centralized risk with \
decentralized branding.

THE MOAT ARGUMENT IGNORES DEX HISTORY: Every DEX that achieved volume leadership has subsequently \
lost it to a competitor. Uniswap → Curve → dYdX → GMX → Hyperliquid. The category leader \
changes every 18–24 months. HYPE is valued as if it is the permanent winner.\
""",

"Trader": json.dumps({
    "action": "sell",
    "size_pct": 7,
    "price_offset_pct": 0.5,
    "rationale": "Triple overbought convergence (RSI 73.6, CCI 142.8, price near BB upper) with MACD bearish divergence is the strongest sell signal in our three-scenario analysis. The fundamental picture includes a scheduled Q3 insider unlock (76% of supply), oracle exploit precedent, and valuation at a premium to Binance on a P/Volume basis. If already holding HYPE, a 7% portfolio reduction here captures exit liquidity while FOMO demand absorbs the sell.",
    "key_risk": "Momentum assets can remain overbought longer than expected. A breakout above $24.90 (BB upper) with volume would invalidate the bearish divergence and potentially target $28-30. Setting stop-loss at $24.50 limits damage if the bull case accelerates."
}),

"Aggressive": json.dumps({
    "approved": True,
    "recommended_size_pct": 10,
    "key_concern": "Stop at $24.50 must be strict — a breakout above BB upper would signal momentum resumption",
    "assessment": "The triple overbought setup is one of the cleaner technical sell signals I've seen. RSI 73.6, CCI 142.8, MACD divergence — all three aligning is uncommon and historically reliable. I'd actually sell a bit more aggressively (10%) given the insider unlock overhang. The BB upper at $24.90 is clear resistance; probability of a clean reversal from here is >70%. The funding rate (+0.12%/8h) means the crowd is over-leveraged long — any reversal will accelerate on liquidations."
}),

"Neutral": json.dumps({
    "approved": True,
    "recommended_size_pct": 7,
    "key_concern": "This is a tactical trade, not a structural short — the protocol may have genuine long-term value",
    "assessment": "The technical setup strongly supports a sell/trim at current levels. However, we should be clear: this is a risk management trade, not a fundamental short. HYPE's volume leadership and fee revenue model have merit. At 7% portfolio size, we capture the mean-reversion toward BB midband ($20.80) or SMA-50 ($18.60) without over-committing to a directional short bet. Stop at $24.50, target $20.00."
}),

"Conservative": json.dumps({
    "approved": True,
    "recommended_size_pct": 5,
    "key_concern": "Shorting a high-beta small cap with active FOMO narrative carries gap-up risk during broad market rallies",
    "assessment": "The Conservative voice approves a modest sell (5%) based on the overwhelming overbought technical evidence. However, I note that shorting any asset with 2.41 beta in a risk-on macro environment carries meaningful gap-up risk. The insider unlock overhang is the strongest fundamental justification for the trade — it is a defined, dated event. Size at 5%, hard stop at $24.00, take profit at $19.50."
}),

"FundManager": json.dumps({
    "self_reflection": "The Bearish Researcher's strongest point is the insider unlock event — a scheduled, public, irreversible supply expansion. This is not speculation; it is a known future catalyst. Our 7% sell position does this risk account for this? Yes — we are not short the maximum, and we have a defined stop at $24.50. The Conservative voice recommended 5%; the Neutral 7%; the Aggressive 10%. Weighted: 5*0.5 + 7*0.3 + 10*0.2 = 2.5 + 2.1 + 2.0 = 6.6%, rounded to 7%. The self-reflection confirms size is appropriate — meaningful but not reckless given the binary unlock event risk.",
    "action": "sell",
    "final_size_pct": 7,
    "price_offset_pct": 0.5,
    "confidence": 0.74,
    "reasoning": "Weighted vote yields 6.6% → 7%. All three risk voices approved the trade with unanimous direction (sell/trim). The technical setup is the strongest sell signal in our three scenarios: RSI 73.6 (overbought), CCI 142.8 (deep overbought), MACD bearish divergence, and price approaching BB upper. Fundamental support: scheduled Q3 insider unlock (76% supply), oracle exploit precedent, valuation premium to CEX comparable. Trade structure: sell 7% at +0.5% offset to current price, stop at $24.50, target $19.50 (SMA-50 zone).",
    "no_trade_reason": None
}),
}

# Fintool mock responses (exchange API confirmations)
FINTOOL_RESPONSES = {
    "buy_confirmed": {
        "status": "ok",
        "order_id": "HL:7842931",
        "symbol": "BTC",
        "side": "buy",
        "type": "limit",
        "amount": 0.009348,
        "price": 63_859.1,
        "filled": 0.009348,
        "status_detail": "filled",
        "fee": 0.0019,
        "timestamp": "2025-03-11T14:32:07Z",
    },
    "sell_confirmed": {
        "status": "ok",
        "order_id": "HL:7842998",
        "symbol": "HYPE",
        "side": "sell",
        "type": "limit",
        "amount": 15.624,
        "price": 22.512,
        "filled": 15.624,
        "status_detail": "filled",
        "fee": 0.0035,
        "timestamp": "2025-03-11T14:35:22Z",
    },
}

# All scenarios in one registry
SCENARIOS = {
    "btc": {
        "label":            "BTC / Hyperliquid Spot — Bullish Setup",
        "ticker":           "BTC",
        "exchange":         "hyperliquid",
        "market_type":      "spot",
        "portfolio_usd":    5_000.0,
        "debate_rounds":    2,
        "fundamental":      BTC_FUNDAMENTAL,
        "technical":        BTC_TECHNICAL,
        "sentiment":        BTC_SENTIMENT,
        "macro":            BTC_MACRO,
        "llm":              BTC_LLM,
        "price":            BTC_TECHNICAL["current_price"],
        "fintool_confirm":  FINTOOL_RESPONSES["buy_confirmed"],
    },
    "eth": {
        "label":            "ETH / Binance Perp — Mixed Signals → HOLD",
        "ticker":           "ETH",
        "exchange":         "binance",
        "market_type":      "perp",
        "portfolio_usd":    10_000.0,
        "debate_rounds":    2,
        "fundamental":      ETH_FUNDAMENTAL,
        "technical":        ETH_TECHNICAL,
        "sentiment":        ETH_SENTIMENT,
        "macro":            ETH_MACRO,
        "llm":              ETH_LLM,
        "price":            ETH_TECHNICAL["current_price"],
        "fintool_confirm":  None,
    },
    "hype": {
        "label":            "HYPE / Hyperliquid Spot — Overbought → SELL",
        "ticker":           "HYPE",
        "exchange":         "hyperliquid",
        "market_type":      "spot",
        "portfolio_usd":    3_500.0,
        "debate_rounds":    2,
        "fundamental":      HYPE_FUNDAMENTAL,
        "technical":        HYPE_TECHNICAL,
        "sentiment":        HYPE_SENTIMENT,
        "macro":            HYPE_MACRO,
        "llm":              HYPE_LLM,
        "price":            HYPE_TECHNICAL["current_price"],
        "fintool_confirm":  FINTOOL_RESPONSES["sell_confirmed"],
    },
}

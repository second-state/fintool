#!/usr/bin/env python3
"""
TradingAgents — Multi-agent LLM trading system powered by Claude and fintool.

Implements the TradingAgents.md framework across three phases:
  Phase I:   Analyst Team  — Fundamental, Technical, Sentiment, Macro analysts
  Phase II:  Researcher Team — Bull/Bear dialectical debate (ReAct + adaptive thinking)
  Phase III: Execution Team — Trader, Risk Management (3 voices), Fund Manager → fintool

Requirements:
    pip install anthropic yfinance pandas

Usage:
    python trading_agents.py BTC --exchange hyperliquid --portfolio 1000
    python trading_agents.py ETH --exchange binance --market perp --rounds 3 --execute
    python trading_agents.py HYPE --exchange hyperliquid --portfolio 500 --output state.json

Environment:
    ANTHROPIC_API_KEY — required
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from dataclasses import asdict, dataclass, field
from typing import Optional

import anthropic

try:
    import pandas as pd
    import yfinance as yf
    HAS_YFINANCE = True
except ImportError:
    HAS_YFINANCE = False

# ──────────────────────────────────────────────────────────────
# Models
#   Phase I  — Haiku 4.5: fast, structured data-to-text tasks
#   Phase II/III — Opus 4.6 + adaptive thinking: deep reasoning
# ──────────────────────────────────────────────────────────────
FAST_MODEL = "claude-haiku-4-5"
DEEP_MODEL = "claude-opus-4-6"

KNOWN_EXCHANGES = ("hyperliquid", "binance", "coinbase", "okx", "polymarket")


# ──────────────────────────────────────────────────────────────
# Global State  (the "brain's memory" shared across all agents)
# ──────────────────────────────────────────────────────────────

@dataclass
class GlobalState:
    """Centralized JSON-like state object — all agents read from and write to this."""
    ticker:             str
    exchange:           str         # e.g. "hyperliquid", "binance"
    market_type:        str         # "spot" or "perp"
    portfolio_size_usd: float
    debate_rounds:      int = 2

    # Populated sequentially through the pipeline
    market_data:        dict = field(default_factory=dict)
    analyst_summaries:  dict = field(default_factory=dict)
    debate_log:         list = field(default_factory=list)
    risk_assessment:    dict = field(default_factory=dict)
    final_decision:     dict = field(default_factory=dict)

    def to_context(self) -> str:
        return json.dumps(asdict(self), indent=2)


# ──────────────────────────────────────────────────────────────
# fintool integration helpers
# ──────────────────────────────────────────────────────────────

def _find_binary(name: str) -> str:
    """Return the path to a fintool binary (PATH → ./target/release/)."""
    if shutil.which(name):
        return name
    for suffix in (".exe", ""):
        local = os.path.join(".", "target", "release", name + suffix)
        if os.path.isfile(local):
            return local
    return name  # will surface a FileNotFoundError on use


def run_fintool(binary: str, json_command: dict) -> dict:
    """Execute a fintool binary with --json mode and return the parsed JSON output."""
    exe = _find_binary(binary)
    cmd = [exe, "--json", json.dumps(json_command)]
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        if not result.stdout.strip():
            return {"error": result.stderr.strip() or "No output from fintool"}
        return json.loads(result.stdout.strip())
    except subprocess.TimeoutExpired:
        return {"error": f"{binary} command timed out"}
    except json.JSONDecodeError as e:
        return {"error": f"Invalid JSON from {binary}: {e}", "raw": result.stdout[:400]}
    except FileNotFoundError:
        return {"error": f"Binary not found: {binary}. Build with `cargo build --release`."}


# ──────────────────────────────────────────────────────────────
# Data tool implementations  (the "Available Toolset" from TradingAgents.md)
# ──────────────────────────────────────────────────────────────

def get_fundamental_data(ticker: str) -> dict:
    """P/E, ROE, Debt/Equity, liquidity ratios via yfinance."""
    if not HAS_YFINANCE:
        return {"error": "yfinance not installed — pip install yfinance pandas"}
    try:
        info = yf.Ticker(ticker).info
        return {
            "pe_ratio":       info.get("trailingPE"),
            "forward_pe":     info.get("forwardPE"),
            "roe":            info.get("returnOnEquity"),
            "roa":            info.get("returnOnAssets"),
            "debt_to_equity": info.get("debtToEquity"),
            "current_ratio":  info.get("currentRatio"),
            "quick_ratio":    info.get("quickRatio"),
            "profit_margin":  info.get("profitMargins"),
            "revenue_growth": info.get("revenueGrowth"),
            "earnings_growth":info.get("earningsGrowth"),
            "market_cap":     info.get("marketCap"),
            "price_to_book":  info.get("priceToBook"),
            "beta":           info.get("beta"),
            "52w_high":       info.get("fiftyTwoWeekHigh"),
            "52w_low":        info.get("fiftyTwoWeekLow"),
            "sector":         info.get("sector"),
            "industry":       info.get("industry"),
        }
    except Exception as e:
        return {"error": str(e)}


def get_technical_indicators(ticker: str) -> dict:
    """RSI, MACD, Bollinger Bands, CCI, SMAs via yfinance + pandas."""
    if not HAS_YFINANCE:
        return {"error": "yfinance not installed — pip install yfinance pandas"}
    try:
        hist = yf.Ticker(ticker).history(period="6mo")
        if hist.empty:
            return {"error": "No price history found"}

        close = hist["Close"]
        high  = hist["High"]
        low   = hist["Low"]
        vol   = hist["Volume"]

        # RSI-14
        delta = close.diff()
        gain  = delta.clip(lower=0).rolling(14).mean()
        loss  = (-delta.clip(upper=0)).rolling(14).mean()
        rs    = gain / loss.replace(0, float("inf"))
        rsi   = float((100 - 100 / (1 + rs)).iloc[-1])

        # MACD (12, 26, 9)
        ema12    = close.ewm(span=12, adjust=False).mean()
        ema26    = close.ewm(span=26, adjust=False).mean()
        macd_val = float((ema12 - ema26).iloc[-1])
        sig_val  = float((ema12 - ema26).ewm(span=9, adjust=False).mean().iloc[-1])

        # Bollinger Bands (20-period, 2σ)
        sma20    = close.rolling(20).mean()
        std20    = close.rolling(20).std()
        bb_upper = float((sma20 + 2 * std20).iloc[-1])
        bb_mid   = float(sma20.iloc[-1])
        bb_lower = float((sma20 - 2 * std20).iloc[-1])

        # CCI-20
        tp     = (high + low + close) / 3
        tp_ma  = tp.rolling(20).mean()
        tp_md  = tp.rolling(20).apply(lambda x: (x - x.mean()).abs().mean())
        cci    = float(((tp - tp_ma) / (0.015 * tp_md)).iloc[-1])

        sma50  = float(close.rolling(50).mean().iloc[-1])
        sma200 = float(close.rolling(200).mean().iloc[-1])
        price  = float(close.iloc[-1])

        return {
            "current_price":   round(price, 4),
            "rsi_14":          round(rsi, 2),
            "macd":            round(macd_val, 4),
            "macd_signal":     round(sig_val, 4),
            "macd_histogram":  round(macd_val - sig_val, 4),
            "bb_upper":        round(bb_upper, 4),
            "bb_mid":          round(bb_mid, 4),
            "bb_lower":        round(bb_lower, 4),
            "cci_20":          round(cci, 2),
            "sma_50":          round(sma50, 4),
            "sma_200":         round(sma200, 4),
            "vol_avg_20":      int(vol.rolling(20).mean().iloc[-1]),
            "price_vs_sma50":  "above" if price > sma50  else "below",
            "price_vs_sma200": "above" if price > sma200 else "below",
        }
    except Exception as e:
        return {"error": str(e)}


def get_sentiment_score(ticker: str) -> dict:
    """News headlines for sentiment analysis via fintool news."""
    data = run_fintool("fintool", {"command": "news", "symbol": ticker})
    if "error" in data:
        return {"ticker": ticker, "headlines": [], "error": data["error"]}
    # fintool news may return a list directly or {"articles": [...]}
    if isinstance(data, list):
        headlines = data
    else:
        headlines = data.get("articles", data.get("headlines", []))
    return {
        "ticker":    ticker,
        "headlines": headlines[:12],
        "count":     len(headlines),
    }


def get_macro_news() -> dict:
    """Macro environment headlines for SPY, gold, and DXY via fintool."""
    results = {}
    for symbol in ("SPY", "GOLD", "DXY"):
        data = run_fintool("fintool", {"command": "news", "symbol": symbol})
        if "error" not in data:
            items = data if isinstance(data, list) else data.get("articles", [])
            results[symbol] = items[:5]
    return results


def get_current_price(ticker: str, exchange: str) -> Optional[float]:
    """Fetch live price via fintool quote, falling back to yfinance."""
    # Try exchange-specific quote for crypto (Hyperliquid/Binance/OKX perp quotes)
    if exchange in ("hyperliquid", "binance", "okx"):
        data = run_fintool(exchange, {"command": "quote", "symbol": ticker})
        for key in ("price", "mark_price", "mid", "last"):
            if data.get(key):
                try:
                    return float(data[key])
                except (TypeError, ValueError):
                    pass
    # Fall back to fintool (Yahoo Finance + CoinGecko)
    data = run_fintool("fintool", {"command": "quote", "symbol": ticker})
    if data.get("price"):
        try:
            return float(data["price"])
        except (TypeError, ValueError):
            pass
    # Last resort: yfinance
    if HAS_YFINANCE:
        try:
            hist = yf.Ticker(ticker).history(period="1d")
            if not hist.empty:
                return float(hist["Close"].iloc[-1])
        except Exception:
            pass
    return None


# ──────────────────────────────────────────────────────────────
# LLM call helpers
# ──────────────────────────────────────────────────────────────

def _extract_text(content: list) -> str:
    """Return the last text block from a message content list."""
    for block in reversed(content):
        if block.type == "text":
            return block.text
    return ""


def llm_call(
    client:       anthropic.Anthropic,
    system:       str,
    user:         str,
    model:        str  = FAST_MODEL,
    thinking:     bool = False,
    max_tokens:   int  = 2048,
) -> str:
    """Stream a Claude call and return the text response."""
    kwargs: dict = dict(
        model=model,
        max_tokens=max_tokens,
        system=system,
        messages=[{"role": "user", "content": user}],
    )
    if thinking:
        kwargs["thinking"] = {"type": "adaptive"}

    with client.messages.stream(**kwargs) as stream:
        return _extract_text(stream.get_final_message().content)


def llm_json(
    client:     anthropic.Anthropic,
    system:     str,
    user:       str,
    model:      str  = FAST_MODEL,
    thinking:   bool = False,
    max_tokens: int  = 2048,
) -> dict:
    """Stream a Claude call expecting a JSON response; parse and return as dict."""
    sys_json = system + "\n\nIMPORTANT: Respond with a single valid JSON object only. No markdown fences, no surrounding text."
    raw = llm_call(client, sys_json, user, model, thinking, max_tokens).strip()

    # Strip ```json ... ``` if present
    raw = re.sub(r"^```(?:json)?\s*", "", raw)
    raw = re.sub(r"\s*```$", "", raw)

    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        # Try to extract the first JSON object
        match = re.search(r"\{.*\}", raw, re.DOTALL)
        if match:
            try:
                return json.loads(match.group())
            except json.JSONDecodeError:
                pass
        return {"error": "JSON parse failed", "raw": raw[:400]}


# ──────────────────────────────────────────────────────────────
# Phase I — Analyst Team
# ──────────────────────────────────────────────────────────────

def _fundamental_analyst(client: anthropic.Anthropic, state: GlobalState) -> str:
    data = get_fundamental_data(state.ticker)
    state.market_data["fundamental"] = data
    return llm_call(
        client,
        system=(
            "You are the Fundamental Analyst on a multi-agent trading team. "
            "Evaluate financial health using ROE, ROA, P/E, debt/equity, and liquidity ratios. "
            "Follow the ReAct framework: reason first, then conclude. "
            "Flag both opportunities and risks with specific numbers."
        ),
        user=(
            f"Analyze the fundamentals for {state.ticker}:\n\n"
            f"{json.dumps(data, indent=2)}\n\n"
            "Cover: (1) valuation, (2) profitability, (3) liquidity risks, (4) growth. "
            "End with a one-sentence bull/bear verdict."
        ),
    )


def _technical_analyst(client: anthropic.Anthropic, state: GlobalState) -> str:
    data = get_technical_indicators(state.ticker)
    state.market_data["technical"] = data
    return llm_call(
        client,
        system=(
            "You are the Technical Analyst on a multi-agent trading team. "
            "Forecast near-term price using MACD, RSI, CCI, and Bollinger Bands. "
            "Follow the ReAct framework. Be specific about signal strength."
        ),
        user=(
            f"Analyze the technical indicators for {state.ticker}:\n\n"
            f"{json.dumps(data, indent=2)}\n\n"
            "Assess: (1) momentum (RSI, MACD), (2) volatility (BBands), "
            "(3) trend (SMAs, CCI), (4) overbought/oversold. "
            "Conclude with directional bias and confidence level."
        ),
    )


def _sentiment_analyst(client: anthropic.Anthropic, state: GlobalState) -> str:
    data = get_sentiment_score(state.ticker)
    state.market_data["sentiment"] = data
    headlines_text = "\n".join(
        f"• {h}" if isinstance(h, str) else f"• {h.get('title', str(h))}"
        for h in data.get("headlines", [])
    ) or "No headlines available."
    return llm_call(
        client,
        system=(
            "You are the Sentiment Analyst on a multi-agent trading team. "
            "Score market sentiment from -1.0 (extreme fear/bearish) to +1.0 (extreme greed/bullish). "
            "Follow the ReAct framework."
        ),
        user=(
            f"Analyze sentiment for {state.ticker} from recent headlines:\n\n"
            f"{headlines_text}\n\n"
            "Provide: (1) sentiment score, (2) key narrative themes, "
            "(3) contrarian signals, (4) overall investor psychology."
        ),
    )


def _news_analyst(client: anthropic.Anthropic, state: GlobalState) -> str:
    data = get_macro_news()
    state.market_data["macro"] = data
    return llm_call(
        client,
        system=(
            "You are the Macro/News Analyst on a multi-agent trading team. "
            "Monitor macroeconomic events (Fed, CPI, geopolitics) and their sector-specific impact. "
            "Follow the ReAct framework."
        ),
        user=(
            f"Analyze the macro environment as it relates to {state.ticker} "
            f"on {state.exchange} ({state.market_type}):\n\n"
            f"{json.dumps(data, indent=2)}\n\n"
            "Cover: (1) key macro risks, (2) sector tailwinds/headwinds, "
            "(3) correlation to macro events, (4) near-term price impact."
        ),
    )


def run_analyst_team(client: anthropic.Anthropic, state: GlobalState) -> None:
    print("\n[Phase I] Analyst Team")
    steps = [
        ("Fundamental", _fundamental_analyst),
        ("Technical",   _technical_analyst),
        ("Sentiment",   _sentiment_analyst),
        ("Macro",       _news_analyst),
    ]
    for name, fn in steps:
        print(f"  [{name} Analyst] Running...")
        state.analyst_summaries[name] = fn(client, state)
        print(f"  [{name} Analyst] ✓")


# ──────────────────────────────────────────────────────────────
# Phase II — Researcher Team (Dialectical Debate)
# ──────────────────────────────────────────────────────────────

def run_researcher_team(client: anthropic.Anthropic, state: GlobalState) -> None:
    print(f"\n[Phase II] Researcher Team — {state.debate_rounds} debate rounds")

    analyst_ctx = json.dumps(state.analyst_summaries, indent=2)
    tech_ctx    = json.dumps(state.market_data.get("technical", {}), indent=2)

    bull_sys = (
        "You are the Bullish Researcher on a trading team. You argue for growth and long-term "
        "potential. Cite specific data from analyst reports. Counter bearish arguments directly. "
        "Be rigorous and data-driven, not cheerleading."
    )
    bear_sys = (
        "You are the Bearish Researcher on a trading team. You stress-test the investment thesis "
        "by focusing on valuation risks, downside scenarios, and counter-evidence. "
        "When technical indicators appear bullish, explain why that signal is likely a trap. "
        "Be specific — cite data points that support caution."
    )

    for round_num in range(1, state.debate_rounds + 1):
        print(f"  [Round {round_num}/{state.debate_rounds}]", end=" ", flush=True)

        prior = (
            "\n\nPrevious rounds:\n" + json.dumps(state.debate_log[-2:], indent=2)
            if state.debate_log else ""
        )
        base = (
            f"Asset: {state.ticker} | Exchange: {state.exchange} | Market: {state.market_type}\n\n"
            f"ANALYST REPORTS:\n{analyst_ctx}\n\nTECHNICAL DATA:\n{tech_ctx}{prior}"
        )

        # Bullish argument
        bull_arg = llm_call(
            client, bull_sys,
            base + f"\n\nRound {round_num}: Make your strongest bullish case with specific data.",
            DEEP_MODEL, thinking=True, max_tokens=1500,
        )
        print("Bull ✓", end=" ", flush=True)

        # Bearish rebuttal — Facilitator explicitly asks the required question
        bear_arg = llm_call(
            client, bear_sys,
            base + (
                f"\n\nRound {round_num} — The Bull just argued:\n{bull_arg}\n\n"
                "Counter this. Given the Technical Analyst's signals, "
                "what is the most likely reason any bullish signal here is a trap? "
                "Be specific about the downside risks the Bull is ignoring."
            ),
            DEEP_MODEL, thinking=True, max_tokens=1500,
        )
        print("Bear ✓")

        state.debate_log.append({"Round": round_num, "Bull": bull_arg, "Bear": bear_arg})


# ──────────────────────────────────────────────────────────────
# Phase III — Execution & Oversight
# ──────────────────────────────────────────────────────────────

def _trader(client: anthropic.Anthropic, state: GlobalState) -> dict:
    return llm_json(
        client,
        system=(
            "You are the Trader. Synthesize analyst reports and the Bull/Bear debate "
            "into a concrete trade proposal. Be specific about direction, size, and rationale.\n\n"
            "Respond with JSON:\n"
            '{"action": "buy"|"sell"|"hold", '
            '"size_pct": <0-100, % of portfolio>, '
            '"price_offset_pct": <-3 to 3, limit price offset from current>, '
            '"rationale": "...", '
            '"key_risk": "..."}'
        ),
        user=(
            f"Asset: {state.ticker} | Exchange: {state.exchange} | Market: {state.market_type}\n"
            f"Portfolio: ${state.portfolio_size_usd:,.2f} USD\n\n"
            f"ANALYST SUMMARIES:\n{json.dumps(state.analyst_summaries, indent=2)}\n\n"
            f"DEBATE LOG (last round):\n{json.dumps(state.debate_log[-1:], indent=2)}\n\n"
            f"TECHNICAL DATA:\n{json.dumps(state.market_data.get('technical', {}), indent=2)}\n\n"
            "Propose a specific trade. If conviction is low, set action to 'hold'."
        ),
        model=DEEP_MODEL, thinking=True, max_tokens=1500,
    )


def _risk_voice(client: anthropic.Anthropic, state: GlobalState, trader_proposal: dict, voice: str) -> dict:
    personas = {
        "Aggressive":   "You favor capturing upside and accept higher risk for higher reward. Support larger sizes when conviction is present.",
        "Neutral":      "You balance risk and reward objectively, looking for asymmetric setups and fair position sizing.",
        "Conservative": "Capital preservation is your primary mandate. You flag extreme volatility, liquidity risks, and worst-case scenarios. You would rather miss a trade than risk significant drawdown.",
    }
    bear_point = state.debate_log[-1]["Bear"][:500] if state.debate_log else "N/A"
    return llm_json(
        client,
        system=(
            f"You are the {voice} voice on the Risk Management Team. {personas[voice]}\n\n"
            "Respond with JSON:\n"
            '{"approved": true|false, '
            '"recommended_size_pct": <0-100>, '
            '"key_concern": "...", '
            '"assessment": "..."}'
        ),
        user=(
            f"Review this trade proposal for {state.ticker}:\n\n"
            f"PROPOSAL:\n{json.dumps(trader_proposal, indent=2)}\n\n"
            f"PORTFOLIO: ${state.portfolio_size_usd:,.2f} USD | MARKET: {state.market_type}\n\n"
            f"BEARISH RESEARCHER'S STRONGEST POINT:\n{bear_point}\n\n"
            "Provide your risk assessment and recommended size adjustment."
        ),
        model=DEEP_MODEL, thinking=True, max_tokens=1000,
    )


def _fund_manager(client: anthropic.Anthropic, state: GlobalState, trader_proposal: dict) -> dict:
    risk = state.risk_assessment
    # Weighted vote: Conservative 50%, Neutral 30%, Aggressive 20%
    weighted_size = (
        risk.get("Conservative", {}).get("recommended_size_pct", 0) * 0.50
        + risk.get("Neutral",    {}).get("recommended_size_pct", 0) * 0.30
        + risk.get("Aggressive", {}).get("recommended_size_pct", 0) * 0.20
    )
    bear_point = state.debate_log[-1]["Bear"][:800] if state.debate_log else "N/A"

    return llm_json(
        client,
        system=(
            "You are the Fund Manager. You resolve risk deliberation using weighted voting "
            "(Conservative: 50%, Neutral: 30%, Aggressive: 20%) and authorize execution.\n\n"
            "MANDATORY Self-Reflection before authorizing: "
            "'Review the Bearish Researcher's strongest point. Does the proposed trade size "
            "adequately account for this specific risk?'\n\n"
            "If the team could not reach consensus or risk is extreme, default to Hold.\n\n"
            "Respond with JSON:\n"
            '{"self_reflection": "...", '
            '"action": "buy"|"sell"|"hold", '
            '"final_size_pct": <0-100>, '
            '"price_offset_pct": <-5 to 5>, '
            '"confidence": <0.0-1.0>, '
            '"reasoning": "...", '
            '"no_trade_reason": null|"..."}'
        ),
        user=(
            f"ASSET: {state.ticker} | EXCHANGE: {state.exchange} | MARKET: {state.market_type}\n"
            f"PORTFOLIO: ${state.portfolio_size_usd:,.2f} USD\n\n"
            f"TRADER PROPOSAL:\n{json.dumps(trader_proposal, indent=2)}\n\n"
            f"RISK ASSESSMENT:\n{json.dumps(risk, indent=2)}\n\n"
            f"WEIGHTED SIZE (C50/N30/A20): {weighted_size:.1f}%\n\n"
            f"BEARISH RESEARCHER'S STRONGEST POINT:\n{bear_point}\n\n"
            "Perform your mandatory Self-Reflection, then authorize or reject the trade."
        ),
        model=DEEP_MODEL, thinking=True, max_tokens=2000,
    )


def run_execution_team(client: anthropic.Anthropic, state: GlobalState) -> None:
    print("\n[Phase III] Execution & Oversight")

    print("  [Trader] Synthesizing debate...", end=" ", flush=True)
    trader_proposal = _trader(client, state)
    print(f"→ {trader_proposal.get('action','?').upper()} {trader_proposal.get('size_pct','?')}%")

    print("  [Risk Management] Three-voice assessment...")
    state.risk_assessment = {}
    for voice in ("Aggressive", "Neutral", "Conservative"):
        print(f"    [{voice}]", end=" ", flush=True)
        state.risk_assessment[voice] = _risk_voice(client, state, trader_proposal, voice)
        a = state.risk_assessment[voice]
        mark = "✓" if a.get("approved") else "✗"
        print(f"{mark} → {a.get('recommended_size_pct','?')}% | {a.get('key_concern','')[:60]}")

    print("  [Fund Manager] Self-reflection + final call...", end=" ", flush=True)
    final = _fund_manager(client, state, trader_proposal)
    state.final_decision = final

    action = final.get("action", "hold").upper()
    size   = final.get("final_size_pct", 0)
    conf   = final.get("confidence", 0)
    print(f"→ {action} | {size}% | {conf:.0%} confidence")
    print(f"\n  Self-reflection: {final.get('self_reflection','N/A')[:180]}...")
    if final.get("no_trade_reason"):
        print(f"  No-trade reason: {final['no_trade_reason']}")


# ──────────────────────────────────────────────────────────────
# Trade execution via fintool
# ──────────────────────────────────────────────────────────────

def _build_fintool_command(state: GlobalState) -> Optional[dict]:
    """
    Translate GlobalState.final_decision into a fintool JSON command dict.
    Returns None if no trade should be executed.
    """
    decision = state.final_decision
    action   = decision.get("action", "hold").lower()
    size_pct = float(decision.get("final_size_pct", 0))
    conf     = float(decision.get("confidence", 0))

    if action == "hold" or size_pct == 0 or conf < 0.35:
        return None

    price = get_current_price(state.ticker, state.exchange)
    if price is None:
        price = state.market_data.get("technical", {}).get("current_price")
    if not price:
        return None

    offset_pct  = float(decision.get("price_offset_pct", 0))
    limit_price = round(price * (1 + offset_pct / 100), 6)
    trade_usd   = state.portfolio_size_usd * (size_pct / 100)
    amount      = round(trade_usd / limit_price, 6)

    # fintool command name: "buy" / "sell" for spot; "perp_buy" / "perp_sell" for perp
    if state.market_type == "perp":
        command = f"perp_{action}"
    else:
        command = action

    return {
        "_meta": {
            "trade_usd":     round(trade_usd, 2),
            "current_price": price,
            "limit_price":   limit_price,
        },
        "command": command,
        "symbol":  state.ticker,
        "amount":  amount,
        "price":   limit_price,
    }


def execute_trade(state: GlobalState, dry_run: bool = True) -> dict:
    """Execute (or simulate) the trade via fintool CLI."""
    decision = state.final_decision
    action   = decision.get("action", "hold").lower()
    conf     = decision.get("confidence", 0)

    print(f"\n[Execution] fintool trade dispatch")
    print(f"  Decision:   {action.upper()} {state.ticker} on {state.exchange} ({state.market_type})")
    print(f"  Confidence: {conf:.0%}")
    print(f"  Reasoning:  {decision.get('reasoning','N/A')[:200]}")

    if action == "hold":
        print("  → No trade. Position unchanged.")
        return {"status": "no_action", "reason": decision.get("no_trade_reason", "hold")}

    cmd = _build_fintool_command(state)
    if cmd is None:
        print("  ⚠ Cannot compute trade parameters (missing price or zero size).")
        return {"status": "no_action", "reason": "parameter computation failed"}

    meta = cmd.pop("_meta")
    print(f"\n  Parameters:")
    print(f"    Amount:      {cmd['amount']} {cmd['symbol']}")
    print(f"    Limit price: ${meta['limit_price']:,.6f}  (current ~${meta['current_price']:,.4f})")
    print(f"    Trade value: ${meta['trade_usd']:,.2f} USD")
    print(f"\n  fintool CLI:")
    print(f"    {state.exchange} --json '{json.dumps(cmd)}'")

    if dry_run:
        print("\n  [DRY RUN] Trade not submitted. Pass --execute to submit.")
        return {"status": "dry_run", "exchange": state.exchange, "command": cmd, "meta": meta}

    print("\n  Submitting...")
    result = run_fintool(state.exchange, cmd)
    if "error" in result:
        print(f"  ✗ Submission failed: {result['error']}")
        return {"status": "error", "error": result["error"]}

    print(f"  ✓ Trade submitted!")
    print(f"  {json.dumps(result, indent=4)}")
    return {"status": "success", "result": result, "meta": meta}


# ──────────────────────────────────────────────────────────────
# Orchestrator
# ──────────────────────────────────────────────────────────────

def run_trading_session(
    ticker:             str,
    exchange:           str,
    market_type:        str,
    portfolio_size_usd: float,
    debate_rounds:      int  = 2,
    execute:            bool = False,
) -> GlobalState:
    client = anthropic.Anthropic()   # reads ANTHROPIC_API_KEY from env

    state = GlobalState(
        ticker=ticker,
        exchange=exchange,
        market_type=market_type,
        portfolio_size_usd=portfolio_size_usd,
        debate_rounds=debate_rounds,
    )

    bar = "═" * 60
    print(f"\n{bar}")
    print(f"  TradingAgents  ·  {ticker}  ·  {exchange} ({market_type})")
    print(f"  Portfolio ${portfolio_size_usd:,.2f}  ·  {debate_rounds} debate round(s)")
    print(f"{bar}")

    run_analyst_team(client, state)       # Phase I
    run_researcher_team(client, state)    # Phase II
    run_execution_team(client, state)     # Phase III
    execute_trade(state, dry_run=not execute)

    d = state.final_decision
    print(f"\n{bar}")
    print(f"  Final Decision: {d.get('action','N/A').upper()}")
    print(f"  Size:           {d.get('final_size_pct', 0)}% of ${portfolio_size_usd:,.2f}")
    print(f"  Confidence:     {d.get('confidence', 0):.0%}")
    print(f"{bar}\n")

    return state


# ──────────────────────────────────────────────────────────────
# CLI
# ──────────────────────────────────────────────────────────────

def main() -> None:
    parser = argparse.ArgumentParser(
        description="TradingAgents — Multi-agent LLM trading powered by Claude + fintool",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
examples:
  # Analyze BTC on Hyperliquid spot, dry run
  python trading_agents.py BTC --exchange hyperliquid --portfolio 1000

  # Analyze ETH perp on Binance, 3 debate rounds, submit trade
  python trading_agents.py ETH --exchange binance --market perp --portfolio 5000 --rounds 3 --execute

  # Analyze HYPE spot on Hyperliquid, save full state
  python trading_agents.py HYPE --exchange hyperliquid --portfolio 500 --output state.json
        """,
    )
    parser.add_argument("ticker",
                        help="Asset symbol (e.g. BTC, ETH, HYPE, SILVER, AAPL)")
    parser.add_argument("--exchange",  default="hyperliquid",
                        choices=KNOWN_EXCHANGES,
                        help="Exchange to trade on (default: hyperliquid)")
    parser.add_argument("--market",    default="spot", choices=("spot", "perp"),
                        dest="market_type",
                        help="Market type: spot or perp (default: spot)")
    parser.add_argument("--portfolio", default=1000.0, type=float,
                        help="Portfolio size in USD (default: 1000)")
    parser.add_argument("--rounds",    default=2, type=int,
                        help="Bull/Bear debate rounds (default: 2)")
    parser.add_argument("--execute",   action="store_true",
                        help="Submit the trade via fintool (default: dry run)")
    parser.add_argument("--output",    default=None,
                        help="Save full Global State JSON to this path")

    args = parser.parse_args()

    if not os.environ.get("ANTHROPIC_API_KEY"):
        print("Error: ANTHROPIC_API_KEY environment variable is not set.", file=sys.stderr)
        sys.exit(1)

    if not HAS_YFINANCE:
        print("⚠ Warning: yfinance/pandas not found. Fundamental and technical data unavailable.")
        print("  Install: pip install yfinance pandas\n")

    state = run_trading_session(
        ticker=args.ticker.upper(),
        exchange=args.exchange,
        market_type=args.market_type,
        portfolio_size_usd=args.portfolio,
        debate_rounds=args.rounds,
        execute=args.execute,
    )

    if args.output:
        with open(args.output, "w") as fh:
            json.dump(asdict(state), fh, indent=2)
        print(f"Global State saved → {args.output}")


if __name__ == "__main__":
    main()

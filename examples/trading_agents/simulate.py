#!/usr/bin/env python3
"""
simulate.py — Full TradingAgents pipeline simulation with mock data.

Patches all live API calls (Claude, yfinance, fintool) with realistic
pre-written fixtures so you can demo the complete user experience without
credentials or network access.

Usage:
    python simulate.py btc          # BTC spot on Hyperliquid → BUY
    python simulate.py eth          # ETH perp on Binance    → HOLD
    python simulate.py hype         # HYPE spot on Hyperliquid → SELL
    python simulate.py all          # Run all three scenarios in sequence
    python simulate.py btc --fast   # Skip streaming delays
"""

import argparse
import io
import json
import sys
import time
from dataclasses import asdict
from typing import Any
from unittest.mock import MagicMock, patch

# Force UTF-8 output on Windows
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
else:
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")

import mock_data as md
import trading_agents as ta

# ──────────────────────────────────────────────────────────────
# Terminal styling helpers
# ──────────────────────────────────────────────────────────────

RESET  = "\033[0m"
BOLD   = "\033[1m"
DIM    = "\033[2m"
GREEN  = "\033[32m"
YELLOW = "\033[33m"
CYAN   = "\033[36m"
RED    = "\033[31m"
BLUE   = "\033[34m"
MAGENTA= "\033[35m"
WHITE  = "\033[97m"

def c(text: str, color: str) -> str:
    return f"{color}{text}{RESET}"

def header(text: str, width: int = 68) -> None:
    bar = "═" * width
    print(f"\n{c(bar, CYAN)}")
    print(f"{c('  ' + text, BOLD + WHITE)}")
    print(f"{c(bar, CYAN)}")

def section(text: str) -> None:
    print(f"\n{c('▶ ' + text, BOLD + YELLOW)}")

def agent_header(role: str, phase: str = "") -> None:
    label = f"[{phase}] " if phase else ""
    print(f"\n  {c('┌─', DIM)} {c(label + role, BOLD + CYAN)}")

def agent_footer() -> None:
    print(f"  {c('└─ ✓ done', DIM + GREEN)}")

def stream_text(text: str, fast: bool = False, indent: str = "  │  ") -> None:
    """Print text simulating LLM streaming, with realistic pacing."""
    delay = 0.0 if fast else 0.006
    print(f"  {c('│', DIM)}", end="\n", flush=True)
    words = text.split(" ")
    line = indent
    for word in words:
        if "\n" in word:
            parts = word.split("\n")
            for i, part in enumerate(parts):
                if i == 0:
                    line += part + " "
                    if not fast:
                        print(line, end="\r", flush=True)
                        time.sleep(delay * 3)
                else:
                    print(c(line.rstrip(), DIM + WHITE))
                    line = indent + part + " "
        else:
            line += word + " "
            chunk = line
            if not fast:
                print(c(chunk, DIM + WHITE), end="\r", flush=True)
                time.sleep(delay)
    if line.strip():
        print(c(line.rstrip(), DIM + WHITE))
    print()

def thinking_spinner(label: str, duration: float, fast: bool = False) -> None:
    """Animate a 'thinking…' spinner to simulate LLM reasoning."""
    if fast:
        print(f"  {c('⟳', BLUE)} {c(label, DIM)} {c('(adaptive thinking)', DIM)}... ", end="")
        print(c("done", GREEN))
        return
    frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
    end_time = time.time() + duration
    i = 0
    while time.time() < end_time:
        frame = frames[i % len(frames)]
        print(f"\r  {c(frame, BLUE)} {c(label, DIM)} {c('(adaptive thinking)', DIM)}", end="", flush=True)
        time.sleep(0.08)
        i += 1
    print(f"\r  {c('✓', GREEN)} {c(label, DIM)} {c('(adaptive thinking complete)', DIM)}          ")

def data_table(data: dict, title: str = "") -> None:
    """Print a dict as a formatted table."""
    if title:
        print(f"\n  {c(title, BOLD)}")
    for k, v in data.items():
        if v is None:
            continue
        key_str   = c(f"    {k:<22}", DIM)
        val_str   = c(str(v), WHITE)
        print(f"{key_str} {val_str}")

def decision_box(decision: dict, ticker: str, exchange: str, portfolio: float, fast: bool = False) -> None:
    """Print the final decision in a prominent box."""
    action = decision.get("action", "hold").upper()
    size   = decision.get("final_size_pct", 0)
    conf   = decision.get("confidence", 0)
    trade_usd = portfolio * size / 100

    color_map = {"BUY": GREEN, "SELL": RED, "HOLD": YELLOW}
    action_color = color_map.get(action, WHITE)

    width = 60
    bar   = "═" * width
    print(f"\n  {c(bar, action_color)}")
    print(f"  {c('  FINAL DECISION', BOLD + action_color)}")
    print(f"  {c(bar, action_color)}")
    print(f"  {c(f'  Action     : {action}', BOLD + action_color)}")
    print(f"  {c(f'  Asset      : {ticker} on {exchange}', WHITE)}")
    print(f"  {c(f'  Size       : {size}% of portfolio = ${trade_usd:,.2f} USD', WHITE)}")
    print(f"  {c(f'  Confidence : {conf:.0%}', WHITE)}")
    print(f"  {c(bar, action_color)}")

    if decision.get("self_reflection"):
        print(f"\n  {c('Self-Reflection:', BOLD)}")
        words = decision["self_reflection"][:300]
        for line in _wrap(words, 62):
            print(f"  {c('  ' + line, DIM)}")

    if decision.get("no_trade_reason"):
        print(f"\n  {c('No-Trade Reason:', BOLD + YELLOW)}")
        for line in _wrap(decision["no_trade_reason"], 62):
            print(f"  {c('  ' + line, YELLOW)}")

def _wrap(text: str, width: int) -> list:
    words, lines, line = text.split(), [], ""
    for w in words:
        if len(line) + len(w) + 1 <= width:
            line += ("" if not line else " ") + w
        else:
            if line:
                lines.append(line)
            line = w
    if line:
        lines.append(line)
    return lines


# ──────────────────────────────────────────────────────────────
# Mock Anthropic client
# ──────────────────────────────────────────────────────────────

class _MockBlock:
    """Mimics an Anthropic TextBlock."""
    def __init__(self, text: str):
        self.type = "text"
        self.text = text

class _MockMessage:
    def __init__(self, text: str):
        self.content = [_MockBlock(text)]

class _MockStream:
    """Context manager that mimics client.messages.stream()."""
    def __init__(self, text: str, fast: bool, show_stream: bool):
        self._text      = text
        self._fast      = fast
        self._show      = show_stream

    def __enter__(self):
        return self

    def __exit__(self, *_):
        pass

    def get_final_message(self):
        return _MockMessage(self._text)


def _make_mock_client(scenario: dict, fast: bool) -> Any:
    """Build a MockAnthropic whose .messages.stream() dispatches by role keyword."""
    llm = scenario["llm"]

    ROLE_MAP = {
        "Fundamental Analyst":  ("Fundamental", False, "Fundamental Analyst"),
        "Technical Analyst":    ("Technical",   False, "Technical Analyst"),
        "Sentiment Analyst":    ("Sentiment",   False, "Sentiment Analyst"),
        "Macro/News Analyst":   ("Macro",       False, "Macro Analyst"),
        "Bullish Researcher":   (None,          True,  "Bullish Researcher"),
        "Bearish Researcher":   (None,          True,  "Bearish Researcher"),
        "You are the Trader":   ("Trader",      True,  "Trader"),
        "Aggressive voice":     ("Aggressive",  True,  "Risk — Aggressive"),
        "Neutral voice":        ("Neutral",     True,  "Risk — Neutral"),
        "Conservative voice":   ("Conservative",True,  "Risk — Conservative"),
        "Fund Manager":         ("FundManager", True,  "Fund Manager"),
    }

    # Track debate round for Bull/Bear dispatch
    _round_counter = {"bull": 0, "bear": 0}

    def _dispatch(system: str, **_) -> _MockStream:
        for keyword, (key, is_deep, label) in ROLE_MAP.items():
            if keyword not in system:
                continue

            # Thinking spinner for deep-model roles
            if is_deep:
                thinking_spinner(label, duration=1.6 if not fast else 0, fast=fast)
            else:
                if not fast:
                    time.sleep(0.3)

            # Bull/Bear round dispatch
            if keyword == "Bullish Researcher":
                _round_counter["bull"] += 1
                r = _round_counter["bull"]
                key = f"Bull_R{r}" if f"Bull_R{r}" in llm else "Bull_R1"
            elif keyword == "Bearish Researcher":
                _round_counter["bear"] += 1
                r = _round_counter["bear"]
                key = f"Bear_R{r}" if f"Bear_R{r}" in llm else "Bear_R1"

            text = llm.get(key, f'{{"error": "no mock for {key}"}}')
            return _MockStream(text, fast=fast, show_stream=True)

        # Fallback
        return _MockStream('{"action":"hold","final_size_pct":0,"confidence":0}', fast=fast, show_stream=False)

    mock_client          = MagicMock()
    mock_client.messages = MagicMock()
    mock_client.messages.stream.side_effect = lambda **kw: _dispatch(**kw)
    return mock_client


# ──────────────────────────────────────────────────────────────
# Patched agent wrappers  (intercept prints to add richer logging)
# ──────────────────────────────────────────────────────────────

def run_scenario(name: str, fast: bool = False) -> None:
    scenario = md.SCENARIOS[name]

    header(f"TradingAgents  ·  {scenario['label']}", width=68)
    print(f"  {c('Ticker:',    DIM)} {c(scenario['ticker'],       WHITE + BOLD)}")
    print(f"  {c('Exchange:',  DIM)} {c(scenario['exchange'],     WHITE)}")
    print(f"  {c('Market:',    DIM)} {c(scenario['market_type'],  WHITE)}")
    portfolio_str = f"${scenario['portfolio_usd']:,.2f} USD"
    print(f"  {c('Portfolio:', DIM)} {c(portfolio_str, WHITE)}")
    print(f"  {c('Mode:',      DIM)} {c('SIMULATION  (no live API calls)', YELLOW)}")

    # Build state
    state = ta.GlobalState(
        ticker             = scenario["ticker"],
        exchange           = scenario["exchange"],
        market_type        = scenario["market_type"],
        portfolio_size_usd = scenario["portfolio_usd"],
        debate_rounds      = scenario["debate_rounds"],
    )

    mock_client = _make_mock_client(scenario, fast)

    # ── Patch all external dependencies ────────────────────────
    patches = [
        patch.object(ta, "get_fundamental_data",  return_value=scenario["fundamental"]),
        patch.object(ta, "get_technical_indicators", return_value=scenario["technical"]),
        patch.object(ta, "get_sentiment_score",    return_value=scenario["sentiment"]),
        patch.object(ta, "get_macro_news",         return_value=scenario["macro"]),
        patch.object(ta, "get_current_price",      return_value=scenario["price"]),
        patch.object(ta, "run_fintool",            return_value=scenario.get("fintool_confirm") or {"status": "no_action"}),
        patch("anthropic.Anthropic",               return_value=mock_client),
    ]

    for p in patches:
        p.start()

    try:
        _run_phase_i(mock_client, state, scenario, fast)
        _run_phase_ii(mock_client, state, scenario, fast)
        _run_phase_iii(mock_client, state, scenario, fast)
        _run_execution(state, scenario, fast)
    finally:
        for p in patches:
            p.stop()


# ──────────────────────────────────────────────────────────────
# Phase runners with rich logging
# ──────────────────────────────────────────────────────────────

def _run_phase_i(client, state: ta.GlobalState, scenario: dict, fast: bool) -> None:
    section("PHASE I  ·  Analyst Team  ─  The Eyes")
    llm = scenario["llm"]

    analyst_steps = [
        ("Fundamental Analyst", "Fundamental", "fundamental", scenario["fundamental"]),
        ("Technical Analyst",   "Technical",   "technical",   scenario["technical"]),
        ("Sentiment Analyst",   "Sentiment",   "sentiment",   scenario["sentiment"]),
        ("Macro Analyst",       "Macro",       "macro",       scenario["macro"]),
    ]

    for display_name, key, data_key, raw_data in analyst_steps:
        agent_header(display_name, "Phase I")

        # Show the raw data being ingested
        print(f"  {c('│  Ingesting data:', DIM)}")
        items = {k: v for k, v in raw_data.items() if v is not None and k != "headlines"}
        for k, v in list(items.items())[:6]:
            print(f"  {c('│    ' + f'{k:<22}', DIM)}{c(str(v), WHITE)}")
        if len(items) > 6:
            print(f"  {c(f'│    … and {len(items)-6} more fields', DIM)}")
        print()

        # Simulate streaming the analysis
        if not fast:
            time.sleep(0.4)
        print(f"  {c('│  Analysis:', DIM)}")
        stream_text(llm[key], fast=fast)

        state.market_data[data_key]       = raw_data
        state.analyst_summaries[key]      = llm[key]
        agent_footer()


def _run_phase_ii(client, state: ta.GlobalState, scenario: dict, fast: bool) -> None:
    section(f"PHASE II  ·  Researcher Team  ─  The Brain  ({scenario['debate_rounds']} rounds)")
    llm = scenario["llm"]

    for round_num in range(1, scenario["debate_rounds"] + 1):
        total_rounds = scenario["debate_rounds"]
        print(f"\n  {c(f'── Round {round_num} / {total_rounds} ──', BOLD + MAGENTA)}")

        # Bullish
        agent_header("Bullish Researcher", f"Round {round_num}")
        bull_key = f"Bull_R{round_num}" if f"Bull_R{round_num}" in llm else "Bull_R1"
        thinking_spinner("Reasoning through the bullish case", 1.8, fast)
        print(f"  {c('│  Argument:', DIM)}")
        stream_text(llm[bull_key], fast=fast)
        agent_footer()

        # Bearish
        agent_header("Bearish Researcher", f"Round {round_num}")
        bear_key = f"Bear_R{round_num}" if f"Bear_R{round_num}" in llm else "Bear_R1"
        thinking_spinner("Stress-testing the bull case", 2.1, fast)
        print(f"\n  {c('│  Facilitator asks:', BOLD)}")
        fq1 = '│  "Given the Technical Analyst\'s signals, what is the most'
        fq2 = '│   likely reason any bullish signal here is a trap?"'
        print(f"  {c(fq1, YELLOW)}")
        print(f"  {c(fq2, YELLOW)}\n")
        print(f"  {c('│  Rebuttal:', DIM)}")
        stream_text(llm[bear_key], fast=fast)
        agent_footer()

        state.debate_log.append({
            "Round": round_num,
            "Bull":  llm[bull_key],
            "Bear":  llm[bear_key],
        })


def _run_phase_iii(client, state: ta.GlobalState, scenario: dict, fast: bool) -> None:
    section("PHASE III  ·  Execution & Oversight  ─  The Hands")
    llm = scenario["llm"]

    # ── Trader ────────────────────────────────────────────────
    agent_header("Trader", "Phase III")
    thinking_spinner("Synthesizing debate into trade proposal", 1.4, fast)
    trader_proposal = json.loads(llm["Trader"])
    state.risk_assessment = {}

    print(f"  {c('│  Proposal:', DIM)}")
    action_color = GREEN if trader_proposal["action"] == "buy" else RED if trader_proposal["action"] == "sell" else YELLOW
    print(f"  {c('│', DIM)}    Action  : {c(trader_proposal['action'].upper(), BOLD + action_color)}")
    print(f"  {c('│', DIM)}    Size    : {c(str(trader_proposal['size_pct']) + '% of portfolio', WHITE)}")
    print(f"  {c('│', DIM)}    Offset  : {c(str(trader_proposal['price_offset_pct']) + '%', WHITE)}")
    print(f"  {c('│  Rationale:', DIM)}")
    for line in _wrap(trader_proposal["rationale"], 60):
        print(f"  {c('│    ', DIM)}{c(line, DIM + WHITE)}")
    print(f"  {c('│  Key Risk:', DIM)}")
    for line in _wrap(trader_proposal["key_risk"], 60):
        print(f"  {c('│    ', DIM)}{c(line, YELLOW)}")
    agent_footer()

    # ── Risk Management (3 voices) ─────────────────────────────
    agent_header("Risk Management Team  (Aggressive · Neutral · Conservative)", "Phase III")
    voices = [("Aggressive", GREEN), ("Neutral", CYAN), ("Conservative", YELLOW)]
    risk_summaries = []

    for voice, color in voices:
        print(f"\n  {c('│', DIM)}  {c(f'[{voice}]', BOLD + color)}", end=" ")
        thinking_spinner(f"{voice} risk assessment", 1.0, fast)
        assessment = json.loads(llm[voice])
        state.risk_assessment[voice] = assessment

        approved = assessment.get("approved", False)
        size     = assessment.get("recommended_size_pct", 0)
        concern  = assessment.get("key_concern", "")[:72]
        mark     = c("✓ APPROVED", GREEN) if approved else c("✗ REJECTED", RED)

        print(f"  {c('│', DIM)}    Status  : {mark}")
        print(f"  {c('│', DIM)}    Size    : {c(str(size) + '%', color)}")
        print(f"  {c('│', DIM)}    Concern : {c(concern, DIM)}")
        risk_summaries.append((voice, approved, size))

    print()
    # Weighted vote summary
    w_size = (
        (state.risk_assessment.get("Conservative", {}).get("recommended_size_pct", 0) * 0.50)
        + (state.risk_assessment.get("Neutral",    {}).get("recommended_size_pct", 0) * 0.30)
        + (state.risk_assessment.get("Aggressive", {}).get("recommended_size_pct", 0) * 0.20)
    )
    print(f"  {c('│', DIM)}  {c('Weighted Vote (C×50 / N×30 / A×20):', BOLD)}  {c(f'{w_size:.1f}%', WHITE + BOLD)}")
    agent_footer()

    # ── Fund Manager ───────────────────────────────────────────
    agent_header("Fund Manager", "Phase III")
    thinking_spinner("Self-reflection + final authorization", 2.4, fast)

    final = json.loads(llm["FundManager"])
    state.final_decision = final

    print(f"  {c('│  Self-Reflection:', DIM)}")
    for line in _wrap(final.get("self_reflection", ""), 62):
        print(f"  {c('│    ', DIM)}{c(line, DIM + WHITE)}")
    agent_footer()


def _run_execution(state: ta.GlobalState, scenario: dict, fast: bool) -> None:
    section("EXECUTION  ·  fintool Trade Dispatch")

    decision = state.final_decision
    action   = decision.get("action", "hold").lower()
    size_pct = decision.get("final_size_pct", 0)
    conf     = decision.get("confidence", 0)
    price    = scenario["price"]

    decision_box(decision, state.ticker, state.exchange, state.portfolio_size_usd, fast)

    if action == "hold" or size_pct == 0:
        print(f"\n  {c('→ No trade submitted.', YELLOW + BOLD)} Capital preserved.")
        if decision.get("no_trade_reason"):
            print(f"  {c('  Reason: ' + decision['no_trade_reason'][:120], DIM)}")
        _print_session_summary(state, scenario)
        return

    # Compute trade parameters
    offset_pct  = float(decision.get("price_offset_pct", 0))
    limit_price = round(price * (1 + offset_pct / 100), 4)
    trade_usd   = state.portfolio_size_usd * (size_pct / 100)
    amount      = round(trade_usd / limit_price, 6)

    if state.market_type == "perp":
        command = f"perp_{action}"
    else:
        command = action

    fintool_cmd = {"command": command, "symbol": state.ticker, "amount": amount, "price": limit_price}

    print(f"\n  {c('Trade Parameters:', BOLD)}")
    print(f"    {c('Amount:     ', DIM)}{c(f'{amount} {state.ticker}', WHITE)}")
    print(f"    {c('Limit price:', DIM)}{c(f'${limit_price:,.4f}', WHITE)}  {c(f'(current ~${price:,.4f}  offset {offset_pct:+.1f}%)', DIM)}")
    print(f"    {c('Trade value:', DIM)}{c(f'${trade_usd:,.2f} USD', WHITE + BOLD)}")

    print(f"\n  {c('fintool CLI command:', BOLD)}")
    fintool_arg = "'" + json.dumps(fintool_cmd) + "'"
    print(f"    {c(state.exchange, CYAN)} {c('--json', DIM)} {c(fintool_arg, WHITE)}")

    # Simulate submission
    if not fast:
        time.sleep(0.6)
    print(f"\n  {c('Submitting order…', DIM)}", end="", flush=True)
    if not fast:
        time.sleep(1.2)

    confirm = scenario.get("fintool_confirm")
    if confirm:
        print(f" {c('✓', GREEN + BOLD)}")
        print(f"\n  {c('Order Confirmation:', BOLD + GREEN)}")
        for k, v in confirm.items():
            print(f"    {c(f'{k:<16}', DIM)}{c(str(v), GREEN)}")
    else:
        print(f" {c('✓ (dry run — no live order)', YELLOW)}")

    _print_session_summary(state, scenario)


def _print_session_summary(state: ta.GlobalState, scenario: dict) -> None:
    """Print a clean end-of-session summary."""
    d = state.final_decision
    action  = d.get("action", "N/A").upper()
    size    = d.get("final_size_pct", 0)
    conf    = d.get("confidence", 0)
    trade_usd = state.portfolio_size_usd * size / 100

    color_map = {"BUY": GREEN, "SELL": RED, "HOLD": YELLOW}
    ac = color_map.get(action, WHITE)

    print(f"\n  {c('─' * 60, DIM)}")
    print(f"  {c('SESSION COMPLETE', BOLD)}")
    print(f"  {c('─' * 60, DIM)}")
    print(f"  Ticker:    {c(state.ticker,       WHITE + BOLD)}")
    print(f"  Exchange:  {c(state.exchange,     WHITE)}")
    print(f"  Market:    {c(state.market_type,  WHITE)}")
    print(f"  Decision:  {c(action,             BOLD + ac)}")
    if action != "HOLD":
        print(f"  Size:      {c(f'{size}%  =  ${trade_usd:,.2f} USD', WHITE)}")
    print(f"  Confidence:{c(f' {conf:.0%}', WHITE)}")

    # Analyst summary scores
    print(f"\n  {c('Analyst signals:', DIM)}")
    tech = state.market_data.get("technical", {})
    if tech:
        rsi = tech.get("rsi_14")
        cci = tech.get("cci_20")
        macd_h = tech.get("macd_histogram")
        bias_vs_sma50  = tech.get("price_vs_sma50", "?")
        bias_vs_sma200 = tech.get("price_vs_sma200", "?")

        rsi_color  = RED if rsi and rsi > 70 else GREEN if rsi and rsi < 30 else WHITE
        cci_color  = RED if cci and cci > 100 else GREEN if cci and cci < -100 else WHITE
        macd_color = GREEN if macd_h and macd_h > 0 else RED if macd_h and macd_h < 0 else WHITE

        print(f"    {c('RSI-14:',   DIM)} {c(str(rsi),   rsi_color)}")
        print(f"    {c('CCI-20:',   DIM)} {c(str(cci),   cci_color)}")
        print(f"    {c('MACD Hist:', DIM)} {c(str(macd_h), macd_color)}")
        print(f"    {c('vs SMA-50:', DIM)} {c(bias_vs_sma50,  GREEN if bias_vs_sma50 == 'above' else RED)}")
        print(f"    {c('vs SMA-200:',DIM)} {c(bias_vs_sma200, GREEN if bias_vs_sma200 == 'above' else RED)}")

    print(f"\n  {c('Risk votes:', DIM)}")
    for voice, assessment in state.risk_assessment.items():
        approved = assessment.get("approved", False)
        size_v   = assessment.get("recommended_size_pct", 0)
        mark     = c("✓", GREEN) if approved else c("✗", RED)
        print(f"    {c(f'{voice:<14}', DIM)} {mark}  {c(f'{size_v}%', WHITE)}")

    print(f"  {c('─' * 60, DIM)}\n")


# ──────────────────────────────────────────────────────────────
# CLI
# ──────────────────────────────────────────────────────────────

def main() -> None:
    parser = argparse.ArgumentParser(
        description="TradingAgents simulation — full pipeline with mock data",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
scenarios:
  btc   BTC spot on Hyperliquid   → BUY  14% at $63,859 limit
  eth   ETH perp on Binance       → HOLD  (risk team vetoes on MACD divergence)
  hype  HYPE spot on Hyperliquid  → SELL 7%  (triple overbought + insider unlock)
  all   Run all three in sequence
        """,
    )
    parser.add_argument("scenario", choices=["btc", "eth", "hype", "all"],
                        help="Which scenario to simulate")
    parser.add_argument("--fast", action="store_true",
                        help="Skip streaming delays and spinners")
    args = parser.parse_args()

    scenarios = ["btc", "eth", "hype"] if args.scenario == "all" else [args.scenario]

    for i, name in enumerate(scenarios):
        if i > 0:
            print(f"\n\n{'▓' * 68}\n")
            if not args.fast:
                time.sleep(1.5)
        run_scenario(name, fast=args.fast)

    print(c("\nSimulation complete.\n", DIM))


if __name__ == "__main__":
    main()

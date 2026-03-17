#!/usr/bin/env python3
"""
Catalyst Hedger — Hourly position adjustment for perp + prediction market hedging.

Uses OpenAI to estimate catalyst probabilities from news, compares with Polymarket
prices, and outputs fintool CLI commands to rebalance positions.

Usage:
    export OPENAI_API_KEY=sk-...
    python3 catalyst_hedger.py --config hedger_config.json
    python3 catalyst_hedger.py --config hedger_config.json --execute  # actually run commands
"""

import argparse
import json
import os
import subprocess
import sys
import time
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

import requests

# ---------------------------------------------------------------------------
# Data structures
# ---------------------------------------------------------------------------

@dataclass
class Catalyst:
    """A named event that could impact the perp position."""
    name: str                          # human label, e.g. "Fed rate hike March 2026"
    polymarket_slug: str               # e.g. "will-fed-raise-rates-march-2026"
    news_query: str                    # search query for news gathering
    expected_drawdown: float           # e.g. 0.15 = 15% drop if catalyst fires
    coverage: float = 1.0             # 0-1, how much of the loss to hedge
    max_premium_pct: float = 0.05     # don't spend more than 5% of notional on this hedge
    current_shares: float = 0.0       # shares currently held on Polymarket

@dataclass
class PerpPosition:
    """The perp position being hedged."""
    exchange: str                      # hyperliquid, binance, okx
    asset: str                         # BTC, ETH, SOL
    side: str                          # long or short
    size: float                        # amount in asset units (e.g. 0.5 BTC)
    entry_price: float                 # entry price
    leverage: float = 1.0

@dataclass
class Config:
    perp: dict = field(default_factory=dict)
    catalysts: list = field(default_factory=list)
    rebalance_threshold: float = 0.1  # only rebalance if position off by >10%
    dry_run: bool = True
    log_file: str = "hedger_log.jsonl"

# ---------------------------------------------------------------------------
# News gathering
# ---------------------------------------------------------------------------

BRAVE_SEARCH_URL = "https://api.search.brave.com/res/v1/web/search"

def fetch_news(query: str, count: int = 10) -> list[dict]:
    """Fetch recent news via Brave Search API."""
    api_key = os.environ.get("BRAVE_API_KEY", "")
    if not api_key:
        print("[WARN] No BRAVE_API_KEY set, using basic web search fallback")
        return _fetch_news_fallback(query)

    resp = requests.get(
        BRAVE_SEARCH_URL,
        headers={"X-Subscription-Token": api_key, "Accept": "application/json"},
        params={"q": query, "count": count, "freshness": "pw"},  # past week
        timeout=15,
    )
    resp.raise_for_status()
    data = resp.json()
    results = []
    for r in data.get("web", {}).get("results", []):
        results.append({
            "title": r.get("title", ""),
            "description": r.get("description", ""),
            "url": r.get("url", ""),
            "published": r.get("age", ""),
        })
    return results


def _fetch_news_fallback(query: str) -> list[dict]:
    """Minimal fallback using DuckDuckGo instant answer (no API key needed)."""
    try:
        resp = requests.get(
            "https://api.duckduckgo.com/",
            params={"q": query, "format": "json", "no_html": 1},
            timeout=10,
        )
        data = resp.json()
        results = []
        for topic in data.get("RelatedTopics", [])[:5]:
            if "Text" in topic:
                results.append({
                    "title": topic.get("Text", "")[:100],
                    "description": topic.get("Text", ""),
                    "url": topic.get("FirstURL", ""),
                    "published": "",
                })
        return results
    except Exception:
        return []

# ---------------------------------------------------------------------------
# OpenAI probability estimation
# ---------------------------------------------------------------------------

def estimate_probability(catalyst: Catalyst, news: list[dict], perp_asset: str) -> dict:
    """
    Ask OpenAI to estimate the probability of a catalyst occurring
    and the expected asset drawdown if it does.

    Returns: {
        "probability": 0.0-1.0,
        "estimated_drawdown": 0.0-1.0,
        "reasoning": "...",
        "confidence": "low" | "medium" | "high"
    }
    """
    api_key = os.environ.get("OPENAI_API_KEY")
    if not api_key:
        print("[ERROR] OPENAI_API_KEY not set")
        sys.exit(1)

    news_text = "\n".join(
        f"- [{n['published']}] {n['title']}: {n['description'][:200]}"
        for n in news[:10]
    )

    prompt = f"""You are a quantitative analyst estimating event probabilities for hedging.

CATALYST: {catalyst.name}
ASSET BEING HEDGED: {perp_asset}
DEFAULT EXPECTED DRAWDOWN IF CATALYST FIRES: {catalyst.expected_drawdown:.0%}

RECENT NEWS:
{news_text if news_text.strip() else "(no recent news found)"}

Based on the news and your knowledge, estimate:

1. **probability**: The probability this catalyst occurs (0.0 to 1.0). Be calibrated — 
   prediction markets are usually well-priced, so deviate from the market only with 
   strong evidence. If no strong signal, default to the Polymarket implied probability.

2. **estimated_drawdown**: If the catalyst fires, how much would {perp_asset} drop 
   as a fraction (e.g., 0.15 = 15% drop)? Consider current market conditions, 
   volatility regime, and how much is already priced in.

3. **reasoning**: 2-3 sentences explaining your estimate.

4. **confidence**: "low", "medium", or "high" — how confident are you in deviating 
   from the market consensus?

Respond ONLY with a JSON object, no markdown:
{{"probability": 0.XX, "estimated_drawdown": 0.XX, "reasoning": "...", "confidence": "..."}}"""

    resp = requests.post(
        "https://api.openai.com/v1/chat/completions",
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
        json={
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0.3,
            "max_tokens": 500,
        },
        timeout=30,
    )
    resp.raise_for_status()
    content = resp.json()["choices"][0]["message"]["content"].strip()

    # Parse JSON (handle potential markdown wrapping)
    if content.startswith("```"):
        content = content.split("\n", 1)[1].rsplit("```", 1)[0]
    return json.loads(content)


# ---------------------------------------------------------------------------
# Polymarket price fetching
# ---------------------------------------------------------------------------

POLYMARKET_API = "https://gamma-api.polymarket.com"

def get_polymarket_price(slug: str) -> Optional[dict]:
    """
    Fetch current YES price and market info from Polymarket.
    Returns: {"yes_price": float, "no_price": float, "volume": float, "slug": str}
    """
    try:
        # Search for the market by slug
        resp = requests.get(
            f"{POLYMARKET_API}/events",
            params={"slug": slug},
            timeout=10,
        )
        resp.raise_for_status()
        events = resp.json()

        if not events:
            # Try searching by text
            resp = requests.get(
                f"{POLYMARKET_API}/events",
                params={"text": slug.replace("-", " "), "limit": 1},
                timeout=10,
            )
            resp.raise_for_status()
            events = resp.json()

        if not events:
            print(f"[WARN] Polymarket market not found: {slug}")
            return None

        event = events[0] if isinstance(events, list) else events
        markets = event.get("markets", [])
        if not markets:
            return None

        market = markets[0]
        yes_price = float(market.get("bestAsk", market.get("lastTradePrice", 0.5)))
        return {
            "yes_price": yes_price,
            "no_price": 1.0 - yes_price,
            "volume": float(market.get("volume", 0)),
            "slug": slug,
            "condition_id": market.get("conditionId", ""),
        }
    except Exception as e:
        print(f"[WARN] Polymarket API error for {slug}: {e}")
        return None


# ---------------------------------------------------------------------------
# Hedging math
# ---------------------------------------------------------------------------

@dataclass
class HedgeCalculation:
    catalyst_name: str
    market_price: float           # Polymarket YES price
    ai_probability: float         # our estimated probability
    ai_drawdown: float            # our estimated drawdown
    ai_confidence: str
    ai_reasoning: str
    notional: float               # perp notional value
    leverage: float
    loss_if_catalyst: float       # $ loss if catalyst fires
    shares_needed: float          # Polymarket shares for full hedge
    shares_target: float          # after coverage ratio
    shares_current: float         # currently held
    shares_delta: float           # buy (+) or sell (-)
    premium_cost: float           # $ cost of the hedge
    premium_pct: float            # as % of notional
    edge: float                   # ai_probability - market_price (+ = hedge is cheap)
    action: str                   # "buy", "sell", "hold", "skip"


def compute_hedge(
    catalyst: Catalyst,
    perp: PerpPosition,
    market_data: dict,
    ai_estimate: dict,
) -> HedgeCalculation:
    """Compute the target hedge position and delta."""

    notional = perp.size * perp.entry_price
    market_price = market_data["yes_price"]
    ai_prob = ai_estimate["probability"]
    ai_drawdown = ai_estimate.get("estimated_drawdown", catalyst.expected_drawdown)
    leverage = perp.leverage

    # Core hedge math
    loss_if_catalyst = notional * leverage * ai_drawdown
    payout_per_share = 1.0 - market_price
    shares_full = loss_if_catalyst / payout_per_share if payout_per_share > 0 else 0

    # Adjust coverage based on edge
    edge = ai_prob - market_price  # positive = we think it's more likely than market
    if edge > 0.05:
        # hedge is cheap relative to our estimate — increase coverage
        effective_coverage = min(catalyst.coverage * 1.5, 1.0)
    elif edge < -0.10:
        # hedge is expensive — reduce coverage
        effective_coverage = catalyst.coverage * 0.5
    else:
        effective_coverage = catalyst.coverage

    shares_target = shares_full * effective_coverage
    premium_cost = shares_target * market_price
    premium_pct = premium_cost / notional if notional > 0 else 0

    # Cap premium
    if premium_pct > catalyst.max_premium_pct:
        shares_target = (catalyst.max_premium_pct * notional) / market_price
        premium_cost = shares_target * market_price
        premium_pct = catalyst.max_premium_pct

    shares_delta = shares_target - catalyst.current_shares

    # Determine action
    if abs(shares_delta) / max(shares_target, 1) < 0.1:  # within 10% of target
        action = "hold"
    elif shares_delta > 0:
        action = "buy"
    else:
        action = "sell"

    # Skip if AI confidence is low and edge is small
    if ai_estimate.get("confidence") == "low" and abs(edge) < 0.05:
        action = "hold"

    return HedgeCalculation(
        catalyst_name=catalyst.name,
        market_price=market_price,
        ai_probability=ai_prob,
        ai_drawdown=ai_drawdown,
        ai_confidence=ai_estimate.get("confidence", "unknown"),
        ai_reasoning=ai_estimate.get("reasoning", ""),
        notional=notional,
        leverage=leverage,
        loss_if_catalyst=loss_if_catalyst,
        shares_needed=shares_full,
        shares_target=shares_target,
        shares_current=catalyst.current_shares,
        shares_delta=shares_delta,
        premium_cost=premium_cost,
        premium_pct=premium_pct,
        edge=edge,
        action=action,
    )


# ---------------------------------------------------------------------------
# Command generation and execution
# ---------------------------------------------------------------------------

def generate_commands(calc: HedgeCalculation, catalyst: Catalyst) -> list[str]:
    """Generate fintool CLI commands for the rebalance."""
    commands = []
    if calc.action == "buy" and calc.shares_delta > 0:
        amt = round(calc.shares_delta, 2)
        commands.append(
            f"polymarket buy {catalyst.polymarket_slug} "
            f"--outcome yes --amount {amt} --price {calc.market_price:.4f}"
        )
    elif calc.action == "sell" and calc.shares_delta < 0:
        amt = round(abs(calc.shares_delta), 2)
        commands.append(
            f"polymarket sell {catalyst.polymarket_slug} "
            f"--outcome yes --amount {amt} --price {calc.market_price:.4f}"
        )
    return commands


def execute_command(cmd: str, dry_run: bool = True) -> dict:
    """Execute a fintool CLI command."""
    if dry_run:
        print(f"  [DRY RUN] {cmd}")
        return {"command": cmd, "status": "dry_run"}

    print(f"  [EXEC] {cmd}")
    try:
        result = subprocess.run(
            cmd.split(),
            capture_output=True,
            text=True,
            timeout=30,
        )
        return {
            "command": cmd,
            "status": "ok" if result.returncode == 0 else "error",
            "stdout": result.stdout,
            "stderr": result.stderr,
            "returncode": result.returncode,
        }
    except Exception as e:
        return {"command": cmd, "status": "error", "error": str(e)}


# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------

def log_run(log_file: str, run_data: dict):
    """Append a run record to the JSONL log."""
    with open(log_file, "a") as f:
        f.write(json.dumps(run_data) + "\n")


# ---------------------------------------------------------------------------
# Main loop
# ---------------------------------------------------------------------------

def load_config(path: str) -> Config:
    with open(path) as f:
        data = json.load(f)
    return Config(
        perp=data.get("perp", {}),
        catalysts=data.get("catalysts", []),
        rebalance_threshold=data.get("rebalance_threshold", 0.1),
        dry_run=data.get("dry_run", True),
        log_file=data.get("log_file", "hedger_log.jsonl"),
    )


def check_perp_position(perp: PerpPosition, dry_run: bool) -> bool:
    """
    Check if the configured perp position exists. If not, open it.
    Returns True if position is ready (existing or newly opened).
    """
    print(f"  Checking {perp.exchange} positions...")
    try:
        result = subprocess.run(
            [perp.exchange, "positions", "--json"],
            capture_output=True, text=True, timeout=15,
        )
        if result.returncode != 0:
            print(f"  [WARN] Could not query positions: {result.stderr.strip()}")
            return False

        positions = json.loads(result.stdout) if result.stdout.strip() else []
        # Look for matching position
        for pos in positions:
            asset = pos.get("asset", pos.get("symbol", "")).upper().replace("-PERP", "")
            side = pos.get("side", "").lower()
            size = abs(float(pos.get("size", pos.get("amount", 0))))
            if asset == perp.asset.upper() and side == perp.side and size > 0:
                print(f"  ✓ Found existing position: {side} {size} {asset}")
                return True

        # No matching position — open one
        print(f"  ✗ No {perp.side} {perp.asset} position found. Opening...")
        direction = "buy" if perp.side == "long" else "sell"

        # Set leverage first
        lev_cmd = f"{perp.exchange} perp leverage {perp.asset} --leverage {int(perp.leverage)}"
        open_cmd = (
            f"{perp.exchange} perp {direction} {perp.asset} "
            f"--amount {perp.size} --price {perp.entry_price}"
        )

        if dry_run:
            print(f"  [DRY RUN] {lev_cmd}")
            print(f"  [DRY RUN] {open_cmd}")
            return True  # assume success in dry run

        # Execute leverage
        print(f"  [EXEC] {lev_cmd}")
        subprocess.run(lev_cmd.split(), timeout=15)

        # Execute position open
        print(f"  [EXEC] {open_cmd}")
        result = subprocess.run(open_cmd.split(), capture_output=True, text=True, timeout=30)
        if result.returncode == 0:
            print(f"  ✓ Position opened successfully")
            return True
        else:
            print(f"  [ERROR] Failed to open position: {result.stderr.strip()}")
            return False

    except json.JSONDecodeError:
        print(f"  [WARN] Could not parse positions output")
        return False
    except FileNotFoundError:
        print(f"  [ERROR] '{perp.exchange}' CLI not found. Is fintool installed?")
        return False
    except Exception as e:
        print(f"  [ERROR] Position check failed: {e}")
        return False


def run_once(config_path: str, execute: bool = False):
    config = load_config(config_path)
    if execute:
        config.dry_run = False

    perp = PerpPosition(**config.perp)
    notional = perp.size * perp.entry_price

    print(f"\n{'='*70}")
    print(f"CATALYST HEDGER — {datetime.now(timezone.utc).isoformat()}")
    print(f"{'='*70}")
    print(f"Perp: {perp.side.upper()} {perp.size} {perp.asset} @ ${perp.entry_price:,.0f} "
          f"({perp.leverage}x) on {perp.exchange}")
    print(f"Notional: ${notional:,.0f} | Effective exposure: ${notional * perp.leverage:,.0f}")
    print(f"Mode: {'LIVE' if not config.dry_run else 'DRY RUN'}")
    print()

    # Step 0: Ensure perp position exists
    print("--- Perp Position Check ---")
    if not check_perp_position(perp, config.dry_run):
        print("[ABORT] Could not verify or open perp position. Skipping hedge cycle.")
        return None
    print()

    run_record = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "perp": asdict(perp),
        "hedges": [],
    }

    for cat_data in config.catalysts:
        catalyst = Catalyst(**cat_data)
        print(f"--- Catalyst: {catalyst.name} ---")

        # 1. Fetch Polymarket price
        print(f"  Fetching Polymarket price for '{catalyst.polymarket_slug}'...")
        market_data = get_polymarket_price(catalyst.polymarket_slug)
        if not market_data:
            print(f"  [SKIP] Market not found")
            continue
        print(f"  Market YES price: ${market_data['yes_price']:.4f}")

        # 2. Fetch news
        print(f"  Fetching news for '{catalyst.news_query}'...")
        news = fetch_news(catalyst.news_query)
        print(f"  Found {len(news)} articles")

        # 3. AI probability estimation
        print(f"  Estimating probability via OpenAI...")
        ai_estimate = estimate_probability(catalyst, news, perp.asset)
        print(f"  AI estimate: {ai_estimate['probability']:.1%} "
              f"(market: {market_data['yes_price']:.1%}) "
              f"[{ai_estimate.get('confidence', '?')}]")
        print(f"  Reasoning: {ai_estimate.get('reasoning', 'N/A')}")

        # 4. Compute hedge
        calc = compute_hedge(catalyst, perp, market_data, ai_estimate)
        edge_dir = "CHEAP" if calc.edge > 0 else "EXPENSIVE"
        print(f"  Edge: {calc.edge:+.1%} ({edge_dir})")
        print(f"  Target shares: {calc.shares_target:,.0f} "
              f"(current: {calc.shares_current:,.0f}, "
              f"delta: {calc.shares_delta:+,.0f})")
        print(f"  Premium: ${calc.premium_cost:,.0f} ({calc.premium_pct:.1%} of notional)")
        print(f"  Action: {calc.action.upper()}")

        # 5. Generate and execute commands
        commands = generate_commands(calc, catalyst)
        cmd_results = []
        for cmd in commands:
            result = execute_command(cmd, dry_run=config.dry_run)
            cmd_results.append(result)

        # 6. Log
        hedge_record = {
            "catalyst": catalyst.name,
            "market_price": calc.market_price,
            "ai_probability": calc.ai_probability,
            "ai_drawdown": calc.ai_drawdown,
            "ai_confidence": calc.ai_confidence,
            "ai_reasoning": calc.ai_reasoning,
            "edge": calc.edge,
            "shares_target": calc.shares_target,
            "shares_delta": calc.shares_delta,
            "premium_pct": calc.premium_pct,
            "action": calc.action,
            "commands": cmd_results,
        }
        run_record["hedges"].append(hedge_record)
        print()

    # Summary
    total_premium = sum(h.get("premium_pct", 0) for h in run_record["hedges"])
    print(f"{'='*70}")
    print(f"TOTAL HEDGE PREMIUM: {total_premium:.1%} of notional")
    print(f"{'='*70}\n")

    log_run(config.log_file, run_record)
    return run_record


def main():
    parser = argparse.ArgumentParser(description="Catalyst Hedger")
    parser.add_argument("--config", required=True, help="Path to config JSON")
    parser.add_argument("--execute", action="store_true", help="Actually execute trades (default: dry run)")
    parser.add_argument("--loop", action="store_true", help="Run continuously every hour")
    parser.add_argument("--interval", type=int, default=3600, help="Loop interval in seconds (default: 3600)")
    args = parser.parse_args()

    if args.loop:
        print(f"Running in loop mode (every {args.interval}s). Ctrl+C to stop.")
        while True:
            try:
                run_once(args.config, execute=args.execute)
            except Exception as e:
                print(f"[ERROR] Run failed: {e}")
            time.sleep(args.interval)
    else:
        run_once(args.config, execute=args.execute)


if __name__ == "__main__":
    main()

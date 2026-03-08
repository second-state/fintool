#!/usr/bin/env python3
"""
Metal Pairs Trading Bot — GOLD vs SILVER

Daily pairs trading: long one metal, short the other on Hyperliquid HIP-3 perps.
Decision based on news sentiment, 24h momentum, and funding rates.

Usage:
    python3 bot.py
    python3 bot.py --dry-run
    python3 bot.py --target-usdt0 100 --position-size 100 --leverage 3

Requires: fintool CLI, OpenAI and Brave API keys (set below)
"""

import argparse
import json
import logging
import os
import subprocess
import sys
import time
import urllib.request
import urllib.parse
from datetime import datetime
from pathlib import Path

# ── API keys (set these before running) ───────────────────────────────────────

OPENAI_API_KEY = ""   # https://platform.openai.com/api-keys
BRAVE_API_KEY = ""    # https://brave.com/search/api/

# ── Config ────────────────────────────────────────────────────────────────────

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_DIR = SCRIPT_DIR.parent.parent

DEFAULTS = {
    "fintool": os.environ.get("FINTOOL", str(REPO_DIR / "target" / "release" / "fintool")),
    "target_usdt0": 50,           # target USDT0 balance (margin for both legs)
    "position_size_usd": 50,      # notional per leg
    "leverage": 2,
    "log_dir": str(SCRIPT_DIR / "logs"),
}

# ── Logging ───────────────────────────────────────────────────────────────────

log = logging.getLogger("metal_pair")


def setup_logging(log_dir: str):
    os.makedirs(log_dir, exist_ok=True)
    log_file = os.path.join(log_dir, f"{datetime.now():%Y-%m-%d}.log")

    fmt = logging.Formatter("[%(asctime)s] %(message)s", datefmt="%Y-%m-%d %H:%M:%S")
    log.setLevel(logging.INFO)

    console = logging.StreamHandler(sys.stdout)
    console.setFormatter(fmt)
    log.addHandler(console)

    fh = logging.FileHandler(log_file, mode="a")
    fh.setFormatter(fmt)
    log.addHandler(fh)


# ── Fintool JSON helper ──────────────────────────────────────────────────────

def ft(cmd: dict, fintool: str) -> dict:
    """Call fintool in JSON mode. Returns parsed JSON output."""
    try:
        result = subprocess.run(
            [fintool, "--json", json.dumps(cmd)],
            capture_output=True, text=True, timeout=30,
        )
        return json.loads(result.stdout)
    except (json.JSONDecodeError, subprocess.TimeoutExpired) as e:
        return {"error": str(e)}


# ── HTTP helpers ──────────────────────────────────────────────────────────────

def http_post_json(url: str, data: dict, headers: dict | None = None, timeout: int = 15) -> dict | list | None:
    """POST JSON and return parsed response."""
    body = json.dumps(data).encode()
    hdrs = {"Content-Type": "application/json"}
    if headers:
        hdrs.update(headers)
    req = urllib.request.Request(url, data=body, headers=hdrs)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return json.loads(resp.read())
    except Exception as e:
        log.error("HTTP POST %s failed: %s", url, e)
        return None


def http_get_json(url: str, headers: dict | None = None, timeout: int = 15) -> dict | None:
    """GET and return parsed JSON response."""
    req = urllib.request.Request(url, headers=headers or {})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return json.loads(resp.read())
    except Exception as e:
        log.error("HTTP GET %s failed: %s", url, e)
        return None


# ── Step 1: News search ──────────────────────────────────────────────────────

def search_news(query: str, brave_key: str) -> str:
    """Search Brave News API for commodity headlines."""
    encoded = urllib.parse.quote_plus(query)
    url = f"https://api.search.brave.com/res/v1/news/search?q={encoded}&count=10&freshness=pd"
    data = http_get_json(url, headers={
        "Accept": "application/json",
        "X-Subscription-Token": brave_key,
    })
    if not data or "results" not in data:
        return "No results"
    lines = []
    for r in data["results"]:
        title = r.get("title", "")
        desc = r.get("description", "")
        lines.append(f"{title} — {desc}" if desc else title)
    return "\n".join(lines)


# ── Step 2: LLM sentiment analysis ───────────────────────────────────────────

def analyze_sentiment(gold_news: str, silver_news: str, openai_key: str) -> dict:
    """Ask OpenAI to analyze sentiment from news headlines."""
    prompt = f"""You are a commodities trading analyst. Below are today's news headlines for GOLD and SILVER.

GOLD NEWS:
{gold_news}

SILVER NEWS:
{silver_news}

Analyze and respond in EXACTLY this JSON format (no markdown, no explanation):
{{
  "more_talked_about": "GOLD" or "SILVER",
  "gold_sentiment": number from -1.0 (very bearish) to 1.0 (very bullish),
  "silver_sentiment": number from -1.0 (very bearish) to 1.0 (very bullish),
  "gold_headline_count": number,
  "silver_headline_count": number,
  "reasoning": "one sentence"
}}"""

    resp = http_post_json(
        "https://api.openai.com/v1/chat/completions",
        data={
            "model": "gpt-4o-mini",
            "temperature": 0.1,
            "messages": [{"role": "user", "content": prompt}],
        },
        headers={"Authorization": f"Bearer {openai_key}"},
        timeout=30,
    )

    if not resp:
        return {"gold_sentiment": 0, "silver_sentiment": 0}

    try:
        content = resp["choices"][0]["message"]["content"]
        return json.loads(content)
    except (KeyError, json.JSONDecodeError) as e:
        log.error("Failed to parse sentiment response: %s", e)
        return {"gold_sentiment": 0, "silver_sentiment": 0}


# ── Step 5: Trading decision ─────────────────────────────────────────────────

def make_decision(gold_data: dict, silver_data: dict, openai_key: str) -> dict:
    """Ask OpenAI to decide which metal to long and which to short."""
    prompt = f"""You are a quantitative trading system deciding a pairs trade between GOLD and SILVER perps.

DATA:
- GOLD:   24h_change={gold_data['change']}%, sentiment={gold_data['sentiment']}, funding_rate={gold_data['funding']}, price={gold_data['price']}
- SILVER: 24h_change={silver_data['change']}%, sentiment={silver_data['sentiment']}, funding_rate={silver_data['funding']}, price={silver_data['price']}

RULES:
1. Long the metal with stronger bullish momentum + sentiment, short the other
2. Prefer longing the one with negative/lower funding (you get paid)
3. If signals conflict, weight: momentum 40%, sentiment 35%, funding 25%
4. If both are nearly identical (within 0.5% change, similar sentiment), output "HOLD"

Respond in EXACTLY this JSON (no markdown):
{{
  "action": "TRADE" or "HOLD",
  "long": "GOLD" or "SILVER",
  "short": "GOLD" or "SILVER",
  "confidence": number 0-1,
  "reasoning": "one sentence"
}}"""

    resp = http_post_json(
        "https://api.openai.com/v1/chat/completions",
        data={
            "model": "gpt-4o-mini",
            "temperature": 0.0,
            "messages": [{"role": "user", "content": prompt}],
        },
        headers={"Authorization": f"Bearer {openai_key}"},
        timeout=30,
    )

    if not resp:
        return {"action": "HOLD", "reasoning": "API error"}

    try:
        content = resp["choices"][0]["message"]["content"]
        return json.loads(content)
    except (KeyError, json.JSONDecodeError) as e:
        log.error("Failed to parse decision response: %s", e)
        return {"action": "HOLD", "reasoning": f"Parse error: {e}"}


# ── Step 6: Close positions ──────────────────────────────────────────────────

def close_position(symbol: str, positions: list | dict, fintool: str, dry_run: bool):
    """Close a single perp position if it exists."""
    if not isinstance(positions, list):
        return

    for p in positions:
        pos = p.get("position", p)
        coin = pos.get("coin") or pos.get("symbol", "")
        # Match "GOLD" or "cash:GOLD"
        if coin != symbol and coin != f"cash:{symbol}":
            continue

        size_str = pos.get("szi") or pos.get("size") or "0"
        size = float(size_str)
        if size == 0:
            continue

        abs_size = abs(size)
        entry_px = float(pos.get("entryPx") or 0)

        if dry_run:
            log.info("  [DRY RUN] Would close %s (size=%s)", symbol, size_str)
            return

        if size > 0:
            close_price = f"{entry_px * 0.95:.2f}"
            log.info("  Closing LONG %s: sell %s @ %s --close", symbol, abs_size, close_price)
            ft({"command": "perp_sell", "symbol": symbol,
                "amount": str(abs_size), "price": close_price, "close": True}, fintool)
        else:
            close_price = f"{entry_px * 1.05:.2f}"
            log.info("  Closing SHORT %s: buy %s @ %s --close", symbol, abs_size, close_price)
            ft({"command": "perp_buy", "symbol": symbol,
                "amount": str(abs_size), "price": close_price, "close": True}, fintool)
        time.sleep(3)
        return


# ── Step 7: USDT0 rebalancing ────────────────────────────────────────────────

def normalize_usdt0(target: float, fintool: str, dry_run: bool):
    """Ensure exactly $target USDT0 is in the HIP-3 dex."""

    # Transfer all USDT0 from HIP-3 dex back to spot
    log.info("Transferring all USDT0 from HIP-3 dex to spot...")
    ft({"command": "transfer", "asset": "USDT0", "amount": "999999",
        "from": "cash", "to": "spot"}, fintool)
    time.sleep(3)

    # Check balances
    balance = ft({"command": "balance"}, fintool)

    usdt0_balance = 0.0
    usdc_balance = 0.0

    if isinstance(balance, dict):
        # Parse spot USDT0
        spot = balance.get("spot", {})
        for b in spot.get("balances", []):
            if b.get("coin") == "USDT0":
                usdt0_balance = float(b.get("total", 0))
        # Parse perp USDC
        perp = balance.get("perp", {})
        margin = perp.get("marginSummary", {})
        usdc_balance = float(margin.get("accountValue", 0) or perp.get("withdrawable", 0))

    log.info("Current balances — USDT0: %.2f, USDC: %.2f", usdt0_balance, usdc_balance)

    diff = usdt0_balance - target

    if dry_run:
        log.info("  [DRY RUN] USDT0 diff=%.2f (would %s)", diff,
                 f"sell {diff:.0f}" if diff > 1 else f"buy {abs(diff):.0f}" if diff < -1 else "no action")
    elif diff > 1:
        sell_amount = int(diff)
        log.info("Excess USDT0: selling %d USDT0 -> USDC", sell_amount)
        ft({"command": "order_sell", "symbol": "USDT0",
            "amount": str(sell_amount), "price": "0.998"}, fintool)
        time.sleep(5)
    elif diff < -1:
        buy_amount = int(abs(diff))
        if usdc_balance < buy_amount:
            bridge_amount = int(buy_amount - usdc_balance + 10)
            log.info("Insufficient USDC (%.2f). Bridging %d USDC from Base...", usdc_balance, bridge_amount)
            ft({"command": "deposit", "asset": "USDC",
                "amount": str(bridge_amount), "from": "base"}, fintool)
            time.sleep(10)
        log.info("Buying %d USDT0 with USDC", buy_amount)
        ft({"command": "order_buy", "symbol": "USDT0",
            "amount": str(buy_amount), "price": "1.003"}, fintool)
        time.sleep(5)
    else:
        log.info("USDT0 balance is within target range (diff: %.2f)", diff)

    # Transfer target amount to HIP-3 dex
    log.info("Transferring $%d USDT0 to HIP-3 dex...", target)
    if not dry_run:
        ft({"command": "transfer", "asset": "USDT0", "amount": str(int(target)),
            "from": "spot", "to": "cash"}, fintool)
        time.sleep(3)


# ── Main ──────────────────────────────────────────────────────────────────────

def run(cfg: dict):
    fintool = cfg["fintool"]
    dry_run = cfg["dry_run"]

    openai_key = OPENAI_API_KEY
    brave_key = BRAVE_API_KEY

    if not openai_key:
        log.error("OPENAI_API_KEY not set — edit the constant at the top of this file")
        sys.exit(1)
    if not brave_key:
        log.error("BRAVE_API_KEY not set — edit the constant at the top of this file")
        sys.exit(1)

    log.info("=== Metal Pairs Bot Starting ===")
    if dry_run:
        log.info("  [DRY RUN MODE]")

    # ── Step 1: Search news ───────────────────────────────────────────────
    log.info("Fetching GOLD news...")
    gold_news = search_news("gold commodity price market", brave_key)
    log.info("Fetching SILVER news...")
    silver_news = search_news("silver commodity price market", brave_key)

    # ── Step 2: Sentiment analysis ────────────────────────────────────────
    log.info("Analyzing sentiment with LLM...")
    sentiment = analyze_sentiment(gold_news, silver_news, openai_key)
    log.info("Sentiment: %s", json.dumps(sentiment))

    gold_sentiment = float(sentiment.get("gold_sentiment", 0))
    silver_sentiment = float(sentiment.get("silver_sentiment", 0))

    # ── Step 3: Get price quotes and funding rates ────────────────────────
    log.info("Fetching price quotes...")
    gold_quote = ft({"command": "perp_quote", "symbol": "GOLD"}, fintool)
    silver_quote = ft({"command": "perp_quote", "symbol": "SILVER"}, fintool)

    gold_price = float(gold_quote.get("markPx") or 0)
    silver_price = float(silver_quote.get("markPx") or 0)
    gold_funding = float(gold_quote.get("funding") or 0)
    silver_funding = float(silver_quote.get("funding") or 0)

    # Compute 24h change from perp data
    gold_prev = float(gold_quote.get("prevDayPx") or gold_price)
    silver_prev = float(silver_quote.get("prevDayPx") or silver_price)
    gold_change = ((gold_price - gold_prev) / gold_prev * 100) if gold_prev else 0
    silver_change = ((silver_price - silver_prev) / silver_prev * 100) if silver_prev else 0

    log.info("GOLD:   price=%.2f  24h_change=%.2f%%  funding=%s", gold_price, gold_change, gold_funding)
    log.info("SILVER: price=%.2f  24h_change=%.2f%%  funding=%s", silver_price, silver_change, silver_funding)

    # ── Step 4: Trading decision ──────────────────────────────────────────
    log.info("Computing trading decision...")
    decision = make_decision(
        {"change": f"{gold_change:.2f}", "sentiment": gold_sentiment,
         "funding": gold_funding, "price": gold_price},
        {"change": f"{silver_change:.2f}", "sentiment": silver_sentiment,
         "funding": silver_funding, "price": silver_price},
        openai_key,
    )
    log.info("Decision: %s", json.dumps(decision))

    action = decision.get("action", "HOLD")
    long_metal = decision.get("long", "GOLD")
    short_metal = decision.get("short", "SILVER")
    confidence = decision.get("confidence", 0)
    reasoning = decision.get("reasoning", "unknown")

    if action == "HOLD":
        log.info("Decision: HOLD — no trade today. Reason: %s", reasoning)
        log.info("=== Bot Complete (no trade) ===")
        return

    log.info("Decision: LONG %s / SHORT %s (confidence: %s)", long_metal, short_metal, confidence)

    # ── Step 5: Close existing positions ──────────────────────────────────
    log.info("Closing all existing positions...")
    positions = ft({"command": "positions"}, fintool)
    close_position("GOLD", positions, fintool, dry_run)
    close_position("SILVER", positions, fintool, dry_run)
    time.sleep(5)

    # ── Step 6: Normalize USDT0 ──────────────────────────────────────────
    target_usdt0 = cfg["target_usdt0"]
    log.info("Normalizing USDT0 balance to $%d...", target_usdt0)
    normalize_usdt0(target_usdt0, fintool, dry_run)

    # ── Step 7: Set leverage and open positions ──────────────────────────
    leverage = cfg["leverage"]
    position_size = cfg["position_size_usd"]

    log.info("Setting leverage to %dx...", leverage)
    if not dry_run:
        ft({"command": "perp_leverage", "symbol": long_metal, "leverage": leverage}, fintool)
        ft({"command": "perp_leverage", "symbol": short_metal, "leverage": leverage}, fintool)

    # Calculate sizes and limits
    long_price = gold_price if long_metal == "GOLD" else silver_price
    short_price = silver_price if long_metal == "GOLD" else gold_price

    long_limit = f"{long_price * 1.005:.2f}"
    short_limit = f"{short_price * 0.995:.2f}"
    long_size = f"{position_size / long_price:.4f}"
    short_size = f"{position_size / short_price:.4f}"

    margin_per_leg = position_size / leverage
    log.info("Opening LONG %s: %s units ($%d notional) @ limit %s (margin: $%.0f)",
             long_metal, long_size, position_size, long_limit, margin_per_leg)

    if not dry_run:
        result = ft({"command": "perp_buy", "symbol": long_metal,
                     "amount": long_size, "price": long_limit}, fintool)
        log.info("  Result: %s", json.dumps(result))

    log.info("Opening SHORT %s: %s units ($%d notional) @ limit %s (margin: $%.0f)",
             short_metal, short_size, position_size, short_limit, margin_per_leg)

    if not dry_run:
        result = ft({"command": "perp_sell", "symbol": short_metal,
                     "amount": short_size, "price": short_limit}, fintool)
        log.info("  Result: %s", json.dumps(result))

    time.sleep(5)

    # ── Step 8: Verify positions ─────────────────────────────────────────
    log.info("Verifying positions...")
    final_positions = ft({"command": "positions"}, fintool)
    final_balance = ft({"command": "balance"}, fintool)
    log.info("Positions: %s", json.dumps(final_positions, indent=2))
    log.info("Balance: %s", json.dumps(final_balance, indent=2))

    log.info("=== Bot Complete ===")
    log.info("Summary: LONG %s / SHORT %s | $%d/leg | %dx leverage",
             long_metal, short_metal, position_size, leverage)
    log.info("Reasoning: %s", reasoning)

    # Output summary JSON for programmatic consumption
    summary = {
        "timestamp": datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ"),
        "action": action,
        "long": long_metal,
        "short": short_metal,
        "position_size_usd": position_size,
        "leverage": leverage,
        "confidence": confidence,
        "gold_24h_change": round(gold_change, 2),
        "silver_24h_change": round(silver_change, 2),
        "gold_sentiment": gold_sentiment,
        "silver_sentiment": silver_sentiment,
        "gold_funding": gold_funding,
        "silver_funding": silver_funding,
        "reasoning": reasoning,
    }
    print(json.dumps(summary, indent=2))


# ── Entry point ───────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Metal Pairs Trading Bot (GOLD vs SILVER)")
    parser.add_argument("--dry-run", action="store_true", help="Log actions without executing trades")
    parser.add_argument("--fintool", default=DEFAULTS["fintool"], help="Path to fintool binary")
    parser.add_argument("--target-usdt0", type=int, default=DEFAULTS["target_usdt0"],
                        help=f"Target USDT0 margin (default: {DEFAULTS['target_usdt0']})")
    parser.add_argument("--position-size", type=int, default=DEFAULTS["position_size_usd"],
                        help=f"Notional per leg in USD (default: {DEFAULTS['position_size_usd']})")
    parser.add_argument("--leverage", type=int, default=DEFAULTS["leverage"],
                        help=f"Leverage (default: {DEFAULTS['leverage']})")
    parser.add_argument("--log-dir", default=DEFAULTS["log_dir"], help="Log directory")
    args = parser.parse_args()

    setup_logging(args.log_dir)

    cfg = {
        "fintool": args.fintool,
        "dry_run": args.dry_run,
        "target_usdt0": args.target_usdt0,
        "position_size_usd": args.position_size,
        "leverage": args.leverage,
    }

    run(cfg)


if __name__ == "__main__":
    main()

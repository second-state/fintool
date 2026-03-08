#!/usr/bin/env python3
"""
Funding Rate Arbitrage Bot

Delta-neutral strategy: buy spot + short perp on the Hyperliquid asset with the
highest positive funding rate. Collect hourly funding payments while staying
market-neutral. If funding turns negative, unwind and wait.

Usage:
    python3 bot.py [--dry-run] [--interval 3600]

Requires: fintool CLI, OPENAI_API_KEY env var (optional)
"""

import argparse
import json
import logging
import os
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

# ── Config ────────────────────────────────────────────────────────────────────

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_DIR = SCRIPT_DIR.parent.parent

DEFAULTS = {
    "fintool": os.environ.get("FINTOOL", str(REPO_DIR / "target" / "release" / "fintool")),
    "check_interval": 3600,       # 1 hour (matches Hyperliquid funding interval)
    "slippage_bps": 50,           # 0.5% slippage tolerance for limit orders
    "min_funding": 0.0001,        # Minimum funding rate to enter (0.01% per hour)
    "min_volume": 1_000_000,      # Minimum 24h perp volume in USD
    "leverage": 1,                # 1x leverage for perp short (delta neutral)
    "position_pct": 90,           # Use 90% of available USDC (keep 10% buffer)
    "log_file": "/tmp/funding_arb.log",
}

# Assets available on both spot and perp (spot ticker -> perp ticker)
SPOT_TO_PERP = {
    "HYPE": "HYPE", "PURR": "PURR", "TRUMP": "TRUMP", "PUMP": "PUMP",
    "BERA": "BERA", "MON": "MON", "ANIME": "ANIME",
    "LINK0": "LINK", "AVAX0": "AVAX", "AAVE0": "AAVE",
    "XMR1": "XMR", "BNB0": "BNB", "XRP1": "XRP",
}

# Reverse map: perp ticker -> spot ticker
PERP_TO_SPOT = {v: k for k, v in SPOT_TO_PERP.items()}

# ── Logging ───────────────────────────────────────────────────────────────────

log = logging.getLogger("funding_arb")


def setup_logging(log_file: str):
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


def ft_or_fail(cmd: dict, fintool: str) -> dict | None:
    """Call fintool in JSON mode; return None on failure."""
    result = ft(cmd, fintool)
    if "error" in result:
        log.error("fintool failed: %s", result["error"])
        return None
    return result


# ── Hyperliquid API helpers ──────────────────────────────────────────────────

HL_API = "https://api.hyperliquid.xyz/info"


def hl_post(payload: dict) -> dict | list | None:
    """POST to Hyperliquid info API."""
    data = json.dumps(payload).encode()
    req = urllib.request.Request(HL_API, data=data, headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            return json.loads(resp.read())
    except Exception as e:
        log.error("Hyperliquid API error: %s", e)
        return None


def fetch_all_funding() -> list | None:
    return hl_post({"type": "metaAndAssetCtxs"})


def fetch_spot_book(coin: str) -> dict | None:
    return hl_post({"type": "l2Book", "coin": coin})


# ── OpenAI analysis ──────────────────────────────────────────────────────────

def analyze_with_openai(candidates: list, api_key: str) -> dict | None:
    """Ask OpenAI to pick the best funding arb candidate."""
    prompt = f"""You are a quantitative trading analyst. Analyze these Hyperliquid assets for a funding rate arbitrage trade (buy spot + short perp to collect positive funding).

For each candidate, I'm providing: symbol, funding rate (hourly), 24h perp volume, open interest, spot bid/ask spread, and spot depth.

Candidates:
{json.dumps(candidates, indent=2)}

Pick the SINGLE best asset to trade. Consider:
1. Funding rate magnitude (higher = more profit)
2. Spot liquidity (tight spread, good depth = lower entry/exit cost)
3. Perp volume and OI (higher = more liquid, easier to short)
4. Risk (avoid very new or volatile meme tokens if possible)

Respond in EXACTLY this JSON format, nothing else:
{{"pick": "SYMBOL", "reason": "one sentence reason", "confidence": "high|medium|low"}}"""

    body = json.dumps({
        "model": "gpt-4o-mini",
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.1,
        "max_tokens": 200,
    }).encode()

    req = urllib.request.Request(
        "https://api.openai.com/v1/chat/completions",
        data=body,
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_key}",
        },
    )

    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.loads(resp.read())
        content = data["choices"][0]["message"]["content"]
        return json.loads(content)
    except Exception as e:
        log.error("OpenAI error: %s", e)
        return None


# ── Core logic ────────────────────────────────────────────────────────────────

def get_current_state(fintool: str) -> dict:
    """Get current positions and USDC balance."""
    positions = ft({"command": "positions"}, fintool)
    balance = ft({"command": "balance"}, fintool)

    has_positions = 0
    if isinstance(positions, list):
        has_positions = sum(
            1 for p in positions
            if p.get("size") not in (None, "0", "0.0", 0)
        )

    usdc = "0"
    if isinstance(balance, list):
        for b in balance:
            coin = b.get("coin") or b.get("asset")
            if coin == "USDC":
                usdc = str(b.get("available") or b.get("free") or b.get("balance", "0"))
                break
    elif isinstance(balance, dict):
        usdc = str(balance.get("USDC") or balance.get("usdc") or balance.get("available", "0"))

    return {
        "has_positions": has_positions,
        "usdc_available": usdc,
        "positions": positions,
        "balance": balance,
    }


def gather_candidates(cfg: dict) -> list:
    """Scan all spot/perp pairs for funding arb opportunities."""
    all_data = fetch_all_funding()
    if not all_data:
        log.error("Failed to fetch funding data")
        return []

    universe = all_data[0]["universe"]
    ctxs = all_data[1]

    # Build perp lookup
    perp_lookup = {}
    for u, c in zip(universe, ctxs):
        perp_lookup[u["name"]] = c

    candidates = []
    for spot_ticker, perp_ticker in SPOT_TO_PERP.items():
        ctx = perp_lookup.get(perp_ticker)
        if not ctx:
            continue

        funding = float(ctx["funding"])
        volume = float(ctx["dayNtlVlm"])
        mark_px = float(ctx["markPx"])
        oi = float(ctx["openInterest"]) * mark_px

        if funding <= 0 or volume < cfg["min_volume"]:
            continue

        # Fetch spot orderbook for spread analysis
        spread_data = {"spread_pct": 99, "bid_depth_usd": 0, "ask_depth_usd": 0}
        book = fetch_spot_book(f"@{spot_ticker}")
        if book and "levels" in book:
            levels = book["levels"]
            bids = levels[0] if len(levels) > 0 else []
            asks = levels[1] if len(levels) > 1 else []
            if bids and asks:
                best_bid = float(bids[0]["px"])
                best_ask = float(asks[0]["px"])
                spread_pct = (best_ask - best_bid) / best_bid * 100 if best_bid > 0 else 99
                bid_depth = sum(float(b["sz"]) * float(b["px"]) for b in bids[:5])
                ask_depth = sum(float(a["sz"]) * float(a["px"]) for a in asks[:5])
                spread_data = {
                    "spread_pct": round(spread_pct, 4),
                    "bid_depth_usd": round(bid_depth, 2),
                    "ask_depth_usd": round(ask_depth, 2),
                }

        candidates.append({
            "perp_ticker": perp_ticker,
            "spot_ticker": spot_ticker,
            "funding": funding,
            "markPx": mark_px,
            "volume24h": volume,
            "openInterest": oi,
            **spread_data,
        })

    candidates.sort(key=lambda c: c["funding"], reverse=True)
    return candidates


def open_position(spot_ticker: str, perp_ticker: str, usdc_amount: float, cfg: dict):
    """Open delta-neutral position: buy spot + short perp."""
    fintool = cfg["fintool"]
    half = usdc_amount / 2

    log.info("Opening position: spot=%s perp=%s amount=$%.2f ($%.2f each side)",
             spot_ticker, perp_ticker, usdc_amount, half)

    # Get current prices via fintool JSON
    perp_quote = ft({"command": "perp_quote", "symbol": perp_ticker}, fintool)
    spot_quote = ft({"command": "quote", "symbol": spot_ticker}, fintool)

    perp_price = float(perp_quote.get("markPx") or 0)
    spot_price = float(spot_quote.get("price") or spot_quote.get("markPx") or 0)

    if not perp_price or not spot_price:
        log.error("Could not fetch prices")
        return

    # Calculate limit prices with slippage and sizes
    slippage = cfg["slippage_bps"] / 10_000
    spot_limit = spot_price * (1 + slippage)
    perp_limit = perp_price * (1 - slippage)
    spot_size = half / spot_price

    log.info("  Spot: buy %.6f %s at limit $%.6f", spot_size, spot_ticker, spot_limit)
    log.info("  Perp: sell (short) %.6f %s at limit $%.6f", spot_size, perp_ticker, perp_limit)

    if cfg["dry_run"]:
        log.info("  [DRY RUN] Skipping actual trades")
        return

    # Set leverage
    ft({"command": "perp_leverage", "symbol": perp_ticker,
        "leverage": cfg["leverage"], "cross": True}, fintool)

    # Buy spot
    spot_result = ft({"command": "order_buy", "symbol": spot_ticker,
                      "amount": f"{spot_size:.6f}", "price": f"{spot_limit:.6f}"}, fintool)
    log.info("  Spot buy status: %s", spot_result.get("fillStatus", "unknown"))

    # Short perp
    perp_result = ft({"command": "perp_sell", "symbol": perp_ticker,
                      "amount": f"{spot_size:.6f}", "price": f"{perp_limit:.6f}"}, fintool)
    log.info("  Perp short status: %s", perp_result.get("fillStatus", "unknown"))

    log.info("  Position opened successfully")


def close_all_positions(cfg: dict):
    """Close all perp positions and sell all spot holdings."""
    fintool = cfg["fintool"]
    slippage = cfg["slippage_bps"] / 10_000

    log.info("Closing all positions...")

    # Close each perp position
    positions = ft({"command": "positions"}, fintool)
    if isinstance(positions, list):
        for pos in positions:
            size_str = pos.get("size") or pos.get("positionSize") or "0"
            size = float(size_str)
            if size == 0:
                continue

            symbol = pos.get("coin") or pos.get("symbol")
            if not symbol:
                continue

            abs_size = abs(size)
            log.info("  Closing perp: %s size=%s", symbol, size_str)

            quote = ft({"command": "perp_quote", "symbol": symbol}, fintool)
            price = float(quote.get("markPx") or 0)
            if not price:
                continue

            if cfg["dry_run"]:
                log.info("  [DRY RUN] Would close %s perp", symbol)
                continue

            if size < 0:  # short -> buy to close
                limit = price * (1 + slippage)
                ft({"command": "perp_buy", "symbol": symbol,
                    "amount": f"{abs_size:.6f}", "price": f"{limit:.6f}", "close": True}, fintool)
            else:  # long -> sell to close
                limit = price * (1 - slippage)
                ft({"command": "perp_sell", "symbol": symbol,
                    "amount": f"{abs_size:.6f}", "price": f"{limit:.6f}", "close": True}, fintool)
            log.info("  Closed perp %s", symbol)

    # Sell all spot holdings except USDC
    balance = ft({"command": "balance"}, fintool)
    if isinstance(balance, list):
        for holding in balance:
            coin = holding.get("coin") or holding.get("asset")
            if not coin or coin == "USDC":
                continue
            amount_str = str(holding.get("total") or holding.get("balance") or holding.get("available", "0"))
            amount = float(amount_str)
            if amount == 0:
                continue

            log.info("  Selling spot: %s amount=%s", coin, amount_str)

            quote = ft({"command": "quote", "symbol": coin}, fintool)
            price = float(quote.get("price") or quote.get("markPx") or 0)
            if not price:
                log.warning("  Cannot get price for %s, skipping", coin)
                continue

            if cfg["dry_run"]:
                log.info("  [DRY RUN] Would sell %s %s", amount_str, coin)
                continue

            sell_limit = price * (1 - slippage)
            ft({"command": "order_sell", "symbol": coin,
                "amount": amount_str, "price": f"{sell_limit:.6f}"}, fintool)
            log.info("  Sold spot %s", coin)

    log.info("All positions closed")


def check_current_funding(cfg: dict) -> str:
    """Check if current position's funding is still positive. Returns 'positive', 'negative', or 'none'."""
    fintool = cfg["fintool"]
    positions = ft({"command": "positions"}, fintool)

    short_symbol = None
    if isinstance(positions, list):
        for pos in positions:
            size_str = pos.get("size") or "0"
            if size_str not in ("0", "0.0", None):
                short_symbol = pos.get("coin") or pos.get("symbol")
                break

    if not short_symbol:
        return "none"

    quote = ft({"command": "perp_quote", "symbol": short_symbol}, fintool)
    funding = float(quote.get("funding") or 0)
    log.info("Current position: %s | Funding rate: %s", short_symbol, funding)

    return "negative" if funding < 0 else "positive"


# ── Main loop ─────────────────────────────────────────────────────────────────

def run(cfg: dict):
    log.info("=" * 55)
    log.info("  Funding Rate Arbitrage Bot")
    log.info("  Dry run: %s | Interval: %ds", cfg["dry_run"], cfg["check_interval"])
    log.info("  Min funding: %s | Min volume: $%s", cfg["min_funding"], cfg["min_volume"])
    log.info("=" * 55)

    openai_key = os.environ.get("OPENAI_API_KEY", "")

    while True:
        log.info("────── Check cycle ──────")

        state = get_current_state(cfg["fintool"])
        has_positions = state["has_positions"]
        usdc = float(state["usdc_available"])
        log.info("Account: positions=%d USDC=%.2f", has_positions, usdc)

        if has_positions > 0:
            funding_status = check_current_funding(cfg)
            if funding_status == "negative":
                log.info("Funding turned NEGATIVE — closing all positions")
                close_all_positions(cfg)
            elif funding_status == "none":
                log.info("No perp position found but expected one — resetting")
                close_all_positions(cfg)
            else:
                log.info("Funding still positive — holding position")
        else:
            log.info("Scanning for funding opportunities...")
            candidates = gather_candidates(cfg)

            if not candidates:
                log.info("No assets with positive funding above threshold. Waiting...")
            else:
                log.info("Found %d candidates with positive funding:", len(candidates))
                for c in candidates:
                    log.info("  %s: funding=%s vol=$%.0f spread=%.2f%% depth=$%.0f",
                             c["perp_ticker"], c["funding"], c["volume24h"],
                             c["spread_pct"], c["bid_depth_usd"])

                pick_perp = None
                pick_spot = None

                if openai_key:
                    log.info("Asking OpenAI to analyze candidates...")
                    analysis = analyze_with_openai(candidates, openai_key)
                    if analysis:
                        log.info("OpenAI analysis: %s", json.dumps(analysis))
                        pick_perp = analysis.get("pick")
                        pick_spot = PERP_TO_SPOT.get(pick_perp)
                    else:
                        log.info("OpenAI did not return a valid pick, skipping this cycle")
                else:
                    # No OpenAI key — just pick highest funding rate
                    top = candidates[0]
                    pick_perp = top["perp_ticker"]
                    pick_spot = top["spot_ticker"]
                    log.info("No OPENAI_API_KEY — picking highest funding: %s", pick_perp)

                if pick_perp and pick_spot:
                    trade_amount = usdc * cfg["position_pct"] / 100
                    log.info("Selected: %s (spot: %s) — deploying $%.2f",
                             pick_perp, pick_spot, trade_amount)
                    open_position(pick_spot, pick_perp, trade_amount, cfg)

        log.info("Sleeping %ds until next check...", cfg["check_interval"])
        time.sleep(cfg["check_interval"])


# ── Entry point ───────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Funding Rate Arbitrage Bot")
    parser.add_argument("--dry-run", action="store_true", help="Log actions without executing trades")
    parser.add_argument("--interval", type=int, default=DEFAULTS["check_interval"],
                        help=f"Seconds between checks (default: {DEFAULTS['check_interval']})")
    parser.add_argument("--fintool", default=DEFAULTS["fintool"], help="Path to fintool binary")
    parser.add_argument("--min-funding", type=float, default=DEFAULTS["min_funding"])
    parser.add_argument("--min-volume", type=float, default=DEFAULTS["min_volume"])
    parser.add_argument("--slippage-bps", type=int, default=DEFAULTS["slippage_bps"])
    parser.add_argument("--leverage", type=int, default=DEFAULTS["leverage"])
    parser.add_argument("--position-pct", type=int, default=DEFAULTS["position_pct"])
    parser.add_argument("--log-file", default=DEFAULTS["log_file"])
    args = parser.parse_args()

    # Build fintool binary if not found
    fintool = args.fintool
    if not os.path.isfile(fintool):
        print("Building fintool...")
        subprocess.run(["cargo", "build", "--release"], cwd=str(REPO_DIR), check=True)
        if not os.path.isfile(fintool):
            print(f"ERROR: Build failed — binary not found at {fintool}")
            sys.exit(1)

    setup_logging(args.log_file)

    cfg = {
        "fintool": fintool,
        "dry_run": args.dry_run,
        "check_interval": args.interval,
        "slippage_bps": args.slippage_bps,
        "min_funding": args.min_funding,
        "min_volume": args.min_volume,
        "leverage": args.leverage,
        "position_pct": args.position_pct,
    }

    run(cfg)


if __name__ == "__main__":
    main()

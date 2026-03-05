#!/usr/bin/env bash
#
# End-to-end fintool workflow
#
# This script illustrates the full deposit → trade → withdraw cycle.
# Each command is a real fintool invocation. Run them individually or
# execute this script end-to-end.
#
# Prerequisites:
#   - cargo build --release
#   - ~/.fintool/config.toml configured with wallet + API keys
#   - ETH on Base for gas fees
#   - USDC on Base to deposit
#

set -euo pipefail

# ── 1. Deposit USDC from Base to Hyperliquid ───────────────────────
# Bridges $15 USDC: Base → Across → Arbitrum → HL Bridge2 → Hyperliquid
fintool deposit USDC --amount 15 --from base

# Wait for deposit to settle (~5 min)
sleep 300

# Enable unified mode so USDC is shared across perp + spot
fintool perp set-mode unified

# Check that funds arrived
fintool balance

# ── 2. Trade crypto perps (ETH) ────────────────────────────────────
# In unified mode, no transfers needed — USDC is shared

# Set leverage and get a quote
fintool perp leverage ETH --leverage 2
fintool perp quote ETH

# Buy 0.006 ETH perp at a limit price (adjust price to current market)
fintool perp buy ETH --amount 0.006 --price 2100.00

# Check positions
fintool positions

# Sell to close the position (adjust size and price to your position)
fintool perp sell ETH --amount 0.006 --price 2050.00 --close

# ── 3. Trade spot (HYPE) ───────────────────────────────────────────
# Get a quote and buy
fintool quote HYPE
fintool order buy HYPE --amount 0.48 --price 25.00

# Check balance to see HYPE tokens
fintool balance

# Sell all HYPE back to USDC
fintool order sell HYPE --amount 0.48 --price 24.50

# ── 4. Trade SILVER perp (HIP-3 cash dex) ──────────────────────────
# The cash dex uses USDT0 as collateral, not USDC.
# Step 1: Swap USDC → USDT0 on spot
fintool order buy USDT0 --amount 30 --price 1.002

# Step 2: Transfer USDT0 from spot to cash dex
fintool transfer USDT0 --amount 30 --from spot --to cash

# Step 3: Set leverage and buy SILVER perp
fintool perp leverage SILVER --leverage 2
fintool perp quote SILVER
fintool perp buy SILVER --amount 0.13 --price 89.00

# Check positions (SILVER shows as "cash:SILVER")
fintool positions

# Sell SILVER to close
fintool perp sell SILVER --amount 0.14 --price 87.00 --close

# Step 4: Transfer USDT0 back to spot and swap to USDC
fintool transfer USDT0 --amount 30 --from cash --to spot
fintool order sell USDT0 --amount 30 --price 0.998

# ── 5. Check status ────────────────────────────────────────────────
fintool positions
fintool orders
fintool balance

# ── 6. Withdraw to Base ────────────────────────────────────────────
# Bridges back: Hyperliquid → Arbitrum → Across → Base
fintool withdraw USDC --amount 10 --to base

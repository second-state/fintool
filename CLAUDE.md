# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

```bash
# Build all binaries (release)
cargo build --release
# Binaries output to: ./target/release/{fintool,hyperliquid,binance,coinbase,okx,polymarket}

# Run Rust unit tests
cargo test --release

# Check formatting
cargo fmt -- --check

# Apply formatting
cargo fmt

# Clippy linting
cargo clippy --release -- -D warnings
```

## Testing

The Rust test suite (`cargo test`) covers unit tests only. Exchange-specific end-to-end tests are shell scripts under `tests/` that require live credentials and real exchange access — they are not run in CI automatically.

```bash
# Run a specific Rust test
cargo test <test_name>

# E2E shell tests (require configured credentials)
bash tests/hyperliquid/e2e_trading.sh
bash tests/binance/e2e_trading.sh
bash tests/okx/e2e_trading.sh
```

## Architecture

This is a multi-binary Rust workspace with one library crate and six CLI binaries.

### Binary Structure

Each binary (`src/bin/*.rs`) handles CLI argument parsing via `clap` and dispatches to shared command handlers:

- `fintool` — exchange-agnostic market intelligence (quotes, news, SEC filings); no authentication required for most commands
- `hyperliquid` — spot + perp + HIP-3 commodity/stock perps; uses EIP-712 signing with a wallet private key
- `binance` — spot + futures; HMAC-SHA256 signed requests with API key/secret
- `coinbase` — spot only; HMAC-SHA256 signed requests
- `okx` — spot + perp; HMAC-SHA256 + base64 signed requests, plus a passphrase
- `polymarket` — prediction markets on Polygon; EIP-712 signing via `alloy`

Each binary also accepts a `--json` flag where the entire command is passed as a JSON string and all output (including errors) is returned as JSON — this is the primary interface for agent/script integration.

There is no `--exchange` flag. Exchange selection is done by invoking the appropriate binary. Internally, each binary has a `const EXCHANGE: &str = "..."` constant that is passed to shared command functions for exchange-specific routing.

### Library Modules (`src/`)

- `config.rs` — Config file loading from `~/.fintool/config.toml`; credential accessors for each exchange
- `signing.rs` — Hyperliquid wallet signing, asset resolution, and order execution via `hyperliquid_rust_sdk`
- `hip3.rs` — HIP-3 builder-deployed perps EIP-712 signing (SILVER, GOLD, TSLA, etc. using USDT0 collateral on the `cash` dex)
- `binance.rs` / `coinbase.rs` / `okx.rs` — Exchange API clients with auth signing
- `bridge.rs` — Across Protocol cross-chain USDC bridge (Ethereum/Base → Arbitrum → Hyperliquid)
- `unit.rs` — HyperUnit bridge for ETH/BTC/SOL deposit/withdraw to/from Hyperliquid
- `polymarket.rs` — Polymarket CLOB/Gamma/Bridge SDK helpers
- `format.rs` — Terminal color formatting and number formatting utilities

### Commands Layer (`src/commands/`)

Command implementations are shared across exchange binaries. Each file handles one logical command (e.g., `order.rs` for spot buy/sell, `perp.rs` for futures, `deposit.rs` for deposits). Each command function receives an exchange name string and executes the operation against the appropriate API.

### Key Design Patterns

- All HTTP is done via `reqwest` with rustls (no OpenSSL dependency, except vendored OpenSSL pulled in transitively)
- Dual output mode: human-readable colored/tabular output by default; JSON via `--json` flag
- Config is loaded fresh from disk on each invocation (no daemon)
- HIP-3 perps are auto-detected by symbol (SILVER, GOLD, TSLA, NVDA, etc.) and routed to the `cash` dex instead of the standard Hyperliquid perp market
- Withdrawal `--to` can be either a chain name or a destination address; `lib.rs::resolve_withdraw_destination` disambiguates using the `KNOWN_CHAINS` constant

## Configuration

Config file at `~/.fintool/config.toml`. Run `fintool init` to generate a template.

Key sections:
- `[wallet]` — `private_key` (hex) or `wallet_json` + `wallet_passcode` (keystore). Used by Hyperliquid and Polymarket.
- `[network]` — `testnet = true` for Hyperliquid testnet
- `[api_keys]` — Per-exchange API credentials; optional `openai_api_key` for LLM-enriched quotes
- `[polymarket]` — `signature_type`: `proxy` (default), `eoa`, or `gnosis-safe`

Additional config keys not in the README: `cryptopanic_token`, `newsapi_key`, `binance_base_url` (set to `https://api.binance.us` for Binance US — disables futures/options support).

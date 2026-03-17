# fintool Skill — Installation Guide

Install the fintool skill for financial trading and market intelligence.

## Prerequisites

- `curl`, `unzip`, `git`, `bash`

## Quick Install (Recommended)

```bash
curl -sSL https://raw.githubusercontent.com/second-state/fintool/main/skills/bootstrap.sh | bash
```

Or clone and run locally:

```bash
git clone --depth 1 https://github.com/second-state/fintool.git /tmp/fintool-repo
bash /tmp/fintool-repo/skills/bootstrap.sh
rm -rf /tmp/fintool-repo
```

The bootstrap script will:
1. Clone skill files to `~/.openclaw/skills/fintool/`
2. Detect your platform (Linux x86_64/aarch64, macOS Apple Silicon)
3. Download all binaries (fintool, hyperliquid, binance, coinbase, polymarket, okx, backtest) from the latest GitHub release
4. Run `fintool init` to create `~/.fintool/config.toml` (never overwrites existing)
5. Check config for required keys and tell you what's missing

After installation, edit `~/.fintool/config.toml` to add your credentials:

**Required:**
- `openai_api_key` — for enriched price quotes with trend/momentum analysis

**At least one exchange:**
- **Hyperliquid** — `private_key` or `wallet_json` + `wallet_passcode` in `[wallet]` (spot + perps)
- **Binance** — `binance_api_key` + `binance_api_secret` in `[api_keys]` (spot + perps + options)
- **Coinbase** — `coinbase_api_key` + `coinbase_api_secret` in `[api_keys]` (spot only)
- **OKX** — `okx_api_key` + `okx_secret_key` + `okx_passphrase` in `[api_keys]` (spot + perps)

Verify installation:

```bash
~/.openclaw/skills/fintool/scripts/fintool quote BTC
```

## Manual Installation

If the bootstrap script fails:

1. Go to https://github.com/second-state/fintool/releases/latest
2. Download the zip for your platform:
   - `fintool-linux-x86_64.zip` (Linux x86_64)
   - `fintool-linux-aarch64.zip` (Linux ARM64)
   - `fintool-macos-aarch64.zip` (macOS Apple Silicon)
   - Windows: use WSL2 and download the Linux x86_64 build
3. Extract and copy the binary:
   ```bash
   mkdir -p ~/.openclaw/skills/fintool/scripts
   unzip fintool-<platform>.zip
   cp fintool-<platform>/{fintool,hyperliquid,binance,coinbase,polymarket,okx,backtest} ~/.openclaw/skills/fintool/scripts/
   chmod +x ~/.openclaw/skills/fintool/scripts/*
   ```
4. Copy the skill definition:
   ```bash
   git clone --depth 1 https://github.com/second-state/fintool.git /tmp/fintool-repo
   cp /tmp/fintool-repo/skills/SKILL.md ~/.openclaw/skills/fintool/SKILL.md
   rm -rf /tmp/fintool-repo
   ```
5. Initialize config:
   ```bash
   ~/.openclaw/skills/fintool/scripts/fintool init
   ```
6. Edit `~/.fintool/config.toml` to add your API keys and exchange credentials.

## Troubleshooting

### Download Failed

```bash
curl -I "https://github.com/second-state/fintool/releases/latest"
```

### Unsupported Platform

```bash
echo "OS: $(uname -s), Arch: $(uname -m)"
```

Supported: Linux (x86_64, aarch64), macOS (Apple Silicon arm64), Windows (via WSL2).

### Config Not Found

```bash
~/.openclaw/skills/fintool/scripts/fintool init
```

Creates the config template if it doesn't exist. Never overwrites an existing one.

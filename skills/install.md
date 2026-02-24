# fintool Skill — Installation Guide

Install the fintool skill for financial trading and market intelligence.

## Prerequisites

- `curl` (for downloading)
- `unzip` (for extraction)
- `bash` (shell)

## Quick Install (Recommended)

```bash
SKILL_DIR="${HOME}/.openclaw/skills/fintool"
mkdir -p "$SKILL_DIR"

# Clone the skill files
git clone --depth 1 https://github.com/second-state/fintool.git /tmp/fintool-repo
cp -r /tmp/fintool-repo/skills/* "$SKILL_DIR/"
rm -rf /tmp/fintool-repo

# Detect platform and download binary
OS=$(uname -s)
ARCH=$(uname -m)

case "${OS}-${ARCH}" in
  Linux-x86_64)   ARTIFACT="fintool-linux-x86_64" ;;
  Linux-aarch64)  ARTIFACT="fintool-linux-aarch64" ;;
  Darwin-arm64)   ARTIFACT="fintool-macos-aarch64" ;;
  *)
    echo "Unsupported platform: ${OS}-${ARCH}"
    echo "Supported: Linux (x86_64, aarch64), macOS (Apple Silicon)"
    exit 1
    ;;
esac

# Download latest release
RELEASE_URL="https://github.com/second-state/fintool/releases/latest/download/${ARTIFACT}.zip"
echo "Downloading ${ARTIFACT}..."
curl -L -o /tmp/fintool.zip "$RELEASE_URL"
unzip -o /tmp/fintool.zip -d /tmp/fintool-extract
cp "/tmp/fintool-extract/${ARTIFACT}/fintool" "$SKILL_DIR/scripts/fintool"
chmod +x "$SKILL_DIR/scripts/fintool"
rm -rf /tmp/fintool.zip /tmp/fintool-extract

# Initialize config (will NOT overwrite existing config)
"$SKILL_DIR/scripts/fintool" init

echo ""
echo "✅ fintool installed to $SKILL_DIR/scripts/fintool"
echo "📝 Edit ~/.fintool/config.toml to add your API keys and exchange credentials."
```

After installation, configure your exchanges and API keys:

```bash
vim ~/.fintool/config.toml
```

Then verify it works:

```bash
~/.openclaw/skills/fintool/scripts/fintool quote BTC
```

## Manual Installation

If the automatic download fails:

1. Go to https://github.com/second-state/fintool/releases/latest
2. Download the zip for your platform:
   - `fintool-linux-x86_64.zip` (Linux x86_64)
   - `fintool-linux-aarch64.zip` (Linux ARM64)
   - `fintool-macos-aarch64.zip` (macOS Apple Silicon)
   - `fintool-windows-x86_64.exe.zip` (Windows x86_64)
3. Extract and copy the binary:
   ```bash
   mkdir -p ~/.openclaw/skills/fintool/scripts
   unzip fintool-<platform>.zip
   cp fintool-<platform>/fintool ~/.openclaw/skills/fintool/scripts/fintool
   chmod +x ~/.openclaw/skills/fintool/scripts/fintool
   ```
4. Copy the skill files:
   ```bash
   git clone --depth 1 https://github.com/second-state/fintool.git /tmp/fintool-repo
   cp /tmp/fintool-repo/skills/SKILL.md ~/.openclaw/skills/fintool/SKILL.md
   rm -rf /tmp/fintool-repo
   ```
5. Initialize config:
   ```bash
   ~/.openclaw/skills/fintool/scripts/fintool init
   ```

## Configuration

Edit `~/.fintool/config.toml` with your credentials:

```toml
[wallet]
# Hyperliquid — spot and perp trading
# private_key = "0x..."

[network]
testnet = false

[api_keys]
# OpenAI — enriched quote analysis (trend, momentum, summary)
# openai_api_key = "sk-..."
# openai_model = "gpt-4.1-mini"

# Binance — spot, perps, and options
# binance_api_key = "..."
# binance_api_secret = "..."

# Coinbase — spot trading
# coinbase_api_key = "..."
# coinbase_api_secret = "..."
```

You need **at least one exchange** configured for trading, and an **OpenAI key** for enriched quotes.

## Troubleshooting

### Download Failed

Check network connectivity:

```bash
curl -I "https://github.com/second-state/fintool/releases/latest"
```

### Unsupported Platform

```bash
echo "OS: $(uname -s), Arch: $(uname -m)"
```

Supported: Linux (x86_64, aarch64), macOS (Apple Silicon arm64), Windows (x86_64).

### Config Not Found

If commands fail with config errors, ensure `~/.fintool/config.toml` exists:

```bash
~/.openclaw/skills/fintool/scripts/fintool init
```

This will create the config template if it doesn't exist, and never overwrite an existing one.

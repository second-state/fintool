#!/bin/bash
set -e

SKILL_DIR="${HOME}/.openclaw/skills/fintool"
REPO_URL="https://github.com/second-state/fintool.git"
RELEASE_BASE="https://github.com/second-state/fintool/releases/latest/download"

echo "📈 Installing fintool skill..."

# 1. Clone skill files
echo "Cloning skill files..."
rm -rf /tmp/fintool-repo
git clone --depth 1 "$REPO_URL" /tmp/fintool-repo
mkdir -p "$SKILL_DIR/scripts"
cp /tmp/fintool-repo/skills/SKILL.md "$SKILL_DIR/SKILL.md"
cp /tmp/fintool-repo/skills/install.md "$SKILL_DIR/install.md"
cp /tmp/fintool-repo/skills/scripts/.gitignore "$SKILL_DIR/scripts/.gitignore"
rm -rf /tmp/fintool-repo

# 2. Detect platform
OS=$(uname -s)
ARCH=$(uname -m)

case "${OS}-${ARCH}" in
  Linux-x86_64)   ARTIFACT="fintool-linux-x86_64" ;;
  Linux-aarch64)  ARTIFACT="fintool-linux-aarch64" ;;
  Darwin-arm64)   ARTIFACT="fintool-macos-aarch64" ;;
  *)
    echo "❌ Unsupported platform: ${OS}-${ARCH}"
    echo "Supported: Linux (x86_64, aarch64), macOS (Apple Silicon)"
    exit 1
    ;;
esac

# 3. Download binary
echo "Downloading ${ARTIFACT}..."
curl -L -o /tmp/fintool.zip "${RELEASE_BASE}/${ARTIFACT}.zip"
unzip -o /tmp/fintool.zip -d /tmp/fintool-extract
cp "/tmp/fintool-extract/${ARTIFACT}/fintool" "$SKILL_DIR/scripts/fintool"
chmod +x "$SKILL_DIR/scripts/fintool"
rm -rf /tmp/fintool.zip /tmp/fintool-extract

# 4. Initialize config (never overwrites existing)
"$SKILL_DIR/scripts/fintool" init

echo ""
echo "✅ fintool installed to $SKILL_DIR/scripts/fintool"
echo ""

# 5. Check config for required keys
CONFIG="$HOME/.fintool/config.toml"
MISSING=()

# Check OpenAI
if ! grep -q '^openai_api_key\s*=' "$CONFIG" 2>/dev/null; then
  MISSING+=("openai_api_key (for enriched price quotes with trend analysis)")
fi

# Check exchanges — need at least one
HAS_HL=false
HAS_BINANCE=false
HAS_COINBASE=false

if grep -q '^private_key\s*=' "$CONFIG" 2>/dev/null || grep -q '^wallet_json\s*=' "$CONFIG" 2>/dev/null; then
  HAS_HL=true
fi
if grep -q '^binance_api_key\s*=' "$CONFIG" 2>/dev/null; then
  HAS_BINANCE=true
fi
if grep -q '^coinbase_api_key\s*=' "$CONFIG" 2>/dev/null; then
  HAS_COINBASE=true
fi

if [ "$HAS_HL" = false ] && [ "$HAS_BINANCE" = false ] && [ "$HAS_COINBASE" = false ]; then
  MISSING+=("At least one exchange (Hyperliquid wallet, Binance API keys, or Coinbase API keys)")
fi

if [ ${#MISSING[@]} -gt 0 ]; then
  echo "⚠️  Configuration needed in ~/.fintool/config.toml:"
  echo ""
  for item in "${MISSING[@]}"; do
    echo "  • $item"
  done
  echo ""
  echo "Edit ~/.fintool/config.toml to add the missing credentials."
  echo ""
  echo "Exchange capabilities:"
  echo "  • Hyperliquid (wallet): spot + perps"
  echo "  • Binance (API key):    spot + perps + options"
  echo "  • Coinbase (API key):   spot only"
else
  echo "✅ Configuration looks good!"
fi

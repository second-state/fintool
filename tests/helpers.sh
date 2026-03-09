#!/usr/bin/env bash
#
# Shared helpers for e2e test scripts
#

FINTOOL="${FINTOOL:-./target/release/fintool}"
HYPERLIQUID="${HYPERLIQUID:-./target/release/hyperliquid}"
BINANCE="${BINANCE:-./target/release/binance}"
COINBASE="${COINBASE:-./target/release/coinbase}"
POLYMARKET="${POLYMARKET:-./target/release/polymarket}"

# Last command results (set by run_tool)
LAST_STDOUT=""
LAST_STDERR=""
LAST_EXIT=0

# ── Formatting ─────────────────────────────────────────────────────────

log()  { echo -e "\n\033[1;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"; echo -e "\033[1;34m▶ $*\033[0m"; echo -e "\033[1;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"; }
info() { echo -e "  \033[0;36mℹ $*\033[0m"; }
ok()   { echo -e "  \033[1;32m✓ $*\033[0m"; }
fail() { echo -e "  \033[1;31m✗ $*\033[0m"; }
warn() { echo -e "  \033[1;33m⚠ $*\033[0m"; }
done_step() { echo -e "\n  \033[1;33m── Result ──\033[0m"; }

# ── Run tool ──────────────────────────────────────────────────────────

# Run a binary tool and capture stdout, stderr, and exit code separately.
# After calling, check LAST_EXIT. LAST_STDOUT has the JSON output,
# LAST_STDERR has error/progress messages.
# Usage: run_tool <binary> [args...]
run_tool() {
    local binary="$1"
    shift
    local tmp_stdout tmp_stderr
    tmp_stdout=$(mktemp)
    tmp_stderr=$(mktemp)
    LAST_EXIT=0
    $binary "$@" >"$tmp_stdout" 2>"$tmp_stderr" || LAST_EXIT=$?
    LAST_STDOUT=$(cat "$tmp_stdout")
    LAST_STDERR=$(cat "$tmp_stderr")
    rm -f "$tmp_stdout" "$tmp_stderr"
}

# Check if last command failed. Prints error details.
# Returns 0 (true) if failed, 1 (false) if succeeded.
check_fail() {
    if [[ -n "$LAST_STDERR" ]]; then
        echo "$LAST_STDERR" | while IFS= read -r line; do
            if [[ $LAST_EXIT -ne 0 ]]; then
                echo -e "  \033[0;31m  $line\033[0m"
            else
                echo -e "  \033[0;90m  $line\033[0m"
            fi
        done
    fi

    if [[ $LAST_EXIT -ne 0 ]]; then
        done_step
        fail "$1 (exit code $LAST_EXIT)"
        if [[ -n "$LAST_STDOUT" ]]; then
            echo -e "  \033[0;31m  stdout: $LAST_STDOUT\033[0m"
        fi
        return 0  # true — command failed
    fi
    return 1  # false — command succeeded
}

# ── Build helper ───────────────────────────────────────────────────────

ensure_built() {
    local need_build=false
    for bin in "$FINTOOL" "$HYPERLIQUID" "$BINANCE" "$COINBASE" "$POLYMARKET"; do
        if [[ ! -x "$bin" ]]; then
            need_build=true
            break
        fi
    done

    if $need_build; then
        info "Building all binaries..."
        cargo build --release 2>&1
        for bin in "$FINTOOL" "$HYPERLIQUID" "$BINANCE" "$COINBASE" "$POLYMARKET"; do
            if [[ ! -x "$bin" ]]; then
                fail "Build failed — binary not found at $bin"
                exit 1
            fi
        done
    fi
}

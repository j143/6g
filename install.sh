#!/usr/bin/env bash
# install.sh — Set up the 6G bench environment.
#
# Usage:
#   ./install.sh                        # core tier only (zero native deps)
#   ./install.sh --baselines            # + Level-2 CSV baseline comparisons
#   ./install.sh --onnx                 # + real ONNX sentence-transformer
#   ./install.sh --plotting             # + Python matplotlib plots
#   ./install.sh --all                  # everything above
#
# Tier summary printed at the end.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# ── Flags ────────────────────────────────────────────────────────────────────
TIER_BASELINES=false
TIER_ONNX=false
TIER_PLOTTING=false

for arg in "$@"; do
    case "$arg" in
        --baselines) TIER_BASELINES=true ;;
        --onnx)      TIER_ONNX=true ;;
        --plotting)  TIER_PLOTTING=true ;;
        --all)       TIER_BASELINES=true; TIER_ONNX=true; TIER_PLOTTING=true ;;
        --help|-h)
            sed -n '2,12p' "$0" | sed 's/^# //'
            exit 0
            ;;
        *) echo "Unknown flag: $arg  (use --help)"; exit 1 ;;
    esac
done

# ── Helpers ───────────────────────────────────────────────────────────────────
ok()   { echo "  ✓  $*"; }
warn() { echo "  ⚠  $*"; }
fail() { echo "  ✗  $*"; }
step() { echo; echo "▶  $*"; }

# ── Step 1: Rust toolchain ────────────────────────────────────────────────────
step "Checking Rust toolchain"
if ! command -v cargo &>/dev/null; then
    warn "cargo not found — installing via rustup"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
fi

RUST_VERSION=$(rustc --version 2>/dev/null || echo "unknown")
ok "Rust: $RUST_VERSION"

# ── Step 2: Build core bench ──────────────────────────────────────────────────
step "Building sixg-bench (core tier, no optional deps)"
FEATURES=""
if $TIER_BASELINES; then
    FEATURES="${FEATURES:+$FEATURES,}baseline-comparison"
fi
if $TIER_ONNX; then
    FEATURES="${FEATURES:+$FEATURES,}onnx"
fi

if [ -n "$FEATURES" ]; then
    cargo build --release --features "$FEATURES"
else
    cargo build --release
fi
ok "Built: target/release/sixg-bench"

# ── Step 3: Baselines tier ────────────────────────────────────────────────────
BASELINES_STATUS="skipped (use --baselines)"
if $TIER_BASELINES; then
    step "Setting up Level-2 baselines"
    mkdir -p baselines
    # The three CSV files are already bundled in the repo (baselines/).
    # If a network-sourced file is needed in the future, wget/curl it here.
    if ls baselines/*.csv &>/dev/null; then
        ok "Baseline CSV files found in baselines/"
        BASELINES_STATUS="enabled ($(ls baselines/*.csv | wc -l | tr -d ' ') CSV files)"
    else
        warn "No CSV files found in baselines/ — run 'cargo test -p sixg-phy --features=baseline-comparison' to check"
        BASELINES_STATUS="enabled (no CSV files present)"
    fi
    export SIXG_BASELINES="$SCRIPT_DIR/baselines"
    ok "SIXG_BASELINES=$SIXG_BASELINES"
fi

# ── Step 4: Plotting tier ─────────────────────────────────────────────────────
PLOTTING_STATUS="skipped (use --plotting)"
if $TIER_PLOTTING; then
    step "Setting up Python plotting tier"
    if ! command -v python3 &>/dev/null; then
        fail "Python 3 not found — install Python 3.9+ and re-run with --plotting"
        PLOTTING_STATUS="FAILED (python3 not found)"
    else
        PY_VERSION=$(python3 --version 2>&1)
        ok "Python: $PY_VERSION"
        if command -v pip3 &>/dev/null; then
            pip3 install --quiet -r requirements-plot.txt
            ok "Python packages installed from requirements-plot.txt"
            PLOTTING_STATUS="enabled ($PY_VERSION)"
        else
            warn "pip3 not found — install pip and run: pip3 install -r requirements-plot.txt"
            PLOTTING_STATUS="partial (pip3 missing)"
        fi
    fi
fi

# ── Step 5: ONNX tier ─────────────────────────────────────────────────────────
ONNX_STATUS="skipped (use --onnx)"
if $TIER_ONNX; then
    step "Setting up ONNX tier"
    MODEL_DIR="models"
    mkdir -p "$MODEL_DIR"
    MODEL_FILE="$MODEL_DIR/all-MiniLM-L6-v2.onnx"

    if [ -f "$MODEL_FILE" ]; then
        ok "Model file already present: $MODEL_FILE"
        ONNX_STATUS="enabled (model found)"
    else
        # The ort crate (when built with the 'onnx' feature) can auto-download
        # the ONNX runtime library.  The model file itself must be obtained
        # separately.  We provide a download URL as a convenience; users can
        # also supply their own .onnx file.
        warn "Model file not found at $MODEL_FILE"
        warn "To use the real ONNX inference backend:"
        warn "  1. Download all-MiniLM-L6-v2.onnx from Hugging Face:"
        warn "     https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2"
        warn "  2. Place it at $MODEL_FILE"
        warn "  3. Re-run:  ./install.sh --onnx"
        warn "The bench will run with the built-in deterministic simulation"
        warn "until the model file is placed in $MODEL_FILE."
        ONNX_STATUS="enabled (simulation mode — model file absent)"
    fi
    export SIXG_ONNX_MODEL="$SCRIPT_DIR/$MODEL_FILE"
    ok "Binary built with --features=onnx"
fi

# ── Summary ───────────────────────────────────────────────────────────────────
echo
echo "╔══════════════════════════════════════════════════════════╗"
echo "║          6G Bench — installation summary                 ║"
echo "╠══════════════════════════════════════════════════════════╣"
printf "║  %-20s  %-33s ║\n" "Tier" "Status"
echo "╠══════════════════════════════════════════════════════════╣"
printf "║  %-20s  %-33s ║\n" "Core (Rust)"       "enabled"
printf "║  %-20s  %-33s ║\n" "Baselines"         "$BASELINES_STATUS"
printf "║  %-20s  %-33s ║\n" "Plotting (Python)" "$PLOTTING_STATUS"
printf "║  %-20s  %-33s ║\n" "ONNX inference"    "$ONNX_STATUS"
echo "╚══════════════════════════════════════════════════════════╝"
echo
echo "Run the bench:"
echo "  ./target/release/sixg-bench --help"
echo "  ./target/release/sixg-bench list"
echo "  ./target/release/sixg-bench run --all"
echo "  ./target/release/sixg-bench validate"

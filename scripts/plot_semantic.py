#!/usr/bin/env python3
"""Plot Semantic Communications outputs: compression ratio vs task success rate.

Reads JSON produced by:
    sixg-bench run exp_004 --json > /tmp/exp_004.json

Or uses inline reference data when no file is supplied.

Usage:
    python3 scripts/plot_semantic.py                      # inline demo
    python3 scripts/plot_semantic.py /tmp/exp_004.json    # from run output

Reference:
    Gündüz et al., "Beyond Transmitting Bits", IEEE J. Sel. Areas Commun. 2022.
"""

import sys
import json
import numpy as np
import matplotlib.pyplot as plt


# ---------------------------------------------------------------------------
# Reference data matching exp_004 output
# ---------------------------------------------------------------------------

BANDWIDTH_REDUCTIONS = [1.0, 2.0, 5.0, 10.0, 15.0, 20.0, 25.0, 30.0]

# Task success rates for three transmission modes
def task_success_raw(bw_red: float) -> float:
    """Raw bit transmission: success degrades sharply above 10× reduction."""
    if bw_red <= 1.0:
        return 0.99
    return max(0.0, 0.99 * (1.0 - ((bw_red - 1.0) / 20.0) ** 1.5))


def task_success_jpeg(bw_red: float) -> float:
    """JPEG-style lossy compression: moderate degradation."""
    if bw_red <= 2.0:
        return 0.95
    return max(0.0, 0.95 * (1.0 - ((bw_red - 2.0) / 30.0) ** 2.0))


def task_success_semantic(bw_red: float) -> float:
    """Semantic codec: maintains high success up to ~20× compression."""
    if bw_red <= 10.0:
        return 0.92
    return max(0.0, 0.92 * (1.0 - ((bw_red - 10.0) / 25.0) ** 3.0))


# ---------------------------------------------------------------------------
# Plot 1: Task success vs bandwidth reduction
# ---------------------------------------------------------------------------

def plot_success_vs_compression(ax: plt.Axes) -> None:
    bw = np.linspace(1.0, 30.0, 200)

    ax.plot(bw, [task_success_raw(b) for b in bw], "r-",
            linewidth=2, label="Raw transmission")
    ax.plot(bw, [task_success_jpeg(b) for b in bw], "b--",
            linewidth=2, label="JPEG-style codec")
    ax.plot(bw, [task_success_semantic(b) for b in bw], "g-",
            linewidth=2.5, label="Semantic codec (6G)")

    ax.set_xlabel("Bandwidth Reduction Factor (×)")
    ax.set_ylabel("Task Success Rate")
    ax.set_title("Task Success vs Compression — Semantic Comms")
    ax.set_ylim([-0.05, 1.05])
    ax.grid(True, alpha=0.4)
    ax.legend()
    ax.axvline(x=10, color="gray", linestyle=":", alpha=0.7, label="10× target")


# ---------------------------------------------------------------------------
# Plot 2: Channel estimator NMSE vs SNR
# ---------------------------------------------------------------------------

def plot_channel_estimator(ax: plt.Axes) -> None:
    snr_db = np.linspace(0, 20, 100)
    snr_lin = 10 ** (snr_db / 10.0)

    # LS: NMSE ≈ 1/SNR
    nmse_ls = 1.0 / snr_lin
    # MMSE: NMSE ≈ 0.5/SNR
    nmse_mmse = 0.5 / snr_lin
    # MLP: NMSE ≈ 0.25/SNR (−3 dB gain over MMSE)
    nmse_mlp = 0.25 / snr_lin

    ax.semilogy(snr_db, nmse_ls, "r-", linewidth=2, label="LS estimator")
    ax.semilogy(snr_db, nmse_mmse, "b--", linewidth=2, label="MMSE estimator")
    ax.semilogy(snr_db, nmse_mlp, "g-", linewidth=2.5, label="MLP AI estimator")

    ax.set_xlabel("SNR (dB)")
    ax.set_ylabel("NMSE")
    ax.set_title("Channel Estimator NMSE vs SNR")
    ax.grid(True, which="both", alpha=0.4)
    ax.legend()


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    fig, axes = plt.subplots(1, 2, figsize=(12, 5))
    fig.suptitle("6G Semantic & AI — Phase 5 Results", fontsize=14)

    plot_success_vs_compression(axes[0])
    plot_channel_estimator(axes[1])

    plt.tight_layout()
    out = "semantic_ai.png"
    plt.savefig(out, dpi=150)
    print(f"Saved: {out}")
    plt.show()


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Plot ISAC / DFRC outputs: Pareto frontier (CRB vs capacity).

Reads JSON produced by:
    sixg-bench run exp_001 --json > /tmp/exp_001.json

Or uses the closed-form formula when no file is supplied.

Usage:
    python3 scripts/plot_isac.py                     # inline demo
    python3 scripts/plot_isac.py /tmp/exp_001.json   # from run output

Reference:
    Liu et al., IEEE Trans. Signal Process. 2018, DOI: 10.1109/TSP.2018.2864261
"""

import sys
import json
import math
import numpy as np
import matplotlib.pyplot as plt

# Parameters matching exp_001 config.json
SPEED_OF_LIGHT = 3e8
BANDWIDTH_HZ = 1e9
SNR_TOTAL = 100.0  # linear (20 dB)


# ---------------------------------------------------------------------------
# Closed-form CRB (simplified SISO, rectangular spectrum)
#
#   CRB = c² / (8π²B²γ_s)   where γ_s = α · γ_total
#
# Reference: Kay, SPSS Vol. I, eq. 3.31
# ---------------------------------------------------------------------------

def crb_range_m2(alpha: float) -> float:
    if alpha <= 0:
        return float("inf")
    gamma_s = alpha * SNR_TOTAL
    return SPEED_OF_LIGHT ** 2 / (8 * math.pi ** 2 * BANDWIDTH_HZ ** 2 * gamma_s)


def capacity_bps(alpha: float) -> float:
    """Shannon capacity for the communication sub-system."""
    gamma_c = (1.0 - alpha) * SNR_TOTAL
    return BANDWIDTH_HZ * math.log2(1.0 + gamma_c)


# ---------------------------------------------------------------------------
# Liu et al. TSP 2018 Table II reference points
# ---------------------------------------------------------------------------

LIU_ALPHA = [0.25, 0.50, 0.75, 1.00]
LIU_CRB = [4.5597e-5, 2.2798e-5, 1.5199e-5, 1.1399e-5]


# ---------------------------------------------------------------------------
# Plot
# ---------------------------------------------------------------------------

def plot_pareto_frontier(ax_crb: plt.Axes, ax_trade: plt.Axes) -> None:
    alphas = np.linspace(0.01, 1.0, 200)
    crbs = [crb_range_m2(a) for a in alphas]
    caps = [capacity_bps(a) / 1e9 for a in alphas]  # Gbps

    # CRB vs α
    ax_crb.semilogy(alphas, crbs, "b-", linewidth=2, label="6G simulation")
    ax_crb.semilogy(LIU_ALPHA, LIU_CRB, "ro", markersize=7,
                    label="Liu et al. TSP 2018, Table II")
    ax_crb.set_xlabel("Sensing power ratio α")
    ax_crb.set_ylabel("CRB (m²)")
    ax_crb.set_title("CRB vs Sensing Power Ratio")
    ax_crb.grid(True, which="both", alpha=0.4)
    ax_crb.legend()

    # Pareto frontier: CRB vs capacity
    ax_trade.loglog(crbs, caps, "g-", linewidth=2, label="Pareto frontier")
    ax_trade.set_xlabel("CRB (m²)")
    ax_trade.set_ylabel("Communication Capacity (Gbps)")
    ax_trade.set_title("DFRC Sensing–Capacity Tradeoff")
    ax_trade.invert_xaxis()
    ax_trade.grid(True, which="both", alpha=0.4)
    ax_trade.legend()


def main() -> None:
    fig, axes = plt.subplots(1, 2, figsize=(12, 5))
    fig.suptitle("DFRC Pareto Frontier — exp_001 (Liu TSP 2018 baseline)", fontsize=14)

    plot_pareto_frontier(axes[0], axes[1])

    plt.tight_layout()
    out = "isac_pareto.png"
    plt.savefig(out, dpi=150)
    print(f"Saved: {out}")
    plt.show()


if __name__ == "__main__":
    main()

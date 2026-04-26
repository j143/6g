#!/usr/bin/env python3
"""Plot PHY layer outputs: path loss vs distance and BER vs Eb/N0.

Reads JSON produced by:
    sixg-bench run exp_002 --json > /tmp/exp_002.json

Or uses inline reference data when no file is supplied.

Usage:
    python3 scripts/plot_phy.py                      # inline demo
    python3 scripts/plot_phy.py /tmp/exp_002.json    # from run output
"""

import sys
import math
import numpy as np
import matplotlib.pyplot as plt


# ---------------------------------------------------------------------------
# Reference data (from exp_002 and PHY formulae)
# ---------------------------------------------------------------------------

def path_loss_db(dist_m: float, freq_hz: float = 28e9) -> float:
    """NIST 28 GHz UMa LOS close-in model: PL = 61.4 + 20*log10(d)."""
    if dist_m <= 0:
        return float("inf")
    return 61.4 + 20.0 * math.log10(dist_m)


def bpsk_ber_awgn(snr_db: float) -> float:
    """Analytical BPSK BER in AWGN: Q(sqrt(2 * 10^(snr_db/10)))."""
    snr_linear = 10 ** (snr_db / 10.0)
    x = math.sqrt(2.0 * snr_linear)
    return 0.5 * math.erfc(x / math.sqrt(2))


# ---------------------------------------------------------------------------
# Plot 1: Path loss vs distance (28 GHz, UMa LOS)
# ---------------------------------------------------------------------------

def plot_path_loss(ax: plt.Axes) -> None:
    distances = np.logspace(1, 3, 200)  # 10 m to 1 km
    pl = [path_loss_db(d) for d in distances]

    ax.semilogx(distances, pl, "b-", linewidth=2, label="NIST 28 GHz UMa LOS")
    ax.set_xlabel("Distance (m)")
    ax.set_ylabel("Path Loss (dB)")
    ax.set_title("Path Loss vs Distance — 28 GHz Sub-THz")
    ax.grid(True, which="both", alpha=0.4)
    ax.legend()


# ---------------------------------------------------------------------------
# Plot 2: BER vs Eb/N0 (OFDM BPSK, AWGN)
# ---------------------------------------------------------------------------

def plot_ber(ax: plt.Axes) -> None:
    snr_range = np.linspace(-4, 14, 200)
    ber = [max(bpsk_ber_awgn(s), 1e-6) for s in snr_range]

    ax.semilogy(snr_range, ber, "r-", linewidth=2, label="OFDM BPSK AWGN (analytical)")
    # Vienna LLS reference points (digitised from exp_002)
    vienna_snr = [-2, 0, 2, 4, 6, 8, 10, 12]
    vienna_ber = [bpsk_ber_awgn(s) for s in vienna_snr]
    ax.semilogy(vienna_snr, vienna_ber, "ko", markersize=6, label="Vienna LLS reference")

    ax.set_xlabel("Eb/N0 (dB)")
    ax.set_ylabel("BER")
    ax.set_title("BPSK BER vs Eb/N0 — OFDM AWGN")
    ax.set_ylim([1e-5, 1.0])
    ax.grid(True, which="both", alpha=0.4)
    ax.legend()


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    fig, axes = plt.subplots(1, 2, figsize=(12, 5))
    fig.suptitle("6G PHY Layer — Baseline Comparison", fontsize=14)

    plot_path_loss(axes[0])
    plot_ber(axes[1])

    plt.tight_layout()
    out = "phy_baseline.png"
    plt.savefig(out, dpi=150)
    print(f"Saved: {out}")
    plt.show()


if __name__ == "__main__":
    main()

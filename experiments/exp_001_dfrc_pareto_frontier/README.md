# Experiment 001 — DFRC Pareto Frontier

## Hypothesis
Sweeping the sensing power ratio α ∈ [0, 1] traces a monotone tradeoff:
higher α reduces the Cramér-Rao Bound (better ranging) at the cost of lower
Shannon capacity (worse communications).

## Method
Use `DfrcConfig` from `6g-isac` with 1 GHz bandwidth and 20 dB SNR.
Call `pareto_frontier(20)` to generate 21 operating points.
Verify monotonicity: CRB non-increasing, capacity non-increasing as α grows.

## Expected Result
- At α = 0: CRB = ∞, capacity = B·log₂(1 + γ_total) ≈ 6.658 Gbps
- At α = 1: CRB ≈ 1.14×10⁻⁵ m², capacity = 0 bps
- Monotone tradeoff with no crossings

## Reference
Liu, F. et al., *Cramér–Rao Bound Optimization for Joint Radar-Communication
Beamforming*, IEEE Trans. Signal Process., 2018, DOI: 10.1109/TSP.2018.2864261.
Kay, *Fundamentals of Statistical Signal Processing*, Vol. I, Ch. 3.

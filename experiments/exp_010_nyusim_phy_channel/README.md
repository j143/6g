# Experiment 010 — NYUSIM PHY Channel Baseline (sub-THz)

## Hypothesis

The `6g-phy` free-space path loss function `fspl_db(d, f)` reproduces the
**NYUSIM close-in (CI)** channel model for LOS conditions at all three
sub-THz frequency windows (28 GHz, 73 GHz, 140 GHz) to within 1 %.

The additional molecular absorption term `molecular_absorption_coeff(f) × d`
provides higher physical fidelity than NYUSIM's empirical PLE model, at the
cost of a documented and expected divergence at 73 GHz (near the O₂ peak).

## Why NYUSIM?

NYUSIM is the de-facto open-source reference for sub-THz channel modelling.
Its close-in model for LOS channels (`PL(d) = FSPL(1 m, f) + 20·log₁₀(d)`,
PLE n = 2) is cited directly in 3GPP TR 38.901 — the same standard
`6g-phy` targets.  The analytical equivalence between the CI model and the
FSPL formula makes this comparison an exact Level 1 cross-check.

## Method

- Sweep distance 10–1000 m for each frequency.
- Compute `fspl_db(d, f)` (free-space only, no absorption).
- Compare against inline NYUSIM CI reference values using `BaselineDataset::compare`.
- Additionally compute `molecular_absorption_coeff(f) × d` to show extra loss
  beyond the NYUSIM baseline (informational Level 3 table).

## Expected Result

| Metric | This simulation | NYUSIM CI reference | Δ |
|--------|-----------------|---------------------|---|
| FSPL @ 28 GHz, 100 m | 101.39 dB | 101.40 dB | < 0.01 % |
| FSPL @ 73 GHz, 100 m | 109.70 dB | 109.70 dB | < 0.01 % |
| FSPL @ 140 GHz, 100 m | 115.37 dB | 115.37 dB | < 0.01 % |
| Extra absorption @ 73 GHz, 100 m | ~5.1 dB | 0 dB (in PLE) | documented gap |
| Extra absorption @ 140 GHz, 100 m | ~0.3 dB | 0 dB (in PLE) | negligible |

## Key Insight: 73 GHz O₂ Absorption Edge

The 73 GHz band sits on the tail of the O₂ absorption resonance at 60 GHz.
Our ITU-R P.676 model predicts an additional ~5 dB/100 m beyond pure FSPL.
NYUSIM's empirical CI model absorbs this into the measured PLE (which can
exceed n = 2.0 in urban environments). The documented divergence is physically
correct and reflects the fidelity advantage of our absorption model.

## References

- Rappaport et al., "Millimeter Wave Mobile Communications for 5G Cellular",
  IEEE Access 2013 (28 GHz NYUSIM baseline)
- MacCartney et al., "73 GHz Millimeter Wave Propagation Measurements for
  Wireless and Backhaul Communications in Urban Environments", IEEE ICC 2015
- Xing & Rappaport, "Propagation Measurements and Path Loss Models for Sub-THz
  Communications", IEEE Transactions on Antennas and Propagation, 2021
- ITU-R P.676-12, "Attenuation by atmospheric gases and related effects", 2019
- 3GPP TR 38.901 v17.0.0 — UMa LOS path loss model (references NYUSIM)
- NYUSIM — https://wireless.engineering.nyu.edu/nyusim/

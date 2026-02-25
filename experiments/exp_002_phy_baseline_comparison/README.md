# Experiment 002 — PHY Baseline Comparison (Phase 1)

## Hypothesis

The `6g-phy` waveform and spectrum models reproduce results from two external
reference systems within the Level-2 tolerance of 5 %:

1. **Vienna 5G LLS** (BER vs Eb/N0): OTFS achieves the AWGN bound in a
   high-Doppler channel (v = 250 km/h, 28 GHz), while CP-OFDM degrades due
   to inter-carrier interference (ICI).
2. **NIST 28 GHz mmWave dataset** (path loss vs distance): the simulated
   FSPL at 28 GHz matches the NIST UMa LOS close-in model
   `PL(d) = 61.4 + 20·log₁₀(d)` to within 1 %.

## Method

- Sweep Eb/N0 from −2 dB to 12 dB; compute BPSK BER for OFDM (AWGN and
  high-Doppler with ε = 0.216) and OTFS (delay-Doppler domain).
- Sweep distance from 10 m to 1 000 m; compute path loss at 28 GHz.
- Compare against inline reference CSV data using `BaselineDataset::compare`.
- AGENTS.md rule: all `pub fn` boundaries use `SnrDb`, `Distance`, `Frequency`
  newtypes — no bare `f64`.

## Expected Result

| Metric | This simulation | Reference | Δ |
|--------|-----------------|-----------|---|
| BER (OFDM AWGN) @ 0 dB | 0.0787 | 0.0787 (Vienna LLS) | < 1 % |
| BER (OFDM AWGN) @ 10 dB | 3.87×10⁻⁶ | 3.87×10⁻⁶ (Vienna LLS) | < 1 % |
| BER (OTFS) @ 10 dB, ε=0.216 | 3.87×10⁻⁶ | 3.87×10⁻⁶ (Vienna LLS) | < 1 % |
| BER (OFDM Doppler) @ 10 dB, ε=0.216 | ~1.55×10⁻⁵ | > OTFS (Vienna LLS) | OTFS wins |
| Path loss @ 28 GHz, 100 m | 101.39 dB | 101.40 dB (NIST) | < 0.01 % |

## References

- Hadani et al., *OTFS Modulation*, IEEE WCNC 2017
- Proakis & Salehi, *Digital Communications*, 5th ed.
- Rappaport et al., *Millimeter Wave Mobile Communications for 5G Cellular*,
  IEEE Access 2013
- NIST Technical Note 2069 (2020) — 5G mmWave channel model
- Vienna 5G LLS — https://www.nt.tuwien.ac.at/research/mobile-communications/vienna-5g-simulators/

# `6g-isac` — Integrated Sensing and Communication

## Purpose

ISAC is one of the clearest differentiators of 6G from 5G. The same waveform and hardware perform both data communication and radio sensing (radar) simultaneously. `6g-isac` models this dual-function operation. The entry point is `IsacLayer`.

## Invariants

<!-- Things that must ALWAYS be true, regardless of changes -->
- The sensing power ratio α ∈ [0, 1]. Values outside this range are a programming error.
- `DfrcConfig::crb_range_m2(α=0)` always returns `f64::INFINITY` (no sensing power → unbounded CRB).
- `DfrcConfig::capacity_bps(α=1)` always returns `0.0` (all power to sensing → no capacity).
- `pareto_frontier()` is always monotone for the scalar power-split model: CRB non-increasing,
  capacity non-increasing as α increases.  This monotonicity is a property of the simplified
  model; DFRC designs with shared precoders may behave differently.
- The CRB formula is `c² / (8π²B²γ_s)` (simplified SISO, flat spectrum, one-way range) —
  do not alter this without updating the paper reference and tests.
- `ParetoPoint` fields are always in SI units: `crb_range_m2` in m², `capacity_bps` in bits/s.

## Architecture

```
                 ┌──────────────────────┐
  TX signal ───► │  ISAC Waveform Gen   │ ◄── Communication bits
                 │  (DFRC / OTFS-ISAC)  │
                 └──────────┬───────────┘
                            │ Transmitted waveform
                    ┌───────┴────────┐
                    │  RF Channel    │
                    └───────┬────────┘
               ┌────────────┴───────────────┐
               │ Sensing echo        Comm RX │
          ┌────▼─────┐             ┌────────▼───┐
          │ Sensing   │             │ Comm       │
          │ Processor │             │ Decoder    │
          └──────────┘             └────────────┘
```

## Modules

### `dfrc.rs` — Dual-Function Radar Communications
SCOPE: Power split model and CRB for range estimation.
Key types: `DfrcConfig`, `ParetoPoint`, `DfrcValidation`.
Formula: simplified SISO time-delay CRB `CRB = c² / (8π²B²γ_s)` (Kay, SPSS
Vol. I, eq. 3.31), assuming flat rectangular spectrum and one-way range
convention (R = c·τ).  Constants are tuned to be numerically comparable to
Liu et al. (IEEE TSP 2018) Table II; see `baselines/liu_tsp2018_crb.csv`.

### `sensing.rs`

Tasks: Localisation, Velocity Estimation, Environment Mapping, Gesture Recognition.
Key types: `SensingTask`, `SensingResult`.

Processing pipeline: raw ADC samples → FFT → range-Doppler map → CFAR detection → target parameter extraction.

### `waveform.rs`
Key type: `IsacWaveform`.

- **DFRC** (Dual-Function Radar Communications): embeds sensing sequences into OFDM pilots. The sensing matrix and communication precoder share the same transmit power.
- **OTFS-ISAC**: delay-Doppler domain waveform enables simultaneous range-Doppler estimation (sensing) and reliable communication in high-mobility channels.
- **AiOptimised**: learned precoder trained to jointly optimise communication rate and sensing CRB (placeholder, Phase 2).

### `detection.rs`

CFAR detection and range-Doppler map processing.
Key types: `RangeDopplerMap`.
Key function: `pd_from_pfa()` — Neyman-Pearson threshold for detection probability.

## Public API Contract

- `IsacLayer::new() -> IsacLayer` — default ISAC layer (1 GHz BW, 20 dB SNR)
- `IsacLayer::sense(task: SensingTask) -> SensingResult`
- `DfrcConfig::crb_range_m2(alpha: f64) -> f64` — CRB in m², α ∈ [0, 1]
- `DfrcConfig::capacity_bps(alpha: f64) -> f64` — Shannon capacity in bits/s
- `DfrcConfig::pareto_frontier(n: usize) -> Vec<ParetoPoint>`
- `pd_from_pfa(pfa: f64, snr: f64) -> f64` — detection probability

## Validation Target (Phase 2)

Pareto frontier: CRB (Cramér-Rao Bound) for range estimation vs Shannon capacity for communication, parameterised by the sensing/communication power split ratio.
Run: `cargo run --example exp_001_dfrc_pareto_frontier`

## What This Crate Does NOT Do

- Does not implement the PHY channel model — import from `6g-phy`.
- Does not schedule resources — that is `6g-mac`'s responsibility.
- Does not implement modulation/demodulation — see `6g-phy/waveform.rs`.
- Does not implement core-network functions.

## References

- Liu, F. et al., *Cramér–Rao Bound Optimization for Joint Radar-Communication
  Beamforming*, IEEE Trans. Signal Process., 2018, DOI: 10.1109/TSP.2018.2864261
- Kay, *Fundamentals of Statistical Signal Processing*, Vol. I (CRB derivation)
- 3GPP TR 22.837 (ISAC use cases)

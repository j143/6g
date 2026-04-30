# `6g-phy` — Physical Layer

## Purpose

Models the 6G air interface physical layer. Entry point: `PhyLayer`. This is the highest-novelty crate in the project — 6G PHY departs significantly from 5G NR by operating at sub-THz/THz frequencies, introducing OTFS waveforms, extremely large aperture arrays (ELAA), and reconfigurable intelligent surfaces (RIS).

## Invariants

<!-- Things that must ALWAYS be true, regardless of changes -->
- `path_loss_db()` always returns a **positive** dB value (path loss, not gain).
- The Rayleigh distance formula is `d_R = 2D²/λ` — do not alter without updating tests.
- Free-space path loss: `FSPL(d, f) = 20·log₁₀(d) + 20·log₁₀(f) + 20·log₁₀(4π/c)` in dB.
- RIS phase shifts Φ are diagonal and each entry has magnitude 1 (lossless reflection assumed).
- OTFS operates in the delay-Doppler domain; do not mix with frequency-domain OFDM assumptions.

## Modules

### `waveform.rs` — Air Interface Waveforms

Key types: `Waveform` (enum), `WaveformImpairments`.
5G baseline: CP-OFDM (15/30/120 kHz SCS). 6G extensions:

- **DFT-s-OFDM** at sub-THz SCS (480 kHz) for reduced PAPR.
- **OTFS** (Orthogonal Time Frequency Space): operates in the delay-Doppler domain. Key advantage: compact channel representation for high-mobility scenarios (LEO passes, fast vehicles). Reference: Hadani et al., IEEE WCNC 2017.
- **AI-Native**: learned waveform where subcarrier shaping is encoded in a latent vector. Placeholder for Phase 5.

#### `WaveformImpairments` — Hardware Impairment Models

Struct capturing three analogue front-end impairments that degrade effective SNR.
All fields are `Option` — set to `None` for ideal (impairment-free) simulation.

| Field | Model | Reference |
|-------|-------|-----------|
| `phase_noise_dbc_hz` | `SNR_pn = T_sym / (2 · L₀)` | Pollet et al., IEEE Trans. Commun. 1995 |
| `iq_imbalance_db` | `SNR_max = IIR_dB` (ceiling) | Windisch & Fettweis, IEEE Commun. Lett. 2004 |
| `adc_bits` | `SQNR = 6.02·b + 1.76 dB` | Widrow & Kollár, *Quantization Noise* 2008 |

The effective SNR after all impairments combines them in the noise-power domain:
`1/SNR_eff = 1/SNR_signal + 1/SNR_pn + 1/SNR_iq + 1/SQNR`

Public functions: `phase_noise_snr_linear`, `adc_sqnr_linear`, `adc_sqnr_db`.

### `spectrum.rs` — THz Spectrum Modeling

Key types: `SpectrumManager`, `ChannelBandwidth`.
Models path loss for sub-THz bands. Key parameters:
- Free-space path loss (FSPL)
- Molecular absorption coefficient α at sub-THz frequencies (oxygen at 60 GHz, water vapour at 183 GHz)
- Model: `PL(d) = FSPL(d) + α · d`

### `mimo.rs` — Massive MIMO / ELAA

Key types: `MimoConfig`, `BeamformingType`, `AntennaPanel`.
Extends 5G massive MIMO (up to 64TRX) to ELAA (hundreds to thousands of elements). Near-field propagation becomes relevant at THz when the Rayleigh distance `d_R = 2D²/λ` exceeds typical cell dimensions.

Beamforming types: Fully Digital, Hybrid Analogue-Digital, Holographic.

### `ris.rs` — Reconfigurable Intelligent Surfaces

Key types: `RisConfig`, `RisChannel`, `PhaseResolution`.
Passive/semi-passive surfaces that apply a diagonal phase-shift matrix Φ to the reflected channel. Effective channel: `H_eff = H_direct + H_reflect · Φ · H_incident`. Optimising Φ to maximise received SNR is the key experiment (Phase 1). Reference: Basar et al., IEEE Access 2019.

## What This Crate Does NOT Do

- Does not implement MAC-layer scheduling or HARQ — see `6g-mac`.
- Does not implement ISAC sensing processing — see `6g-isac`.
- Does not manage core-network sessions or bearers.
- Does not implement AI model training or inference — see `6g-ai`.

## Validation Target (Phase 1)

- OTFS vs OFDM BER curves at the same Eb/N0 in a high-Doppler channel.
- RIS gain: received SNR with optimised Φ vs no RIS in a shadowed 150 GHz scenario. Success criterion: > 10 dB gain.

### `PhyValidation` — Level 1 + Level 2 checks

`PhyValidation` implements the `Validate` trait (Level 1, analytical bounds within 1 %):

- BPSK BER at 0 dB Eb/N0 ≈ 0.0786 (Q(√2), Proakis & Salehi 5th ed.)
- BPSK BER at 10 dB Eb/N0 ≈ 3.87×10⁻⁶ (Q(√20))
- FSPL at 28 GHz, 100 m ≈ 101.39 dB (matches NIST close-in model to < 0.01 %)
- OTFS BER / OFDM BER ratio ≈ 4× at SNR=10 dB, ε=0.216 (Hadani et al. WCNC 2017)

Level 2 baseline comparison tests (gate: `--features=baseline-comparison`) compare
against inline reference data representing:

- **Vienna 5G LLS**: BPSK BER in AWGN and OTFS BER at v=250 km/h, 28 GHz
- **NIST 28 GHz mmWave dataset**: path loss vs distance (UMa LOS, close-in model)

See `experiments/exp_002_phy_baseline_comparison/` for the runnable experiment.

## References

- Hadani et al., *OTFS Modulation*, IEEE WCNC 2017
- Basar et al., *Wireless Communications Through RIS*, IEEE Access 2019
- Björnson et al., *Massive MIMO Networks*, Foundations and Trends 2017
- 3GPP TR 38.901 (CDL channel models)
- Pollet et al., *BER Sensitivity of OFDM to CFO and Wiener Phase Noise*, IEEE Trans. Commun. 1995
- Widrow & Kollár, *Quantization Noise*, Cambridge 2008
- Windisch & Fettweis, *Performance Degradation Due to IQ Imbalance in OFDM*, IEEE Commun. Lett. 2004

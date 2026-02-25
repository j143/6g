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

Key types: `WaveformType` (enum), `OfdmConfig`, `OtfsConfig`.
5G baseline: CP-OFDM (15/30/120 kHz SCS). 6G extensions:

- **DFT-s-OFDM** at sub-THz SCS (480 kHz) for reduced PAPR.
- **OTFS** (Orthogonal Time Frequency Space): operates in the delay-Doppler domain. Key advantage: compact channel representation for high-mobility scenarios (LEO passes, fast vehicles). Reference: Hadani et al., IEEE WCNC 2017.
- **AI-Native**: learned waveform where subcarrier shaping is encoded in a latent vector. Placeholder for Phase 5.

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

## References

- Hadani et al., *OTFS Modulation*, IEEE WCNC 2017
- Basar et al., *Wireless Communications Through RIS*, IEEE Access 2019
- Björnson et al., *Massive MIMO Networks*, Foundations and Trends 2017
- 3GPP TR 38.901 (CDL channel models)

# `6g-phy` — Physical Layer

## Purpose

Models the 6G air interface physical layer. This is the highest-novelty crate in the project — 6G PHY departs significantly from 5G NR by operating at sub-THz/THz frequencies, introducing OTFS waveforms, extremely large aperture arrays (ELAA), and reconfigurable intelligent surfaces (RIS).

## Modules

### `waveform.rs` — Air Interface Waveforms

5G baseline: CP-OFDM (15/30/120 kHz SCS). 6G extensions:

- **DFT-s-OFDM** at sub-THz SCS (480 kHz) for reduced PAPR.
- **OTFS** (Orthogonal Time Frequency Space): operates in the delay-Doppler domain. Key advantage: compact channel representation for high-mobility scenarios (LEO passes, fast vehicles). Reference: Hadani et al., IEEE WCNC 2017.
- **AI-Native**: learned waveform where subcarrier shaping is encoded in a latent vector. Placeholder for Phase 5.

### `spectrum.rs` — THz Spectrum Modeling

Models path loss for sub-THz bands. Key parameters:
- Free-space path loss (FSPL)
- Molecular absorption coefficient α at sub-THz frequencies (oxygen at 60 GHz, water vapour at 183 GHz)
- Model: `PL(d) = FSPL(d) + α · d`

### `mimo.rs` — Massive MIMO / ELAA

Extends 5G massive MIMO (up to 64TRX) to ELAA (hundreds to thousands of elements). Near-field propagation becomes relevant at THz when the Rayleigh distance `d_R = 2D²/λ` exceeds typical cell dimensions.

Beamforming types: Fully Digital, Hybrid Analogue-Digital, Holographic.

### `ris.rs` — Reconfigurable Intelligent Surfaces

Passive/semi-passive surfaces that apply a diagonal phase-shift matrix Φ to the reflected channel. Effective channel: `H_eff = H_direct + H_reflect · Φ · H_incident`. Optimising Φ to maximise received SNR is the key experiment (Phase 1). Reference: Basar et al., IEEE Access 2019.

## Validation Target (Phase 1)

- OTFS vs OFDM BER curves at the same Eb/N0 in a high-Doppler channel.
- RIS gain: received SNR with optimised Φ vs no RIS in a shadowed 150 GHz scenario. Success criterion: > 10 dB gain.

## References

- Hadani et al., *OTFS Modulation*, IEEE WCNC 2017
- Basar et al., *Wireless Communications Through RIS*, IEEE Access 2019
- Björnson et al., *Massive MIMO Networks*, Foundations and Trends 2017
- 3GPP TR 38.901 (CDL channel models)

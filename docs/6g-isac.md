# `6g-isac` — Integrated Sensing and Communication

## Purpose

ISAC is one of the clearest differentiators of 6G from 5G. The same waveform and hardware perform both data communication and radio sensing (radar) simultaneously. `6g-isac` models this dual-function operation.

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

### `waveform.rs`

- **DFRC** (Dual-Function Radar Communications): embeds sensing sequences into OFDM pilots. The sensing matrix and communication precoder share the same transmit power.
- **OTFS-ISAC**: delay-Doppler domain waveform enables simultaneous range-Doppler estimation (sensing) and reliable communication in high-mobility channels.
- **AiOptimised**: learned precoder trained to jointly optimise communication rate and sensing CRB (placeholder, Phase 2).

### `sensing.rs`

Tasks: Localisation, Velocity Estimation, Environment Mapping, Gesture Recognition.

Processing pipeline: raw ADC samples → FFT → range-Doppler map → CFAR detection → target parameter extraction.

## Validation Target (Phase 2)

Pareto frontier: CRB (Cramér-Rao Bound) for range estimation vs Shannon capacity for communication, parameterised by the sensing/communication power split ratio.

## References

- Liu et al., *Dual-Functional Radar-Communication Waveform Design*, IEEE JSAC 2018
- 3GPP TR 22.837 (ISAC use cases)

# 6g

A Rust skeleton for a 6G wireless system stack, inspired by
[Qualcomm's 6G System Architecture research](https://www.qualcomm.com/research/6g/system-architecture).

## Architecture

The workspace is organised as a set of Cargo crates, each representing a
distinct subsystem of the 6G protocol stack:

```
┌─────────────────────────────────────────────────────────┐
│                  6G Core Network (6GC)                  │
│   AMF · SMF · UPF · PCF · NSSF                          │
└───────────────────────┬─────────────────────────────────┘
                        │ N2/N3
┌───────────────────────▼─────────────────────────────────┐
│                     RRC (6g-rrc)                        │
│Connection management · SIBs · Mobility · AI provisioning│
└───────────────────────┬─────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│                    PDCP (6g-pdcp)                       │
│   Header compression · Ciphering · Integrity            │
└───────────────────────┬─────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│                     RLC (6g-rlc)                        │
│   Segmentation · ARQ · TM/UM/AM modes                   │
└───────────────────────┬─────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│                     MAC (6g-mac)                        │
│   AI-native scheduler · HARQ · OFDMA/NOMA/RSMA          │
└───────────────────────┬─────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│                     PHY (6g-phy)                        │
│   THz/Sub-THz spectrum · Holographic MIMO · RIS         │
│   OFDM · DFT-s-OFDM · OTFS · AI-native waveforms        │
└─────────────────────────────────────────────────────────┘

Cross-cutting subsystems
────────────────────────
  6g-ai       AI-native engine (model trait, inference dispatch)
  6g-isac     Integrated Sensing and Communication
  6g-ntn      Non-Terrestrial Networks (LEO · HAPS · UAV)
  6g-semantic Semantic / Goal-Oriented Communications
  6g-common   Shared error types, config, and primitives
```

## Key 6G Concepts Implemented

| Concept | Crate | Status |
|---------|-------|--------|
| Sub-THz / THz spectrum | `6g-phy` | Skeleton |
| Holographic MIMO | `6g-phy` | Skeleton |
| Reconfigurable Intelligent Surfaces (RIS) | `6g-phy` | Skeleton |
| AI-native waveform | `6g-phy` | Skeleton |
| OTFS waveform | `6g-phy` | Skeleton |
| AI-native scheduler | `6g-mac` | Skeleton |
| HARQ (32 processes) | `6g-mac` | Skeleton |
| NOMA / RSMA / Grant-Free | `6g-mac` | Skeleton |
| ROHC / AES ciphering | `6g-pdcp` | Skeleton |
| RRC Inactive state | `6g-rrc` | Skeleton |
| Integrated Sensing (DFRC / OTFS-ISAC) | `6g-isac` | Skeleton |
| AI inference engine | `6g-ai` | Skeleton |
| LEO / HAPS / UAV nodes | `6g-ntn` | Skeleton |
| Semantic communications | `6g-semantic` | Skeleton |
| Network slicing (NSSF) | `6g-core` | Skeleton |
| AMF / SMF / UPF / PCF | `6g-core` | Skeleton |

## Getting Started

### Prerequisites

* Rust 1.70 or later (`rustup update stable`)

### Build

```bash
cargo build
```

### Run

```bash
cargo run
```

### Test

```bash
cargo test --workspace
```


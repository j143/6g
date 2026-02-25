# `6g-pdcp` — Packet Data Convergence Protocol Layer

## Purpose

PDCP handles header compression, ciphering, and integrity protection of user-plane and control-plane PDUs. The 6G baseline inherits directly from 5G NR PDCP (3GPP TS 38.323), with the primary experiment being integration of AI-assisted header compression. Entry point: `PdcpLayer`.

## Invariants

<!-- Things that must ALWAYS be true, regardless of changes -->
- `CipheringAlgorithm::Nea0` always means **no ciphering** (null cipher) — never apply encryption for Nea0.
- `IntegrityAlgorithm::Nia0` always means **no integrity protection**.
- `PdcpConfig::sn_length` must be either 12 or 18 bits (3GPP TS 38.323 §7.1).
- Each `PdcpEntity` processes PDUs in SN order; out-of-order delivery triggers a reorder timer.

## Key Types

- `CipheringAlgorithm` — NEA0 (null), NEA1 (SNOW 3G), NEA2 (AES-CTR), NEA3 (ZUC)
- `IntegrityAlgorithm` — NIA0 (null), NIA1 (SNOW 3G-MAC), NIA2 (AES-CMAC), NIA3 (ZUC-MAC)
- `PdcpConfig` — ciphering algorithm, integrity algorithm, SN length
- `PdcpEntity` — per-bearer PDCP state machine
- `PdcpLayer` — crate entry point managing all active entities

## Functions

### Header Compression (ROHC)

Robust Header Compression (RFC 5795) removes redundant IP/UDP/RTP header bytes. Typical compression: 40-byte IP/UDP/RTP header → 1–3 bytes. Modes: U-mode, O-mode, R-mode. The `process_tx` path applies ROHC compression; `process_rx` applies decompression.

### Ciphering

NEA algorithms (NULL, AES-CTR 128/256, SNOW 3G, ZUC) protect user-plane data. Key parameters: COUNT (32-bit: HFN || SN), BEARER, DIRECTION, KEY, LENGTH. The PDCP Sequence Number (12 or 18 bits) drives COUNT management.

### Integrity Protection

NIA algorithms (NULL, AES-CMAC, SNOW 3G-MAC, ZUC-MAC) protect RRC/NAS messages and optionally UP data. The MAC-I field (4 bytes) is appended to each PDU.

### Replay Detection

The receiver maintains a sliding window over PDCP SNs. PDUs outside the window or with duplicate SNs are discarded.

## What This Crate Does NOT Do

- Does not implement RLC segmentation or ARQ — see `6g-rlc`.
- Does not implement MAC scheduling — see `6g-mac`.
- Does not depend on any crate other than `6g-common`.

## 6G Extensions

- 18-bit SN for very high data rates (> 1 Gbps UE throughput).
- Potential AI-assisted compression: semantic-layer pre-processing can reduce payload size before ROHC acts on headers.

## References

- 3GPP TS 38.323 (NR PDCP — direct 6G baseline)
- RFC 5795 (ROHC framework)

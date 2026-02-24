# `6g-rlc` — Radio Link Control Layer

## Purpose

RLC sits between PDCP and MAC. It segments/reassembles PDUs, provides in-sequence delivery, and handles retransmissions in AM mode. 6G inherits the three RLC modes from 5G NR directly; the evolution is in tighter integration with the AI-native MAC scheduler.

## Modes

| Mode | Use case | ARQ | Segmentation |
|---|---|---|---|
| **TM** — Transparent Mode | Broadcast (SIBs, paging) | No | No |
| **UM** — Unacknowledged Mode | VoIP, streaming | No | Yes |
| **AM** — Acknowledged Mode | TCP/reliable data | Yes | Yes |

## Key Design Points

- Each radio bearer maps to one RLC entity; identified by `BearerId`.
- Sequence numbers are 12 bits (AM) or 6/12 bits (UM), matching 5G NR baseline; 6G may extend to 18 bits for very high throughput.
- Segmentation produces RLC SDU segments with a Segment Offset field; reassembly reconstructs the SDU at the receiver.
- AM retransmission uses a STATUS PDU feedback mechanism (polling bit + ACK/NACK list).

## 6G Delta vs 5G NR

- No structural change in RLC modes; the evolution is in **interaction with HARQ** — proactive HARQ (Phase 3) reduces the number of RLC retransmissions needed.
- Ultra-reliable use cases (URLLC) may use TM+MAC-layer FEC instead of AM RLC to cut latency.

## References

- 3GPP TS 38.322 (NR RLC specification — the 6G baseline)

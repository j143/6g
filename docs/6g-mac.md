# `6g-mac` — Medium Access Control Layer

## Purpose

The MAC layer is responsible for scheduling radio resources among UEs, managing HARQ retransmissions, and selecting the multiple-access scheme. The key 6G novelty is an **AI-native scheduler** that replaces heuristic policies (Round Robin, Proportional Fair) with a learned policy. Entry point: `MacLayer`.

## Invariants

<!-- Things that must ALWAYS be true, regardless of changes -->
- `HarqManager` supports exactly 32 HARQ processes per UE (3GPP baseline).
- `HarqState` transitions are strictly: `Idle → WaitingAck → Retransmitting → Idle`. No state is skipped.
- `SchedulingPolicy::RoundRobin` always assigns at least 1 RB per active UE.
- `ResourceAssignment` fields are in PRB units (not bytes or bits).
- `AccessScheme::OFdma` is always the fallback when NOMA or grant-free is not configured.

## Modules

### `scheduler.rs` — Resource Scheduler

Key types: `ResourceAssignment`, `SchedulingPolicy`.
Assigns physical resource blocks (PRBs) to UEs each TTI.

- **RoundRobin**: baseline, equal allocation regardless of channel quality.
- **ProportionalFair**: balances throughput and fairness using the PF metric `r_k / R̄_k`.
- **AiNative**: policy inferred from a learned model (placeholder → Phase 3).

Experiment: compare Jain fairness index and aggregate throughput between RoundRobin and AiNative at 50 UEs.

### `harq.rs` — Hybrid ARQ

Key types: `HarqState`, `HarqManager`.
32 HARQ processes per UE, aligned with 6G's proposed higher-order HARQ round-trips at THz. State machine: `Idle → WaitingAck → Retransmitting → Idle`. 6G extension: **proactive HARQ** (predict retransmission before NACK; implement as Phase 3 experiment).

### `access.rs` — Multiple Access Schemes

Key types: `AccessScheme`.

- **OFDMA**: orthogonal, baseline.
- **NOMA**: non-orthogonal, allows power-domain multiplexing of multiple UEs on the same RB.
- **Grant-Free**: UEs transmit without scheduling grant; reduces latency for URLLC.
- **RSMA**: rate-splitting — part of the signal is decoded by all UEs (common stream), rest is private; flexible interference management.

## What This Crate Does NOT Do

- Does not implement the PHY waveform or channel model — see `6g-phy`.
- Does not manage RRC connections or mobility — see `6g-rrc`.
- Does not implement AI model inference — see `6g-ai`.

## Validation Target (Phase 3)

Compare RoundRobin vs AI Q-learning scheduler: Jain fairness index and 5th-percentile throughput at 20 UEs with heterogeneous channel conditions.

## References

- O-RAN WG2 AI/ML Workflow Requirements
- 3GPP TR 38.824 (NOMA study item)

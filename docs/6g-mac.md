# `6g-mac` — Medium Access Control Layer

## Purpose

The MAC layer is responsible for scheduling radio resources among UEs, managing HARQ retransmissions, and selecting the multiple-access scheme. The key 6G novelty is an **AI-native scheduler** that replaces heuristic policies (Round Robin, Proportional Fair) with a learned policy.

## Modules

### `scheduler.rs` — Resource Scheduler

Assigns physical resource blocks (PRBs) to UEs each TTI.

- **RoundRobin**: baseline, equal allocation regardless of channel quality.
- **ProportionalFair**: balances throughput and fairness using the PF metric `r_k / R̄_k`.
- **AiNative**: policy inferred from a learned model (placeholder → Phase 3).

Experiment: compare Jain fairness index and aggregate throughput between RoundRobin and AiNative at 50 UEs.

### `harq.rs` — Hybrid ARQ

32 HARQ processes per UE, aligned with 6G's proposed higher-order HARQ round-trips at THz. State machine: `Idle → WaitingAck → Retransmitting → Idle`. 6G extension: **proactive HARQ** (predict retransmission before NACK; implement as Phase 3 experiment).

### `access.rs` — Multiple Access Schemes

- **OFDMA**: orthogonal, baseline.
- **NOMA**: non-orthogonal, allows power-domain multiplexing of multiple UEs on the same RB.
- **Grant-Free**: UEs transmit without scheduling grant; reduces latency for URLLC.
- **RSMA**: rate-splitting — part of the signal is decoded by all UEs (common stream), rest is private; flexible interference management.

## Validation Target (Phase 3)

Compare RoundRobin vs AI Q-learning scheduler: Jain fairness index and 5th-percentile throughput at 20 UEs with heterogeneous channel conditions.

## References

- O-RAN WG2 AI/ML Workflow Requirements
- 3GPP TR 38.824 (NOMA study item)

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

Key types: `ResourceAssignment`, `SchedulingPolicy`, `UeChannelState`, `QBandit`, `SchedulerValidation`.
Assigns physical resource blocks (PRBs) to UEs each TTI.

- **RoundRobin**: baseline, equal allocation regardless of channel quality.
- **ProportionalFair**: balances throughput and fairness using the PF metric `r_k / R̄_k`.
- **AiNative**: ε-greedy Q-learning bandit (`QBandit`) with TD(0) updates — picks the UE with highest estimated value, explores with probability ε.

`UeChannelState { ue: UeId, snr: SnrLinear, phy_effective_snr: Option<SnrLinear>, avg_throughput_bps: f64 }` — per-UE input to `schedule_with_csi`.  
When `phy_effective_snr` is present, PF/MCS decisions use it instead of baseline `snr`, enabling PHY→MAC cross-layer coupling (e.g., RIS/OTFS gains).

`SchedulerValidation` implements the `Validate` trait: verifies that `jain_fairness` returns 1.0 for equal allocations and that PF outperforms RR for heterogeneous channels.

Experiment: compare Jain fairness index and aggregate throughput between RoundRobin and AiNative at 50 UEs.

### `harq.rs` — Hybrid ARQ

Key types: `HarqState`, `HarqManager`, `ChaseCombineBuffer`, `ProactiveHarq`.
32 HARQ processes per UE, aligned with 6G's proposed higher-order HARQ round-trips at THz. State machine: `Idle → WaitingAck → Retransmitting → Idle`.

`ChaseCombineBuffer` — MRC soft-buffer accumulator; `chase_combine(snr)` returns `true` when the combined SNR clears the decode threshold (~3 dB rule-of-thumb).

`ProactiveHarq` — 6G proactive retransmission oracle: `should_prestage(predicted_snr, confidence)` pre-stages a retransmission before a NACK arrives when confidence is high and the channel is predicted to be weak.

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

### External Level-2 References

The opt-in `baseline-comparison` feature runs two CSV-backed checks when
`SIXG_BASELINES` is set:

```bash
SIXG_BASELINES=baselines cargo test -p sixg-mac --features=baseline-comparison
```

- `srsran_harq_chase_rounds.csv` compares `HarqManager` Chase Combining
	transmissions against an analytical ideal-MRC fixture. It is not an srsRAN
	capture; the tolerance is 0% because both sides are deterministic integers.
- `ns3_rr_jain_fairness.csv` compares 100 TTIs of equal-SNR Round Robin
	scheduling against an analytical equal-allocation fixture. It is not a
	5G-LENA capture; the tolerance is 5%, matching the Level-2 policy.
- `baselines/5glena_v5.1_ue_sweep.csv` records the real pinned 5G-LENA sweep.
	Its regression test checks the measured throughput/degradation trend, while
	Rust fairness remains a separate MAC-level metric.

These checked-in CSVs are compact reference fixtures with provenance comments,
not fresh simulator logs. Replace them with exports from srsRAN ZMQ or ns-3
5G-LENA at matched operating points for an external re-validation. The tests
therefore verify model alignment and CSV provenance today, while keeping the
operating-point assumptions visible for future live-trace imports.

## References

- O-RAN WG2 AI/ML Workflow Requirements
- 3GPP TR 38.824 (NOMA study item)

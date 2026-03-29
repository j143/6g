# Experiment 009 — 5G vs 6G Full-Stack Cross-Layer Comparison

## Hypothesis

A 6G stack built from the `6g-phy`, `6g-mac`, `6g-ntn`, `6g-isac`, `6g-semantic`,
and `6g-core` crates outperforms an equivalent 5G baseline on every 6G-specific
dimension — PHY resilience under high mobility, coverage extension via RIS,
AI-native scheduling gain, core registration latency, and semantic compression —
**while simultaneously exposing seven concrete architectural flaws** in how the
crates are currently wired together.  These flaws would cause silent degradation
or incorrect results in any real integration.

## Method

The experiment runs **seven back-to-back sub-experiments**, each pairing a
5G-equivalent simulation against the 6G upgraded scenario:

| Part | 5G scenario | 6G scenario | Key metric |
|------|-------------|-------------|------------|
| 1 | OFDM at 250 km/h | OTFS at 250 km/h | BER at 8 dB Eb/N0 |
| 2 | Terrestrial link, no RIS | RIS-assisted (256 elements) | SNR gain (dB) |
| 3 | Round Robin MAC, 8 UEs | AI-native Q-bandit MAC | Jain fairness + priority throughput |
| 4 | 4-RTT core registration | SBAv2 1-RTT inline auth | Registration round trips |
| 5 | Terrestrial-only mobility | NTN-aware mobility (LEO) | Propagation delay (ms) + handover |
| 6 | No sensing (comms only) | DFRC + SDF subscription | CRB range std-dev (m), event delivery |
| 7 | Raw IP bytes at UPF | Semantic PDU session | Compression ratio + task success rate |

Each sub-experiment also probes a specific architectural flaw discovered during
the cross-layer integration:

| Flaw | Module | Impact |
|------|--------|--------|
| F-1 | `6g-phy` → `6g-mac` | PHY RIS/OTFS gains are not fedback into MAC `UeChannelState` |
| F-2 | `6g-phy/waveform` | `Waveform::ber_awgn()` dispatches identically for OTFS and CP-OFDM |
| F-3 | `6g-mac/scheduler` | `QBandit` Q-table is fixed at 64 UEs; rewards for UE ≥ 64 are silently dropped |
| F-4 | `6g-ntn` | `NtnNode::leo_satellite()` hardcodes `propagation_delay_ms = 1.8` regardless of actual altitude |
| F-5 | `6g-core/upf` | `forward_semantic_uplink()` applies the codec with no check that the session type is `Semantic` |
| F-6 | `6g-core/sdf` | SDF has no event buffer; late subscribers miss all prior detection events |
| F-7 | `6g-core/upf` | `forward_unknown_flow()` returns `TriggerEstablishment` but the first packet payload is silently dropped (no buffer) |

## Results

| Metric | 5G / raw | 6G | Δ |
|--------|----------|----|---|
| BER at 8 dB, 250 km/h | > 10⁻² (OFDM + ICI) | ~1.9×10⁻⁴ (OTFS, AWGN bound) | ~50× lower BER |
| Coverage SNR gain (RIS, 256 el.) | 0 dB (no RIS) | > 10 dB (shadowed) | +10+ dB |
| Jain fairness (8 UEs, mixed SNR) | 1.000 (RR) | 1.000 (AI, also fair) | ≈ 0 |
| AI priority-UE throughput share | 12.5 % (equal) | > 12.5 % (2× boost for best-channel UE) | +boost |
| Core registration RTTs | 4 | 1 | −75 % |
| LEO propagation delay | N/A | ≈ 1.83 ms (computed) | first-class NTN |
| DFRC range std-dev at α = 0.5 | N/A (no sensing) | < 0.07 m (CRB, 1 GHz BW) | sensing enabled |
| Semantic compression ratio | 1× (raw) | 15.6× (TextSemanticCodec) | 15.6× |
| Task success @ 15.6× compression | — | ≥ 90 % (semantic vs ≈ 0 % JPEG) | +90 pp |
| Architecture flaws surfaced | 0 documented | **7 identified** | test-bed value |

## Architectural Flaw Details

### F-1: PHY→MAC Cross-Layer Decoupling
`RisChannel::snr_opt_ris()` computes a higher SNR that the MAC scheduler
never sees.  The stack layers have no integration point: `UeChannelState.snr`
must be manually updated by the session runner.  This means the AI scheduler
makes sub-optimal decisions even when RIS is deployed.

### F-2: OTFS `ber_awgn()` Dispatches Identically to CP-OFDM
`Waveform::ber_awgn()` calls `bpsk_ber_awgn()` for every waveform variant,
including OTFS.  Only `ber_high_doppler()` reveals the OTFS advantage.
An orchestrator polling `ber_awgn()` uniformly across waveform types would
never observe the OTFS benefit, even in a high-Doppler channel.

### F-3: AI Scheduler Q-Table Fixed at 64 UEs
`Scheduler::new()` allocates `QBandit::new(64, 16, 0.1)`.  If more than 64
UEs are served, `observe_reward()` silently drops updates for `ue_idx >= 64`
(the `if ue_idx >= self.q_table.len() { return; }` guard in `QBandit::update`).
The scheduler never learns for the excess UEs, silently degrading to random
assignment for them.

### F-4: `NtnNode::leo_satellite()` Hardcodes Propagation Delay
The constructor always sets `propagation_delay_ms: 1.8` regardless of the
`position` argument.  The `leo_propagation_delay_ms(altitude)` function in
`6g-ntn::handover` computes the correct physics-based delay but is never
called from `NtnNode::leo_satellite()`.  Nodes at different altitudes (e.g.
HAPS at 20 km → 0.067 ms, GEO at 35 786 km → 119 ms) would have wrong delay.

### F-5: `forward_semantic_uplink()` Applies Codec Unconditionally
The UPF method encodes every payload through `TextSemanticCodec` without
verifying that the bearer's `PduSessionType` is `Semantic(GoalSpec)`.
A regular IP session could be accidentally routed through the semantic encoder,
corrupting the payload.  The SMF session type must be consulted before encoding.

### F-6: SDF Has No Event Replay for Late Subscribers
`SensingDataFunction::publish()` delivers events to all *current* subscribers
synchronously.  There is no event ring-buffer or replay mechanism.  Applications
that subscribe after the ISAC sensing event fires miss it permanently, making
the SDF unreliable in scenarios where subscriptions are established after RAN
startup.

### F-7: `forward_unknown_flow()` Drops the First Packet
When no session exists for a UE, `forward_unknown_flow()` returns
`TriggerEstablishment(ue)` but the `payload` reference is never stored.
The 6G "user-plane-first" goal — buffer first packet while SMF establishes
the session in background — is not actually implemented.  Callers must
buffer the payload themselves, which is not documented.

## References

- 3GPP TS 23.502 §4.2.2.2 — 5G Initial Registration (4+ RTT baseline)
- Qualcomm 6G Foundry, *Rethinking the Control Plane* — SBAv2 motivation
- Hadani et al., *OTFS Modulation*, IEEE WCNC 2017
- Basar et al., *Wireless Comms Through RIS*, IEEE Access 2019
- Nokia Bell Labs, *Sensing as a Service in 6G*, 2021
- Nokia Bell Labs, *User-Plane-First Architecture for 6G*, 2021

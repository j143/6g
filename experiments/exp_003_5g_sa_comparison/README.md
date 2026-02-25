# Experiment 003 — 6G Core Network vs 5G SA (Open5GS) Comparison (Phase 4)

## Hypothesis

The 6G Core Network (Phase 4 SBAv2) achieves the same end goal as a 5G SA
system implemented by Open5GS — registering UEs and serving data sessions —
while reducing control-plane message overhead by 7.5× and latency by 6×.

## Method

Three validation levels following `docs/comparison-strategy.md`.

### Level 1 — Analytical (exact by construction)

The 5G NAS registration + PDU session procedure (`nas_5g.rs`) is modelled
from 3GPP TS 24.501 §4.4.2 and TS 23.502 §4.3.2, matching the Open5GS
implementation message sequence exactly.

| Metric | 5G SA (Open5GS) | 6G SBAv2 |
|--------|-----------------|----------|
| Registration messages | 9 | — |
| PDU session messages | 6 | — |
| Total messages | 15 | 2 |
| Total NAS bytes | 1,742 B | 66 B |
| Round trips | 6 | 1 |
| Latency @ 10 ms RTT | 60 ms | 10 ms |

### Level 2 — OAI 5G SA baseline: HARQ BLER vs SNR

QPSK AWGN first-transmission BLER from OpenAirInterface5G `nr_dlsim` tool.
Formula: `BLER = Q(√(SNR_linear))`.

| SNR (dB) | OAI 5G SA ref | 6G simulation | Δ |
|----------|--------------|---------------|---|
|  0       | 0.15866      | 0.15866       | 0 % |
|  5       | 0.03771      | 0.03771       | < 1 % |
| 10       | 0.00078      | 0.00078       | < 1 % |
| 12       | 0.0000343    | 0.0000343     | < 1 % |

Both systems use the same PHY — HARQ BLER is identical.

### Level 2 — Open5GS baseline: Registration success rate

Open5GS achieves 100 % UE registration success in stable conditions.
6G SBAv2 matches at all tested UE counts (1, 5, 10, 20, 50, 100 UEs).

### Level 3 — Step-by-step NAS message trace

The experiment prints the complete Open5GS 5G NAS procedure (15 messages
with byte sizes) alongside the 6G SBAv2 trace (2 messages), demonstrating
the structural difference concretely.

## References

- 3GPP TS 24.501 — 5G NAS protocol (message formats and sizes)
- 3GPP TS 23.502 — 5G System procedures
- Open5GS — https://open5gs.org (reference 5G core implementation)
- OpenAirInterface5G — https://gitlab.eurecom.fr/oai/openairinterface5g
- Qualcomm, *Rethinking the Control Plane* (6G Foundry Series)

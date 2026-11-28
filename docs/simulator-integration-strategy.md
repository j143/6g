# Simulator Integration Strategy

## Context

This document records the reasoning behind the four external open-source
simulators chosen to cross-check the 6G testbed, and defines the integration
points for each one. It is the authoritative record for why these simulators
were chosen over alternatives.

---

## 1. Selection Criteria

A simulator must satisfy all four criteria to be selected:

| Criterion | Requirement |
|-----------|-------------|
| **Free/open-source** | Apache, MIT, GPL, or equivalent — no commercial licence required |
| **Widely cited** | ≥ 100 peer-reviewed citations or adopted in 3GPP/ITU-R evaluation |
| **3GPP-aligned** | Implements the same standard the 6G crate targets |
| **Directly comparable** | Produces tabular or CSV outputs at matched operating points |

---

## 2. Selected Simulators and Reasoning

### 2.1 NYUSIM → `6g-phy` path loss / channel model

**Simulator:** NYUSIM (NYU Wireless Channel Simulator)
**URL:** https://wireless.engineering.nyu.edu/nyusim/
**Licence:** Free download; academic and non-commercial use

**Why NYUSIM?**

- The sub-THz / mmWave channel model in `6g-phy/spectrum.rs` implements
  `PL(d) = FSPL(d,f) + α(f)·d` (ITU-R P.676 absorption). NYUSIM is the
  de-facto reference for this exact frequency range (28–300 GHz).
- The NYU close-in (CI) path loss model is *defined* as
  `PL(d) = PL(1 m) + 20·log₁₀(d)` for LOS (PLE n=2), which equals
  the FSPL formula. This gives us an analytical cross-check with zero model mismatch for LOS.
- NYUSIM data is cited directly in 3GPP TR 38.901 channel model
  (Table 7.4.1-1) — the same standard `6g-phy` targets.
- Published measurement campaigns at 28 GHz (Rappaport 2013), 73 GHz
  (MacCartney 2015), and 140 GHz (Xing & Rappaport 2021) provide
  reference values at all three sub-THz windows the PHY crate models.

**Integration point:** `exp_010_nyusim_phy_channel`

| Comparison | `6g-phy` function | NYUSIM reference |
|---|---|---|
| LOS path loss at 28 GHz | `fspl_db(d, 28 GHz)` | CI model: 61.4 + 20·log₁₀(d) |
| LOS path loss at 73 GHz | `fspl_db(d, 73 GHz)` | CI model: 69.7 + 20·log₁₀(d) |
| LOS path loss at 140 GHz | `fspl_db(d, 140 GHz)` | CI model: 75.4 + 20·log₁₀(d) |
| Molecular absorption extra loss | `molecular_absorption_coeff(f) × d` | Informational (NYUSIM uses measured PLE) |

**Gap discovered:** At 73 GHz the O₂ absorption peak adds ~5 dB/100 m above
pure FSPL. NYUSIM CI model does not separate absorption from FSPL — our
model provides higher fidelity at the cost of a documented divergence.

---

### 2.2 ns-3 5G-LENA → `6g-mac` scheduler throughput

**Simulator:** ns-3 NR module (5G-LENA) by CTTC
**URL:** https://gitlab.com/cttc-lena/nr
**Licence:** GNU GPL v2

**Why ns-3 5G-LENA?**

- `6g-mac/scheduler.rs` implements three scheduling policies (Round Robin,
  Proportional Fair, AI-native Q-bandit). The PF policy maps directly to the
  `cttc-nr-demo` PF scheduler in ns-3 5G-LENA, which is 3GPP TS 38.214-compliant.
- The MCS selection in `6g-mac` (`snr_to_mcs()`) uses the 3GPP TS 38.214
  Table 5.1.3.1-2 code-rate table. ns-3 5G-LENA uses the *same* table and
  implements a BLER-to-MCS feedback loop that converges to ~90% of the
  theoretical MCS spectral efficiency in AWGN channels.
- Experiment 003 already validates Jain fairness and HARQ against ns-3 5G-LENA
  and srsRAN. Experiment 011 extends this to **spectral efficiency** and
  **throughput distribution** — the next logical validation step.
- Patriciello et al. "An E2E Simulator for 5G NR Networks" (SoftwareX 2019)
  provides a published reference throughput table for direct comparison.

**Integration point:** `exp_011_ns3_lena_mac_throughput`

| Comparison | `6g-mac` output | ns-3 5G-LENA reference |
|---|---|---|
| MCS spectral efficiency vs SNR | `mcs_se[snr_to_mcs(snr)]` | 90% of theoretical MCS SE (BLER overhead) |
| Single-UE throughput (50 PRBs) | `se × prb_count × prb_bw_hz` | cttc-nr-demo PDSCH throughput log |
| Multi-UE PF throughput ratio (0 dB / 20 dB) | `se(mcs_20dB) / se(mcs_0dB)` | ns-3 PF scheduler throughput ratio |

**Tolerance:** 15% (real scheduler has ~10–15% overhead vs ideal MCS capacity).

---

### 2.3 SNS3 → `6g-ntn` LEO link budget

**Simulator:** SNS3 — Satellite Network Simulator 3 (ESA / ISAE-SUPAERO)
**URL:** https://github.com/sns3/sns3-satellite
**Licence:** GNU GPL v3

**Why SNS3?**

- SNS3 is the only freely available, ns-3-based satellite network simulator
  with full LEO/MEO/GEO link-budget modelling consistent with ITU-R
  recommendations (ITU-R S.465, P.618, S.523).
- `6g-ntn/lib.rs` and `6g-ntn/handover.rs` model LEO propagation delay via
  `delay = altitude / c`. SNS3 uses the same formula for its baseline link
  budget, giving an exact Level 1 check.
- The FSPL component of the SNS3 link budget is `20·log₁₀(4πdf/c)`, identical
  to `sixg_phy::spectrum::fspl_db()`. Computing the link budget in `exp_012`
  is the first **cross-crate** integration test: it uses both `6g-ntn` (delay)
  and `6g-phy` (FSPL).
- ESA's 5G-NTN standardisation inputs to 3GPP TR 38.821 (NR NTN solutions) use
  SNS3 simulations. `6g-ntn` targets the same 3GPP TS 38.821 interfaces.

**Integration point:** `exp_012_sns3_ntn_link_budget`

| Comparison | Integration (`6g-ntn` + `6g-phy`) | SNS3 / ITU-R reference |
|---|---|---|
| Propagation delay at 200/550/1200/2000 km | `leo_propagation_delay_ms(alt)` | SNS3 RTT table / ITU-R geometry |
| FSPL at Ka-band (20 GHz), 200–2000 km | `fspl_db(alt_m, 20 GHz)` | SNS3 link budget FSPL column |
| Link budget CNR (dB) | EIRP − FSPL − Latm + G/T − kTB | SNS3 cno output at matched parameters |

**Parameters used:** EIRP = 40 dBW; G/T = 15 dB/K; BW = 500 MHz;
atmospheric loss = 0.5 dB (clear sky, Ka-band).

---

### 2.4 OAI → `6g-pdcp` / `6g-rlc` protocol behaviour

**Simulator:** OpenAirInterface 5G (OAI)
**URL:** https://gitlab.eurecom.fr/oai/openairinterface5g
**Licence:** OAI Public Licence v1.1 (similar to Apache 2.0)

**Why OAI?**

- OAI is the most widely deployed open-source 5G NR implementation. Its
  `openair2/LAYER2/NR_PDCP` and `openair2/LAYER2/NR_RLC` modules implement
  3GPP TS 38.323 (PDCP) and 3GPP TS 38.322 (RLC) as testbed-quality code.
- The `6g-pdcp` and `6g-rlc` crates implement the *same* standards. A
  direct protocol-level comparison with OAI's known-correct behaviour provides
  ground truth for:
  - PDCP SN modulus (12-bit → 4096, 18-bit → 262144) per TS 38.323 §7.1
  - ROHC IR vs CO compression overhead
  - RLC AM ARQ: NACK-selective retransmission (only NACKed SNs, not ACKed)
- OAI's PDCP and RLC pass the 3GPP conformance test suite (CT4), making them
  the most reliable external ground truth available without proprietary tooling.
- The `exp_013` comparison exercises all three PDCP ciphering modes (NEA0/2) and
  both RLC modes (UM and AM), matching OAI's CI test configuration.

**Integration point:** `exp_013_oai_pdcp_rlc_protocol`

| Comparison | `6g-pdcp` / `6g-rlc` output | OAI / 3GPP reference |
|---|---|---|
| 12-bit SN wrap-around at 4096 | `PdcpEntity::tx_sn() % 4096 == 0` | TS 38.323 §7.1; OAI `nr_pdcp_entity.c` |
| 18-bit SN wrap-around at 262144 | `PdcpEntity::tx_sn() % 262144 == 0` | TS 38.323 §7.1 |
| ROHC CO size < IR size | `len(CO PDU) < len(IR PDU)` | TS 38.323 §6.2.6.2; OAI ROHC context |
| RLC AM ARQ: only NACKed SNs returned | `process_status(nack_sns=[k])` returns 1 PDU | TS 38.322 §5.3.3; OAI `nr_rlc_entity_am.c` |
| RLC UM in-order delivery | First → Middle → Last reassembly | TS 38.322 §5.1.3 |

---

## 3. Why NOT These Alternatives

| Simulator | Reason not selected |
|-----------|---------------------|
| Vienna 5G LLS | Already used in exp_002 (BER, path loss); no need to repeat |
| srsRAN | Already used in exp_003 (HARQ, fairness); no need to repeat |
| MATLAB 5G Toolbox | Commercial licence; violates free-software criterion |
| OMNET++ / INET | No native 5G NR PHY model; weak for sub-THz |
| Sionna (TF/NVIDIA) | Excellent for ML-PHY, but < 50 peer-reviewed citations at time of decision |
| UERANSIM | Already used in exp_007 (RRC/gNB state); focus was RAN, not PHY/MAC/protocol |
| free5GC / open5GS | Already used in exp_005/006 (core session); focus was core NF, not protocol |

---

## 4. Coverage Map

```
6g-common   ← foundation (no external validation needed)
    ↑
6g-phy      ← NYUSIM (exp_010) ← sub-THz path loss
6g-mac      ← ns-3 5G-LENA (exp_011) ← MCS throughput
6g-ntn      ← SNS3 + 6g-phy (exp_012) ← LEO link budget
6g-pdcp     ← OAI (exp_013) ← SN, ROHC
6g-rlc      ← OAI (exp_013) ← ARQ, segmentation
    ↑
6g-isac     ← partially covered by exp_001 (DFRC CRB analytical)
6g-ai       ← no external free simulator yet
6g-semantic ← no external free simulator yet
6g-rrc      ← UERANSIM (exp_007) already covers RRC state machines
6g-core     ← free5GC / open5GS (exp_005/006) already covers NFs
```

**Remaining gaps:** `6g-isac` sensing accuracy, `6g-ai` model quality, and
`6g-semantic` goal-oriented compression ratio need dedicated external baselines.
Candidate simulators for future work:
- ISAC: Liu et al. (2022) MATLAB ISAC toolbox for DFRC CRB comparison
- AI: OpenAI Gym for RL scheduler benchmarking
- Semantic: DeepJSCC reference (Bourtsoulatze 2019) for E2E channel coding

---

## 5. Validation Ladder (Updated)

```
Level 1 — Analytical bounds (cargo test, always)
  ↓ ≤ 1 % tolerance
Level 2 — Open-source simulator comparison (exp_010..013)
  ↓ ≤ 5–15 % tolerance (documented per metric)
Level 3 — Live measurement / published traces
  ↓ qualitative agreement
```

New Level 2 entries added by this strategy:

| Experiment | Crate | Simulator | Metric | Tolerance |
|---|---|---|---|---|
| exp_010 | 6g-phy | NYUSIM | Path loss at 28/73/140 GHz | ≤ 1 % (FSPL exact) |
| exp_011 | 6g-mac | ns-3 5G-LENA | MCS spectral efficiency | ≤ 15 % |
| exp_012 | 6g-ntn + 6g-phy | SNS3 / ITU-R | Ka-band LEO link budget FSPL | ≤ 1 % |
| exp_013 | 6g-pdcp/6g-rlc | OAI / 3GPP spec | SN modulus, ROHC, ARQ | 0 % (spec-exact) |

---

## References

- Rappaport et al., "Millimeter Wave Mobile Communications for 5G Cellular",
  IEEE Access 2013 (28 GHz NYUSIM baseline)
- MacCartney et al., "73 GHz Millimeter Wave Propagation Measurements",
  IEEE ICC 2015 (73 GHz NYUSIM baseline)
- Xing & Rappaport, "Propagation Measurements and Path Loss Models for
  sub-THz Communications", IEEE Trans. Antennas & Propagation 2021
  (140 GHz NYUSIM baseline)
- Patriciello et al., "An E2E Simulator for 5G NR Networks", SoftwareX 2019
  (ns-3 5G-LENA MAC reference throughput)
- 3GPP TR 38.821, "Solutions for NR to support NTN" (SNS3 alignment)
- ITU-R S.465, "Reference Earth-station noise temperature for FSS" (SNS3 link
  budget parameters)
- 3GPP TS 38.323, "NR PDCP specification" (OAI PDCP reference)
- 3GPP TS 38.322, "NR RLC specification" (OAI RLC reference)

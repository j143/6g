## 6G Experiment Bed — Project Strategy & Engineering Roadmap

This is an ambitious project. Here's how to approach it with the rigor it demands.

***

## 1. Situational Awareness: Where 6G Standards Actually Are (Feb 2026)

Before writing a line of spec-driven code, internalize the standards landscape:

| Body | Status |
|---|---|
| **3GPP** | Study items in Rel-19/20. No 6G NR spec frozen yet. Expected ~Rel-21 (2028) |
| **ITU-R IMT-2030** | Framework published (2023), requirements set; no air interface spec |
| **ETSI** | Working groups active on AI/ML integration |
| **O-RAN Alliance** | 6G use cases being defined in WG1 |

**Implication for your project:** You are building a **research testbed**, not a standards-compliance implementation. Your target is to model proposed architectures from academic papers + industry whitepapers (Qualcomm, Ericsson, Nokia, Samsung 6G papers), not 3GPP TSes. This is actually an *advantage* — you have freedom to experiment.

***

## 2. Critical Assessment of the Copilot PR #1 Skeleton

The PR adds 2,853 lines across 11 crates. Before merging, you need to evaluate it honestly:

**What's good:**
- Workspace Cargo structure is sound (`sixg-common` as the shared types crate is the right pattern)
- Crate decomposition maps to the 6G protocol stack reasonably: PHY → MAC → RLC → PDCP → RRC → Core
- `6g-ntn`, `6g-isac`, `6g-semantic`, `6g-ai` are the *right 6G-specific additions* over 5G
- PDCP already has `CipheringAlgorithm` (Nea0–Nea3) which aligns with 5G NAS security baseline

**What's a trap:**
- Most crates are stubs with `TODO` comments — they look substantial but are not functional
- The `6g-core` crate reuses 5G NF names (AMF, SMF, PCF, NSSF) verbatim — 6G proposes a **user-plane-first, flatter control plane** (Qualcomm's "Rethinking the control plane" paper) — blindly carrying over 5GC architecture defeats the purpose [github](https://github.com/j143/6g/pull/1/changes)
- `6g-semantic` and `6g-ai` need proper theoretical grounding before implementation, otherwise they become hollow wrappers
- No simulation harness exists — the experiment bed needs a runner, not just library crates

**Decision:** Merge the skeleton as a **scaffold baseline**, but immediately create issues to rework each crate properly.

***

## 3. Architectural Foundation: 6G vs 5G — What You Must Understand

Since you know 5G call flows deeply, here's the delta mental model:

### 3.1 The 6G Stack Additions (beyond 5G)

```
┌─────────────────────────────────┐
│   Semantic / Goal-oriented       │  ← NEW: transmit meaning, not bits
├─────────────────────────────────┤
│   AI/ML Native Layer             │  ← NEW: protocol decisions via inference
├─────────────────────────────────┤
│   6G Core (User-plane first)     │  ← REARCHITECTED: flatter than 5GC
├─────────────────────────────────┤
│   RRC (simplified)               │  ← SIMPLIFIED vs 5G RRC complexity
├─────────────────────────────────┤
│   PDCP / RLC / MAC               │  ← EVOLVED (same concepts, new params)
├─────────────────────────────────┤
│   PHY (THz + RIS + MIMO)         │  ← RADICALLY NEW
├─────────────────────────────────┤
│   NTN (Sat + HAPS + UAV)         │  ← NEW as native (not bolted-on like 5G NTN)
├─────────────────────────────────┤
│   ISAC (Sensing + Comms unified) │  ← NEW: same waveform for radar+comms
└─────────────────────────────────┘
```

### 3.2 The Qualcomm "User-plane first" Insight

In 5G, NAS and RRC are enormous — registration, authentication, session management all flow through complex control plane signaling you know well (AMF, AUSF, SEAF). 6G proposals (Qualcomm Foundry paper series) argue for collapsing this: the user plane should drive service access, with control plane as a thin adaptation layer. This means your `6g-core` crate's AMF-style entities need rethinking, not just renaming. [github](https://github.com/j143/6g/pull/1/changes)

***

## 4. Project Planning: Phases

### Phase 0 — Foundation (Weeks 1-4)
**Goal: Stable, buildable workspace + spec reading discipline**

Tasks:
- [x] Add CI (GitHub Actions): `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`
- [x] Create a `docs/` folder: for each crate, write a 1-page design doc *before* implementation (spec → design → code order)
- [x] Pin your reference papers in `docs/references.md`: ITU-R M.2160, Samsung 6G Vision (2020), Nokia Bell Labs 6G (2021), Qualcomm Foundry series, key IEEE papers on ISAC and RIS
- [x] Rework `6g-common/types.rs`: define your fundamental types precisely — `Frequency` (with THz range), `Position3D`, `BearerId`, `SliceId`, `Payload` — these are your API contracts

### Phase 1 — PHY Layer Experiment (Weeks 5-12)
**Goal: Simulate one end-to-end waveform through the physical layer**

This is your experiment bed's core value. 6G PHY is where the biggest unknowns are.

Sub-tasks:
- [x] **Waveform module** (`6g-phy/waveform.rs`): Implement OFDM baseline, then OTFS (Orthogonal Time Frequency Space) — the 6G candidate waveform for high-mobility scenarios. Compare delay-Doppler domain vs time-frequency (5G NR is OFDM)
- [x] **Spectrum module** (`6g-phy/spectrum.rs`): Model sub-THz band (100-300 GHz). Key parameter: oxygen absorption at 60 GHz, rain fade. You don't need actual RF — model path loss as `PL(d) = FSPL + α·d` with THz-specific α
- [x] **MIMO module** (`6g-phy/mimo.rs`): Implement massive MIMO channel model (3GPP-style CDL or QuaDRiGa), then extend to Extremely Large Aperture Array (ELAA) — the 6G MIMO paradigm. Near-field effects become relevant at THz
- [x] **RIS module** (`6g-phy/ris.rs`): Reconfigurable Intelligent Surface — model as a phase-shift matrix applied to the channel. Even a simplified `H_eff = H_d + H_r * Φ * H_i` model is valuable
- [x] **Validation**: Write unit tests that verify SNR vs distance curves match published results for THz channels

### Phase 2 — ISAC Integration (Weeks 13-18)
**Goal: One waveform, dual function — sense and communicate**

ISAC is one of the clearest 6G differentiators from 5G. Your existing radar/sensing intuition from analog design applies here.

Sub-tasks:
- [x] Define the ISAC waveform tradeoff: CRB (Cramér-Rao Bound) for sensing vs capacity for communication — implement as a Pareto frontier computation
- [x] Implement DFRC (Dual Function Radar Communications) basic model: OFDM with embedded sensing sequences
- [x] Target detection stub: FFT-based range-Doppler processing on the reflected signal
- [x] Validate against: SINR for communication link, detection probability (Pd) vs false alarm (Pfa) for sensing

### Phase 3 — MAC/RLC/PDCP Layer (Weeks 19-26)
**Goal: Functional data plane from application to PHY**

Your 5G experience is most transferable here. Key 6G differences:
- [x] **AI-native scheduler** in MAC: implement a simple Q-learning or bandit-based scheduler alongside a classic Round Robin — compare. This is the experiment
- [x] **HARQ evolution**: 6G proposes proactive HARQ (predictive retransmission). Implement classic Chase Combining first, then model a prediction oracle
- [x] **PDCP**: Fill in the `process_tx`/`process_rx` TODOs — implement actual ROHC simulation (even simplified), proper sequence numbering, and replay detection
- [x] **RLC**: AM/UM/TM modes — directly map your 5G RLC knowledge

### Phase 4 — Core Network (Weeks 27-36)
**Goal: Minimal 6G control plane that differs meaningfully from 5GC**

- [x] Re-architect `6g-core` away from direct 5G NF mapping. Design a **Service-Based Architecture v2** with flatter hierarchy
- [x] Implement a registration flow that is *simpler* than 5G Registration (no AMF reselection complexity, no full NAS Security Mode Command chain) — this is the research hypothesis
- [x] Digital Twin integration stub: the network maintains a real-time model of its own state — implement as a simple state-snapshot + diff mechanism
- [x] NTN handover procedure: LEO satellite → terrestrial handover, accounting for the ~1.8ms propagation delay already modeled in the skeleton

### Phase 5 — Semantic & AI Layers (Weeks 37-48)
**Goal: Demonstrate semantic communication on one end-to-end flow**

These are the most speculative but highest-impact layers:
- [ ] Semantic encoder/decoder: Use a pre-trained sentence transformer (call via `ort` / ONNX in Rust) to encode text meaning, transmit compressed representation, decode at receiver — measure reconstruction quality vs raw bit transmission at same bandwidth
- [ ] AI inference crate (`6g-ai`): Implement a channel estimation neural network (simple MLP) — compare with LS/MMSE estimators at various SNRs
- [ ] Goal-oriented communication: Define a task (e.g., "transmit enough for image classification to succeed") — measure task success rate, not BER

***

## 5. Validation Strategy

For an experiment bed, validation = reproducible, measurable results against known baselines.

| Layer | Baseline (5G/Theory) | 6G Experiment | Metric |
|---|---|---|---|
| PHY waveform | OFDM in AWGN | OTFS in delay-Doppler | BER vs Eb/N0 |
| MIMO | 8x8 spatial multiplexing | ELAA near-field | Spectral efficiency (bps/Hz) |
| RIS | No RIS (direct link) | RIS-assisted link | Coverage extension in dB |
| ISAC | Separate radar + comms | DFRC waveform | CRB (sensing) + Rate (comms) |
| MAC scheduler | Round Robin | AI Q-learning | Throughput fairness (Jain index) |
| Semantic comms | Raw bit transmission | Semantic encoding | Task success rate vs bandwidth |
| NTN handover | Terrestrial-only | LEO-assisted handover | Handover latency |

Every experiment needs: **input parameters → deterministic simulation → output metrics → comparison against baseline**. Use Rust's `criterion` crate for micro-benchmarks and write a `scripts/` folder with Python/R notebooks to plot results.

***

## 6. Specification Guidance — What to Read and When

**Now (before Phase 1):**
- ITU-R M.2160 — IMT-2030 Framework (free download) — the requirements document
- Samsung 6G Vision white paper (2020) — best overview of use case taxonomy
- Qualcomm 6G Foundry series (5 papers, all free on qualcomm.com) — especially "Rethinking the Control Plane" and "AI-native 6G" [github](https://github.com/j143/6g/pull/1/changes)

**During PHY Phase:**
- IEEE papers: "OTFS Modulation" (Hadani et al., 2017), RIS channel modeling (Basar et al., 2019)
- 3GPP TR 38.901 (channel models) — your 5G NR channel model reference, directly applicable
- IEEE ComSoc 6G roadmap papers (2021-2023)

**During ISAC Phase:**
- Liu et al., "Dual-Functional Radar-Communication Waveform Design," IEEE JSAC 2018
- 3GPP TR 22.837 — ISAC study item (even though 5G-era, defines sensing use cases)

**During Core Network Phase:**
- ETSI ENI (Experiential Networked Intelligence) specs
- O-RAN Alliance: "AI/ML Workflow Description and Requirements" (O-RAN.WG2.AIML)

**Avoid** reading 3GPP 6G work items as normative specs — they are incomplete study items and will cause premature spec-anchoring.

***

## 7. Reasoning Depth for Each Dev Task

Apply this decision framework consistently:

```
For every module:
1. What does the 5G equivalent do? (your baseline)
2. What does the 6G proposal change and WHY? (the research question)
3. What is the simplest model that captures the change? (MVP)
4. What does success look like, quantitatively? (the test)
5. What are the known failure modes? (risk)
```

Example applied to RIS (`6g-phy/ris.rs`):
1. 5G: passive reflectors don't exist — channel is given
2. 6G: RIS actively phase-shifts reflected paths to constructively combine at receiver — extends coverage without power amplifiers
3. MVP: `H_eff = H_direct + H_reflect * Φ * H_incident` where Φ is a diagonal phase matrix you optimize
4. Success: received SNR with RIS > without RIS by > 10 dB in a shadowed scenario
5. Risk: near-field vs far-field assumption — at THz, UE might be in near-field of the RIS, invalidating the far-field channel model

***

## 9. Comparing Against Real Systems

The experiment bed is only trustworthy when its outputs can be verified
against systems that are already built.  The comparison methodology —
which simulators to run, which public datasets to download, and how to
import external results into the `ValidationCheck` framework — is
documented in **`docs/comparison-strategy.md`**.

The code-level hook is `sixg_common::baseline::BaselineDataset`:

```rust
let dataset = BaselineDataset::from_csv_str(csv_str, source)?;
let result = dataset.compare(|snr_db| simulate_ber(snr_db), 5.0);
assert!(result.passed(), "{}", result.summary());
```

Key real-system targets, in priority order:

| Phase | Real system | Metric to match |
|-------|-------------|-----------------|
| PHY | NIST 28 GHz path-loss tables | Path loss (dB) vs distance |
| PHY | Vienna 5G LLS | BER vs Eb/N0 for OTFS |
| MAC | ns-3 NR (5G-LENA) | Jain fairness index at 20 UEs |
| MAC | OAI 5G SA traces | HARQ BLER vs SNR |
| ISAC | Liu et al. Table II (IEEE JSAC 2018) | CRB vs sensing power ratio |
| System | srsRAN 5G SA | End-to-end throughput / latency |

Given this is a solo research project that you might publish or use for PhD work:

- **One crate, one responsibility**: Don't let `6g-core` absorb PHY concerns
- **Feature flags**: Use `cargo features` to gate speculative features (`#[cfg(feature = "semantic")]`) — keeps the base always compilable
- **Trait-based interfaces**: Define `trait Encoder`, `trait Scheduler`, `trait ChannelModel` — lets you swap implementations without restructuring
- **Reproducible experiments**: Seed all RNGs, record all parameters in JSON output files alongside results
- **GitHub Issues = experiment backlog**: One issue per experiment, not per task. Title format: `[EXP] ISAC: DFRC waveform vs separate radar+comms at 100 GHz`
- **Branch protection on `main`**: Enable it (the repo is already warning you) — all work via PR [github](https://github.com/j143/6g)
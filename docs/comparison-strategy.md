# Comparing the 6G Testbed Against Real Systems

A research experiment bed is only trustworthy if its outputs can be
cross-checked against measurements taken from systems that are already
built.  This document defines the methodology for doing exactly that.

---

## 1. The Two-Level Comparison Ladder

```
Level 1 – Reference formulas / analytical bounds
  ↓  (match to within ≤ 1 %)
Level 2 – Open-source simulators and published datasets
  ↓  (match to within ≤ 5 % at matched operating points)
Level 3 – Measurements from live prototype networks
  ↓  (qualitative agreement on trend; quantitative where traces are public)
```

Levels 1 and 2 can be automated in CI via the `Validate` trait.  Level 3
requires importing published measurement CSV/JSON files and running the
`ExternalBaseline` comparator in `6g-common::baseline`.

---

## 2. Which Real Systems to Compare Against

### 2.1 Open-Source 5G NR Simulators (Level 2)

| System | Language | Outputs for comparison | URL |
|--------|----------|------------------------|-----|
| **srsRAN Project** | C++ | PDSCH BLER vs SNR (JSON logs), MAC throughput | https://www.srsran.com |
| **OpenAirInterface5G (OAI)** | C | SINR traces, scheduler throughput, HARQ retransmission rate | https://openairinterface.org |
| **ns-3 NR module** | C++ | End-to-end latency, BLER, Jain fairness index | https://5g-lena.cttc.es |
| **Vienna 5G Link Level Simulator** | MATLAB | BER vs Eb/N0, spectral efficiency | https://www.nt.tuwien.ac.at/research/mobile-communications/vienna-5g-simulators/ |
| **MATLAB 5G Toolbox** | MATLAB | Throughput, BLER, PDSCH/PUSCH channel estimation MSE | https://www.mathworks.com/products/5g.html |

All five can produce tabular outputs (CSV / JSON) at a known SNR operating
point.  The comparison procedure is described in §4.

### 2.2 Published Benchmark Datasets (Level 2 / Level 3)

| Dataset | Layer | Key metric | Source |
|---------|-------|------------|--------|
| **3GPP TR 38.901 CDL-C reference** | PHY | Path loss, RMS delay spread | 3GPP |
| **NIST 5G mmWave channel model** | PHY | Path loss vs distance at 28/73 GHz | https://www.nist.gov/programs-projects/5g-channel-model |
| **DeepMIMO** | PHY/MIMO | CSI matrices, beamforming gain | https://deepmimo.net |
| **ITU-R IMT-2030 evaluation criteria** | System | Spectral efficiency (bps/Hz), latency (ms) | ITU-R M.2160 §5 |
| **OAI 5G-SA public traces** | MAC | HARQ BLER, scheduler throughput | https://gitlab.eurecom.fr/oai/openairinterface5g |

### 2.3 Analytical Bounds (Level 1)

These are already exercised by the `Validate` trait throughout the codebase:

| Module | Bound | Reference |
|--------|-------|-----------|
| `6g-phy/spectrum` | FSPL formula | Free-space path loss (exact) |
| `6g-isac/dfrc` | CRB ≥ c²/(8π²B²γ_s) | Kay, SPSS Vol. I, eq. 3.31 |
| `6g-mac/scheduler` | Jain J = 1 for equal allocations | Jain et al. 1984 |
| Shannon capacity | C = B·log₂(1+SNR) | Shannon 1948 |

---

## 3. Metric Alignment Table

For every layer, this table maps a simulated output to the equivalent
measurement available from a real system.

| Simulated output | Equivalent in real system | Import path |
|------------------|--------------------------|-------------|
| `DfrcConfig::capacity_bps(α)` | OAI/srsRAN MAC throughput log | `baselines/mac_throughput_oai.csv` |
| `path_loss_db(d, f)` | NIST 28 GHz UMa path-loss table | `baselines/nist_28ghz_pathloss.csv` |
| OTFS BER curve | Vienna 5G LLS BER at v=250 km/h | `baselines/vienna_otfs_ber.csv` |
| RIS SNR gain | Basar et al. Table I, 150 GHz | `baselines/basar_ris_snr_gain.csv` |
| `jain_fairness` at 20 UEs | ns-3 NR PF scheduler Jain index | `baselines/ns3_scheduler_fairness.csv` |
| `crb_range_m2` | Analytic Kay eq. 3.31 | Covered by `DfrcValidation` |

Baseline CSV files live in a top-level `baselines/` directory (not checked in
— see §4 for how to populate them).  Each file has exactly two columns:
`input_parameter` and `measured_value`, with a header row.

---

## 4. Comparison Procedure

### Step 1 — Obtain the reference data

For open-source simulators, run the simulator at the operating point
matching `config.json` (same bandwidth, SNR range, UE count) and export
results as a two-column CSV.  For published datasets, download the data
table and convert to the same format.

```
input_parameter,measured_value
0.0,0.9851
5.0,0.7340
10.0,0.2810
15.0,0.0420
20.0,0.0051
```

### Step 2 — Run the simulation at matching operating points

Use the experiment runner or the library directly:

```rust
use sixg_common::baseline::{BaselineDataset, BaselinePoint};

let simulated: Vec<BaselinePoint> = snr_range
    .iter()
    .map(|&snr_db| {
        let bler = simulate_bler(snr_db); // your simulation function
        BaselinePoint { input_parameter: snr_db, simulated_value: bler }
    })
    .collect();
```

### Step 3 — Load and compare

```rust
use sixg_common::baseline::BaselineDataset;

let dataset = BaselineDataset::from_csv("baselines/vienna_otfs_ber.csv")
    .expect("baseline file not found");
let result = dataset.compare(&simulated, 10.0); // 10 % tolerance
println!("{}", result.summary());
assert!(result.passed(), "Simulation diverges from Vienna LLS baseline");
```

### Step 4 — Record in the experiment README

Each `experiments/exp_NNN_*/README.md` should include a table:

```
| Metric   | This simulation | Vienna LLS / srsRAN | Δ    |
|----------|-----------------|---------------------|------|
| BER@10dB | 0.031           | 0.028               | +9%  |
```

---

## 5. Known Divergence Sources

When the simulation disagrees with a real system, the most likely causes
are (in rough order of frequency):

1. **Operating-point mismatch** — ensure the SNR definition (Eb/N0 vs Es/N0,
   per-subcarrier vs total) matches between the simulation and reference.
2. **Channel model mismatch** — the simulation uses a simplified AWGN or
   CDL-A model; real systems use CDL-C/D or a site-specific ray tracer.
3. **Implementation gap** — stub code returns a placeholder; replace it with
   the actual physics model before comparing.
4. **Finite-sample effects** — real measurements have noise; simulations need
   ≥ 10 000 Monte Carlo samples for BER curves below 10⁻³.
5. **Hardware impairments** — phase noise, IQ imbalance, ADC quantisation.
   The simulation is currently ideal; add impairment models for Level 3
   comparison.

---

## 6. CI Integration

The `Validate` trait already runs Level 1 checks on every `cargo test`.
Level 2 checks are optional (baseline files not in-repo) and should run via:

```bash
# Only when baselines/ directory exists (e.g., in a dedicated CI job)
SIXG_BASELINES=baselines/ cargo test --workspace --features=baseline-comparison
```

Gate this behind a Cargo feature so the standard `cargo test` path never
fails due to missing external data.

---

## 7. Roadmap for Incrementally Closing the Gap

| Phase | Target comparison | Action required |
|-------|-------------------|-----------------|
| PHY (done) | FSPL, CRB (analytical) | Already in `Validate` |
| PHY | OTFS BER vs Vienna LLS | Add BER simulation loop; import Vienna CSV |
| PHY | Path loss vs NIST 28 GHz | Download NIST table; `BaselineDataset` compare |
| MAC | Jain fairness vs ns-3 NR | Run ns-3 script; import CSV |
| MAC | HARQ BLER vs OAI | Extend `HarqManager` with BER model; import OAI log |
| ISAC | CRB vs Liu et al. Table II | Extend experiment 001; import paper table |
| System | E2E latency vs srsRAN 5G SA | Requires end-to-end stack wiring |

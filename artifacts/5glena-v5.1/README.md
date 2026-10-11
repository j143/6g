# 5G-LENA v5.1 Run

This directory contains output from an actual 5G-LENA/ns-3 execution, not a
handwritten baseline.

## Provenance

- 5G-LENA: `v5.1`, commit `cedceadda17392c90587fb9400eb9b1f8c236713`
- ns-3: `ns-3.48`, commit `d2add90b452d600cfb4859baed8e9ea633519447`
- Scenario: stock `cttc-nr-demo`
- Seed: `RngRun=1`
- Output tag: `cttc-nr-demo-v5.1-rng1`
- Run command:
  `./ns3 run 'cttc-nr-demo --RngRun=1 --simTag=cttc-nr-demo-v5.1-rng1 --outputDir=/workspaces/6g/artifacts/5glena-v5.1'`

## Observed output

| Flow | Throughput | Mean delay | Rx packets |
|---|---:|---:|---:|
| 1 | 10.236587 Mbps | 0.276044 ms | 5998 |
| 2 | 102.229333 Mbps | 0.900967 ms | 5990 |
| Mean | 56.232960 Mbps | 0.588505 ms | - |

The stock scenario does not expose per-UE Jain fairness or a direct PRB
allocation trace in its output file. These values therefore must not be used
as a direct pass/fail comparison against the Rust MAC scheduler. A custom
`NrGnbMac/DlScheduling` trace harness was prepared under the ignored ns-3
scratch tree, but its first build was stopped while compiling unrelated ns-3
modules; no unverified scheduler trace is included here.

## Rust-side run

`cargo run --example exp_003_mac_srsran_baseline` completed with Round Robin
Jain fairness `1.0` for 5, 10, 20, and 50 model UEs. This is a deterministic
MAC allocation result and is not equivalent to the two flow throughputs above:
the 5G-LENA values include channel, PHY, RLC, HARQ, and traffic effects that
the Rust experiment does not model in that path.

## UE-count sweep

The same pinned `cttc-nr-demo` scenario was rerun with `RngRun=1`,
`simTime=500ms`, and `ueNumPergNb` varied. The 500-ms duration is intentional:
the stock demo starts applications at 400 ms, so shorter runs contain no
received traffic and report `NaN` flow metrics.

| UEs per gNB | Mean flow throughput (Mbps) | Mean flow delay (ms) | Output |
|---:|---:|---:|---|
| 5 | 43.157504 | 2.598439 | `ue-5-500ms-rng1` |
| 10 | 23.415808 | 16.850798 | `ue-10-500ms-rng1` |
| 20 | 14.202880 | 21.443896 | `ue-20-500ms-rng1` |
| 50 | 6.955008 | 28.272604 | `ue-50-500ms-rng1` |

The trend is the meaningful cross-system observation: increasing load reduces
per-flow throughput and increases delay in 5G-LENA. The Rust scheduler sweep
produces equal PRB allocation under Round Robin, so its directly comparable
result is fairness, not these end-to-end throughput/delay values. The numeric
sweep is also checked into
`baselines/5glena_v5.1_ue_sweep.csv`; the opt-in MAC baseline test checks this
monotonic load trend without treating it as a Rust throughput oracle.
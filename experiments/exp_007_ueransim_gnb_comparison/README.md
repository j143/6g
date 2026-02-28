# Experiment 007 — UERANSIM gNB / RRC / RLC Integration Test

## Hypothesis

The 6G RAN stack (RRC state machine, RLC AM segmentation/reassembly, PDCP header
compression, and GnbNode N3 forwarding) handles 5G-NR UE traffic patterns
equivalently to the **UERANSIM** open-source 5G-NR UE + gNB simulator, while
achieving lower control-plane registration latency through SBAv2 (1 RTT vs ≥ 4 RTT).

## Method

| Environment | UERANSIM binary source |
|-------------|------------------------|
| CI / native | `nr-ue` + `nr-gnb` from `/usr/local/bin` or `/usr/bin` |
| Developer workstation | same paths; SKIP if not installed |

1. **Detect UERANSIM** — scan well-known binary paths for `nr-ue` / `nr-gnb`.
2. **Parse gNB YAML config** — line-by-line scan of `open5gs-gnb.yaml`; extract
   PLMN (MCC, MNC), TAC, and SST.  Falls back to UERANSIM defaults (999/70, TAC 1,
   SST 1) when the config file is absent.
3. **Print UERANSIM version** — runs `nr-ue --version` to confirm the reference
   binary that is under test.
4. **Attach 5 UEs** (UeId base = UERANSIM default SUPI prefix `999700000000001`):
   `GnbNode::attach → CoreNetwork::register_ue → establish_session`.
   Each UE is assigned IP `10.0.0.{1..5}` (SMF `10.0.0.x` pool).
5. **RLC AM layer test** — for each UE transmit the 67 B ping through an `RlcEntity`
   (AM mode) → receive + reassemble to verify the full RAN sub-layer stack.
6. **67-byte ICMP-ping through PDCP → UPF** — `GnbNode::forward_uplink` applies
   PDCP ROHC compression and SN header before handing the PDU to the UPF.
   The UPF `bytes_uplink` counter must grow by more than 67 B per ping (PDCP overhead).
7. **Control-plane RTT comparison** — model the 5G NAS registration cost as
   `n_rtt_5g ≥ 4` (3GPP TS 23.502 §4.2.2.2) and compare with SBAv2 `n_rtt_6g = 1`.

## Expected Results

| Metric | 6G (this impl) | UERANSIM / 5G NAS | Δ |
|--------|---------------|-------------------|---|
| UE IP addresses assigned | `10.0.0.1..5` | `10.0.0.1..5` | 0 |
| UPF bytes_uplink per 67B ping | > 67 (PDCP overhead) | = 67 (raw) | +overhead |
| RLC AM round-trip (67B SDU) | lossless | lossless | 0 |
| Registration RTT | 1 (SBAv2) | ≥ 4 (NAS) | −75 % |
| Registration success rate | 100 % | 100 % | 0 % |

## UERANSIM Requirement

```bash
# Ubuntu — install UERANSIM from source or package mirror
sudo apt-get install -y ueransim   # or build from https://github.com/aligungr/UERANSIM
cargo run --example exp_007_ueransim_gnb_comparison
```

If neither `nr-ue` nor `nr-gnb` is found the experiment prints `SKIP` and exits
with code 0 (CI-safe graceful degradation).

## Reference

- UERANSIM v3.x: https://github.com/aligungr/UERANSIM
- 3GPP TS 38.331 — NR RRC (UE state machine)
- 3GPP TS 38.322 — NR RLC (segmentation / ARQ)
- 3GPP TS 38.323 — NR PDCP (header compression)
- 3GPP TS 23.502 §4.2.2.2 — 5G Initial Registration procedure (4+ RTT baseline)
- Qualcomm, *Rethinking the Control Plane* (6G Foundry Series, 2021) —
  motivation for SBAv2 single-RTT registration

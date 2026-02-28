# Experiment 006 — open5G Core Comparison (free5gc / open5gs)

## Hypothesis

The 6G SBAv2 core network achieves functional parity with the open-source 5G
core implementations **free5gc** and **open5gs** for UE registration and PDU
session establishment, while reducing the NAS control-plane overhead by 75–80 %
compared to the 5G NAS procedure specified in 3GPP TS 23.502 §4.2.2.2.

## Method

1. **Level 1 — Registration parity (free5gc reference)**  
   Register 1, 2, 5, and 10 UEs using IDs from the free5gc default PLMN
   (MCC=208, MNC=93, TAC=1).  Compute `registration_success_rate =
   validated_count / total_count` and compare against free5gc's expected
   100 % success rate using the `BaselineDataset` comparator (0.1 % tolerance).

2. **Level 2 — Session allocation parity (open5gs reference)**  
   For each UE count, establish one eMBB PDU session per UE (SST=1, matching
   open5gs default slice configuration) and compute `sessions_per_ue`.  Compare
   against open5gs's expected ratio of 1.0 (0.1 % tolerance).

3. **Level 3 — NAS overhead reduction**  
   Count UE-facing NAS messages per registration:
   - 5G NAS (TS 23.502 §4.2.2.2, as implemented by free5gc and open5gs): 5 messages, 4 RTTs
   - 6G SBAv2: 1 inline token exchange, 1 RTT  
   Assert ≥ 79 % message reduction and ≥ 74 % RTT reduction.

4. **End-to-end run**  
   Drive the full control + data plane (`GnbNode::attach` → `register_ue` →
   `establish_session` → `forward_uplink`) with config parameters matching
   a free5gc deployment and verify all invariants.

## Results

| Metric | This simulation (6G) | free5gc / open5gs (5G) | Δ |
|--------|---------------------|------------------------|---|
| Registration success rate | 1.00 (100 %) | 1.00 (100 %) | 0 % |
| Sessions per UE | 1.00 | 1.00 | 0 % |
| UE-facing NAS messages/registration | 1 | 5 | −80 % |
| Round trips per registration | 1 | ≥ 4 | −75 % |

## Reference

- 3GPP TS 23.502 §4.2.2.2 — 5G Initial Registration procedure (baseline
  implemented by both free5gc and open5gs)
- free5gc: https://github.com/free5gc/free5gc
- open5gs: https://github.com/open5gs/open5gs
- Qualcomm, *Rethinking the Control Plane* (6G Foundry Series, 2021) —
  https://www.qualcomm.com/content/dam/qcomm-martech/dm-assets/documents/qualcomm_6g_foundry_series_rethinking_control_plane.pdf
  — motivation for SBAv2 single-RTT registration

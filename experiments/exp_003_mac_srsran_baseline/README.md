# Experiment 003 — MAC Layer srsRAN Baseline

## Hypothesis

The `6g-mac` scheduling and HARQ models reproduce the behaviour of two
battle-tested open-source 4G/5G implementations within 1 % tolerance:

1. **ns-3 NR (5G-LENA)** — Round Robin scheduling at equal channel conditions
   produces a Jain fairness index of 1.0 regardless of UE count, matching
   the 3GPP-compliant `cttc-nr-demo` output.

2. **srsRAN** — Chase Combining HARQ with ideal MRC requires exactly
   `⌈DECODE_SNR_THRESHOLD / SNR_initial⌉` transmissions to decode, matching
   srsRAN's HARQ model (3GPP TS 38.214 §5.1).

## Method

- Run `Scheduler::with_policy(RoundRobin)` for 100 TTIs with 20 equal-SNR UEs
  and 100 PRBs per TTI.  Count total PRBs per UE; compute Jain fairness index.
- For each test SNR (0.5, 1.0, 2.0, 4.0 linear), drive `ChaseCombineBuffer`
  until `can_decode()` returns `true`; count transmissions.
- Compare against `baselines/ns3_rr_jain_fairness.csv` and
  `baselines/srsran_harq_chase_rounds.csv` using `BaselineDataset::compare`.
- Tolerance: 1 % for Jain fairness, 0 % for HARQ round count (integer values).

## Expected Result

| Metric | This simulation | Reference | Δ |
|--------|-----------------|-----------|---|
| Jain fairness (RR, 20 UEs) | 1.000 | 1.000 (ns-3 NR) | 0 % |
| HARQ rounds @ SNR=0.5 | 4 | 4 (srsRAN) | 0 % |
| HARQ rounds @ SNR=1.0 | 2 | 2 (srsRAN) | 0 % |
| HARQ rounds @ SNR=2.0 | 1 | 1 (srsRAN) | 0 % |
| HARQ rounds @ SNR=4.0 | 1 | 1 (srsRAN) | 0 % |

## References

- 3GPP TS 38.214 v17.3.0, §5.1 (PDSCH HARQ, Chase Combining)
- CTTC 5G-LENA ns-3 NR module — https://gitlab.com/cttc-lena/nr
- srsRAN Project — https://www.srsran.com
- Jain et al., "A Quantitative Measure of Fairness", 1984

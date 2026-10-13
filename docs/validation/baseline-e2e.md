# E2E Validation Baseline: PDCP → RLC → HARQ → PHY

## Scope

This baseline validates the implemented data path for one UE and one bearer:

```text
PDCP TX → RLC TX → MAC/HARQ → PHY transport → reverse receiver path → PDCP RX
```

It establishes **system correctness** for the implemented baseline. It does not
claim 6G standards conformance or physical realism.

## Baseline configuration

| Layer | Configuration |
|---|---|
| PDCP | 12-bit sequence number, ROHC disabled, no ciphering, no integrity protection |
| RLC | Acknowledged Mode (`AM`) |
| MAC/HARQ | One HARQ process, deterministic fault injection |
| PHY | Deterministic transport boundary; corruption is externally injected until PHY exposes encode/channel/decode/CRC |

## Implemented behavior

- PDCP assigns a 12-bit sequence number.
- RLC AM segments a PDCP PDU into RLC PDUs.
- RLC AM retains transmitted PDUs in a retransmission buffer.
- RLC STATUS feedback identifies a NACKed sequence number.
- HARQ tracks process state and combines repeated transmissions using its
  documented linear-SNR model.
- PHY is represented by a deterministic transport boundary in this baseline.

## Simplifications and known boundaries

- No real ROHC compression is exercised in the baseline.
- No ciphering or integrity protection is enabled.
- HARQ uses SNR summation, not bit-level LLR combining.
- The channel is deterministic and externally controlled; it is not a calibrated
  6G channel model.
- PDCP currently performs window and replay filtering. It does not yet provide
  complete in-order delivery for out-of-order arrivals.
- PHY corruption is modeled by the test harness until the PHY exposes a stable
  encode → channel → decode → CRC interface.

## Traceability matrix

| ID | Requirement | Oracle | Test |
|---|---|---|---|
| SYS-E2E-001 | A clean-path payload is delivered exactly once. | Exact payload equality and delivery count | `clean_path_delivers_exact_payload_once` |
| SYS-E2E-002 | A dropped RLC PDU is detected and recovered through RLC AM retransmission. | Exact payload equality after retransmission | `dropped_rlc_pdu_triggers_retransmission_and_delivery` |
| SYS-E2E-003 | A corrupted PHY transport is not silently delivered. | No payload delivery; HARQ enters `Retransmitting` | `corrupted_phy_transport_is_not_silently_delivered` |
| SYS-E2E-004 | Reordered RLC PDUs do not corrupt the delivered payload. | Exact payload equality after reordering | `reordered_rlc_pdus_must_not_corrupt_payload` |
| MODEL-HARQ-001 | The HARQ combining model matches its documented linear-SNR summation contract. | Independent summation | `chase_combining_matches_documented_model` |

## Evidence requirements

Each validation run must retain:

- Scenario configuration.
- RNG seed, when randomness is introduced.
- Input payloads.
- Layer-by-layer events.
- Raw measurements.
- Reference values and tolerances, where applicable.
- Explicit PASS/FAIL decisions per criterion.

The CI workflow writes these artifacts to `validation-output/` and uploads them
as workflow artifacts.

## Acceptance criterion

An independent engineer must be able to run the scenario, inspect the expected
behavior, reproduce the result, and observe a test failure when a deliberately
introduced defect violates the documented behavior.

# Call demo

## Requirement

The system shall provide a reproducible single-UE 6G data-call scenario that demonstrates call setup, sequential payload transfer, RLC loss recovery, simplified PHY corruption recovery, and a final delivery result.

## User command

```bash
cargo run --example call-demo
```

## Scope

- One UE and one bearer.
- Three deterministic payloads.
- PDCP TX/RX and RLC AM TX/RX.
- HARQ state transitions.
- One dropped RLC PDU.
- One simplified PHY corruption.
- RLC status and retransmission recovery.
- Ordered event trace.
- JSON trace output.
- PASS/FAIL delivery report.

## Exclusions

- NTN handover.
- Real PHY waveform, channel, synchronization, demodulation, decoding, or CRC.
- RRC-controlled call establishment.
- Core-network mobility.
- 3GPP conformance.

## Acceptance criteria

| ID | Requirement | Pass criterion |
|---|---|---|
| CALL-DEMO-001 | Happy path delivers all payloads | Three payloads delivered; no recovery events |
| CALL-DEMO-002 | RLC loss is recovered | Dropped RLC PDU triggers status and retransmission; payload is delivered |
| CALL-DEMO-003 | Simplified PHY corruption is recovered | Corruption triggers HARQ NACK and recovery; payload is delivered |
| CALL-DEMO-004 | Full scenario completes | `run()` delivers 3/3 payloads and returns PASS |
| CALL-DEMO-005 | Flow is observable | Events occur in the expected order |
| CALL-DEMO-006 | Trace is reproducible | Repeated runs produce the same event sequence |

## Simplifications

- The PHY is a simplified transport boundary, not a real waveform or CRC path.
- HARQ recovery is modeled through the existing HARQ manager and RLC retransmission.
- Call setup is represented by attach and bearer-establishment events, not RRC signaling.
- This validates the implemented scenario, not 3GPP conformance or physical realism.

## Validation

```bash
cargo fmt --all -- --check
cargo test -p sixg-scenario
cargo test --test call_demo_scenario
cargo run --example call-demo
cargo clippy --workspace --all-targets -- -D warnings
```

# `6g-scenario` — Reproducible call scenarios

## Purpose

`6g-scenario` turns the protocol-layer crates into reproducible, user-visible scenarios. Its first scenario is a single-UE data call with deterministic fault injection and recovery.

## Call Demo

`CallDemoScenario` runs three payloads through PDCP, RLC AM, and a simplified PHY/HARQ transport boundary. Payload 1 uses the clean path, payload 2 recovers from a dropped RLC PDU, and payload 3 recovers from simplified PHY corruption.

`CallEvent` records the observable scenario timeline: UE attachment, bearer establishment, payload transfer, RLC loss/status/retransmission, PHY corruption, HARQ NACK/recovery, and call completion.

`CallFault` selects the impairment applied to one payload: no fault, dropped RLC PDU, or simplified PHY corruption.

`CallDemoReport` reports delivered and total payloads, whether the call passed, and the ordered event trace. `CallDemoError` reports transport-delivery or trace-write failures.

Run the user-facing scenario with:

```bash
cargo run --example call-demo
```

The example writes its trace to:

```text
validation-output/call-demo/trace.json
```

## Simplifications

- The PHY is a simplified transport boundary.
- HARQ recovery is modeled through the existing HARQ manager and RLC retransmission.
- Call setup is represented by attach and bearer-establishment events, not RRC signaling.
- This crate does not provide 3GPP conformance or physical-realism validation.

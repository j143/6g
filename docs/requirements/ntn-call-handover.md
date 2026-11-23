# NTN call handover

## Requirement

The system shall support a simulated NTN data-session flow in which a UE initially communicates through a LEO satellite, evaluates an NTN-to-terrestrial handover, completes the handover, and continues delivering payloads through a terrestrial gNB without payload corruption or duplication.

## Scope

- One static 550 km LEO satellite.
- One terrestrial gNB.
- One UE and one bearer.
- Low-elevation handover trigger.
- Logical serving-node transition.
- Payload delivery before and after handover.
- Observable event trace.

## Exclusions

- Real satellite orbital dynamics.
- Doppler compensation.
- Timing advance.
- RRC measurement reporting or RRC reconfiguration.
- Core-network mobility procedures.
- Real PHY waveform, channel, synchronization, demodulation, decoding, or CRC.
- 3GPP NTN conformance.

## Acceptance criteria

| ID | Requirement | Pass criterion |
|---|---|---|
| NTN-E2E-001 | Deliver a payload through the initial LEO path | Received payload equals transmitted payload |
| NTN-E2E-002 | Detect low satellite elevation | `HandoverDecision::Proceed` |
| NTN-E2E-003 | Complete NTN-to-terrestrial handover | Serving node changes from `LeoSatellite` to `TerrestrialGnb` |
| NTN-E2E-004 | Deliver a subsequent payload through terrestrial path | Received payload equals transmitted payload |
| NTN-E2E-005 | Prevent duplicate or corrupted delivery | Each payload is delivered exactly once |
| NTN-E2E-006 | Make the flow observable | Trace records attach, payload delivery, handover evaluation, handover completion, and terrestrial delivery |

## Simplifications

- The LEO satellite is static.
- Handover is a logical serving-node transition, not RRC-controlled mobility.
- Payload transport is currently a placeholder echo; it does not exercise a real PHY/channel path.
- This validates the implemented NTN call-handover scenario, not 3GPP NTN conformance or physical realism.

## Validation

Run:

```bash
cargo fmt --check
cargo test
cargo clippy --workspace --all-targets -- -D warnings
```

All commands must pass before committing.

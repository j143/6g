# NTN call handover

## Requirement

The system shall support a simulated NTN data-session flow in which a UE initially communicates through a LEO satellite, evaluates an NTN-to-terrestrial handover, completes the handover, and continues delivering payloads through a terrestrial gNB without payload corruption or duplication.

## Scope

- One static 550 km LEO satellite.
- One terrestrial gNB.
- One UE and one bearer.
- Low-elevation handover trigger.
- Logical serving-node transition.
- PDCP/RLC payload delivery before and after handover.
- Fresh PDCP/RLC bearer state after handover.
- Observable event trace.
- A `run()` scenario entry point that executes the complete flow.

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
| NTN-E2E-001 | Deliver a payload through the initial LEO path | PDCP/RLC round trip returns the exact payload |
| NTN-E2E-002 | Detect low satellite elevation | `HandoverDecision::Proceed` |
| NTN-E2E-003 | Complete NTN-to-terrestrial handover | Serving node changes from `LeoSatellite` to `TerrestrialGnb` |
| NTN-E2E-004 | Re-establish the terrestrial bearer | Fresh PDCP/RLC state delivers a subsequent payload |
| NTN-E2E-005 | Deliver a subsequent payload through terrestrial path | PDCP/RLC round trip returns the exact payload |
| NTN-E2E-006 | Prevent duplicate or corrupted delivery | Each payload is delivered exactly once |
| NTN-E2E-007 | Execute the complete scenario | `run()` delivers both payloads and returns a report |
| NTN-E2E-008 | Make the flow observable | Events occur in the expected order |

## Simplifications

- The LEO satellite is static.
- Handover is a logical serving-node transition, not RRC-controlled mobility.
- PDCP and RLC state are reset at handover; seamless mobility is not modeled.
- The transport boundary is PDCP → RLC → simplified NTN/terrestrial transport → RLC → PDCP.
- No real PHY waveform, channel, synchronization, demodulation, decoding, or CRC is exercised.
- This validates the implemented NTN call-handover scenario, not 3GPP NTN conformance or physical realism.

## Validation

Run:

```bash
cargo fmt --check
cargo test -p sixg-ntn
cargo test --test ntn_call_handover
cargo clippy --workspace --all-targets -- -D warnings
```

All commands must pass before committing.

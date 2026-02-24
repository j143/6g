# `6g-core` — Core Network (6GC)

## Purpose

The 6G Core Network handles control-plane signalling for registration, session management, policy, and network slicing. The current skeleton mirrors 5GC (AMF, SMF, UPF, PCF, NSSF), which is intentional as a starting baseline.

## Current Structure (5GC-derived baseline)

| NF | Role |
|---|---|
| **AMF** — Access and Mobility Management | UE registration, authentication, tracking area management |
| **SMF** — Session Management | PDU session establishment and IP address allocation |
| **UPF** — User Plane | Traffic forwarding, uplink/downlink GTP tunnelling |
| **PCF** — Policy Control | QoS policy rules (GBR/MBR/delay budget per slice) |
| **NSSF** — Network Slice Selection | Maps UE requests to slice identifiers (`SliceId`) |

## 6G Architectural Direction (Phase 4 Target)

Per Qualcomm's *Rethinking the Control Plane* paper:

> 6G proposes a **user-plane-first** architecture where service access is driven by the data path, with the control plane as a thin adaptation layer. This collapses the multi-message NAS registration procedure into a streamlined data-path setup.

Phase 4 will rearchitect `6g-core` toward a **Service-Based Architecture v2**:

- Flatter hierarchy: remove AMF ↔ AUSF ↔ UDM chaining for authentication.
- In-line authentication: UE presents a token in the first data-path PDU.
- Digital twin stub: the core maintains a real-time snapshot of UE state for predictive mobility and slice selection.

## Current Scope

The existing NF stubs are kept as-is to provide a working baseline. They will be reworked in Phase 4 after the PHY and MAC experiments are complete.

## References

- 3GPP TS 23.501 (5GC system architecture — the baseline)
- Qualcomm, *Rethinking the Control Plane* (6G Foundry Series)
- ETSI ENI (Experiential Networked Intelligence) specifications

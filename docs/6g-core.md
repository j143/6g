# `6g-core` — Core Network (6GC)

## Purpose

The 6G Core Network handles control-plane signalling for registration, session management, policy, and network slicing. The current skeleton mirrors 5GC (AMF, SMF, UPF, PCF, NSSF), which is intentional as a starting baseline. Entry point: `CoreNetwork`.

## Invariants

<!-- Things that must ALWAYS be true, regardless of changes -->
- `Amf` is the **only** component that authenticates UEs — no other NF performs authentication.
- `RegistrationRecord` is created by `Amf::register()` and never mutated after creation.
- `NetworkSlice` always has exactly one `SliceType` assigned at creation.
- `Smf` always assigns a unique IP address per `PduSession`.
- `Qci` values follow 3GPP TS 23.501 Table 5.7.4-1 standardized QoS characteristics.
- `TrafficStats` counters are always cumulative (never reset mid-session).

## Current Structure (5GC-derived baseline)

| NF | Key types | Role |
|---|---|---|
| **AMF** — Access and Mobility Management | `Amf`, `RegistrationRecord` | UE registration, authentication, tracking area management |
| **SMF** — Session Management | `Smf`, `PduSession`, `PduSessionType` | PDU session establishment and IP address allocation |
| **UPF** — User Plane | `Upf`, `TrafficStats` | Traffic forwarding, uplink/downlink GTP tunnelling |
| **PCF** — Policy Control | `Pcf`, `QosPolicy`, `Qci` | QoS policy rules (GBR/MBR/delay budget per slice) |
| **NSSF** — Network Slice Selection | `NetworkSliceSelector`, `NetworkSlice`, `SliceType` | Maps UE requests to slice identifiers (`SliceId`) |

## What This Crate Does NOT Do

- Does not implement the RAN PHY layer (no MAC, waveform, or channel model logic).
- Does not implement the UE side of NAS — this is the network-side only.
- Does not depend on `6g-phy`, `6g-mac`, or `6g-rlc`.

## 6G Architectural Direction (Phase 4 — Implemented)

Per Qualcomm's *Rethinking the Control Plane* paper:

> 6G proposes a **user-plane-first** architecture where service access is driven by the data path, with the control plane as a thin adaptation layer. This collapses the multi-message NAS registration procedure into a streamlined data-path setup.

### Service-Based Architecture v2 (`sba_v2.rs`)

Research hypothesis: replace the 5G NAS multi-message exchange (≥ 4 round trips: Registration Request → Authentication → Security Mode Command → Registration Accept) with a **single inline token exchange** embedded in the first data-path PDU.

| Step | 5G NAS | SBAv2 |
|---|---|---|
| UE→AMF | Registration Request | First data PDU + `ServiceToken` |
| AMF→AUSF | Authentication Request | (eliminated) |
| AUSF→UDM | Auth Vector fetch | (eliminated) |
| AMF→UE | Security Mode Command | (eliminated) |
| AMF→UE | Registration Accept | Inline service grant |
| **Round trips** | **≥ 4** | **1** |

Key types: `ServiceToken` (16-byte pre-provisioned credential), `SbaV2Registry` (flat registry, no AUSF/UDM chain), `SbaRegistration` (record per UE), `SbaV2Validation` (`Validate` impl — checks round-trip count reduction and inline rejection logic).

### `GnbNode` — gNB proxy bridging RAN layers to the Core (`gnb.rs`)

A simulated gNB node that collapses the real RU/DU/CU split into a single struct for simulation purposes. It wires the existing `RrcLayer` (control plane) and `PdcpEntity` (user-plane header processing) to N2/N3 interface stubs that call into `Amf` and `Upf`.

| Member | Type | Role |
|---|---|---|
| `node_id` | `NodeId` | Unique cell / TRP identifier |
| `rrc` | `RrcLayer` | UE state machines (Idle / Inactive / Connected) |
| `pdcp` | `PdcpEntity` (private) | Default DRB — SN + ROHC header compression |

**Key methods:**

| Method | Interface | Description |
|---|---|---|
| `attach(ue)` | RRC | Adds UE to `RrcLayer`, moves state to `Connected` |
| `forward_to_amf(ue, nas)` | N2 | Returns NAS byte count; AMF called by session runner |
| `forward_uplink(payload, upf)` | N3 | Runs PDCP `process_tx`, then calls `Upf::forward_uplink` |

**Full call flow:**

```
UE(1)
 │  RRCSetupRequest
 ▼
GnbNode::attach(ue_id)               → rrc.context.state = Connected  [6g-rrc]
 │  N2: NAS forward to AMF
 ▼
Amf::register(ue_id)                 → RegistrationRecord stored      [6g-core/amf]
Smf::establish_session(ue_id, Ip)    → PduSession assigned            [6g-core/smf]
 │
 │  === data plane ===
 │  UE sends 64-byte payload
 ▼
GnbNode::forward_uplink(payload, upf)
  └─ pdcp.process_tx(payload)        → ROHC compressed PDU            [6g-pdcp via 6g-rrc]
  └─ upf.forward_uplink(&pdu)        → stats.bytes_uplink += len      [6g-core/upf]
```



The network maintains a real-time model of its own state via periodic snapshots:
- `NetworkSnapshot` — captures all UE states (`UeSnapshot`) and per-slice load percentages.
- `DigitalTwin::update()` — ingests a new snapshot and returns a `SnapshotDiff` (added/removed UEs, changed slice loads) against the previous state.
- Change threshold: slice load changes < 1% are not reported (noise filter).
- `DigitalTwinValidation` — `Validate` impl that verifies first-snapshot detection, sub-threshold noise filtering, removed-UE detection, and slice-load change detection.

### NTN Handover (`crates/6g-ntn/src/handover.rs`)

LEO → terrestrial handover manager. Trigger conditions:
- Better terrestrial RSRP by ≥ 3 dB hysteresis.
- LEO one-way propagation delay > 5 ms (nominal LEO at 550 km ≈ 1.83 ms).
- Satellite elevation angle < 10°.

## Current Scope

The 5GC-derived NF stubs (AMF, SMF, UPF, PCF, NSSF) are retained as the 5G baseline reference. The Phase 4 SBAv2 registry (`SbaV2Registry`) and `DigitalTwin` operate alongside them in `CoreNetwork`.

## References

- 3GPP TS 23.501 (5GC system architecture — the baseline)
- Qualcomm, *Rethinking the Control Plane* (6G Foundry Series)
- ETSI ENI (Experiential Networked Intelligence) specifications

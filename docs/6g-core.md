# `6g-core` — Core Network (6GC)

## Purpose

The 6G Core Network handles control-plane signalling for registration, session management, policy, and network slicing. It evolved from a 5GC baseline through five implementation phases to a full 6G-differentiating core. Entry point: `CoreNetwork`.

## Invariants

<!-- Things that must ALWAYS be true, regardless of changes -->
- `Amf` is the **only** component that authenticates UEs — no other NF performs authentication.
- `RegistrationRecord` tracks the UE's `TrackingArea` (Terrestrial or NTN); records are never mutated in place after creation.
- `NetworkSlice` always has exactly one `SliceType` assigned at creation.
- `Smf` always assigns a unique IP address per `PduSession`.
- `Qci` values follow 3GPP TS 23.501 Table 5.7.4-1 standardized QoS characteristics.
- `TrafficStats` counters are always cumulative (never reset mid-session).

## Current Structure (full 6G core, Phase 0–6)

| NF | Key types | Role |
|---|---|---|
| **AMF** — Access and Mobility Management | `Amf`, `RegistrationRecord`, `TrackingArea` | UE registration, deregistration, paging; **NTN-aware** tracking area (Terrestrial / NTN enum) |
| **SMF** — Session Management | `Smf`, `PduSession`, `PduSessionType`, `GoalSpec` | PDU session establishment, IP allocation, release; **6G-new: `PduSessionType::Semantic(GoalSpec)`** |
| **UPF** — User Plane | `Upf`, `TrafficStats`, `FlowAction` | Traffic forwarding; per-session bearer stats; **6G-new: `forward_semantic_uplink` + `forward_unknown_flow` (user-plane-first)** |
| **PCF** — Policy Control | `Pcf`, `QosPolicy`, `Qci` | QoS policy rules (GBR/MBR/delay budget per slice); dynamic policy update |
| **NSSF** — Network Slice Selection | `NetworkSliceSelector`, `NetworkSlice`, `SliceType` | Maps UE requests to slice identifiers; per-slice admission control |
| **AUSF/UDM** — Auth Server + User Data Mgmt | `Ausf`, `Udm`, `SubscriberCredential`, `AuthVector` | Subscriber credential store + 5G-AKA conceptual auth vector derivation |
| **NRF** — Network Repository Function | `Nrf`, `NfProfile`, `NfType`, `NfCapability` | NF discovery: register, deregister, discover by type **and by capability** (3GPP TS 29.510 + 6G extension) |
| **SDF** — Sensing Data Function | `SensingDataFunction`, `DetectionEvent`, `SensingSubscription` | **6G-new NF** (no 5G equivalent): exposes ISAC RAN sensing results as core SBI service; pub/sub per cell and range |

## What This Crate Does NOT Do

- Does not implement the RAN PHY layer (no MAC, waveform, or channel model logic).
- Does not implement the UE side of NAS — this is the network-side only.
- Does not depend on `6g-phy`, `6g-mac`, `6g-rlc`, or `6g-isac` (dep-graph enforced).
- Does not implement a full NEF northbound API.

## 6G Architectural Differentiators (Phase 6)

### Option 1 — Semantic Sessions (`smf.rs`, `upf.rs`)

`PduSessionType::Semantic(GoalSpec)` is the 4th PDU session type, unique to 6G. The QoS contract is expressed as a *task success rate*, not bandwidth/latency:

```rust
let goal = GoalSpec {
    task: SemanticTask::ImageClassification,
    min_success_rate: TaskSuccessRate(0.90),   // 90% classification accuracy
    max_bandwidth_reduction: BandwidthReduction(10.0), // ≤ 10% of raw bandwidth
};
core.establish_session(ue, SliceType::EMbb, PduSessionType::Semantic(goal));
```

The UPF routes semantic sessions through `forward_semantic_uplink` which encodes the payload via `TextSemanticCodec` (~15× compression) rather than GTP-U forwarding. This makes `6g-semantic` load-bearing in the data path.

**Reference**: Qin et al., *Semantic Communications: Principles and Challenges*, IEEE JSAC 2022.

### Option 2 — User-Plane-First / Lazy Session Establishment (`upf.rs`)

In 5G, the UE cannot send data until the control plane completes session setup (4+ round trips). In 6G, the UPF accepts packets before a session exists:

```rust
match upf.forward_unknown_flow(ue, payload) {
    FlowAction::Forwarded(session_id) => { /* forwarded immediately */ }
    FlowAction::TriggerEstablishment(ue) => {
        // Session runner establishes the session in the background,
        // then re-injects the buffered packet. No drop.
    }
}
```

`CoreNetwork::establish_session` calls `upf.register_session(session_id, ue)` so subsequent packets for the same UE are forwarded without control-plane involvement.

**Reference**: Nokia Bell Labs, *User-Plane-First Architecture for 6G*, 2021.

### Option 3 — SDF: Sensing Data Function (`sdf.rs`)

A 6G-new NF with **no 5G equivalent**. Bridges ISAC RAN sensing results into the core as a subscription service:

```rust
// Application subscribes to detections from cell 1 within 500 m:
core.sdf.subscribe(NodeId(1), Distance::from_m(500.0));

// Session runner publishes after each ISAC radar sweep:
let n_notified = core.sdf.publish(&DetectionEvent {
    cell_id: NodeId(1),
    range: Distance::from_m(200.0),
    velocity: Velocity::from_m_per_s(30.0),
    ue_id: Some(UeId(42)),
});
```

The SDF does not depend on `6g-isac` directly — the session runner bridges them using `6g-common` types, preserving the dep-graph contract.

**Reference**: 3GPP TR 22.837; Nokia Bell Labs *Sensing as a Service in 6G*, 2021.

### Option 4 — NTN-Aware AMF (`amf.rs`)

`TrackingArea` replaces the flat `u32` TAC:

```rust
// Terrestrial UE:
core.register_ue(ue, 2001);  // backward-compat; uses TrackingArea::Terrestrial

// NTN (LEO-served) UE:
core.register_ue_ntn(ue, ntn_node_id: 42, beam_id: 3, Duration::from_ms(1.83));
```

`amf.ntn_ue_count()` and `amf.tracking_area(ue)` let the session runner pre-trigger handover before the satellite pass ends, using the NTN handover manager in `6g-ntn`.

**Reference**: 3GPP TR 38.821; Nokia Bell Labs NTN Architecture White Paper, 2022.

### Option 5 — NRF Capability Graph (`nrf.rs`)

In 5G, NFs register by type. In 6G, NFs register **capabilities**:

```rust
nrf.register(NfProfile::with_capabilities(
    NodeId(1), NfType::Smf, 80,
    vec![NfCapability::SemanticSession, NfCapability::NtnHandover],
));

// Capability-based discovery — returns all active NFs supporting SemanticSession,
// ordered by capacity desc:
let smfs = nrf.discover_by_capability(NfCapability::SemanticSession);
```

`NfCapability` variants: `SubTHz`, `NtnHandover`, `SemanticSession`, `IsacProcessing`, `PostQuantumAuth`, `InNetworkAi`, `SensingExposure`.

**Reference**: Nokia Bell Labs *6G SBA as a Knowledge Graph*, 2022.

---

## Service-Based Architecture v2 (`sba_v2.rs`)

SBAv2 collapses ≥ 4 NAS round trips into 1 RTT via inline token auth:

| Step | 5G NAS | SBAv2 |
|---|---|---|
| UE→AMF | Registration Request | First data PDU + `ServiceToken` |
| AMF→AUSF | Authentication Request | (eliminated) |
| **Round trips** | **≥ 4** | **1** |

Key types:

| Type | Role |
|---|---|
| `SbaRegistration` | Per-UE record: holds `ue`, `token`, `validated` flag, and `registered_at` timestamp. Never mutated after initial validation — audit trail is preserved even after `deregister()`. |
| `SbaV2Registry` | Registry holding all `SbaRegistration` records. `register_with_token(ue, token)` — validates the token and marks the record active. `deregister(ue)` — marks inactive, retains record. `validated_ue_count()` — active registrations. `registration_count()` — all records including deregistered. |
| `SbaV2Validation` | `Validate` impl: checks token derivation determinism and round-trip register/deregister consistency. |

`CoreNetwork::register_ue(ue, tracking_area)` — SBAv2 1-RTT flow.  
`CoreNetwork::register_ue_ntn(ue, ntn_node_id, beam_id, propagation_delay)` — NTN variant.

## `SessionGrant` — Session Establishment Result

`SessionGrant` is the success value returned by `CoreNetwork::establish_session()`. It bundles everything the caller needs to use a newly created PDU session:

| Field | Type | Description |
|---|---|---|
| `session_id` | `u8` | SMF-assigned session identifier |
| `ip_addr` | `Ipv4Addr` | UPF-allocated IPv4 address for this session |
| `slice` | `SliceType` | Network slice selected by NSSF |
| `qci` | `u8` | QCI from PCF policy |
| `gbr` | `Bitrate` | Guaranteed bit rate from PCF |

## `CoreNetwork` Orchestrator Methods

| Method | Description |
|---|---|
| `register_ue(ue, tac)` | SBAv2 1-RTT auth + AMF + DigitalTwin |
| `register_ue_ntn(ue, ntn_node_id, beam_id, delay)` | NTN-aware variant |
| `establish_session(ue, slice, pdu_type)` | NSSF→SMF→UPF→PCF chain; registers session in UPF for lazy lookup |
| `release_session(session_id)` | SMF + UPF bearer teardown |
| `deregister_ue(ue)` | Full AMF+SMF+UPF+SBAv2 teardown |

## `GnbNode` — gNB proxy bridging RAN layers to the Core (`gnb.rs`)

Wires `RrcLayer` and `PdcpEntity` to N2/N3 stubs calling `Amf` and `Upf`. Key methods: `attach(ue)`, `detach(ue)`, `forward_to_amf(ue, nas)`, `forward_uplink(payload, upf)`.

## NTN Handover (`crates/6g-ntn/src/handover.rs`)

LEO → terrestrial handover manager. Trigger conditions: better terrestrial RSRP ≥ 3 dB, LEO delay > 5 ms, satellite elevation < 10°.

## Digital Twin (`digital_twin.rs`)

The Digital Twin gives the core a real-time self-model: it snapshots the network state each registration/session event and computes incremental diffs for anomaly detection and AI-native policy decisions.

| Type | Role |
|---|---|
| `UeSnapshot` | Per-UE state at one point in time: `ue_id`, `tracking_area` (as `u32` TAC), `session_count`. |
| `NetworkSnapshot` | Full network state at one sequence number: collection of `UeSnapshot`s plus per-slice load percentages (`s_nssai → load_pct`). `add_ue()` / `set_slice_load()` build the snapshot incrementally. |
| `SnapshotDiff` | Delta between two consecutive `NetworkSnapshot`s: `added_ues`, `removed_ues`, `load_changes`. `is_empty()` returns `true` when nothing changed. |
| `DigitalTwinValidation` | `Validate` impl: checks that snapshotting a single UE produces a non-empty diff, and that a second identical snapshot produces an empty diff. |

## AUSF / UDM Validation and NRF / SDF Validation

Each new Phase 5/6 NF exports a `Validate` impl so CI can exercise known-good numerical / state checks:

| Type | Module | What it checks |
|---|---|---|
| `AusfValidation` | `ausf.rs` | `SubscriberCredential::new` → `initiate_auth` → `verify_response` round-trip succeeds; wrong response is rejected. |
| `NrfValidation` | `nrf.rs` | Register two NF profiles → discover by type returns both → deregister one → active_count decrements; capability query returns matching NFs only. |
| `SdfValidation` | `sdf.rs` | Subscribe with a 500 m range → publish a 200 m event delivers to subscriber; publish a 600 m event does not. |

## References

- 3GPP TS 23.501 (5GC system architecture — baseline)
- 3GPP TS 29.510 (NRF NF discovery service)
- 3GPP TR 38.821 (NTN support for 5G)
- 3GPP TR 22.837 (Integrated Sensing and Communication)
- Qualcomm, *Rethinking the Control Plane* (6G Foundry Series)
- Nokia Bell Labs, *User-Plane-First Architecture for 6G* (2021)
- Nokia Bell Labs, *6G SBA as a Knowledge Graph* (2022)
- Nokia Bell Labs, *Sensing as a Service in 6G* (2021)
- Qin et al., *Semantic Communications: Principles and Challenges*, IEEE JSAC 2022

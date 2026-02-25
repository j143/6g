# `6g-rrc` — Radio Resource Control Layer

## Purpose

RRC manages the connection lifecycle between the UE and the network: establishment, reconfiguration, mobility, and release. 6G proposals simplify RRC by reducing the number of state transitions and moving more decisions into the AI engine. Entry point: `RrcLayer`.

## Invariants

<!-- Things that must ALWAYS be true, regardless of changes -->
- `RrcState` transitions are strictly: `Idle → Inactive → Connected` (forward) and `Connected → Inactive → Idle` (backward). No state skips allowed.
- `UeContext` is created on `RrcState::Connected` entry and destroyed on `RrcState::Idle` entry.
- `RrcLayer` is the only component that modifies `RrcState` — no other crate changes UE RRC state directly.

## Key Types

- `RrcState` — Idle, Inactive, Connected
- `UeContext` — per-UE state during an active RRC connection
- `RrcLayer` — crate entry point managing all UE contexts

## States

```
        ┌──────────┐
        │   Idle   │  ← No RRC connection; UE camped on cell
        └────┬─────┘
             │ RRC Setup Request
        ┌────▼─────┐
        │ Inactive │  ← Lightweight suspended state (new in NR, prominent in 6G)
        └────┬─────┘
             │ RRC Resume / RRC Setup
        ┌────▼──────┐
        │ Connected │  ← Active data transfer
        └───────────┘
```

The `Inactive` state is a key 6G efficiency feature: the UE retains the AS context while the RRC connection is suspended, enabling fast resumption without full re-establishment.

## What This Crate Does NOT Do

- Does not implement MAC or PHY — connection setup triggers those layers separately.
- Does not implement NAS (non-access stratum) — NAS is handled by `6g-core` (AMF).
- Does not store persistent UE subscription data — that is UDM's responsibility.

## 6G Simplification Hypothesis

5G RRC involves >20 message types for setup alone (RRC Setup Request, RRC Setup, RRC Setup Complete, Registration Request over NAS, …). The 6G research hypothesis is that a **user-plane-first** architecture can achieve the same result in fewer round trips by moving authentication/session establishment into a lightweight data-path header.

This crate will implement both the 5G-inherited flow and the simplified 6G flow so they can be compared directly.

## SIB Broadcasting

System Information Blocks (SIBs) carry cell configuration. The scheduler module controls SIB periodicity. 6G reduces the number of mandatory SIBs by making more configuration on-demand (on-demand SI).

## References

- 3GPP TS 38.331 (NR RRC — 6G baseline)
- Qualcomm, *Rethinking the Control Plane* (6G Foundry Series)

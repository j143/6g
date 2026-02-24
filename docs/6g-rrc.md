# `6g-rrc` — Radio Resource Control Layer

## Purpose

RRC manages the connection lifecycle between the UE and the network: establishment, reconfiguration, mobility, and release. 6G proposals simplify RRC by reducing the number of state transitions and moving more decisions into the AI engine.

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

## 6G Simplification Hypothesis

5G RRC involves >20 message types for setup alone (RRC Setup Request, RRC Setup, RRC Setup Complete, Registration Request over NAS, …). The 6G research hypothesis is that a **user-plane-first** architecture can achieve the same result in fewer round trips by moving authentication/session establishment into a lightweight data-path header.

This crate will implement both the 5G-inherited flow and the simplified 6G flow so they can be compared directly.

## SIB Broadcasting

System Information Blocks (SIBs) carry cell configuration. The scheduler module controls SIB periodicity. 6G reduces the number of mandatory SIBs by making more configuration on-demand (on-demand SI).

## References

- 3GPP TS 38.331 (NR RRC — 6G baseline)
- Qualcomm, *Rethinking the Control Plane* (6G Foundry Series)

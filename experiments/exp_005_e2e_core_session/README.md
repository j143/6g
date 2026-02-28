# Experiment 005 — End-to-End Core Session

## Hypothesis
The individual 6G crate layers (RRC, PDCP, AMF, SMF, UPF) can be sequenced into
a complete attach + data-plane flow that mirrors the 5G call flow:
`RRCSetupRequest → Registration → Auth → PDU Session → UPF uplink`.

## Method
Instantiate `GnbNode`, `Amf`, `Smf`, and `Upf`.  Drive them through the five
steps documented in the issue audit:

1. `gnb.attach(ue)` — RRC state machine: Idle → Connected
2. `gnb.forward_to_amf()` — N2 NAS stub returns byte count
3. `amf.register()` + `amf.authenticate()` — RegistrationRecord stored
4. `smf.establish_session()` — PduSession ID assigned
5. `gnb.forward_uplink()` — PDCP header compression + UPF byte accumulation

## Expected Result
- UE RRC state = `Connected` after step 1
- N2 stub echoes the NAS payload length
- `amf.registered_ue_count()` = 1
- `smf` returns `session_id` > 0
- `upf.stats.bytes_uplink` > 0 (PDCP overhead means PDU > raw payload)

## Reference
3GPP TS 38.331 (RRC), TS 38.323 (PDCP), TS 23.502 (5G procedures — baseline
for 6G attach flow), gnb.rs `gnb_attach_and_uplink_flow` test.

# `6g-ntn` — Non-Terrestrial Networks

## Purpose

NTN support is a native 6G feature (not a bolt-on as in 5G Rel-17). `6g-ntn` models LEO satellites, HAPS (High-Altitude Platform Stations), and UAVs as first-class network nodes with propagation delay, Doppler compensation, and handover procedures. Entry point: `NtnLayer`.

## Invariants

<!-- Things that must ALWAYS be true, regardless of changes -->
- `NtnNodeType` determines propagation delay; do not compute delay independently of this enum.
- `NtnNode` always carries a `Position3D` in metres (WGS-84 or local frame) from `6g-common`.
- Doppler shift formula: `f_d = (v/c) × f_carrier` — do not alter without updating tests.

## Node Types

Key types: `NtnNodeType`, `NtnNode`, `NtnLayer`.

| Type | Altitude | Propagation delay (one-way) | Orbital period |
|---|---|---|---|
| LEO | 500–2000 km | ~1.7–6.7 ms | ~90–120 min |
| MEO | 2000–35786 km | ~7–120 ms | ~2–24 hr |
| GEO | 35786 km | ~238 ms | 24 hr (geostationary) |
| HAPS | 20–50 km (stratosphere) | ~67–167 µs | Quasi-stationary |
| UAV | 0.1–10 km | < 33 µs | Mission-dependent |

## Key Engineering Challenges

### Doppler Compensation

LEO satellite at 7.5 km/s induces up to ±120 kHz Doppler at 150 GHz (sub-THz). The PHY layer must pre-compensate at the transmitter (downlink) or the UE must apply Doppler correction (uplink). OTFS waveform (see `6g-phy`) handles Doppler naturally in the delay-Doppler domain.

### Timing Advance

Propagation delay variation across a LEO pass can be 10s of ms. The MAC timing advance must track this continuously.

### LEO Handover

A LEO satellite moves out of view every ~10 minutes. The NTN handover procedure must:
1. Predict the handover time from ephemeris data.
2. Pre-configure the target satellite's air interface.
3. Execute seamless handover before coverage loss.

Target (Phase 4): handover latency < 50 ms for a 500 km LEO orbit.

## What This Crate Does NOT Do

- Does not implement waveform or channel models — import from `6g-phy`.
- Does not implement MAC scheduling — see `6g-mac`.
- Does not depend on any crate other than `6g-common`.

## References

- 3GPP TS 38.821 (NR NTN solutions)
- Samsung 6G White Paper §4.3 (NTN as native 6G feature)

# `6g-common` — Shared Types and Configuration

## Purpose

`6g-common` is the foundational crate of the workspace. It provides the shared type vocabulary used by every other crate. All cross-crate API boundaries are expressed in terms of the types defined here.

## Design Decisions

### `Frequency`

Represents a frequency value in Hz with a `f64` internal representation. Covers the full 6G spectrum from Sub-6 GHz to THz (up to 3 THz). Provides constructors `from_hz`, `from_ghz`, `from_thz` and accessors `as_hz`, `as_ghz`, `as_thz`. The `band()` method classifies a frequency into a `FrequencyBand` variant using the boundaries:

| Band | Range |
|---|---|
| `SubSixGhz` | < 6 GHz |
| `MidBand` | 6–24 GHz |
| `MmWave` | 24–100 GHz |
| `SubThz` | 100–300 GHz |
| `Thz` | > 300 GHz |

### `Position3D`

Three-dimensional Cartesian coordinates in metres. Used by `6g-ntn` for satellite/HAPS/UAV positions and by `6g-phy` for near-field channel geometry.

### `BearerId` / `SliceId`

Typed wrappers over primitive integers to prevent accidental mixing of bearer and slice identifiers at compile time.

### `Payload`

Type alias for `Vec<u8>`. All protocol data units flow as byte vectors; upper layers add semantic structure.

## What This Is Not

- No I/O, no async, no network sockets.
- No feature-gated code — this crate must compile on all targets without optional features.

## References

- ITU-R M.2160 §5 (performance targets drive the `SystemConfig` defaults)

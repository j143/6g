# `6g-common` — Shared Types and Configuration

## Purpose

`6g-common` is the foundational crate of the workspace. It provides the shared type vocabulary used by every other crate. All cross-crate API boundaries are expressed in terms of the types defined here.

## Invariants

<!-- Things that must ALWAYS be true, regardless of changes -->
- `Frequency` stores Hz internally as `f64`; constructors and accessors are lossless for values up to 3 THz.
- `FrequencyBand` boundaries are fixed: Sub-6 GHz < 6 GHz, MidBand 6–24 GHz, MmWave 24–100 GHz, SubThz 100–300 GHz, Thz ≥ 300 GHz.
- `SystemConfig` is the **single** feature-flag registry. Every optional subsystem is gated through it — never through ad-hoc booleans in individual crates.
- `ValidationCheck::new` automatically computes `passed` from `|actual − expected| / |expected| × 100 ≤ tolerance_pct`.
- `Payload` is always `Vec<u8>` — no compression or encoding at this layer.

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

### `UeId` / `NodeId` / `BearerId` / `SliceId`

Typed wrappers over primitive integers to prevent accidental mixing of bearer and slice identifiers at compile time. `UeId` identifies a User Equipment, `NodeId` identifies a base station (gNB/TRP), `BearerId` identifies a radio bearer, `SliceId` identifies a network slice.

### `SnrDb`

Signal-to-Noise Ratio stored as a `f64` dB value. Use this instead of bare `f64` at API boundaries where SNR is a meaningful physical quantity.

### `SnrLinear`

Linear (not dB) signal-to-noise ratio P_signal / P_noise (dimensionless ratio). Used in RIS channel models and ISAC detection functions where the linear scale is more natural than dB.

### `Distance`

Physical distance in metres. Use `Distance::from_m(x)` to construct and `.as_m()` to extract. Required at all `pub fn` boundaries where a distance parameter appears.

### `PowerDb`

Power, gain, or loss in decibels (dB or dBm depending on context). Covers path loss, transmit power, noise figure, and SNR gain. Use `PowerDb::new(db)` / `.as_db()`.

### `Bandwidth`

Signal bandwidth in hertz (Hz). Constructors: `from_hz`, `from_mhz`, `from_ghz`. Accessor: `as_hz()`. Distinct from `Frequency` even though they share units — bandwidth is a spectral width, not a carrier position.

### `Velocity`

Velocity in metres per second (m/s). Used in Doppler shift calculations in `6g-isac/detection.rs`. Constructor: `Velocity::from_m_per_s(x)`. Accessor: `as_m_per_s()`.

### `Payload`

Type alias for `Vec<u8>`. All protocol data units flow as byte vectors; upper layers add semantic structure.

### `ValidationResult` / `ValidationCheck` / `Validate`

Structured numerical self-validation framework. Every physics module implements the `Validate` trait so that CI can verify known-good numerical results automatically. See `validation.rs`.

## Public API Contract

- `Frequency::from_hz(hz: f64) -> Frequency` — wrap a raw Hz value
- `Frequency::from_ghz(ghz: f64) -> Frequency` — convert GHz → Hz internally
- `Frequency::from_thz(thz: f64) -> Frequency` — convert THz → Hz internally
- `Frequency::band(self) -> FrequencyBand` — classify by spectrum band
- `SystemConfig::default() -> SystemConfig` — returns a 6G-ready default config
- `ValidationCheck::new(name, actual, expected, tolerance_pct) -> ValidationCheck`
- `ValidationResult::passed(&self) -> bool`
- `ValidationResult::summary(&self) -> String`

## What This Crate Does NOT Do

- No I/O, no async, no network sockets.
- No feature-gated code — this crate must compile on all targets without optional features.
- No physics calculations — those belong in `6g-phy`, `6g-isac`, etc.
- No protocol logic of any kind.

## References

- ITU-R M.2160 §5 (performance targets drive the `SystemConfig` defaults)

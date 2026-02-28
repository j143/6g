//! Fundamental types used throughout the 6G stack.

use serde::{Deserialize, Serialize};

/// Frequency value in Hz, covering the full 6G spectrum up to THz range.
///
/// Use the convenience constructors (`from_ghz`, `from_thz`) to create values.
/// The internal representation is Hz stored as `f64`.
///
/// # Examples
/// ```
/// use sixg_common::types::Frequency;
/// let f = Frequency::from_ghz(150.0); // 150 GHz (Sub-THz band)
/// assert!((f.as_hz() - 150e9).abs() < 1.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Frequency(f64); // Hz

impl Frequency {
    /// Create a `Frequency` from a value in hertz.
    pub fn from_hz(hz: f64) -> Self {
        Self(hz)
    }

    /// Create a `Frequency` from a value in gigahertz (GHz).
    pub fn from_ghz(ghz: f64) -> Self {
        Self(ghz * 1e9)
    }

    /// Create a `Frequency` from a value in terahertz (THz).
    pub fn from_thz(thz: f64) -> Self {
        Self(thz * 1e12)
    }

    /// Return the frequency in hertz.
    pub fn as_hz(self) -> f64 {
        self.0
    }

    /// Return the frequency in gigahertz.
    pub fn as_ghz(self) -> f64 {
        self.0 / 1e9
    }

    /// Return the frequency in terahertz.
    pub fn as_thz(self) -> f64 {
        self.0 / 1e12
    }

    /// Classify this frequency into a [`FrequencyBand`].
    pub fn band(self) -> FrequencyBand {
        let ghz = self.as_ghz();
        if ghz < 6.0 {
            FrequencyBand::SubSixGhz
        } else if ghz < 24.0 {
            FrequencyBand::MidBand
        } else if ghz < 100.0 {
            FrequencyBand::MmWave
        } else if ghz < 300.0 {
            FrequencyBand::SubThz
        } else {
            FrequencyBand::Thz
        }
    }
}

/// Frequency band classification for 6G spectrum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrequencyBand {
    /// Sub-6 GHz (FR1 extended)
    SubSixGhz,
    /// Mid-band: 7–24 GHz
    MidBand,
    /// mmWave: 24–100 GHz
    MmWave,
    /// Sub-THz: 100–300 GHz
    SubThz,
    /// THz: 300 GHz – 3 THz
    Thz,
}

/// Physical distance in metres.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Distance(f64); // metres

impl Distance {
    /// Create a `Distance` from a value in metres.
    pub fn from_m(m: f64) -> Self {
        Self(m)
    }

    /// Return the distance in metres.
    pub fn as_m(self) -> f64 {
        self.0
    }
}

/// Power, gain or loss value in decibels (dB or dBm depending on context).
///
/// Used for path loss (dB), transmit power (dBm), noise figure (dB),
/// and SNR gain (dB) throughout the 6G PHY stack.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct PowerDb(f64); // dB

impl PowerDb {
    /// Wrap a raw dB value.
    pub fn new(db: f64) -> Self {
        Self(db)
    }

    /// Return the raw dB value.
    pub fn as_db(self) -> f64 {
        self.0
    }
}

/// Signal bandwidth in hertz (Hz).
///
/// Use `from_hz`, `from_mhz`, or `from_ghz` constructors and `as_hz`
/// to retrieve the underlying value.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Bandwidth(f64); // Hz

impl Bandwidth {
    /// Create a `Bandwidth` from a value in hertz.
    pub fn from_hz(hz: f64) -> Self {
        Self(hz)
    }

    /// Create a `Bandwidth` from a value in megahertz.
    pub fn from_mhz(mhz: f64) -> Self {
        Self(mhz * 1e6)
    }

    /// Create a `Bandwidth` from a value in gigahertz.
    pub fn from_ghz(ghz: f64) -> Self {
        Self(ghz * 1e9)
    }

    /// Return the bandwidth in hertz.
    pub fn as_hz(self) -> f64 {
        self.0
    }
}

/// Velocity in metres per second (m/s).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Velocity(f64); // m/s

impl Velocity {
    /// Create a `Velocity` from a value in metres per second.
    pub fn from_m_per_s(mps: f64) -> Self {
        Self(mps)
    }

    /// Return the velocity in metres per second.
    pub fn as_m_per_s(self) -> f64 {
        self.0
    }
}

/// Time duration in milliseconds (ms).
///
/// Use this at API boundaries where a delay, latency, or timer interval is
/// expressed as a physical time quantity.  Use `from_ms` / `from_s` to
/// construct values and `as_ms` / `as_s` to read them.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Duration(f64); // milliseconds

impl Duration {
    /// Create a `Duration` from a value in milliseconds.
    pub fn from_ms(ms: f64) -> Self {
        Self(ms)
    }

    /// Create a `Duration` from a value in seconds.
    pub fn from_s(s: f64) -> Self {
        Self(s * 1_000.0)
    }

    /// Return the duration in milliseconds.
    pub fn as_ms(self) -> f64 {
        self.0
    }

    /// Return the duration in seconds.
    pub fn as_s(self) -> f64 {
        self.0 / 1_000.0
    }
}

/// Linear (not dB) signal-to-noise ratio: P_signal / P_noise (dimensionless).
///
/// Use this at API boundaries where the SNR is expressed as a linear ratio
/// rather than in dB. See also [`SnrDb`] for the dB representation.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct SnrLinear(f64);

impl SnrLinear {
    /// Wrap a raw linear SNR value.
    pub fn new(linear: f64) -> Self {
        Self(linear)
    }

    /// Return the raw linear SNR value.
    pub fn as_linear(self) -> f64 {
        self.0
    }
}

/// Unique identifier for a User Equipment (UE).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UeId(pub u64);

/// Unique identifier for a base station (gNB / TRP).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

/// Radio bearer identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BearerId(pub u8);

/// Network slice identifier (S-NSSAI / slice index).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SliceId(pub u16);

/// Bit rate in bits per second (bps).
///
/// Use the convenience constructors (`from_kbps`, `from_mbps`, `from_gbps`) so
/// that call sites are self-documenting.  Internal representation is bps stored
/// as `f64`.
///
/// # Examples
/// ```
/// use sixg_common::types::Bitrate;
/// let r = Bitrate::from_mbps(100.0); // 100 Mbps downlink
/// assert!((r.as_kbps() - 100_000.0).abs() < 1.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Bitrate(f64); // bits per second

impl Bitrate {
    /// Create a `Bitrate` from a value in bits per second.
    pub fn from_bps(bps: f64) -> Self {
        Self(bps)
    }

    /// Create a `Bitrate` from a value in kilobits per second (kbps).
    pub fn from_kbps(kbps: f64) -> Self {
        Self(kbps * 1_000.0)
    }

    /// Create a `Bitrate` from a value in megabits per second (Mbps).
    pub fn from_mbps(mbps: f64) -> Self {
        Self(mbps * 1_000_000.0)
    }

    /// Create a `Bitrate` from a value in gigabits per second (Gbps).
    pub fn from_gbps(gbps: f64) -> Self {
        Self(gbps * 1_000_000_000.0)
    }

    /// Return the bit rate in bits per second.
    pub fn as_bps(self) -> f64 {
        self.0
    }

    /// Return the bit rate in kilobits per second.
    pub fn as_kbps(self) -> f64 {
        self.0 / 1_000.0
    }

    /// Return the bit rate in megabits per second.
    pub fn as_mbps(self) -> f64 {
        self.0 / 1_000_000.0
    }

    /// Return the bit rate in gigabits per second.
    pub fn as_gbps(self) -> f64 {
        self.0 / 1_000_000_000.0
    }
}

/// A raw byte payload.
pub type Payload = Vec<u8>;

/// Signal-to-Noise Ratio in dB.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SnrDb(pub f64);

/// Coordinates in 3D space (metres, WGS-84 or local frame).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Position3D {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_thz_round_trip() {
        let f = Frequency::from_thz(1.0);
        assert!((f.as_thz() - 1.0).abs() < 1e-9);
        assert!((f.as_hz() - 1e12).abs() < 1.0);
    }

    #[test]
    fn frequency_ghz_round_trip() {
        let f = Frequency::from_ghz(150.0);
        assert!((f.as_ghz() - 150.0).abs() < 1e-6);
    }

    #[test]
    fn frequency_band_classification() {
        assert_eq!(Frequency::from_ghz(3.5).band(), FrequencyBand::SubSixGhz);
        assert_eq!(Frequency::from_ghz(15.0).band(), FrequencyBand::MidBand);
        assert_eq!(Frequency::from_ghz(60.0).band(), FrequencyBand::MmWave);
        assert_eq!(Frequency::from_ghz(150.0).band(), FrequencyBand::SubThz);
        assert_eq!(Frequency::from_thz(1.0).band(), FrequencyBand::Thz);
    }

    #[test]
    fn slice_id_distinct() {
        let s1 = SliceId(1);
        let s2 = SliceId(2);
        assert_ne!(s1, s2);
    }

    #[test]
    fn distance_round_trip() {
        let d = Distance::from_m(100.0);
        assert!((d.as_m() - 100.0).abs() < 1e-10);
    }

    #[test]
    fn power_db_round_trip() {
        let p = PowerDb::new(-30.0);
        assert!((p.as_db() - -30.0).abs() < 1e-10);
    }

    #[test]
    fn bandwidth_round_trip() {
        let b = Bandwidth::from_ghz(1.0);
        assert!((b.as_hz() - 1e9).abs() < 1.0);
    }

    #[test]
    fn velocity_round_trip() {
        let v = Velocity::from_m_per_s(7500.0);
        assert!((v.as_m_per_s() - 7500.0).abs() < 1e-10);
    }

    #[test]
    fn snr_linear_round_trip() {
        let s = SnrLinear::new(100.0);
        assert!((s.as_linear() - 100.0).abs() < 1e-10);
    }

    #[test]
    fn bitrate_round_trips() {
        let r = Bitrate::from_mbps(100.0);
        assert!((r.as_mbps() - 100.0).abs() < 1e-6, "Mbps round-trip");
        assert!((r.as_kbps() - 100_000.0).abs() < 0.1, "kbps from Mbps");
        assert!((r.as_gbps() - 0.1).abs() < 1e-9, "Gbps from Mbps");

        let zero = Bitrate::from_kbps(0.0);
        assert_eq!(zero.as_bps(), 0.0, "zero bitrate");

        let gbps = Bitrate::from_gbps(1.0);
        assert!((gbps.as_mbps() - 1_000.0).abs() < 1e-3, "Gbps to Mbps");
    }
}

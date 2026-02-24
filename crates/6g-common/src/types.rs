//! Fundamental types used throughout the 6G stack.

use serde::{Deserialize, Serialize};

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

/// Unique identifier for a User Equipment (UE).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UeId(pub u64);

/// Unique identifier for a base station (gNB / TRP).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

/// Radio bearer identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BearerId(pub u8);

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

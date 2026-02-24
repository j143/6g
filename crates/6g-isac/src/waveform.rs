//! ISAC waveform design.
//!
//! The ISAC waveform must balance communication and sensing performance.
//! OFDM-based designs (DFRC – Dual-Function Radar Communications) are the
//! leading candidates for sub-THz 6G deployments.

use serde::{Deserialize, Serialize};

/// ISAC waveform type.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsacWaveform {
    /// OFDM-based Dual-Function Radar Communications.
    #[default]
    Dfrc,
    /// OTFS-based joint sensing and communication.
    OtfsIsac,
    /// AI-optimised joint waveform.
    AiOptimised,
}

//! Integrated Sensing and Communication (ISAC).
//!
//! 6G unifies wireless communication and radar sensing into a single
//! waveform and hardware platform. Key use-cases:
//! * Vehicular / infrastructure sensing
//! * Gesture recognition and indoor positioning
//! * Environment mapping for beam management
//! * Simultaneous Localisation and Mapping (SLAM)

pub mod detection;
pub mod dfrc;
pub mod sensing;
pub mod waveform;

pub use detection::{pd_from_pfa, RangeDopplerMap};
pub use dfrc::{DfrcConfig, DfrcValidation, ParetoPoint};
pub use sensing::{SensingResult, SensingTask};
pub use waveform::IsacWaveform;

/// ISAC layer entry point.
pub struct IsacLayer {
    waveform: IsacWaveform,
    dfrc: DfrcConfig,
}

impl IsacLayer {
    pub fn new() -> Self {
        Self {
            waveform: IsacWaveform::default(),
            // Default: 1 GHz bandwidth, 20 dB SNR, 64/256 sensing subcarriers
            dfrc: DfrcConfig::new(100.0, 1e9, 64, 256),
        }
    }

    pub fn waveform(&self) -> &IsacWaveform {
        &self.waveform
    }

    pub fn dfrc(&self) -> &DfrcConfig {
        &self.dfrc
    }

    /// Execute a sensing task and return a placeholder result.
    pub fn sense(&self, task: SensingTask) -> SensingResult {
        SensingResult::stub(task)
    }
}

impl Default for IsacLayer {
    fn default() -> Self {
        Self::new()
    }
}

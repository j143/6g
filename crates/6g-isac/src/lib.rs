//! Integrated Sensing and Communication (ISAC).
//!
//! 6G unifies wireless communication and radar sensing into a single
//! waveform and hardware platform. Key use-cases:
//! * Vehicular / infrastructure sensing
//! * Gesture recognition and indoor positioning
//! * Environment mapping for beam management
//! * Simultaneous Localisation and Mapping (SLAM)

pub mod sensing;
pub mod waveform;

pub use sensing::{SensingResult, SensingTask};
pub use waveform::IsacWaveform;

/// ISAC layer entry point.
pub struct IsacLayer {
    waveform: IsacWaveform,
}

impl IsacLayer {
    pub fn new() -> Self {
        Self {
            waveform: IsacWaveform::default(),
        }
    }

    pub fn waveform(&self) -> &IsacWaveform {
        &self.waveform
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

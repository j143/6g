//! Reconfigurable Intelligent Surfaces (RIS).
//!
//! RIS are passive (or semi-passive) surfaces made of programmable
//! meta-material elements that can alter the phase and amplitude of
//! incoming electromagnetic waves to create "smart" propagation
//! environments without active RF chains.

use serde::{Deserialize, Serialize};

/// Phase-shift resolution of each RIS element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseResolution {
    /// 1-bit: two discrete phase states (0°, 180°).
    OneBit,
    /// 2-bit: four discrete phase states.
    TwoBit,
    /// Continuous phase control.
    Continuous,
}

/// Configuration for a single RIS panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RisConfig {
    /// Number of RIS reflecting elements.
    pub num_elements: usize,
    /// Number of rows in the RIS panel.
    pub rows: usize,
    /// Number of columns in the RIS panel.
    pub columns: usize,
    /// Phase-shift resolution per element.
    pub phase_resolution: PhaseResolution,
    /// Whether the RIS has active sensing capabilities (active RIS).
    pub active_sensing: bool,
}

impl Default for RisConfig {
    fn default() -> Self {
        Self {
            num_elements: 256,
            rows: 16,
            columns: 16,
            phase_resolution: PhaseResolution::TwoBit,
            active_sensing: false,
        }
    }
}

impl RisConfig {
    /// Return the phase-shift codebook size (number of distinct states).
    pub fn codebook_size(&self) -> usize {
        match self.phase_resolution {
            PhaseResolution::OneBit => 2_usize.pow(1),
            PhaseResolution::TwoBit => 2_usize.pow(2),
            PhaseResolution::Continuous => usize::MAX,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_bit_codebook_has_four_entries() {
        let ris = RisConfig::default();
        assert_eq!(ris.codebook_size(), 4);
    }
}

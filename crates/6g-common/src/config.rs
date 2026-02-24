//! System-wide configuration for the 6G stack.

use crate::types::FrequencyBand;
use serde::{Deserialize, Serialize};

/// Top-level system configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// Primary frequency band.
    pub frequency_band: FrequencyBand,
    /// Maximum number of simultaneously served UEs.
    pub max_ues: usize,
    /// Number of transmit/receive antenna elements at the base station.
    pub antenna_elements: usize,
    /// Enable AI-native air interface.
    pub ai_native_enabled: bool,
    /// Enable Integrated Sensing and Communication.
    pub isac_enabled: bool,
    /// Enable Non-Terrestrial Network integration.
    pub ntn_enabled: bool,
    /// Enable Semantic Communications.
    pub semantic_enabled: bool,
    /// Target energy efficiency in Mb/J.
    pub target_energy_efficiency_mb_per_j: f64,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            frequency_band: FrequencyBand::SubThz,
            max_ues: 1_000_000,
            antenna_elements: 1024,
            ai_native_enabled: true,
            isac_enabled: true,
            ntn_enabled: true,
            semantic_enabled: true,
            target_energy_efficiency_mb_per_j: 1000.0,
        }
    }
}

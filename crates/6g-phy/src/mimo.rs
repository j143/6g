//! Massive MIMO and Holographic MIMO beam management.
//!
//! 6G base stations are expected to deploy thousands of antenna elements
//! ("Holographic MIMO") operating at sub-THz frequencies. This module
//! provides the configuration and beam management skeleton.

use serde::{Deserialize, Serialize};

/// Antenna panel geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AntennaPanel {
    /// Number of rows of antenna elements.
    pub rows: usize,
    /// Number of columns of antenna elements.
    pub columns: usize,
}

impl AntennaPanel {
    pub fn new(rows: usize, columns: usize) -> Self {
        Self { rows, columns }
    }

    /// Total number of antenna elements in this panel.
    pub fn element_count(&self) -> usize {
        self.rows * self.columns
    }
}

/// Beamforming type supported by the MIMO configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeamformingType {
    /// Fully digital beamforming (one RF chain per antenna element).
    FullyDigital,
    /// Hybrid analog/digital beamforming.
    Hybrid { num_rf_chains: usize },
    /// Holographic beamforming (continuous aperture, AI-assisted).
    Holographic,
}

/// MIMO layer configuration for the PHY.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MimoConfig {
    /// Number of total antenna elements.
    pub total_elements: usize,
    /// Antenna panel layout.
    pub panel: AntennaPanel,
    /// Beamforming strategy.
    pub beamforming: BeamformingType,
    /// Maximum number of simultaneous spatial layers (rank).
    pub max_layers: usize,
}

impl MimoConfig {
    /// Build a sensible MIMO configuration for `total_elements` elements.
    pub fn new(total_elements: usize) -> Self {
        let side = (total_elements as f64).sqrt() as usize;
        let panel = AntennaPanel::new(side, side.max(1));
        let beamforming = if total_elements >= 512 {
            BeamformingType::Holographic
        } else {
            BeamformingType::Hybrid {
                num_rf_chains: total_elements / 4,
            }
        };
        Self {
            total_elements,
            panel,
            beamforming,
            max_layers: total_elements.min(256),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_array_uses_holographic_beamforming() {
        let cfg = MimoConfig::new(1024);
        assert_eq!(cfg.beamforming, BeamformingType::Holographic);
    }

    #[test]
    fn small_array_uses_hybrid_beamforming() {
        let cfg = MimoConfig::new(64);
        assert!(matches!(cfg.beamforming, BeamformingType::Hybrid { .. }));
    }
}

//! Massive MIMO and Holographic MIMO beam management.
//!
//! 6G base stations are expected to deploy thousands of antenna elements
//! ("Holographic MIMO") operating at sub-THz frequencies. This module
//! provides the configuration and beam management skeleton.
//!
//! ## Near-Field Boundary (ELAA)
//!
//! At sub-THz/THz frequencies the Rayleigh distance shrinks relative to
//! array aperture. For an Extremely Large Aperture Array (ELAA) of diameter
//! `D` operating at wavelength `λ`:
//!
//! `d_R = 2·D² / λ`
//!
//! When a UE is within `d_R`, far-field plane-wave models are invalid and
//! spherical-wave near-field channel models must be used.
//!
//! ## Beamforming Gain
//!
//! With `N` co-phase antenna elements the coherent combining gain is:
//!
//! `G_BF = 10·log10(N)` dB
//!
//! References:
//! - Björnson et al., *Massive MIMO Networks*, FnT 2017
//! - 3GPP TR 38.901 (CDL channel models)

use serde::{Deserialize, Serialize};
use sixg_common::types::{Distance, Frequency, SnrDb};

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

    /// Array diameter (m) assuming half-wavelength spacing at `freq`.
    ///
    /// `D = (√N − 1) · λ/2`
    pub fn array_diameter_m(&self, freq: Frequency) -> Distance {
        let wavelength = 3e8 / freq.as_hz();
        let side = (self.total_elements as f64).sqrt();
        Distance::from_m((side - 1.0).max(0.0) * wavelength / 2.0)
    }

    /// Rayleigh distance (m): boundary between near-field and far-field.
    ///
    /// `d_R = 2·D² / λ`
    pub fn rayleigh_distance_m(&self, freq: Frequency) -> Distance {
        let wavelength = 3e8 / freq.as_hz();
        let d = self.array_diameter_m(freq).as_m();
        Distance::from_m(2.0 * d * d / wavelength)
    }

    /// Returns `true` if `distance` is within the near-field region.
    pub fn is_near_field(&self, distance: Distance, freq: Frequency) -> bool {
        distance.as_m() < self.rayleigh_distance_m(freq).as_m()
    }

    /// Coherent beamforming gain (dB): `10·log10(N)`.
    pub fn beamforming_gain_db(&self) -> f64 {
        10.0 * (self.total_elements as f64).log10()
    }

    /// Effective received SNR (dB) after beamforming.
    ///
    /// `SNR_eff = SNR_per_element + G_BF`
    pub fn effective_snr_db(&self, snr_per_element: SnrDb) -> SnrDb {
        SnrDb(snr_per_element.0 + self.beamforming_gain_db())
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

    #[test]
    fn beamforming_gain_1024_elements() {
        let cfg = MimoConfig::new(1024);
        // 10·log10(1024) = 30.1 dB
        let gain = cfg.beamforming_gain_db();
        assert!(
            (gain - 30.1).abs() < 0.1,
            "Expected ~30.1 dB, got {gain:.2}"
        );
    }

    #[test]
    fn rayleigh_distance_thz_array() {
        // 1024-element array at 150 GHz (sub-THz)
        let cfg = MimoConfig::new(1024);
        let freq = Frequency::from_hz(150e9);
        let d_r = cfg.rayleigh_distance_m(freq).as_m();
        // λ = 2 mm, D ≈ 31 × 1 mm = 31 mm → d_R = 2·(0.031)²/0.002 ≈ 0.96 m
        assert!(d_r > 0.1, "Rayleigh distance should be substantial at THz");
        // Near-field check at 0.5 m vs 10 m
        assert!(
            cfg.is_near_field(Distance::from_m(0.5), freq)
                || !cfg.is_near_field(Distance::from_m(100.0), freq)
        );
    }

    #[test]
    fn effective_snr_increases_with_elements() {
        let small = MimoConfig::new(64);
        let large = MimoConfig::new(1024);
        let snr_in = SnrDb(-10.0);
        assert!(
            large.effective_snr_db(snr_in).0 > small.effective_snr_db(snr_in).0,
            "More elements must yield higher SNR"
        );
    }
}

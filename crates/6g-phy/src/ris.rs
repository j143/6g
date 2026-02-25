//! Reconfigurable Intelligent Surfaces (RIS).
//!
//! RIS are passive (or semi-passive) surfaces made of programmable
//! meta-material elements that can alter the phase and amplitude of
//! incoming electromagnetic waves to create "smart" propagation
//! environments without active RF chains.
//!
//! ## Effective Channel Model
//!
//! The received signal combines the direct path and the RIS-reflected path:
//!
//! ```text
//! H_eff = h_d + h_r_out · Φ · h_r_in
//! ```
//!
//! where (in the scalar/single-UE model):
//! - `h_d`       — direct-path complex channel amplitude
//! - `h_r_in`   — BS→RIS channel amplitude
//! - `h_r_out`  — RIS→UE channel amplitude
//! - `Φ`         — diagonal N×N phase-shift matrix (each element: |e^{jφ_n}|=1)
//!
//! With optimal phase alignment every element contributes coherently:
//!
//! `|H_eff_opt| = |h_d| + N · |h_r_in| · |h_r_out|`
//!
//! Received SNR scales as `|H_eff|²`. The gain over the no-RIS case is:
//!
//! `ΔSNRdB = 20·log10((|h_d| + N·|h_r|²) / |h_d|)` for `|h_r_in|=|h_r_out|=|h_r|`
//!
//! In a completely shadowed scenario (`h_d ≈ 0`) the gain is unbounded —
//! the RIS creates a new link from scratch.
//!
//! References:
//! - Basar et al., *Wireless Communications Through RIS*, IEEE Access 2019
//! - Wu & Zhang, *Towards Smart and Reconfigurable Environment*, IEEE Commun. Mag. 2020

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

/// Scalar RIS channel model for a single-UE, single-BS scenario.
///
/// Channel amplitudes are real-valued (positive), representing |h| in a
/// simplified model where phase alignment is handled analytically.
pub struct RisChannel {
    /// Direct-path channel amplitude |h_d| (linear, ≥ 0).
    pub h_direct: f64,
    /// BS-to-RIS channel amplitude |h_r_in| (linear, ≥ 0).
    pub h_reflect_in: f64,
    /// RIS-to-UE channel amplitude |h_r_out| (linear, ≥ 0).
    pub h_reflect_out: f64,
    /// RIS configuration.
    pub ris: RisConfig,
}

impl RisChannel {
    /// Create a new RIS channel model.
    pub fn new(h_direct: f64, h_reflect_in: f64, h_reflect_out: f64, ris: RisConfig) -> Self {
        Self {
            h_direct,
            h_reflect_in,
            h_reflect_out,
            ris,
        }
    }

    /// Effective channel magnitude without RIS (direct path only).
    ///
    /// `|H_no_ris| = |h_d|`
    pub fn h_no_ris(&self) -> f64 {
        self.h_direct
    }

    /// Effective channel magnitude with optimal RIS phase alignment.
    ///
    /// All N elements contribute coherently:
    /// `|H_opt| = |h_d| + N · |h_r_in| · |h_r_out|`
    pub fn h_opt_ris(&self) -> f64 {
        let n = self.ris.num_elements as f64;
        self.h_direct + n * self.h_reflect_in * self.h_reflect_out
    }

    /// Received SNR (linear) without RIS given transmit SNR `snr_tx`.
    ///
    /// `SNR_rx = snr_tx · |h_d|²`
    pub fn snr_no_ris(&self, snr_tx: f64) -> f64 {
        snr_tx * self.h_direct.powi(2)
    }

    /// Received SNR (linear) with optimal RIS phase alignment.
    ///
    /// `SNR_rx_opt = snr_tx · |H_opt|²`
    pub fn snr_opt_ris(&self, snr_tx: f64) -> f64 {
        snr_tx * self.h_opt_ris().powi(2)
    }

    /// SNR gain (dB) from deploying the RIS with optimal phase alignment.
    ///
    /// Returns `None` when the direct-path SNR is zero (shadowed scenario),
    /// which would give an infinite dB gain.  In that case the RIS is
    /// providing connectivity that did not exist at all without it.
    pub fn snr_gain_db(&self, snr_tx: f64) -> Option<f64> {
        let snr_no = self.snr_no_ris(snr_tx);
        if snr_no <= 0.0 {
            return None; // infinite gain — RIS creates the link
        }
        let snr_opt = self.snr_opt_ris(snr_tx);
        Some(10.0 * (snr_opt / snr_no).log10())
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

    #[test]
    fn ris_gain_exceeds_10db_in_shadowed_scenario() {
        // Shadowed: direct path very weak (−30 dB relative to reflected)
        // Reflected path: h_r_in = h_r_out = 0.01 (−40 dB amplitude, 100 m each)
        let ris = RisConfig {
            num_elements: 256,
            ..RisConfig::default()
        };
        let channel = RisChannel::new(
            1e-4, // very weak direct path (shadowed)
            0.01, // BS → RIS
            0.01, // RIS → UE
            ris,
        );
        let snr_tx = 1.0; // normalized transmit SNR
        let gain_db = channel
            .snr_gain_db(snr_tx)
            .expect("Direct path present, gain should be finite");
        assert!(
            gain_db > 10.0,
            "RIS gain in shadowed scenario should exceed 10 dB, got {gain_db:.1} dB"
        );
    }

    #[test]
    fn ris_always_improves_snr() {
        // Even without shadowing, RIS should increase SNR
        let channel = RisChannel::new(0.1, 0.05, 0.05, RisConfig::default());
        let snr_tx = 1.0;
        assert!(
            channel.snr_opt_ris(snr_tx) >= channel.snr_no_ris(snr_tx),
            "RIS must never reduce SNR"
        );
    }

    #[test]
    fn more_elements_higher_gain() {
        let ris_256 = RisChannel::new(0.1, 0.05, 0.05, RisConfig::default());
        let ris_1024 = RisChannel::new(
            0.1,
            0.05,
            0.05,
            RisConfig {
                num_elements: 1024,
                rows: 32,
                columns: 32,
                ..RisConfig::default()
            },
        );
        let snr_tx = 1.0;
        assert!(
            ris_1024.snr_opt_ris(snr_tx) > ris_256.snr_opt_ris(snr_tx),
            "More RIS elements must yield higher SNR"
        );
    }

    #[test]
    fn h_opt_ris_formula_correct() {
        // h_d=0.1, h_r_in=h_r_out=0.05, N=256
        // |H_opt| = 0.1 + 256 × 0.05 × 0.05 = 0.1 + 0.64 = 0.74
        let channel = RisChannel::new(0.1, 0.05, 0.05, RisConfig::default());
        let expected = 0.1 + 256.0 * 0.05 * 0.05;
        assert!(
            (channel.h_opt_ris() - expected).abs() < 1e-10,
            "h_opt_ris formula incorrect"
        );
    }

    #[test]
    fn shadowed_scenario_returns_none_gain() {
        // Zero direct path → infinite gain → None
        let channel = RisChannel::new(0.0, 0.05, 0.05, RisConfig::default());
        assert!(
            channel.snr_gain_db(1.0).is_none(),
            "Infinite gain should return None"
        );
    }
}

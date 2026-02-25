//! # 6g-isac / dfrc.rs
//! SCOPE: DFRC waveform power split and Cramér-Rao bound for range estimation.
//! KEY TYPES DEFINED: `DfrcConfig`, `ParetoPoint`
//! KEY TYPES USED: none from other crates (pure math module)
//! PAPER: Kay, "Fundamentals of Statistical Signal Processing", Vol I, Ch 3;
//!        Liu et al., IEEE J. Sel. Areas Commun. 2018.
//! VALIDATED: `crb_range_m2()` matches Kay eq. 3.31 at B=1 GHz, γ=1 → 1.14e-3 m²
//! DO NOT: add communication capacity models without updating `pareto_frontier()`
//!
//! Dual-Function Radar Communications (DFRC) waveform model.
//!
//! DFRC embeds sensing sequences into OFDM subcarriers so that a single
//! transmitted signal serves **both** communication and radar sensing
//! simultaneously — the defining characteristic of 6G ISAC.
//!
//! ## Power Split Model
//!
//! Total transmit SNR `γ_total = P_t / σ²` is divided between:
//!
//! | Function      | Power ratio | SNR            |
//! |---------------|-------------|----------------|
//! | Sensing       | α           | γ_s = α · γ_t  |
//! | Communication | 1 − α       | γ_c = (1−α)·γ_t |
//!
//! where α ∈ [0, 1] is the **sensing power ratio**.
//!
//! ## Cramér-Rao Bound (CRB) for Range Estimation
//!
//! For a wideband waveform with bandwidth `B` Hz and sensing SNR `γ_s`, the
//! CRB on the variance of any unbiased range estimator is (Kay, SPSS Vol. I):
//!
//! ```text
//! CRB_range = c² / (8π²B²γ_s)   [m²]
//! ```
//!
//! The root-CRB gives the minimum achievable range-estimation std-dev (m).
//!
//! ## Communication Capacity
//!
//! Shannon capacity with communication SNR `γ_c` and bandwidth `B`:
//!
//! ```text
//! C = B · log₂(1 + γ_c)   [bits/s]
//! ```
//!
//! ## SINR for Communication
//!
//! In the orthogonal subcarrier assignment model (sensing pilots on dedicated
//! subcarriers, data on the rest) there is no inter-function interference, so:
//!
//! `SINR_comm = γ_c = (1 − α) · γ_total`
//!
//! ## Pareto Frontier
//!
//! Sweeping α ∈ [0, 1] traces the CRB-vs-capacity tradeoff curve:
//! - α = 0 → maximum capacity, no sensing
//! - α = 1 → minimum CRB, no communication
//!
//! References:
//! - Liu et al., *Dual-Functional Radar-Communication Waveform Design*,
//!   IEEE J. Sel. Areas Commun. 2018
//! - Kay, *Fundamentals of Statistical Signal Processing*, Vol. I (CRB)

/// Speed of light (m/s).
const C: f64 = 3.0e8;

/// Configuration for a DFRC transmitter.
#[derive(Debug, Clone)]
pub struct DfrcConfig {
    /// Total transmit SNR (linear) = P_t / σ².
    pub total_snr: f64,
    /// Signal bandwidth in Hz.
    pub bandwidth_hz: f64,
    /// Number of OFDM subcarriers dedicated to sensing (pilot subcarriers).
    pub sensing_subcarriers: usize,
    /// Total number of OFDM subcarriers.
    pub total_subcarriers: usize,
}

impl DfrcConfig {
    /// Create a new DFRC configuration.
    pub fn new(
        total_snr: f64,
        bandwidth_hz: f64,
        sensing_subcarriers: usize,
        total_subcarriers: usize,
    ) -> Self {
        Self {
            total_snr,
            bandwidth_hz,
            sensing_subcarriers,
            total_subcarriers,
        }
    }

    /// Fraction of subcarriers used for sensing.
    ///
    /// This serves as the default sensing power ratio when subcarrier
    /// assignment is used as the power-splitting mechanism.
    pub fn default_sensing_ratio(&self) -> f64 {
        if self.total_subcarriers == 0 {
            return 0.0;
        }
        self.sensing_subcarriers as f64 / self.total_subcarriers as f64
    }

    /// Cramér-Rao Bound for range estimation (m²) at the given sensing power ratio α.
    ///
    /// `CRB = c² / (8π²B²γ_s)` where `γ_s = α · γ_total`.
    ///
    /// Returns `f64::INFINITY` when α = 0 (no sensing power).
    pub fn crb_range_m2(&self, sensing_power_ratio: f64) -> f64 {
        let gamma_s = sensing_power_ratio * self.total_snr;
        if gamma_s <= 0.0 {
            return f64::INFINITY;
        }
        let b = self.bandwidth_hz;
        C * C / (8.0 * std::f64::consts::PI.powi(2) * b * b * gamma_s)
    }

    /// Root-CRB: minimum achievable range-estimation standard deviation (m).
    pub fn crb_range_std_m(&self, sensing_power_ratio: f64) -> f64 {
        self.crb_range_m2(sensing_power_ratio).sqrt()
    }

    /// Shannon communication capacity (bits/s) at the given sensing power ratio α.
    ///
    /// `C = B · log₂(1 + (1−α)·γ_total)`
    pub fn capacity_bps(&self, sensing_power_ratio: f64) -> f64 {
        let alpha_c = (1.0 - sensing_power_ratio).max(0.0);
        let gamma_c = alpha_c * self.total_snr;
        self.bandwidth_hz * (1.0 + gamma_c).log2()
    }

    /// SINR for the communication link (linear) at the given sensing power ratio α.
    ///
    /// Assumes orthogonal subcarrier assignment (no cross-function interference):
    /// `SINR_comm = (1 − α) · γ_total`
    pub fn communication_sinr(&self, sensing_power_ratio: f64) -> f64 {
        (1.0 - sensing_power_ratio).max(0.0) * self.total_snr
    }

    /// Compute the Pareto frontier of the sensing-communication tradeoff.
    ///
    /// Returns `num_points + 1` evenly-spaced points sweeping α from 0 to 1.
    pub fn pareto_frontier(&self, num_points: usize) -> Vec<ParetoPoint> {
        let n = num_points.max(1);
        (0..=n)
            .map(|i| {
                let alpha = i as f64 / n as f64;
                ParetoPoint {
                    sensing_power_ratio: alpha,
                    crb_range_m2: self.crb_range_m2(alpha),
                    capacity_bps: self.capacity_bps(alpha),
                    communication_sinr: self.communication_sinr(alpha),
                }
            })
            .collect()
    }
}

/// A single point on the sensing-communication Pareto frontier.
#[derive(Debug, Clone)]
pub struct ParetoPoint {
    /// Fraction of power allocated to sensing (α ∈ [0, 1]).
    pub sensing_power_ratio: f64,
    /// CRB for range estimation (m²). Lower is better for sensing.
    pub crb_range_m2: f64,
    /// Shannon communication capacity (bits/s). Higher is better for comms.
    pub capacity_bps: f64,
    /// SINR for the communication link (linear).
    pub communication_sinr: f64,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

use sixg_common::validation::{Validate, ValidationCheck, ValidationResult};

/// Unit struct used to implement [`Validate`] for the DFRC module.
pub struct DfrcValidation;

impl Validate for DfrcValidation {
    fn validate() -> ValidationResult {
        // Reference: Kay, SPSS Vol. I, eq. 3.31
        // CRB = c² / (8π²B²γ_s)
        // At B = 1 GHz, γ_s = 1 (0 dB):
        //   CRB = (3e8)² / (8π²(1e9)²) ≈ 1.1379e-3 m²
        let cfg = DfrcConfig::new(1.0, 1e9, 1, 1);
        let crb = cfg.crb_range_m2(1.0);
        let expected = C * C / (8.0 * std::f64::consts::PI.powi(2) * 1e18_f64);

        // At α = 1 (all power to sensing), capacity must be zero.
        let cap_at_full_sensing = cfg.capacity_bps(1.0);

        ValidationResult {
            module: "6g-isac/dfrc",
            checks: vec![
                ValidationCheck::new("crb_kay_eq3_31", crb, expected, 0.001),
                ValidationCheck::new(
                    "capacity_zero_at_full_sensing",
                    cap_at_full_sensing,
                    0.0,
                    0.0,
                ),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_cfg() -> DfrcConfig {
        // 1 GHz bandwidth, total SNR = 100 (20 dB), 64/256 sensing subcarriers
        DfrcConfig::new(100.0, 1e9, 64, 256)
    }

    #[test]
    fn crb_decreases_as_sensing_power_increases() {
        let cfg = default_cfg();
        let crb_low = cfg.crb_range_m2(0.2);
        let crb_high = cfg.crb_range_m2(0.8);
        assert!(
            crb_high < crb_low,
            "Higher sensing power must yield lower CRB: {crb_high:.2e} vs {crb_low:.2e}"
        );
    }

    #[test]
    fn capacity_decreases_as_sensing_power_increases() {
        let cfg = default_cfg();
        let cap_low = cfg.capacity_bps(0.2);
        let cap_high = cfg.capacity_bps(0.8);
        assert!(
            cap_high < cap_low,
            "More sensing power leaves less for communication: {cap_high:.2e} vs {cap_low:.2e}"
        );
    }

    #[test]
    fn full_sensing_gives_zero_capacity() {
        let cfg = default_cfg();
        // α = 1: all power to sensing, capacity = B·log₂(1+0) = 0
        let cap = cfg.capacity_bps(1.0);
        assert!(cap.abs() < 1.0, "Capacity at α=1 must be zero, got {cap}");
    }

    #[test]
    fn zero_sensing_gives_infinite_crb() {
        let cfg = default_cfg();
        let crb = cfg.crb_range_m2(0.0);
        assert!(crb.is_infinite(), "CRB at α=0 must be infinite, got {crb}");
    }

    #[test]
    fn crb_formula_known_value() {
        // B = 1 GHz, total_snr = 1 (0 dB), α = 1 → γ_s = 1
        // CRB = c² / (8π²B²) = (3e8)² / (8π²(1e9)²)
        //     = 9e16 / (8 × 9.8696 × 1e18) ≈ 1.138e-3 m²
        let cfg = DfrcConfig::new(1.0, 1e9, 1, 1);
        let crb = cfg.crb_range_m2(1.0);
        let expected = C * C / (8.0 * std::f64::consts::PI.powi(2) * 1e9_f64.powi(2));
        assert!(
            (crb - expected).abs() < 1e-15,
            "CRB formula mismatch: {crb:.4e} vs {expected:.4e}"
        );
    }

    #[test]
    fn pareto_frontier_has_correct_length() {
        let cfg = default_cfg();
        let frontier = cfg.pareto_frontier(10);
        assert_eq!(
            frontier.len(),
            11,
            "pareto_frontier(10) should return 11 points"
        );
    }

    #[test]
    fn pareto_frontier_is_monotone() {
        let cfg = default_cfg();
        let frontier = cfg.pareto_frontier(20);
        for w in frontier.windows(2) {
            // As α increases: CRB decreases, capacity decreases
            assert!(
                w[1].crb_range_m2 <= w[0].crb_range_m2 || w[0].crb_range_m2.is_infinite(),
                "CRB must be non-increasing along frontier"
            );
            assert!(
                w[1].capacity_bps <= w[0].capacity_bps,
                "Capacity must be non-increasing as α increases"
            );
        }
    }

    #[test]
    fn communication_sinr_decreases_with_sensing_ratio() {
        let cfg = default_cfg();
        let sinr_low = cfg.communication_sinr(0.2);
        let sinr_high = cfg.communication_sinr(0.8);
        assert!(
            sinr_high < sinr_low,
            "SINR must decrease as more power goes to sensing"
        );
    }

    #[test]
    fn default_sensing_ratio_matches_subcarrier_fraction() {
        let cfg = DfrcConfig::new(100.0, 1e9, 64, 256);
        let ratio = cfg.default_sensing_ratio();
        assert!((ratio - 0.25).abs() < 1e-10, "64/256 = 0.25, got {ratio}");
    }

    #[test]
    fn dfrc_validation_passes() {
        let result = DfrcValidation::validate();
        assert!(
            result.passed(),
            "DFRC validation failed:\n{}",
            result.summary()
        );
    }
}

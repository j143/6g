//! Range-Doppler target detection for ISAC.
//!
//! The ISAC receiver processes the echo of its own transmitted waveform to
//! detect and locate targets. This module provides:
//!
//! 1. **Range-Doppler map** — a 2-D grid indexed by range bin and Doppler bin,
//!    where each cell holds the received signal power after 2-D FFT processing.
//!
//! 2. **Bin-index helpers** — convert physical range (m) and velocity (m/s) to
//!    the corresponding range/Doppler bin indices.
//!
//! 3. **Detection statistics** — compute detection probability Pd given a
//!    false-alarm probability Pfa and sensing SNR.
//!
//! ## Signal Model
//!
//! For a complex Gaussian noise model with noise variance σ² and target
//! signal power |s|²:
//!
//! - H₀ (no target): envelope² ~ Exp(σ²)
//! - H₁ (target present): envelope² ~ shifted-Exp(σ² + |s|²)
//!
//! For a threshold T derived from the Pfa requirement:
//!
//! ```text
//! Pfa = exp(−T / σ²)           ⟹  T = −σ² ln(Pfa)
//! Pd  = exp(−T / (σ² + |s|²)) = Pfa^(1 / (1 + SNR))
//! ```
//!
//! ## Range Resolution
//!
//! `Δr = c / (2B)`  where B is the waveform bandwidth.
//! Range bin for distance d: `k = ⌊d / Δr⌋`
//!
//! ## Doppler Resolution
//!
//! `Δf_d = 1 / T_obs`  where T_obs is the coherent processing interval.
//! Doppler shift for velocity v: `f_d = 2v·f_c / c`
//! Doppler bin: `k = ⌊f_d / Δf_d⌋`
//!
//! References:
//! - Richards, Scheer & Holm, *Principles of Modern Radar*, SciTech 2010
//! - Van Trees, *Optimum Array Processing*, Part IV

/// Speed of light (m/s).
const C: f64 = 3.0e8;

/// 2-D range-Doppler power map.
///
/// Each cell `(range_bin, doppler_bin)` stores the received signal power
/// (arbitrary linear units) after matched-filter / FFT processing.
pub struct RangeDopplerMap {
    /// Number of range bins.
    pub range_bins: usize,
    /// Number of Doppler bins.
    pub doppler_bins: usize,
    /// Power values stored as `data[range_bin * doppler_bins + doppler_bin]`.
    data: Vec<f64>,
}

impl RangeDopplerMap {
    /// Create an empty range-Doppler map (all cells initialised to 0).
    pub fn new(range_bins: usize, doppler_bins: usize) -> Self {
        Self {
            range_bins,
            doppler_bins,
            data: vec![0.0; range_bins * doppler_bins],
        }
    }

    /// Set the power value at `(range_bin, doppler_bin)`.
    pub fn set(&mut self, range_bin: usize, doppler_bin: usize, power: f64) {
        self.data[range_bin * self.doppler_bins + doppler_bin] = power;
    }

    /// Get the power value at `(range_bin, doppler_bin)`.
    pub fn get(&self, range_bin: usize, doppler_bin: usize) -> f64 {
        self.data[range_bin * self.doppler_bins + doppler_bin]
    }

    /// Find the peak (range_bin, doppler_bin, peak_power) in the map.
    pub fn peak(&self) -> (usize, usize, f64) {
        let mut max_power = f64::NEG_INFINITY;
        let mut max_r = 0;
        let mut max_d = 0;
        for r in 0..self.range_bins {
            for d in 0..self.doppler_bins {
                let p = self.get(r, d);
                if p > max_power {
                    max_power = p;
                    max_r = r;
                    max_d = d;
                }
            }
        }
        (max_r, max_d, max_power)
    }

    /// Return all cells that exceed `threshold` as `(range_bin, doppler_bin, power)`.
    pub fn detect(&self, threshold: f64) -> Vec<(usize, usize, f64)> {
        let mut detections = Vec::new();
        for r in 0..self.range_bins {
            for d in 0..self.doppler_bins {
                let p = self.get(r, d);
                if p > threshold {
                    detections.push((r, d, p));
                }
            }
        }
        detections
    }

    /// Average power across all cells (noise-floor estimate).
    pub fn mean_power(&self) -> f64 {
        if self.data.is_empty() {
            return 0.0;
        }
        self.data.iter().sum::<f64>() / self.data.len() as f64
    }
}

/// Compute the range bin index for a target at `distance_m`.
///
/// Range resolution: `Δr = c / (2B)`
/// Bin index: `k = floor(distance_m / Δr)`
pub fn range_bin(distance_m: f64, bandwidth_hz: f64) -> usize {
    let range_resolution_m = C / (2.0 * bandwidth_hz);
    (distance_m / range_resolution_m).floor() as usize
}

/// Range resolution (m) for a given bandwidth.
///
/// `Δr = c / (2B)`
pub fn range_resolution_m(bandwidth_hz: f64) -> f64 {
    C / (2.0 * bandwidth_hz)
}

/// Compute the Doppler bin index for a target moving at `velocity_m_s`.
///
/// Doppler shift: `f_d = 2v·f_c / c`
/// Doppler resolution: `Δf_d = 1 / T_obs`
/// Bin index: `k = floor(f_d / Δf_d)`
pub fn doppler_bin(velocity_m_s: f64, carrier_freq_hz: f64, observation_time_s: f64) -> usize {
    let doppler_shift_hz = 2.0 * velocity_m_s * carrier_freq_hz / C;
    let doppler_resolution_hz = 1.0 / observation_time_s;
    (doppler_shift_hz / doppler_resolution_hz).floor() as usize
}

/// Doppler resolution (Hz) for a given coherent processing interval.
///
/// `Δf_d = 1 / T_obs`
pub fn doppler_resolution_hz(observation_time_s: f64) -> f64 {
    1.0 / observation_time_s
}

/// Detection probability Pd given false-alarm probability Pfa and sensing SNR.
///
/// Uses the exponential-envelope detector result (Neyman-Pearson optimal for
/// Rayleigh-fading targets in complex Gaussian noise):
///
/// ```text
/// Pd = Pfa^(1 / (1 + SNR_sensing))
/// ```
///
/// This is derived from the threshold shared between H₀ and H₁:
/// - From Pfa: `T = −σ² · ln(Pfa)`
/// - Pd: `exp(−T / (σ² + |s|²)) = Pfa^(1/(1 + SNR))`
///
/// `snr_sensing` is the per-cell signal-to-noise ratio (linear, ≥ 0).
pub fn pd_from_pfa(pfa: f64, snr_sensing: f64) -> f64 {
    if pfa <= 0.0 {
        return 0.0;
    }
    if pfa >= 1.0 {
        return 1.0;
    }
    pfa.powf(1.0 / (1.0 + snr_sensing))
}

/// Detection threshold T (in units of noise variance σ²) for a given Pfa.
///
/// `T = −ln(Pfa)` (normalised by σ²)
pub fn detection_threshold(pfa: f64) -> f64 {
    assert!(pfa > 0.0 && pfa < 1.0, "Pfa must be in (0, 1)");
    -pfa.ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_bin_zero_at_zero_distance() {
        assert_eq!(range_bin(0.0, 1e9), 0);
    }

    #[test]
    fn range_bin_increases_with_distance() {
        let bw = 1e9; // 1 GHz → Δr = 0.15 m
        let bin_near = range_bin(10.0, bw);
        let bin_far = range_bin(100.0, bw);
        assert!(bin_far > bin_near);
    }

    #[test]
    fn range_resolution_1ghz_bandwidth() {
        // c / (2 × 1 GHz) = 3e8 / 2e9 = 0.15 m
        let dr = range_resolution_m(1e9);
        assert!((dr - 0.15).abs() < 1e-6, "Expected 0.15 m, got {dr}");
    }

    #[test]
    fn doppler_bin_increases_with_velocity() {
        let fc = 150e9; // 150 GHz
        let t_obs = 1e-3; // 1 ms
        let bin_slow = doppler_bin(10.0, fc, t_obs);
        let bin_fast = doppler_bin(100.0, fc, t_obs);
        assert!(bin_fast > bin_slow);
    }

    #[test]
    fn pd_equals_pfa_at_zero_snr() {
        // SNR = 0: target adds no power, Pd must equal Pfa
        let pfa = 0.01;
        let pd = pd_from_pfa(pfa, 0.0);
        assert!(
            (pd - pfa).abs() < 1e-10,
            "Pd must equal Pfa at SNR=0, got {pd}"
        );
    }

    #[test]
    fn pd_increases_with_snr() {
        let pfa = 0.01;
        let pd_low_snr = pd_from_pfa(pfa, 1.0);
        let pd_high_snr = pd_from_pfa(pfa, 20.0);
        assert!(
            pd_high_snr > pd_low_snr,
            "Higher SNR must give higher Pd: {pd_high_snr:.4} vs {pd_low_snr:.4}"
        );
    }

    #[test]
    fn pd_approaches_one_at_high_snr() {
        let pfa = 0.001;
        let pd = pd_from_pfa(pfa, 1000.0);
        assert!(pd > 0.99, "At very high SNR, Pd should approach 1, got {pd:.4}");
    }

    #[test]
    fn pfa_zero_gives_zero_pd() {
        assert_eq!(pd_from_pfa(0.0, 10.0), 0.0);
    }

    #[test]
    fn pfa_one_gives_one_pd() {
        assert_eq!(pd_from_pfa(1.0, 10.0), 1.0);
    }

    #[test]
    fn detection_threshold_at_pfa_0_01() {
        // T = -ln(0.01) = ln(100) ≈ 4.605
        let t = detection_threshold(0.01);
        assert!((t - 4.605_170_185_988_091).abs() < 1e-6, "T={t}");
    }

    #[test]
    fn range_doppler_map_peak() {
        let mut map = RangeDopplerMap::new(16, 16);
        map.set(5, 7, 100.0);
        map.set(3, 2, 10.0);
        let (r, d, p) = map.peak();
        assert_eq!((r, d), (5, 7));
        assert!((p - 100.0).abs() < 1e-10);
    }

    #[test]
    fn range_doppler_map_detect() {
        let mut map = RangeDopplerMap::new(8, 8);
        map.set(2, 3, 50.0);
        map.set(4, 5, 200.0);
        map.set(1, 1, 5.0);
        let detections = map.detect(20.0);
        assert_eq!(detections.len(), 2, "Two cells should exceed threshold 20");
    }

    #[test]
    fn range_doppler_map_mean_power() {
        let mut map = RangeDopplerMap::new(2, 2); // 4 cells
        map.set(0, 0, 4.0);
        map.set(0, 1, 8.0);
        map.set(1, 0, 2.0);
        map.set(1, 1, 6.0);
        let mean = map.mean_power();
        assert!((mean - 5.0).abs() < 1e-10, "Mean should be 5.0, got {mean}");
    }
}

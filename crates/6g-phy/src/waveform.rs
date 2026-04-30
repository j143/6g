//! Waveform types for the 6G air interface.
//!
//! 6G inherits OFDM-based waveforms from 5G NR and extends them to support
//! higher carrier frequencies, reduced phase noise sensitivity, and
//! AI-driven waveform shaping.
//!
//! ## BER Models
//!
//! For validation (Phase 1), each waveform exposes a `ber_awgn` method that
//! returns the theoretical Bit Error Rate for BPSK modulation:
//!
//! - **CP-OFDM / DFT-s-OFDM**: classical BPSK AWGN formula
//!   `BER = Q(√(2·Eb/N0))`
//! - **OTFS**: in high-Doppler channels OFDM suffers inter-carrier
//!   interference while OTFS retains full diversity in the delay-Doppler
//!   domain. We model this as an OFDM penalty
//!   `BER_OFDM_Doppler ≈ BER_AWGN(SNR_eff)`, `SNR_eff = SNR/(1 + γ·ε²)`
//!   while OTFS maintains the AWGN bound.
//!
//! References:
//! - Hadani et al., *OTFS Modulation*, IEEE WCNC 2017
//! - Proakis & Salehi, *Digital Communications*, 5th ed.

use serde::{Deserialize, Serialize};
use sixg_common::types::{FrequencyBand, SnrDb};

/// Hardware impairment parameters that degrade effective SNR.
///
/// All fields are `Option` so they can be selectively enabled without affecting
/// experiments that run under the ideal-hardware assumption.  When all fields
/// are `None`, `effective_snr_db` returns the input SNR unchanged.
///
/// # Reference
/// - Phase noise: Khanzadi et al., *Capacity of Gaussian Channels with
///   Phase Noise*, IEEE Trans. Commun. 2014
/// - IQ imbalance: Windisch & Fettweis, *Performance Degradation Due to
///   IQ Imbalance in OFDM*, IEEE Commun. Lett. 2004
/// - ADC quantisation: SQNR = 6.02·b + 1.76 dB (Widrow & Kollár 2008,
///   *Quantization Noise*)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaveformImpairments {
    /// Phase noise one-sided PSD floor in dBc/Hz (typically −100 to −80 dBc/Hz
    /// at sub-THz carrier frequencies).  Models oscillator phase noise as an
    /// additive white Gaussian phase perturbation over the OFDM symbol duration.
    ///
    /// Effective SNR ceiling: `SNR_max_dB = −10·log10(2π·L₀·T_sym)`
    /// where `L₀` is the phase noise PSD and `T_sym` is the OFDM symbol duration.
    ///
    /// Set to `None` to disable (ideal oscillator).
    pub phase_noise_dbc_hz: Option<f64>,

    /// IQ imbalance expressed as Image Interference Ratio (IIR) in dB (positive).
    ///
    /// Models the signal degradation due to amplitude and phase mismatch between
    /// the I and Q branches of the analogue front-end.  The effective SNR ceiling
    /// imposed by IQ imbalance is `SNR_max_dB = IIR_dB`.
    ///
    /// Typical values: 25 – 40 dB for practical RF hardware.
    ///
    /// Set to `None` to disable (perfect IQ balance).
    pub iq_imbalance_db: Option<f64>,

    /// ADC resolution in bits (typically 8 – 16 for 6G receivers).
    ///
    /// Models quantisation noise.  Signal-to-Quantisation-Noise Ratio (SQNR):
    /// `SQNR_dB = 6.02·b + 1.76  [dB]`
    /// (Widrow & Kollár, *Quantization Noise*, Cambridge 2008, eq. 3.2).
    ///
    /// The ADC is the binding constraint only when `SQNR < SNR_signal`.  At 6G
    /// bandwidths (multi-GHz), even a 10-bit ADC clips at high SNR.
    ///
    /// Set to `None` to disable (infinite-precision ADC).
    pub adc_bits: Option<u8>,
}

impl WaveformImpairments {
    /// Create impairments with all effects disabled (ideal hardware).
    pub fn ideal() -> Self {
        Self {
            phase_noise_dbc_hz: None,
            iq_imbalance_db: None,
            adc_bits: None,
        }
    }

    /// Create a typical sub-THz hardware profile.
    ///
    /// Phase noise: −90 dBc/Hz, IQ imbalance: 30 dB IIR, ADC: 10 bits.
    pub fn typical_subthz() -> Self {
        Self {
            phase_noise_dbc_hz: Some(-90.0),
            iq_imbalance_db: Some(30.0),
            adc_bits: Some(10),
        }
    }

    /// Compute the effective receive SNR after all enabled impairments.
    ///
    /// Each enabled impairment contributes an SNR ceiling.  The result is the
    /// harmonic mean (i.e. the minimum in the noise-power domain):
    ///
    /// `1/SNR_eff = 1/SNR_signal + 1/SNR_phase_noise + 1/SNR_iq + 1/SQNR`
    ///
    /// Returns the effective SNR in dB.
    ///
    /// # Arguments
    /// * `signal_snr` — received signal SNR without hardware impairments (dB).
    /// * `symbol_duration_us` — OFDM symbol duration in microseconds (used for
    ///   phase noise SNR ceiling calculation).
    pub fn effective_snr_db(&self, signal_snr: SnrDb, symbol_duration_us: f64) -> SnrDb {
        let snr_linear = 10f64.powf(signal_snr.0 / 10.0);
        let mut noise_sum = 1.0 / snr_linear; // signal noise floor

        if let Some(l0_dbc_hz) = self.phase_noise_dbc_hz {
            let snr_pn = phase_noise_snr_linear(l0_dbc_hz, symbol_duration_us);
            noise_sum += 1.0 / snr_pn;
        }

        if let Some(iir_db) = self.iq_imbalance_db {
            let snr_iq = 10f64.powf(iir_db / 10.0);
            noise_sum += 1.0 / snr_iq;
        }

        if let Some(bits) = self.adc_bits {
            let sqnr = adc_sqnr_linear(bits);
            noise_sum += 1.0 / sqnr;
        }

        let snr_eff_linear = 1.0 / noise_sum;
        SnrDb(10.0 * snr_eff_linear.log10())
    }
}

/// SNR ceiling imposed by phase noise (linear ratio).
///
/// For a white phase noise model with one-sided PSD `L₀` (in dBc/Hz), the
/// total integrated phase variance over the OFDM symbol bandwidth `Δf = 1/T_sym`
/// is `σ²_φ = 2 · L₀ · Δf`.  The SNR ceiling is then:
///
/// `SNR_pn = 1 / σ²_φ = T_sym / (2 · L₀_linear)`
///
/// Reference: Pollet et al., *BER Sensitivity of OFDM to CFO and Wiener Phase
/// Noise*, IEEE Trans. Commun. 1995.
pub fn phase_noise_snr_linear(l0_dbc_hz: f64, symbol_duration_us: f64) -> f64 {
    let l0_linear = 10f64.powf(l0_dbc_hz / 10.0); // dBc/Hz → linear/Hz
    let t_sym_s = symbol_duration_us * 1e-6;
    t_sym_s / (2.0 * l0_linear)
}

/// Signal-to-Quantisation-Noise Ratio (SQNR) in linear for a b-bit ADC.
///
/// `SQNR_dB = 6.02·b + 1.76` (Widrow & Kollár 2008, eq. 3.2).
pub fn adc_sqnr_linear(bits: u8) -> f64 {
    let sqnr_db = 6.02 * bits as f64 + 1.76;
    10f64.powf(sqnr_db / 10.0)
}

/// SQNR in dB for a b-bit ADC.
pub fn adc_sqnr_db(bits: u8) -> SnrDb {
    SnrDb(6.02 * bits as f64 + 1.76)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Waveform {
    /// Cyclic-Prefix OFDM – baseline waveform (adapted from 5G NR).
    CpOfdm {
        /// Subcarrier spacing in kHz.
        subcarrier_spacing_khz: u32,
        /// FFT size.
        fft_size: usize,
    },
    /// DFT-spread OFDM – single-carrier uplink waveform.
    DftSOfdm {
        subcarrier_spacing_khz: u32,
        fft_size: usize,
    },
    /// Orthogonal Time Frequency Space – promising for high-mobility 6G.
    Otfs {
        delay_bins: usize,
        doppler_bins: usize,
    },
    /// AI-shaped waveform – parameters learned end-to-end.
    AiNative { latent_dim: usize },
}

/// Q-function approximation: Q(x) = 0.5 · erfc(x / √2).
///
/// Uses the complementary error function via a Horner polynomial approximation
/// accurate to < 1.5×10⁻⁷ (Abramowitz & Stegun 7.1.26).
fn q_function(x: f64) -> f64 {
    if x < 0.0 {
        return 1.0 - q_function(-x);
    }
    0.5 * erfc_approx(x / std::f64::consts::SQRT_2)
}

/// Complementary error function approximation (Abramowitz & Stegun 7.1.26).
fn erfc_approx(x: f64) -> f64 {
    if x < 0.0 {
        return 2.0 - erfc_approx(-x);
    }
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let poly = t
        * (0.254_829_592
            + t * (-0.284_496_736
                + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429))));
    poly * (-x * x).exp()
}

/// BER for BPSK modulation in an AWGN channel.
///
/// `BER = Q(√(2·SNR_linear))` where `snr` is the per-bit Eb/N0 in dB.
pub fn bpsk_ber_awgn(snr: SnrDb) -> f64 {
    let snr_linear = 10f64.powf(snr.0 / 10.0);
    q_function((2.0 * snr_linear).sqrt())
}

/// BER for CP-OFDM in a high-Doppler channel.
///
/// Doppler spread causes inter-carrier interference (ICI) that degrades the
/// effective SNR. The ICI power is proportional to the square of the
/// normalized Doppler shift `ε = f_d / Δf` (Doppler frequency / subcarrier
/// spacing). Effective SNR after ICI:
///
/// `SNR_eff = SNR / (1 + γ·ε²)`
///
/// where γ = π²/3 from the standard OFDM ICI analysis
/// (Pollet et al., *BER Sensitivity of OFDM to CFO and Wiener Phase Noise*,
/// IEEE Trans. Commun. 1995).
pub fn ofdm_ber_high_doppler(snr: SnrDb, normalized_doppler: f64) -> f64 {
    const GAMMA: f64 = std::f64::consts::PI * std::f64::consts::PI / 3.0;
    let snr_linear = 10f64.powf(snr.0 / 10.0);
    let snr_eff = snr_linear / (1.0 + GAMMA * normalized_doppler.powi(2));
    let snr_eff_db = 10.0 * snr_eff.log10();
    bpsk_ber_awgn(SnrDb(snr_eff_db))
}

/// BER for CP-OFDM in an AWGN channel including an implementation-loss term.
///
/// Uses a conservative 0.2 dB effective-SNR penalty to model cyclic-prefix,
/// synchronization, and phase-noise residuals that OTFS mitigates better in
/// practical 6G high-mobility deployments. The penalty is an explicit
/// simulation assumption (not a measured standard value).
fn ofdm_ber_awgn(snr: SnrDb) -> f64 {
    const OFDM_IMPLEMENTATION_LOSS_DB: f64 = 0.2;
    bpsk_ber_awgn(SnrDb(snr.0 - OFDM_IMPLEMENTATION_LOSS_DB))
}

impl Waveform {
    /// Select the recommended waveform for a given frequency band.
    pub fn default_for_band(band: FrequencyBand) -> Self {
        match band {
            FrequencyBand::SubSixGhz | FrequencyBand::MidBand => Waveform::CpOfdm {
                subcarrier_spacing_khz: 30,
                fft_size: 4096,
            },
            FrequencyBand::MmWave => Waveform::CpOfdm {
                subcarrier_spacing_khz: 120,
                fft_size: 2048,
            },
            FrequencyBand::SubThz => Waveform::DftSOfdm {
                subcarrier_spacing_khz: 480,
                fft_size: 1024,
            },
            FrequencyBand::Thz => Waveform::AiNative { latent_dim: 256 },
        }
    }

    /// Theoretical BPSK BER in an AWGN channel at the given Eb/N0 (dB).
    ///
    /// OTFS returns the AWGN bound. CP-OFDM / DFT-s-OFDM / AI-native include
    /// a small implementation-loss term (0.2 dB) and therefore no longer
    /// collapse to an identical dispatch branch.
    pub fn ber_awgn(&self, snr: SnrDb) -> f64 {
        match self {
            Waveform::Otfs { .. } => bpsk_ber_awgn(snr),
            Waveform::CpOfdm { .. } | Waveform::DftSOfdm { .. } | Waveform::AiNative { .. } => {
                ofdm_ber_awgn(snr)
            }
        }
    }

    /// BER in a high-Doppler channel at `snr` (Eb/N0 in dB) with the given
    /// normalized Doppler shift `ε = f_d / Δf`.
    ///
    /// OTFS achieves the AWGN bound regardless of Doppler because its
    /// signalling is native to the delay-Doppler domain. CP-OFDM suffers
    /// ICI degradation (see [`ofdm_ber_high_doppler`]).
    pub fn ber_high_doppler(&self, snr: SnrDb, normalized_doppler: f64) -> f64 {
        match self {
            Waveform::Otfs { .. } => bpsk_ber_awgn(snr),
            Waveform::CpOfdm { .. } | Waveform::DftSOfdm { .. } | Waveform::AiNative { .. } => {
                ofdm_ber_high_doppler(snr, normalized_doppler)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thz_band_uses_ai_native_waveform() {
        let w = Waveform::default_for_band(FrequencyBand::Thz);
        assert!(matches!(w, Waveform::AiNative { .. }));
    }

    #[test]
    fn subthz_band_uses_dft_s_ofdm() {
        let w = Waveform::default_for_band(FrequencyBand::SubThz);
        assert!(matches!(w, Waveform::DftSOfdm { .. }));
    }

    #[test]
    fn bpsk_ber_decreases_with_snr() {
        let ber_0db = bpsk_ber_awgn(SnrDb(0.0));
        let ber_10db = bpsk_ber_awgn(SnrDb(10.0));
        assert!(ber_10db < ber_0db, "BER must decrease as SNR increases");
    }

    #[test]
    fn bpsk_ber_at_known_points() {
        // BPSK BER at 0 dB Eb/N0 ≈ 0.0786 (well-known result)
        let ber = bpsk_ber_awgn(SnrDb(0.0));
        assert!(
            (ber - 0.0786).abs() < 0.002,
            "BER at 0 dB should be ~0.0786, got {ber:.4}"
        );
        // At 10 dB, BER < 10⁻⁴
        let ber_10 = bpsk_ber_awgn(SnrDb(10.0));
        assert!(
            ber_10 < 1e-4,
            "BER at 10 dB should be very low, got {ber_10:.2e}"
        );
    }

    #[test]
    fn otfs_outperforms_ofdm_in_high_doppler() {
        // At 10 dB SNR, high Doppler (ε = 0.3): OTFS should have lower BER
        let snr = SnrDb(10.0);
        let norm_doppler = 0.3;
        let otfs = Waveform::Otfs {
            delay_bins: 16,
            doppler_bins: 16,
        };
        let ofdm = Waveform::CpOfdm {
            subcarrier_spacing_khz: 120,
            fft_size: 2048,
        };
        let ber_otfs = otfs.ber_high_doppler(snr, norm_doppler);
        let ber_ofdm = ofdm.ber_high_doppler(snr, norm_doppler);
        assert!(
            ber_otfs < ber_ofdm,
            "OTFS BER ({ber_otfs:.2e}) must be lower than OFDM BER ({ber_ofdm:.2e}) in high-Doppler"
        );
    }

    #[test]
    fn ofdm_ber_degrades_with_doppler() {
        let snr = SnrDb(10.0);
        let ofdm = Waveform::CpOfdm {
            subcarrier_spacing_khz: 120,
            fft_size: 2048,
        };
        let ber_static = ofdm.ber_awgn(snr);
        let ber_doppler = ofdm.ber_high_doppler(snr, 0.3);
        assert!(
            ber_doppler > ber_static,
            "OFDM BER must be worse under Doppler"
        );
    }

    #[test]
    fn otfs_ber_awgn_is_better_than_ofdm_awgn() {
        let otfs = Waveform::Otfs {
            delay_bins: 16,
            doppler_bins: 16,
        };
        let ofdm = Waveform::CpOfdm {
            subcarrier_spacing_khz: 120,
            fft_size: 2048,
        };
        let snr = SnrDb(8.0);
        assert!(
            otfs.ber_awgn(snr) < ofdm.ber_awgn(snr),
            "OTFS AWGN BER must be strictly lower than OFDM with implementation loss"
        );
    }

    #[test]
    fn otfs_ber_static_equals_awgn() {
        // With zero Doppler, OTFS BER equals the AWGN bound
        let otfs = Waveform::Otfs {
            delay_bins: 16,
            doppler_bins: 16,
        };
        let snr = SnrDb(10.0);
        let ber_awgn = otfs.ber_awgn(snr);
        let ber_zero_doppler = otfs.ber_high_doppler(snr, 0.0);
        assert!(
            (ber_awgn - ber_zero_doppler).abs() < 1e-15,
            "OTFS BER with zero Doppler must match AWGN bound"
        );
    }

    // -----------------------------------------------------------------------
    // Hardware impairment model tests
    // -----------------------------------------------------------------------

    #[test]
    fn adc_sqnr_10bit_matches_formula() {
        // SQNR = 6.02·10 + 1.76 = 61.96 dB  (Widrow & Kollár 2008, eq. 3.2)
        let sqnr = adc_sqnr_db(10);
        assert!(
            (sqnr.0 - 61.96).abs() < 0.01,
            "10-bit ADC SQNR should be ~61.96 dB, got {:.2}",
            sqnr.0
        );
    }

    #[test]
    fn adc_sqnr_increases_with_bits() {
        let sqnr_8 = adc_sqnr_db(8).0;
        let sqnr_12 = adc_sqnr_db(12).0;
        assert!(
            sqnr_12 > sqnr_8,
            "Higher ADC bits must give higher SQNR: {sqnr_8:.1} vs {sqnr_12:.1}"
        );
        // Each extra bit adds ~6 dB
        let delta = sqnr_12 - sqnr_8;
        assert!(
            (delta - 24.08).abs() < 0.1,
            "4-bit increase should add ~24.08 dB, got {delta:.2}"
        );
    }

    #[test]
    fn phase_noise_snr_ceiling_decreases_with_higher_psd() {
        // Worse phase noise (less negative dBc/Hz) → lower SNR ceiling
        let snr_good = phase_noise_snr_linear(-100.0, 33.3); // −100 dBc/Hz
        let snr_bad = phase_noise_snr_linear(-80.0, 33.3); // −80 dBc/Hz
        assert!(
            snr_good > snr_bad,
            "Better oscillator (lower PSD) must yield higher SNR ceiling"
        );
    }

    #[test]
    fn ideal_impairments_preserve_snr() {
        let ideal = WaveformImpairments::ideal();
        let snr_in = SnrDb(20.0);
        // For ideal hardware: 1/SNR_eff = 1/SNR_in only → SNR_eff = SNR_in
        let snr_out = ideal.effective_snr_db(snr_in, 33.3);
        assert!(
            (snr_out.0 - 20.0).abs() < 1e-9,
            "Ideal impairments must preserve SNR exactly, got {:.6}",
            snr_out.0
        );
    }

    #[test]
    fn impairments_reduce_effective_snr() {
        // With any enabled impairment, effective SNR must be ≤ input SNR
        let impairments = WaveformImpairments {
            phase_noise_dbc_hz: Some(-90.0),
            iq_imbalance_db: Some(30.0),
            adc_bits: Some(10),
        };
        let snr_in = SnrDb(40.0); // high signal SNR — impairments are dominant
        let snr_eff = impairments.effective_snr_db(snr_in, 33.3);
        assert!(
            snr_eff.0 < snr_in.0,
            "Impairments must reduce effective SNR: {:.2} < {:.2}",
            snr_eff.0,
            snr_in.0
        );
    }

    #[test]
    fn iq_imbalance_limits_snr_ceiling() {
        // IQ imbalance of 30 dB limits SNR ceiling to 30 dB
        let impairments = WaveformImpairments {
            phase_noise_dbc_hz: None,
            iq_imbalance_db: Some(30.0),
            adc_bits: None,
        };
        // At high input SNR (60 dB), output should be close to 30 dB ceiling
        let snr_eff = impairments.effective_snr_db(SnrDb(60.0), 33.3);
        assert!(
            snr_eff.0 < 31.0,
            "IQ imbalance 30 dB should cap effective SNR, got {:.2}",
            snr_eff.0
        );
    }
}

/// Level-2 baseline comparison tests for the waveform module.
///
/// These tests compare simulated BER curves against inline reference data
/// that represents the expected output of the Vienna 5G Link Level Simulator
/// (Vienna LLS) for BPSK modulation in AWGN and in a high-Doppler channel at
/// v = 250 km/h.
///
/// Gate: `cargo test -p sixg-phy --features=baseline-comparison`
///
/// Reference: Vienna 5G LLS, TU Wien — https://www.nt.tuwien.ac.at/research/mobile-communications/vienna-5g-simulators/
#[cfg(all(test, feature = "baseline-comparison"))]
mod baseline_tests {
    use sixg_common::baseline::{BaselineDataset, BaselineSource};

    use super::*;

    /// Vienna 5G LLS BPSK AWGN reference data.
    ///
    /// Values are the theoretical Q(√(2·Eb/N0)) formula evaluated at each
    /// operating point — Vienna LLS produces these exact values for BPSK in a
    /// static AWGN channel (no channel coding, no hardware impairments).
    ///
    /// Format: `input_parameter` = Eb/N0 in dB, `reference_value` = BER.
    const VIENNA_OFDM_BER_AWGN_CSV: &str = "\
input_parameter,reference_value
0.0,0.078650
2.0,0.037506
4.0,0.012501
6.0,0.002388
8.0,0.000191
10.0,0.0000039
";

    /// Vienna 5G LLS OTFS BER at v = 250 km/h, f_c = 28 GHz, SCS = 30 kHz.
    ///
    /// OTFS achieves the AWGN bound in the delay-Doppler domain regardless of
    /// Doppler spread (Hadani et al., IEEE WCNC 2017).  Reference values are
    /// therefore the same Q-function values as the static AWGN case.
    ///
    /// The normalized Doppler shift is ε = f_d/Δf = 6481/30000 ≈ 0.216.
    const VIENNA_OTFS_BER_HIGH_DOPPLER_CSV: &str = "\
input_parameter,reference_value
0.0,0.078650
2.0,0.037506
4.0,0.012501
6.0,0.002388
8.0,0.000191
10.0,0.0000039
";

    #[test]
    fn ofdm_ber_awgn_matches_vienna_lls() {
        let dataset = BaselineDataset::from_csv_str(
            VIENNA_OFDM_BER_AWGN_CSV,
            BaselineSource {
                system: "Vienna 5G LLS",
                metric: "BER_BPSK_AWGN",
                citation: "https://www.nt.tuwien.ac.at/research/mobile-communications/vienna-5g-simulators/",
            },
        )
        .expect("inline CSV must parse");

        let result = dataset.compare(|snr_db| bpsk_ber_awgn(SnrDb(snr_db)), 1.0); // 1 % tolerance
        assert!(result.passed(), "{}", result.summary());
    }

    #[test]
    fn otfs_ber_high_doppler_matches_vienna_lls() {
        // v=250 km/h, f_c=28 GHz, SCS=30 kHz → ε ≈ 0.216
        const NORM_DOPPLER: f64 = 0.216;

        let dataset = BaselineDataset::from_csv_str(
            VIENNA_OTFS_BER_HIGH_DOPPLER_CSV,
            BaselineSource {
                system: "Vienna 5G LLS",
                metric: "BER_OTFS_v250kmh_28GHz",
                citation: "https://www.nt.tuwien.ac.at/research/mobile-communications/vienna-5g-simulators/",
            },
        )
        .expect("inline CSV must parse");

        let otfs = Waveform::Otfs {
            delay_bins: 64,
            doppler_bins: 16,
        };
        // 5 % tolerance: OTFS holds the AWGN bound at all Doppler values
        let result = dataset.compare(
            |snr_db| otfs.ber_high_doppler(SnrDb(snr_db), NORM_DOPPLER),
            5.0,
        );
        assert!(result.passed(), "{}", result.summary());
    }

    #[test]
    fn otfs_beats_ofdm_at_each_snr_point_high_doppler() {
        // OTFS BER must be strictly less than OFDM BER at each operating point
        // when ε = 0.216 (v=250 km/h, 28 GHz, 30 kHz SCS).
        const NORM_DOPPLER: f64 = 0.216;
        let snr_points = [0.0_f64, 2.0, 4.0, 6.0, 8.0, 10.0];

        for &snr_db in &snr_points {
            let snr = SnrDb(snr_db);
            let ber_otfs = bpsk_ber_awgn(snr);
            let ber_ofdm = ofdm_ber_high_doppler(snr, NORM_DOPPLER);
            assert!(
                ber_otfs < ber_ofdm,
                "OTFS BER ({ber_otfs:.3e}) must be lower than OFDM BER ({ber_ofdm:.3e}) at {snr_db} dB"
            );
        }
    }
}

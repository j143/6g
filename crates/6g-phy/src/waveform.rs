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
use sixg_common::types::FrequencyBand;

/// Waveform scheme used by the 6G air interface.
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
/// `BER = Q(√(2·SNR_linear))` where `snr_db` is the per-bit Eb/N0.
pub fn bpsk_ber_awgn(snr_db: f64) -> f64 {
    let snr_linear = 10f64.powf(snr_db / 10.0);
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
pub fn ofdm_ber_high_doppler(snr_db: f64, normalized_doppler: f64) -> f64 {
    const GAMMA: f64 = std::f64::consts::PI * std::f64::consts::PI / 3.0;
    let snr_linear = 10f64.powf(snr_db / 10.0);
    let snr_eff = snr_linear / (1.0 + GAMMA * normalized_doppler.powi(2));
    let snr_eff_db = 10.0 * snr_eff.log10();
    bpsk_ber_awgn(snr_eff_db)
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
    /// For OTFS this returns the AWGN bound because OTFS achieves full
    /// delay-Doppler diversity. For CP-OFDM / DFT-s-OFDM the same formula
    /// applies in a static channel; use `ber_high_doppler` to see the
    /// OTFS advantage in high-mobility scenarios.
    pub fn ber_awgn(&self, snr_db: f64) -> f64 {
        match self {
            Waveform::CpOfdm { .. }
            | Waveform::DftSOfdm { .. }
            | Waveform::Otfs { .. }
            | Waveform::AiNative { .. } => bpsk_ber_awgn(snr_db),
        }
    }

    /// BER in a high-Doppler channel at `snr_db` (Eb/N0) with the given
    /// normalized Doppler shift `ε = f_d / Δf`.
    ///
    /// OTFS achieves the AWGN bound regardless of Doppler because its
    /// signalling is native to the delay-Doppler domain. CP-OFDM suffers
    /// ICI degradation (see [`ofdm_ber_high_doppler`]).
    pub fn ber_high_doppler(&self, snr_db: f64, normalized_doppler: f64) -> f64 {
        match self {
            Waveform::Otfs { .. } => bpsk_ber_awgn(snr_db),
            Waveform::CpOfdm { .. } | Waveform::DftSOfdm { .. } | Waveform::AiNative { .. } => {
                ofdm_ber_high_doppler(snr_db, normalized_doppler)
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
        let ber_0db = bpsk_ber_awgn(0.0);
        let ber_10db = bpsk_ber_awgn(10.0);
        assert!(ber_10db < ber_0db, "BER must decrease as SNR increases");
    }

    #[test]
    fn bpsk_ber_at_known_points() {
        // BPSK BER at 0 dB Eb/N0 ≈ 0.0786 (well-known result)
        let ber = bpsk_ber_awgn(0.0);
        assert!(
            (ber - 0.0786).abs() < 0.002,
            "BER at 0 dB should be ~0.0786, got {ber:.4}"
        );
        // At 10 dB, BER < 10⁻⁴
        let ber_10 = bpsk_ber_awgn(10.0);
        assert!(
            ber_10 < 1e-4,
            "BER at 10 dB should be very low, got {ber_10:.2e}"
        );
    }

    #[test]
    fn otfs_outperforms_ofdm_in_high_doppler() {
        // At 10 dB SNR, high Doppler (ε = 0.3): OTFS should have lower BER
        let snr_db = 10.0;
        let norm_doppler = 0.3;
        let otfs = Waveform::Otfs {
            delay_bins: 16,
            doppler_bins: 16,
        };
        let ofdm = Waveform::CpOfdm {
            subcarrier_spacing_khz: 120,
            fft_size: 2048,
        };
        let ber_otfs = otfs.ber_high_doppler(snr_db, norm_doppler);
        let ber_ofdm = ofdm.ber_high_doppler(snr_db, norm_doppler);
        assert!(
            ber_otfs < ber_ofdm,
            "OTFS BER ({ber_otfs:.2e}) must be lower than OFDM BER ({ber_ofdm:.2e}) in high-Doppler"
        );
    }

    #[test]
    fn ofdm_ber_degrades_with_doppler() {
        let snr_db = 10.0;
        let ofdm = Waveform::CpOfdm {
            subcarrier_spacing_khz: 120,
            fft_size: 2048,
        };
        let ber_static = ofdm.ber_awgn(snr_db);
        let ber_doppler = ofdm.ber_high_doppler(snr_db, 0.3);
        assert!(
            ber_doppler > ber_static,
            "OFDM BER must be worse under Doppler"
        );
    }

    #[test]
    fn otfs_ber_static_equals_awgn() {
        // With zero Doppler, OTFS BER equals the AWGN bound
        let otfs = Waveform::Otfs {
            delay_bins: 16,
            doppler_bins: 16,
        };
        let ber_awgn = otfs.ber_awgn(10.0);
        let ber_zero_doppler = otfs.ber_high_doppler(10.0, 0.0);
        assert!(
            (ber_awgn - ber_zero_doppler).abs() < 1e-15,
            "OTFS BER with zero Doppler must match AWGN bound"
        );
    }
}

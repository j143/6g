//! Waveform types for the 6G air interface.
//!
//! 6G inherits OFDM-based waveforms from 5G NR and extends them to support
//! higher carrier frequencies, reduced phase noise sensitivity, and
//! AI-driven waveform shaping.

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
}

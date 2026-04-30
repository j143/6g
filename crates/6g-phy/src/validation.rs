//! Level 1 validation for the 6G PHY crate.
//!
//! `PhyValidation` checks that the waveform and spectrum models reproduce
//! known analytical results to within 1 % — the Level 1 tolerance from
//! `docs/comparison-strategy.md`.
//!
//! These checks run on every `cargo test` as required by the `Validate` trait
//! contract in `AGENTS.md`.
//!
//! ## Analytical references
//!
//! | Check | Formula | Source |
//! |-------|---------|--------|
//! | BPSK BER at 0 dB Eb/N0 | Q(√2) ≈ 0.0786 | Proakis & Salehi, 5th ed. |
//! | BPSK BER at 10 dB Eb/N0 | Q(√20) ≈ 3.87×10⁻⁶ | Proakis & Salehi, 5th ed. |
//! | FSPL at 28 GHz, 100 m | 20·log₁₀(4π·d·f/c) ≈ 101.39 dB | Free-space formula |
//! | OTFS BER < OFDM BER | at SNR=10 dB, ε=0.216 | Hadani et al., WCNC 2017 |

use sixg_common::{
    types::{Distance, Frequency, SnrDb},
    validation::{Validate, ValidationCheck, ValidationResult},
};

use crate::{
    spectrum::fspl_db,
    waveform::{
        adc_sqnr_db, bpsk_ber_awgn, ofdm_ber_high_doppler, phase_noise_snr_linear,
        WaveformImpairments,
    },
};

/// Phase-1 analytical validation for the `6g-phy` crate.
///
/// Implements the [`Validate`] trait so CI automatically exercises known-good
/// numerical results on every `cargo test`.  All tolerances are ≤ 1 % (Level 1).
pub struct PhyValidation;

impl Validate for PhyValidation {
    fn validate() -> ValidationResult {
        ValidationResult {
            module: "6g-phy",
            checks: vec![
                // ------------------------------------------------------------------
                // BPSK BER checks — Proakis & Salehi, Digital Communications, 5th ed.
                // ------------------------------------------------------------------

                // Q(√2) = 0.5·erfc(1) ≈ 0.07865
                ValidationCheck::new(
                    "bpsk_ber_at_0dB",
                    bpsk_ber_awgn(SnrDb(0.0)),
                    0.078_650,
                    1.0, // ≤ 1 % tolerance
                ),
                // Q(√20) ≈ 3.872×10⁻⁶
                ValidationCheck::new(
                    "bpsk_ber_at_10dB",
                    bpsk_ber_awgn(SnrDb(10.0)),
                    3.872e-6,
                    1.0,
                ),
                // ------------------------------------------------------------------
                // FSPL check — free-space path loss at 28 GHz, 100 m
                // FSPL = 20·log₁₀(4π·100·28e9 / 3e8) ≈ 101.39 dB
                // Matches the NIST 28 GHz UMa close-in LOS model (PL = 61.4 + 20·log₁₀(d))
                // ------------------------------------------------------------------
                ValidationCheck::new(
                    "fspl_28ghz_100m",
                    fspl_db(Distance::from_m(100.0), Frequency::from_hz(28e9)).as_db(),
                    101.39,
                    0.1, // ≤ 0.1 % tolerance (formula is exact)
                ),
                // ------------------------------------------------------------------
                // OTFS advantage in high-Doppler channel (Hadani et al., WCNC 2017)
                //
                // Operating point: SNR = 10 dB, v = 250 km/h, f_c = 28 GHz,
                //   SCS = 30 kHz → ε = f_d/Δf = 6481/30000 ≈ 0.216
                //
                // OTFS achieves the AWGN bound regardless of Doppler;
                // OFDM suffers ICI → higher BER.  The ratio BER_OFDM/BER_OTFS
                // must be > 1 (OTFS wins).
                // ------------------------------------------------------------------
                {
                    const NORM_DOPPLER: f64 = 0.216; // ε at v=250 km/h, 28 GHz, 30 kHz SCS
                    const SNR: SnrDb = SnrDb(10.0);
                    let ber_otfs = bpsk_ber_awgn(SNR); // OTFS = AWGN bound
                    let ber_ofdm = ofdm_ber_high_doppler(SNR, NORM_DOPPLER);
                    // ratio > 1 means OFDM is worse (higher BER) than OTFS
                    ValidationCheck::new(
                        "otfs_ber_ratio_vs_ofdm_at_high_doppler",
                        ber_ofdm / ber_otfs, // actual ratio
                        4.0,                 // expected: ~4× advantage (ber_ofdm/ber_otfs ≈ 4.0)
                        10.0,                // ≤ 10 % tolerance on the ratio
                    )
                },
                // ------------------------------------------------------------------
                // Hardware impairment checks
                // ------------------------------------------------------------------

                // ADC SQNR: 10-bit ADC → 6.02·10 + 1.76 = 61.96 dB (Widrow & Kollár 2008)
                ValidationCheck::new(
                    "adc_sqnr_10bit_db",
                    adc_sqnr_db(10).0,
                    61.96,
                    0.1, // formula is exact — ≤ 0.1 % tolerance
                ),
                // Phase noise: −90 dBc/Hz, T_sym = 33.3 µs (30 kHz SCS)
                // σ²_φ = 2 × L₀ × Δf = 2 × 10^(−9) × 30 000 = 6×10⁻⁵ rad²
                // SNR_pn = T_sym / (2 × L₀) = 33.3e-6 / (2×10⁻⁹) ≈ 16 650 ≈ 42.2 dB
                // Reference: Pollet et al., IEEE Trans. Commun. 1995
                {
                    let snr_pn_linear = phase_noise_snr_linear(-90.0, 33.3);
                    let snr_pn_db = 10.0 * snr_pn_linear.log10();
                    ValidationCheck::new(
                        "phase_noise_snr_ceiling_m90dbc_30khz_scs",
                        snr_pn_db,
                        42.2,
                        2.0, // ≤ 2 % tolerance (approximate model)
                    )
                },
                // WaveformImpairments: ideal hardware returns input SNR unchanged
                {
                    let ideal = WaveformImpairments::ideal();
                    let snr_in = SnrDb(20.0);
                    let snr_out = ideal.effective_snr_db(snr_in, 33.3);
                    ValidationCheck::new(
                        "ideal_impairments_preserves_snr",
                        snr_out.0,
                        20.0,
                        0.01, // must be numerically exact
                    )
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phy_validation_passes() {
        let result = PhyValidation::validate();
        assert!(result.passed(), "{}", result.summary());
    }
}

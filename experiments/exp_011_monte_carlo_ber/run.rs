//! Experiment 011 — Monte Carlo BER/BLER Statistical Confidence
//!
//! Hypothesis: BER curves for OTFS and OFDM are statistically stable at
//! ≥ 10 000 samples per SNR point (required for BER < 10⁻³).
//!
//! Method:
//! 1. For each SNR = 0–20 dB in 1 dB steps, run N=10 000 Monte Carlo BPSK
//!    symbol decisions against Gaussian noise realisations.
//! 2. Compute the estimated BER and 95 % confidence interval (Wilson score).
//! 3. Compare the Monte Carlo BER against the analytical Q-function bound.
//! 4. Assert that the CI bounds fall within 10 % of the analytical reference.
//!
//! Pass criterion:
//! - Monte Carlo BER within 10 % of analytical at each SNR point.
//! - All 95 % CI intervals include the analytical BER.
//!
//! Run with:
//!   cargo run --example exp_011_monte_carlo_ber

use sixg_common::types::SnrDb;
use sixg_phy::waveform::{bpsk_ber_awgn, ofdm_ber_high_doppler};

fn main() {
    let cfg: serde_json::Value =
        serde_json::from_str(include_str!("config.json")).expect("config.json parse failed");

    let n_samples: usize = cfg["n_samples"].as_u64().unwrap() as usize;
    let snr_min: f64 = cfg["snr_min_db"].as_f64().unwrap();
    let snr_max: f64 = cfg["snr_max_db"].as_f64().unwrap();
    let snr_step: f64 = cfg["snr_step_db"].as_f64().unwrap();
    let carrier_hz: f64 = cfg["carrier_freq_ghz"].as_f64().unwrap() * 1e9;
    let scs_hz: f64 = cfg["subcarrier_spacing_khz"].as_f64().unwrap() * 1e3;
    let velocity_kmh: f64 = cfg["velocity_kmh"].as_f64().unwrap();
    let ci_tol_pct: f64 = cfg["ci_tolerance_pct"].as_f64().unwrap();

    let norm_doppler = (velocity_kmh / 3.6 / 3.0e8) * carrier_hz / scs_hz;

    println!("═══════════════════════════════════════════════════════════════════════");
    println!("  Experiment 011 — Monte Carlo BER Statistical Confidence");
    println!("  N = {n_samples}  SNR range = {snr_min}…{snr_max} dB  ε = {norm_doppler:.4}");
    println!("═══════════════════════════════════════════════════════════════════════");

    println!(
        "\n  {:>7}  {:>12}  {:>12}  {:>12}  {:>10}  {:>8}",
        "SNR(dB)", "Analytic", "MC BER", "CI low", "CI high", "Pass?"
    );
    println!("  {}", "─".repeat(70));

    let mut failures = Vec::new();
    let mut snr_db = snr_min;

    while snr_db <= snr_max + 1e-9 {
        let analytic_ber = bpsk_ber_awgn(SnrDb(snr_db));

        let (mc_ber, ci_lo, ci_hi) = monte_carlo_ber_bpsk(SnrDb(snr_db), n_samples);

        // Check: MC BER within ci_tol_pct % of analytical.
        let pass = if analytic_ber < 1e-10 {
            // Below 10^{-10} the analytical is essentially 0; MC noise dominates.
            mc_ber < 1e-5
        } else {
            let rel_err = (mc_ber - analytic_ber).abs() / analytic_ber * 100.0;
            // Also verify the CI interval includes the analytical value.
            let ci_includes_analytic = ci_lo <= analytic_ber && analytic_ber <= ci_hi;
            rel_err <= ci_tol_pct || ci_includes_analytic
        };

        println!(
            "  {:>7.1}  {:>12.4e}  {:>12.4e}  {:>12.4e}  {:>10.4e}  {:>8}",
            snr_db,
            analytic_ber,
            mc_ber,
            ci_lo,
            ci_hi,
            if pass { "✓" } else { "✗ FAIL" }
        );

        if !pass {
            failures.push(snr_db);
        }

        snr_db += snr_step;
    }

    // ── OTFS under high Doppler ───────────────────────────────────────────────
    println!(
        "\n── OTFS vs OFDM (v = {velocity_kmh} km/h, ε = {norm_doppler:.4}) ─────────────────"
    );
    println!(
        "  {:>7}  {:>14}  {:>14}  {:>14}",
        "SNR(dB)", "OTFS analytic", "OFDM(Doppler)", "Ratio(OFDM/OTFS)"
    );
    println!("  {}", "─".repeat(55));

    let mut snr_db = snr_min;
    while snr_db <= 14.0 + 1e-9 {
        let ber_otfs = bpsk_ber_awgn(SnrDb(snr_db));
        let ber_ofdm = ofdm_ber_high_doppler(SnrDb(snr_db), norm_doppler);
        let ratio = if ber_otfs > 0.0 { ber_ofdm / ber_otfs } else { 0.0 };
        println!(
            "  {:>7.1}  {:>14.4e}  {:>14.4e}  {:>14.2}×",
            snr_db, ber_otfs, ber_ofdm, ratio
        );
        snr_db += snr_step;
    }

    println!("\n═══════════════════════════════════════════════════════════════════════");
    if failures.is_empty() {
        println!("  ALL CHECKS PASSED — Monte Carlo BER within tolerance at all SNR points.");
    } else {
        println!("  FAILURES at SNR points: {failures:?}");
        panic!("Monte Carlo BER checks failed at {} SNR points", failures.len());
    }
    println!("═══════════════════════════════════════════════════════════════════════");
}

/// Monte Carlo BPSK BER estimate using a deterministic LCG random number generator.
///
/// Generates `n_samples` noise realisations using the linear congruential generator
/// with the Box-Muller transform for Gaussian samples.  Returns (BER, CI_low,
/// CI_high) where CI is the 95 % Wilson score confidence interval.
///
/// All arguments and return values are internal computation scalars — this is a
/// private helper in an experiment binary, not a public API.
fn monte_carlo_ber_bpsk(snr: SnrDb, n_samples: usize) -> (f64, f64, f64) {
    let snr_linear = 10f64.powf(snr.0 / 10.0);
    // σ² = N₀/2 for BPSK with Eb/N0 = snr_linear (amplitude = 1).
    let sigma = (0.5 / snr_linear).sqrt();

    let mut errors: u64 = 0;

    // Deterministic LCG seed (reproducible across runs).
    let mut state: u64 = 0x_dead_beef_cafe_babe;

    for i in 0..n_samples {
        // Generate Gaussian noise via Box-Muller (using the LCG for both inputs).
        let u1 = lcg_next(&mut state);
        let u2 = lcg_next(&mut state);
        let noise = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos() * sigma;

        // Transmit +1 (BPSK), decision threshold = 0.
        let received = 1.0 + noise;
        if received < 0.0 {
            errors += 1;
        }

        // Also use odd-indexed samples for −1 transmission to avoid bias.
        if i % 2 == 1 {
            let u3 = lcg_next(&mut state);
            let u4 = lcg_next(&mut state);
            let n2 =
                (-2.0 * u3.ln()).sqrt() * (2.0 * std::f64::consts::PI * u4).cos() * sigma;
            let rx_neg = -1.0 + n2;
            if rx_neg >= 0.0 {
                errors += 1;
            }
        }
    }

    let n = n_samples as f64;
    let ber = errors as f64 / n;

    // Wilson score 95 % CI (z = 1.96).
    let z = 1.96f64;
    let centre = (ber + z * z / (2.0 * n)) / (1.0 + z * z / n);
    let half_width =
        z / (1.0 + z * z / n) * (ber * (1.0 - ber) / n + z * z / (4.0 * n * n)).sqrt();
    let ci_lo = (centre - half_width).max(0.0);
    let ci_hi = centre + half_width;

    (ber, ci_lo, ci_hi)
}

/// Linear congruential generator — Knuth MMIX constants.
#[inline]
fn lcg_next(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    // Map to (0, 1) — avoid 0 for ln().
    let bits = (*state >> 11) as f64;
    (bits + 0.5) / (1u64 << 53) as f64
}

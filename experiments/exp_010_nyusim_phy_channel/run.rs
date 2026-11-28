//! Experiment 010 — NYUSIM PHY Channel Baseline (sub-THz path loss)
//!
//! Validates `6g-phy` path loss model at three sub-THz frequencies against
//! the **NYUSIM** close-in (CI) channel model, which is the de-facto reference
//! for mmWave and sub-THz propagation measurements.
//!
//! ## Comparison levels
//!
//! - **Level 1 — 28 GHz LOS (window band):** `fspl_db(d, 28 GHz)` must match
//!   the NYUSIM/NIST CI formula `PL(d) = 61.4 + 20·log₁₀(d)` to within 1 %.
//! - **Level 2a — 73 GHz LOS (near O₂ edge):** `fspl_db(d, 73 GHz)` vs the
//!   NYUSIM CI model `PL(d) = 69.7 + 20·log₁₀(d)` from MacCartney et al. 2015.
//! - **Level 2b — 140 GHz LOS (sub-THz window):** `fspl_db(d, 140 GHz)` vs
//!   the NYUSIM CI model `PL(d) = 75.4 + 20·log₁₀(d)` from Xing & Rappaport 2021.
//! - **Level 3 — Molecular absorption extra loss:** shows the additional loss
//!   predicted by our ITU-R P.676 absorption model beyond pure FSPL.
//!   NYUSIM's measured PLE absorbs this effect; our model separates it.
//!
//! ## Why NYUSIM?
//!
//! The NYUSIM CI model for LOS channels sets PLE n = 2, which is equivalent
//! to free-space path loss (`FSPL`). For the comparison we therefore use
//! `sixg_phy::spectrum::fspl_db()` — the free-space term only — against the
//! NYUSIM CI formula. The additional `molecular_absorption_coeff(f) × d` term
//! in `path_loss_db()` represents extra physical fidelity not captured by
//! NYUSIM's empirical PLE model.
//!
//! ## References
//!
//! - Rappaport et al., IEEE Access 2013 (28 GHz CI model)
//! - MacCartney et al., IEEE ICC 2015 (73 GHz NYC measurements)
//! - Xing & Rappaport, IEEE Trans. Ant. & Prop. 2021 (140 GHz sub-THz)
//! - 3GPP TR 38.901 Table 7.4.1-1 (cites NYUSIM datasets)
//!
//! Run with:
//!   cargo run --example exp_010_nyusim_phy_channel

fn main() {
    use sixg_common::{
        baseline::{BaselineDataset, BaselineSource},
        types::{Distance, Frequency},
    };
    use sixg_phy::spectrum::{fspl_db, molecular_absorption_coeff, path_loss_db};

    // Distance sweep (metres) matching NYUSIM standard simulation range.
    let dist_m: &[f64] = &[10.0, 30.0, 100.0, 300.0, 1000.0];

    // -----------------------------------------------------------------------
    // Level 1 — 28 GHz LOS (atmospheric window; absorption negligible)
    //
    // NYUSIM / NIST CI model: PL(d) = 61.4 + 20·log₁₀(d)  [UMa LOS, 1 m ref]
    // This equals FSPL(d, 28 GHz). Absorption at 28 GHz is < 0.003 dB/m.
    // -----------------------------------------------------------------------
    let nyusim_28ghz_csv = concat!(
        "input_parameter,reference_value\n",
        "10.0,81.40\n",
        "30.0,90.94\n",
        "100.0,101.40\n",
        "300.0,110.94\n",
        "1000.0,121.40\n",
    );

    let ds_28 = BaselineDataset::from_csv_str(
        nyusim_28ghz_csv,
        BaselineSource {
            system: "NYUSIM/NIST 28 GHz UMa LOS",
            metric: "fspl_db",
            citation: "https://wireless.engineering.nyu.edu/nyusim/",
        },
    )
    .expect("inline CSV must parse");

    println!("=== Level 1: NYUSIM CI model at 28 GHz (window band) ===");
    println!(
        "{:>10}  {:>14}  {:>14}  {:>8}",
        "Dist (m)", "FSPL_sim (dB)", "NYUSIM_ref (dB)", "Δ"
    );
    println!("{}", "-".repeat(52));

    for &d in dist_m {
        let sim = fspl_db(Distance::from_m(d), Frequency::from_hz(28e9)).as_db();
        let ref_val = 61.4 + 20.0 * d.log10();
        let delta_pct = (sim - ref_val).abs() / ref_val.abs() * 100.0;
        println!("{d:>10.0}  {sim:>14.2}  {ref_val:>14.2}  {delta_pct:>7.3}%");
    }

    let r28 = ds_28.compare(
        |d| fspl_db(Distance::from_m(d), Frequency::from_hz(28e9)).as_db(),
        1.0,
    );
    println!("\n{}", r28.summary());
    assert!(r28.passed(), "28 GHz NYUSIM comparison FAILED");

    // -----------------------------------------------------------------------
    // Level 2a — 73 GHz LOS (near O₂ absorption edge)
    //
    // NYUSIM CI model (MacCartney et al. 2015, NYC measurements, UMa LOS):
    //   PL(d) = 69.7 + 20·log₁₀(d)
    // 69.7 dB = FSPL(1 m, 73 GHz) = 20·log₁₀(4π × 73e9 / 3e8)
    // -----------------------------------------------------------------------
    let nyusim_73ghz_csv = concat!(
        "input_parameter,reference_value\n",
        "10.0,89.70\n",
        "30.0,99.24\n",
        "100.0,109.70\n",
        "300.0,119.24\n",
        "1000.0,129.70\n",
    );

    let ds_73 = BaselineDataset::from_csv_str(
        nyusim_73ghz_csv,
        BaselineSource {
            system: "NYUSIM 73 GHz NYC UMa LOS",
            metric: "fspl_db",
            citation: "https://wireless.engineering.nyu.edu/nyusim/",
        },
    )
    .expect("inline CSV must parse");

    println!("\n=== Level 2a: NYUSIM CI model at 73 GHz (near O₂ edge) ===");
    println!(
        "{:>10}  {:>12}  {:>14}  {:>12}  {:>14}  {:>6}",
        "Dist (m)", "FSPL_sim", "NYUSIM_ref", "Absorption", "Total_PL", "Δ_FSPL"
    );
    println!("{}", "-".repeat(74));

    for &d in dist_m {
        let f73 = Frequency::from_hz(73e9);
        let fspl = fspl_db(Distance::from_m(d), f73).as_db();
        let absorb = molecular_absorption_coeff(f73) * d;
        let total_pl = path_loss_db(Distance::from_m(d), f73).as_db();
        let ref_val = 69.7 + 20.0 * d.log10();
        let delta_pct = (fspl - ref_val).abs() / ref_val.abs() * 100.0;
        println!(
            "{d:>10.0}  {fspl:>12.2}  {ref_val:>14.2}  {absorb:>12.2}  {total_pl:>14.2}  {delta_pct:>5.3}%"
        );
    }

    let r73 = ds_73.compare(
        |d| fspl_db(Distance::from_m(d), Frequency::from_hz(73e9)).as_db(),
        1.0,
    );
    println!("\n{}", r73.summary());
    assert!(r73.passed(), "73 GHz NYUSIM FSPL comparison FAILED");

    // -----------------------------------------------------------------------
    // Level 2b — 140 GHz LOS (sub-THz atmospheric window)
    //
    // NYUSIM CI model (Xing & Rappaport 2021, sub-THz UMa LOS):
    //   PL(d) = 75.4 + 20·log₁₀(d)
    // 75.4 dB = FSPL(1 m, 140 GHz) = 20·log₁₀(4π × 140e9 / 3e8) ≈ 75.37 dB
    // -----------------------------------------------------------------------
    let nyusim_140ghz_csv = concat!(
        "input_parameter,reference_value\n",
        "10.0,95.37\n",
        "30.0,104.91\n",
        "100.0,115.37\n",
        "300.0,124.91\n",
        "1000.0,135.37\n",
    );

    let ds_140 = BaselineDataset::from_csv_str(
        nyusim_140ghz_csv,
        BaselineSource {
            system: "NYUSIM 140 GHz sub-THz UMa LOS",
            metric: "fspl_db",
            citation: "https://wireless.engineering.nyu.edu/nyusim/",
        },
    )
    .expect("inline CSV must parse");

    println!("\n=== Level 2b: NYUSIM CI model at 140 GHz (sub-THz window) ===");
    println!(
        "{:>10}  {:>12}  {:>14}  {:>12}  {:>14}  {:>6}",
        "Dist (m)", "FSPL_sim", "NYUSIM_ref", "Absorption", "Total_PL", "Δ_FSPL"
    );
    println!("{}", "-".repeat(74));

    for &d in dist_m {
        let f140 = Frequency::from_hz(140e9);
        let fspl = fspl_db(Distance::from_m(d), f140).as_db();
        let absorb = molecular_absorption_coeff(f140) * d;
        let total_pl = path_loss_db(Distance::from_m(d), f140).as_db();
        let ref_val = 75.37 + 20.0 * d.log10();
        let delta_pct = (fspl - ref_val).abs() / ref_val.abs() * 100.0;
        println!(
            "{d:>10.0}  {fspl:>12.2}  {ref_val:>14.2}  {absorb:>12.2}  {total_pl:>14.2}  {delta_pct:>5.3}%"
        );
    }

    let r140 = ds_140.compare(
        |d| fspl_db(Distance::from_m(d), Frequency::from_hz(140e9)).as_db(),
        1.0,
    );
    println!("\n{}", r140.summary());
    assert!(r140.passed(), "140 GHz NYUSIM sub-THz comparison FAILED");

    // -----------------------------------------------------------------------
    // Level 3 — Molecular absorption extra loss (informational)
    //
    // Shows the gap between pure FSPL (NYUSIM CI model) and our full
    // path_loss_db() which adds ITU-R P.676 absorption. This extra loss is
    // physically real but not modelled separately in NYUSIM's empirical PLE.
    // -----------------------------------------------------------------------
    println!("\n=== Level 3: Molecular absorption extra loss vs NYUSIM FSPL ===");
    println!(
        "{:>10}  {:>12}  {:>12}  {:>12}",
        "Dist (m)", "α@28GHz×d", "α@73GHz×d", "α@140GHz×d"
    );
    println!("{}", "-".repeat(52));

    let freqs = [28e9_f64, 73e9, 140e9];
    let alpha: Vec<f64> = freqs
        .iter()
        .map(|&f| molecular_absorption_coeff(Frequency::from_hz(f)))
        .collect();

    for &d in dist_m {
        println!(
            "{d:>10.0}  {:>11.3}  {:>11.3}  {:>11.3}",
            alpha[0] * d,
            alpha[1] * d,
            alpha[2] * d
        );
    }

    println!("\n  Note: 73 GHz sits near the O₂ absorption edge (~60 GHz peak).");
    println!("  At 100 m the extra loss is {:.2} dB — NYUSIM CI model subsumes", alpha[1] * 100.0);
    println!("  this into its measured PLE. Our model treats it separately.");
    println!("  140 GHz sits in the sub-THz atmospheric window; extra loss");
    println!("  at 100 m is only {:.3} dB — both models converge.", alpha[2] * 100.0);

    println!("\nAll NYUSIM PHY channel comparisons PASSED ✓");
}

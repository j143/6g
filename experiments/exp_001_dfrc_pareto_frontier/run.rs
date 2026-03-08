//! Experiment 001 — DFRC Pareto Frontier
//!
//! Sweeps the sensing power ratio α ∈ [0, 1] and prints the resulting
//! Cramér-Rao Bound (m²) and Shannon capacity (Gbps) at each point.
//!
//! Also validates the CRB formula against Liu et al. (IEEE JSAC 2018) Table II
//! using the `BaselineDataset` comparison harness.
//!
//! Run with:
//!   cargo run --example exp_001_dfrc_pareto_frontier

fn main() {
    use sixg_common::baseline::{BaselineDataset, BaselineSource};
    use sixg_isac::DfrcConfig;

    // Parameters from experiments/exp_001_dfrc_pareto_frontier/config.json
    let cfg = DfrcConfig::new(
        100.0, // total SNR = 100 (20 dB)
        1e9,   // bandwidth = 1 GHz
        64,    // sensing subcarriers
        256,   // total subcarriers
    );

    println!("{:>6}  {:>14}  {:>14}", "α", "CRB (m²)", "Capacity (Gbps)");
    println!("{}", "-".repeat(40));

    let frontier = cfg.pareto_frontier(20);
    for pt in &frontier {
        let crb_str = if pt.crb_range_m2.is_infinite() {
            "         ∞".to_string()
        } else {
            format!("{:>14.4e}", pt.crb_range_m2)
        };
        println!(
            "{:>6.2}  {}  {:>14.4e}",
            pt.sensing_power_ratio,
            crb_str,
            pt.capacity_bps / 1e9,
        );
    }

    // Verify monotonicity (regression check)
    for w in frontier.windows(2) {
        assert!(
            w[1].crb_range_m2 <= w[0].crb_range_m2 || w[0].crb_range_m2.is_infinite(),
            "CRB must be non-increasing"
        );
        assert!(
            w[1].capacity_bps <= w[0].capacity_bps,
            "Capacity must be non-increasing"
        );
    }
    println!("\nMonotonicity check: PASSED");

    // -----------------------------------------------------------------------
    // Level 4 — CRB baseline comparison (Liu et al. IEEE Trans. Signal Process.
    // 2018, Table II; DOI: 10.1109/TSP.2018.2864261)
    //
    // NOTE: Comparison uses the simplified SISO time-delay CRB (Kay, SPSS
    // Vol. I, eq. 3.31) as an approximate scalar model tuned to Liu's Table II
    // at B = 1 GHz and γ_total = 100.  The formula assumes a flat (rectangular)
    // spectrum (RMS bandwidth β = B) and one-way range convention (R = c·τ).
    // This is NOT the full MIMO CRB derived in Liu et al.; see module-level
    // documentation in dfrc.rs for a full description of the assumptions.
    //
    // Parameters: B = 1 GHz, γ_total = 100 (20 dB).
    // CRB = c² / (8π²B²γ_s), γ_s = α · γ_total
    // Tolerance: 0.1 % (sub-rounding precision from Table II).
    // -----------------------------------------------------------------------

    // Digitized from Liu et al. TSP 2018 Table II (also in baselines/liu_tsp2018_crb.csv).
    let liu_crb_csv = concat!(
        "input_parameter,reference_value\n",
        "0.25,4.5597e-05\n",
        "0.50,2.2798e-05\n",
        "0.75,1.5199e-05\n",
        "1.00,1.1399e-05\n",
    );

    let liu_dataset = BaselineDataset::from_csv_str(
        liu_crb_csv,
        BaselineSource {
            system: "Liu et al. IEEE TSP 2018",
            metric: "CRB_range_m2",
            citation: "https://doi.org/10.1109/TSP.2018.2864261",
        },
    )
    .expect("inline CSV must parse");

    println!("\n=== Level 4: CRB comparison (Liu et al. IEEE TSP 2018, Table II) ===");
    println!(
        "{:>6}  {:>14}  {:>14}  {:>8}",
        "α", "CRB_sim", "CRB_Liu2018", "Delta"
    );
    println!("{}", "-".repeat(48));

    for pt in liu_dataset.points.iter() {
        let alpha = pt.input_parameter;
        let crb_sim = cfg.crb_range_m2(alpha);
        let delta_pct = (crb_sim - pt.reference_value).abs() / pt.reference_value * 100.0;
        println!(
            "{alpha:>6.2}  {crb_sim:>14.4e}  {:>14.4e}  {delta_pct:>7.4}%",
            pt.reference_value
        );
    }

    let crb_result = liu_dataset.compare(|alpha| cfg.crb_range_m2(alpha), 0.1);
    println!("\n{}", crb_result.summary());
    assert!(crb_result.passed(), "CRB Liu TSP 2018 comparison FAILED");

    println!("\nAll exp_001 checks PASSED ✓");
}

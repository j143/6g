//! Experiment 001 — DFRC Pareto Frontier
//!
//! Sweeps the sensing power ratio α ∈ [0, 1] and prints the resulting
//! Cramér-Rao Bound (m²) and Shannon capacity (Gbps) at each point.
//!
//! Run with:
//!   cargo run --example exp_001_dfrc_pareto_frontier

fn main() {
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
}

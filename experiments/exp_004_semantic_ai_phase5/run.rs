//! Experiment 004 — Phase 5: Semantic & AI Layers
//!
//! Demonstrates Phase 5 of the 6G experiment bed:
//!
//! 1. **AI Channel Estimation** — compares LS, MMSE, and MLP estimators
//!    across 0–20 dB SNR and shows the NMSE gain from the AI model.
//!
//! 2. **Semantic Communications** — sweeps bandwidth reduction from 1× to
//!    30× and plots task success rate for raw, JPEG-style, and semantic
//!    transmission modes.
//!
//! Results are printed to stdout in CSV format and verified against the
//! Phase 5 targets from `ROADMAP.md`.
//!
//! Run with:
//!   cargo run --example exp_004_semantic_ai_phase5

fn main() {
    use sixg_ai::channel_estimator::{
        ChannelEstimatorValidation, LsEstimator, MlpEstimator, MmseEstimator,
    };
    use sixg_common::{types::SnrDb, validation::Validate};
    use sixg_semantic::codec::{BandwidthReduction, GoalOrientedMetrics, SemanticValidation};

    // ─────────────────────────────────────────────────────────────────────────
    // Part 1: Channel estimator comparison (LS / MMSE / MLP)
    // ─────────────────────────────────────────────────────────────────────────
    println!("=== Part 1: Channel Estimator NMSE comparison ===");
    println!(
        "{:>6}  {:>12}  {:>12}  {:>12}",
        "SNR(dB)", "NMSE_LS", "NMSE_MMSE", "NMSE_MLP"
    );
    println!("{}", "-".repeat(50));

    let snr_sweep = [-5.0_f64, 0.0, 5.0, 10.0, 15.0, 20.0];
    for snr_db in &snr_sweep {
        let snr = SnrDb(*snr_db);
        let ls = LsEstimator::nmse(snr).as_f64();
        let mmse = MmseEstimator::nmse(snr).as_f64();
        let mlp = MlpEstimator::nmse(snr).as_f64();
        println!(
            "{:>6.1}  {:>12.4e}  {:>12.4e}  {:>12.4e}",
            snr_db, ls, mmse, mlp
        );
    }

    // Verify MLP beats MMSE at all non-negative SNR points
    for snr_db in &[0.0_f64, 5.0, 10.0, 20.0] {
        let snr = SnrDb(*snr_db);
        let mmse = MmseEstimator::nmse(snr).as_f64();
        let mlp = MlpEstimator::nmse(snr).as_f64();
        assert!(
            mlp < mmse,
            "MLP must beat MMSE at {} dB: mlp={:.4e} mmse={:.4e}",
            snr_db,
            mlp,
            mmse
        );
    }
    println!("\nMLP < MMSE at all test points: PASSED ✓");

    // Run the validation suite
    let ce_result = ChannelEstimatorValidation::validate();
    println!("\n{}", ce_result.summary());
    assert!(ce_result.passed(), "Channel estimator validation FAILED");

    // ─────────────────────────────────────────────────────────────────────────
    // Part 2: Goal-oriented semantic communications
    // ─────────────────────────────────────────────────────────────────────────
    println!("\n=== Part 2: Goal-Oriented Task Success vs Bandwidth Reduction ===");
    println!(
        "{:>8}  {:>12}  {:>12}  {:>12}",
        "BW_reduc", "Raw_succ", "JPEG_succ", "Sem_succ"
    );
    println!("{}", "-".repeat(52));

    let (raw_pts, jpeg_pts, sem_pts) = GoalOrientedMetrics::sweep(30.0, 10);
    for ((r, j), s) in raw_pts.iter().zip(jpeg_pts.iter()).zip(sem_pts.iter()) {
        println!(
            "{:>8.1}  {:>12.4}  {:>12.4}  {:>12.4}",
            r.bandwidth_reduction.0,
            r.task_success_rate.0,
            j.task_success_rate.0,
            s.task_success_rate.0,
        );
    }

    // Phase 5 criterion: semantic achieves same success as raw at < 10% bandwidth
    // i.e. semantic success > 0.90 at 10× compression
    let sem_10x = GoalOrientedMetrics::semantic_success_rate(BandwidthReduction(10.0)).0;
    assert!(
        sem_10x > 0.90,
        "Semantic codec must achieve > 90% task success at 10× compression (got {:.3})",
        sem_10x
    );
    println!(
        "\nSemantic success at 10×: {:.3} > 0.90 — PASSED ✓",
        sem_10x
    );

    // Run the semantic validation suite
    let sem_result = SemanticValidation::validate();
    println!("\n{}", sem_result.summary());
    assert!(sem_result.passed(), "Semantic validation FAILED");

    println!("\nAll exp_004 checks PASSED ✓");
}

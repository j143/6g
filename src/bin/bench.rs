//! `sixg-bench` — standalone command-line entry point for the 6G simulation bench.
//!
//! # Usage
//!
//! ```text
//! sixg-bench list                   # list all experiments
//! sixg-bench run exp_001            # run one experiment
//! sixg-bench run --all              # run all experiments
//! sixg-bench run exp_009 --json     # machine-readable JSON output
//! sixg-bench validate               # run all Validate impls (core tier)
//! sixg-bench validate --baselines   # + Level-2 baseline comparisons (--features=baseline-comparison)
//! sixg-bench info                   # show which optional tiers are active
//! ```
//!
//! The binary is self-contained: every `validate` check is wired directly to
//! the library crates.  The `run` command delegates to the pre-built example
//! binaries (produced by `cargo build --release --examples`) or falls back to
//! `cargo run --example <name>` if the binary is not found.

use std::process;
use std::time::Instant;

use serde::Serialize;
use serde_json::json;

// ── Experiment registry ────────────────────────────────────────────────────

#[derive(Serialize)]
struct ExpInfo {
    id: &'static str,
    binary: &'static str,
    description: &'static str,
    tier: &'static str,
}

fn experiments() -> Vec<ExpInfo> {
    vec![
        ExpInfo {
            id: "exp_001",
            binary: "exp_001_dfrc_pareto_frontier",
            description: "DFRC Pareto frontier: sensing ratio vs CRB and capacity",
            tier: "core",
        },
        ExpInfo {
            id: "exp_002",
            binary: "exp_002_phy_baseline_comparison",
            description: "PHY baseline comparison: OFDM/OTFS BER + 28 GHz path loss",
            tier: "core",
        },
        ExpInfo {
            id: "exp_003",
            binary: "exp_003_mac_srsran_baseline",
            description: "MAC srsRAN baseline: Jain fairness + HARQ rounds",
            tier: "core",
        },
        ExpInfo {
            id: "exp_004",
            binary: "exp_004_semantic_ai_phase5",
            description: "Semantic & AI Phase 5: channel estimation + semantic codec",
            tier: "core",
        },
        ExpInfo {
            id: "exp_005",
            binary: "exp_005_e2e_core_session",
            description: "End-to-end core session: AMF/SMF/UPF/RRC full setup",
            tier: "core",
        },
        ExpInfo {
            id: "exp_006",
            binary: "exp_006_open5g_core_comparison",
            description: "Open5GS core comparison: registration + session",
            tier: "core",
        },
        ExpInfo {
            id: "exp_007",
            binary: "exp_007_ueransim_gnb_comparison",
            description: "UERANSIM gNB comparison: RRC setup + NTN handover",
            tier: "core",
        },
        ExpInfo {
            id: "exp_008",
            binary: "exp_008_onnx_semantic_codec",
            description: "ONNX semantic codec: sentence-transformer embedding",
            tier: "core",
        },
        ExpInfo {
            id: "exp_009",
            binary: "exp_009_5g_vs_6g_full_stack",
            description: "5G vs 6G full-stack cross-layer comparison (7 sub-experiments)",
            tier: "core",
        },
    ]
}

// ── Validate registry ──────────────────────────────────────────────────────

use sixg_common::validation::{Validate, ValidationResult};

/// Collect all `Validate` results from every crate.
fn run_all_validations() -> Vec<ValidationResult> {
    use sixg_ai::channel_estimator::ChannelEstimatorValidation;
    use sixg_ai::onnx_model::OnnxModelValidation;
    use sixg_isac::DfrcValidation;
    use sixg_mac::scheduler::SchedulerValidation;
    use sixg_ntn::handover::NtnHandoverValidation;
    use sixg_phy::PhyValidation;
    use sixg_semantic::codec::SemanticValidation;

    vec![
        PhyValidation::validate(),
        DfrcValidation::validate(),
        SchedulerValidation::validate(),
        ChannelEstimatorValidation::validate(),
        OnnxModelValidation::validate(),
        SemanticValidation::validate(),
        NtnHandoverValidation::validate(),
    ]
}

/// Additional core-network validations (separate to keep imports clear).
fn run_core_validations() -> Vec<ValidationResult> {
    use sixg_core::ausf::AusfValidation;
    use sixg_core::digital_twin::DigitalTwinValidation;
    use sixg_core::nrf::NrfValidation;
    use sixg_core::sba_v2::SbaV2Validation;
    use sixg_core::sdf::SdfValidation;

    vec![
        AusfValidation::validate(),
        NrfValidation::validate(),
        SbaV2Validation::validate(),
        SdfValidation::validate(),
        DigitalTwinValidation::validate(),
    ]
}

// ── Tier detection ─────────────────────────────────────────────────────────

fn tier_info() -> serde_json::Value {
    json!({
        "core": true,
        "baseline_comparison": cfg!(feature = "baseline-comparison"),
        "onnx": cfg!(feature = "onnx"),
        "plotting": cfg!(feature = "plotting"),
    })
}

// ── CLI ────────────────────────────────────────────────────────────────────

fn usage() {
    eprintln!(
        "sixg-bench — 6G simulation bench\n\
         \n\
         USAGE:\n\
         \n\
           sixg-bench list                 List all experiments\n\
           sixg-bench run <id|--all>       Run experiment(s)\n\
           sixg-bench run <id> --json      JSON output\n\
           sixg-bench validate             Run all Validate checks\n\
           sixg-bench validate --baselines Include Level-2 baseline tests\n\
           sixg-bench info                 Show active tiers\n\
           sixg-bench help                 Show this message\n\
         \n\
         EXAMPLES:\n\
         \n\
           sixg-bench run exp_001\n\
           sixg-bench run --all\n\
           sixg-bench validate\n"
    );
}

fn cmd_list(json_output: bool) {
    let exps = experiments();
    if json_output {
        println!("{}", serde_json::to_string_pretty(&exps).unwrap());
        return;
    }
    println!("{:<12}  {:<10}  Description", "ID", "Tier");
    println!("{}", "─".repeat(72));
    for e in &exps {
        println!("{:<12}  {:<10}  {}", e.id, e.tier, e.description);
    }
}

/// Resolve the path to a pre-built example binary.
///
/// Looks in `<repo_root>/target/release/examples/<name>` first, then falls
/// back to `cargo run --example <name>`.
fn run_example(binary: &str, use_cargo: bool) -> i32 {
    // Try the pre-built binary first.
    let repo_root = std::env::current_exe()
        .ok()
        .and_then(|p| {
            // typical path: .../target/release/sixg-bench
            p.parent()?.parent()?.parent().map(|p| p.to_path_buf())
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    let prebuilt = repo_root
        .join("target")
        .join("release")
        .join("examples")
        .join(binary);

    let status = if !use_cargo && prebuilt.exists() {
        process::Command::new(&prebuilt)
            .status()
            .unwrap_or_else(|e| {
                eprintln!("Failed to run {}: {e}", prebuilt.display());
                process::exit(1);
            })
    } else {
        // Fall back to `cargo run`.
        process::Command::new("cargo")
            .args(["run", "--example", binary])
            .status()
            .unwrap_or_else(|e| {
                eprintln!("Failed to run cargo: {e}");
                process::exit(1);
            })
    };

    status.code().unwrap_or(1)
}

fn cmd_run(args: &[String], json_output: bool) -> i32 {
    let exps = experiments();

    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        eprintln!("Usage: sixg-bench run <exp_id|--all> [--json]");
        eprintln!(
            "IDs: {}",
            exps.iter().map(|e| e.id).collect::<Vec<_>>().join(", ")
        );
        return 1;
    }

    let run_all = args[0] == "--all";
    let targets: Vec<&ExpInfo> = if run_all {
        exps.iter().collect()
    } else {
        let id = &args[0];
        let found: Vec<&ExpInfo> = exps
            .iter()
            .filter(|e| e.id == id.as_str() || e.binary == id.as_str())
            .collect();
        if found.is_empty() {
            eprintln!("Unknown experiment: {id}");
            eprintln!(
                "Known IDs: {}",
                exps.iter().map(|e| e.id).collect::<Vec<_>>().join(", ")
            );
            return 1;
        }
        found
    };

    if json_output {
        println!("[");
    }

    let mut all_ok = true;
    let total = targets.len();
    for (i, exp) in targets.iter().enumerate() {
        let t0 = Instant::now();
        eprintln!("▶  Running {} — {}…", exp.id, exp.description);
        let code = run_example(exp.binary, false);
        let elapsed = t0.elapsed();
        let ok = code == 0;
        if !ok {
            all_ok = false;
        }
        if json_output {
            let sep = if i + 1 < total { "," } else { "" };
            println!(
                "  {}{sep}",
                serde_json::to_string_pretty(&json!({
                    "id": exp.id,
                    "description": exp.description,
                    "exit_code": code,
                    "passed": ok,
                    "elapsed_ms": elapsed.as_millis(),
                }))
                .unwrap()
            );
        } else {
            let status = if ok { "PASSED ✓" } else { "FAILED ✗" };
            eprintln!("   {} ({:.2}s)", status, elapsed.as_secs_f64());
        }
    }

    if json_output {
        println!("]");
    }

    if all_ok {
        0
    } else {
        1
    }
}

fn cmd_validate(with_baselines: bool, json_output: bool) -> i32 {
    let mut results = run_all_validations();
    results.extend(run_core_validations());

    if with_baselines {
        // Re-run the baseline-comparison experiment which bundles its own
        // assertions.  This surfaces Level-2 failures without a separate
        // baseline flag in the binary itself.
        eprintln!("▶  Running Level-2 baseline experiment (exp_002)…");
        let code = run_example("exp_002_phy_baseline_comparison", false);
        if code != 0 {
            eprintln!("  Level-2 baseline comparison FAILED");
            if !json_output {
                return 1;
            }
        }
    }

    let passed: Vec<&ValidationResult> = results.iter().filter(|r| r.passed()).collect();
    let failed: Vec<&ValidationResult> = results.iter().filter(|r| !r.passed()).collect();

    if json_output {
        let records: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                json!({
                    "module": r.module,
                    "passed": r.passed(),
                    "summary": r.summary(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&records).unwrap());
    } else {
        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║                 Validation Results                       ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        for r in &results {
            let marker = if r.passed() { "✓" } else { "✗" };
            println!("║  {}  {:<52} ║", marker, r.module);
        }
        println!("╠══════════════════════════════════════════════════════════╣");
        println!(
            "║  Total: {}/{} passed{:<41} ║",
            passed.len(),
            results.len(),
            ""
        );
        println!("╚══════════════════════════════════════════════════════════╝");

        for r in &failed {
            eprintln!("\n{}", r.summary());
        }
    }

    if failed.is_empty() {
        0
    } else {
        1
    }
}

fn cmd_info(json_output: bool) {
    let info = tier_info();
    if json_output {
        println!("{}", serde_json::to_string_pretty(&info).unwrap());
        return;
    }
    let features = info.as_object().unwrap();
    println!("╔══════════════════════════════════════════════════╗");
    println!("║           sixg-bench — Active Tiers              ║");
    println!("╠══════════════════════════════════════════════════╣");
    for (k, v) in features {
        let marker = if v.as_bool().unwrap_or(false) {
            "✓  enabled "
        } else {
            "○  disabled"
        };
        println!("║  {}  {:<34} ║", marker, k);
    }
    println!("╠══════════════════════════════════════════════════╣");
    println!("║  To enable optional tiers:                       ║");
    println!("║    ./install.sh --baselines                      ║");
    println!("║    ./install.sh --plotting                       ║");
    println!("║    ./install.sh --onnx                           ║");
    println!("╚══════════════════════════════════════════════════╝");
}

// ── Main ───────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json_output = args.iter().any(|a| a == "--json");
    let remaining: Vec<String> = args.into_iter().filter(|a| a != "--json").collect();

    let subcommand = remaining
        .first()
        .map(|s| s.as_str())
        .unwrap_or("help")
        .to_string();
    let sub_args: Vec<String> = remaining.into_iter().skip(1).collect();

    let exit_code = match subcommand.as_str() {
        "list" => {
            cmd_list(json_output);
            0
        }
        "run" => cmd_run(&sub_args, json_output),
        "validate" => {
            let with_baselines = sub_args.iter().any(|a| a == "--baselines");
            cmd_validate(with_baselines, json_output)
        }
        "info" => {
            cmd_info(json_output);
            0
        }
        "help" | "--help" | "-h" => {
            usage();
            0
        }
        other => {
            eprintln!("Unknown command: {other}");
            usage();
            1
        }
    };

    process::exit(exit_code);
}

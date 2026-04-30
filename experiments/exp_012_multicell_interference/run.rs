//! Experiment 012 — Multi-Cell Interference Capacity
//!
//! Hypothesis: Path loss and RIS models remain numerically stable and
//! monotone across a 19-cell hexagonal layout (standard 5G evaluation
//! geometry per ITU-R M.2160).
//!
//! Method:
//! 1. Place 19 cells in a hexagonal pattern (1 centre + 6 inner + 12 outer).
//! 2. Place one UE at the centre of each cell.
//! 3. For each UE compute SINR = serving signal / (thermal noise + interference).
//! 4. Sweep inter-site distance from 100 m to 500 m.
//! 5. Verify no NaN or infinite path-loss values.
//! 6. Check SINR median is within 5 dB of the ITU-R reference (≈ 10 dB).
//!
//! Run with:
//!   cargo run --example exp_012_multicell_interference

use sixg_common::types::{Distance, Frequency};
use sixg_phy::spectrum::path_loss_db;

fn main() {
    let cfg: serde_json::Value =
        serde_json::from_str(include_str!("config.json")).expect("config.json parse failed");

    let n_cells: usize = cfg["n_cells"].as_u64().unwrap() as usize;
    let carrier_hz: f64 = cfg["carrier_freq_ghz"].as_f64().unwrap() * 1e9;
    let tx_power_dbm: f64 = cfg["tx_power_dbm"].as_f64().unwrap();
    let noise_figure_db: f64 = cfg["noise_figure_db"].as_f64().unwrap();
    let sinr_median_ref: f64 = cfg["sinr_median_ref_db"].as_f64().unwrap();
    let sinr_tol: f64 = cfg["sinr_median_tolerance_db"].as_f64().unwrap();

    let isd_values: Vec<f64> = cfg["inter_site_distances_m"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();

    let freq = Frequency::from_hz(carrier_hz);

    println!("═══════════════════════════════════════════════════════════════════════");
    println!("  Experiment 012 — Multi-Cell Interference Capacity ({n_cells} cells)");
    println!("  Carrier: {:.0} GHz  Tx: {tx_power_dbm} dBm  NF: {noise_figure_db} dB",
             carrier_hz / 1e9);
    println!("═══════════════════════════════════════════════════════════════════════");

    // Hexagonal cell centre positions (normalised by ISD, scaled per sweep).
    // Standard 19-cell hex pattern: tier-0 (1), tier-1 (6), tier-2 (12).
    let cell_positions = hexagonal_19_cell_positions();
    assert_eq!(
        cell_positions.len(),
        n_cells,
        "Expected {n_cells} cells, got {}",
        cell_positions.len()
    );

    println!(
        "\n  {:>8}  {:>10}  {:>12}  {:>12}  {:>8}",
        "ISD(m)", "NaN free?", "SINR med(dB)", "Ref±5dB?", "PASS?"
    );
    println!("  {}", "─".repeat(58));

    let mut all_pass = true;

    for &isd in &isd_values {
        // Scale cell centres by ISD.
        let cells_m: Vec<(f64, f64)> = cell_positions
            .iter()
            .map(|&(x, y)| (x * isd, y * isd))
            .collect();

        let mut sinr_values: Vec<f64> = Vec::with_capacity(n_cells);
        let mut nan_found = false;

        for serving_cell in 0..n_cells {
            let (sx, sy) = cells_m[serving_cell];

            // UE at the midpoint between serving cell and the next tier boundary.
            // Place it at half the minimum distance to the nearest neighbour.
            let ue_offset = isd * 0.5;
            let ue_x = sx + ue_offset;
            let ue_y = sy;

            // Serving signal power.
            let d_serving = distance(sx, sy, ue_x, ue_y).max(1.0);
            let pl_serving =
                path_loss_db(Distance::from_m(d_serving), freq).as_db();
            let rx_serving_dbm = tx_power_dbm - pl_serving;

            if pl_serving.is_nan() || pl_serving.is_infinite() {
                nan_found = true;
                break;
            }

            // Interference from all other cells.
            let noise_power_dbm = thermal_noise_dbm(100e6) + noise_figure_db;
            let noise_linear = dbm_to_linear(noise_power_dbm);

            let mut interference_linear = 0.0f64;
            for (ci, &(cx, cy)) in cells_m.iter().enumerate() {
                if ci == serving_cell {
                    continue;
                }
                let d_int = distance(cx, cy, ue_x, ue_y).max(1.0);
                let pl_int = path_loss_db(Distance::from_m(d_int), freq).as_db();
                if pl_int.is_nan() || pl_int.is_infinite() {
                    nan_found = true;
                    break;
                }
                let rx_int_dbm = tx_power_dbm - pl_int;
                interference_linear += dbm_to_linear(rx_int_dbm);
            }

            if nan_found {
                break;
            }

            let rx_serving_linear = dbm_to_linear(rx_serving_dbm);
            let sinr_linear =
                rx_serving_linear / (interference_linear + noise_linear);
            sinr_values.push(10.0 * sinr_linear.log10());
        }

        let nan_free = !nan_found;
        let sinr_median = if sinr_values.is_empty() {
            f64::NAN
        } else {
            median(&mut sinr_values)
        };

        let within_tolerance =
            !sinr_median.is_nan() && (sinr_median - sinr_median_ref).abs() <= sinr_tol;
        let pass = nan_free && within_tolerance;
        if !pass {
            all_pass = false;
        }

        println!(
            "  {:>8.0}  {:>10}  {:>12.2}  {:>12}  {:>8}",
            isd,
            if nan_free { "✓" } else { "✗ NaN!" },
            sinr_median,
            if within_tolerance { "✓" } else { "✗" },
            if pass { "✓" } else { "✗ FAIL" }
        );
    }

    println!("\n═══════════════════════════════════════════════════════════════════════");
    if all_pass {
        println!("  ALL CHECKS PASSED — path loss is numerically stable across 19 cells.");
    } else {
        panic!("Multi-cell interference checks failed");
    }
    println!("═══════════════════════════════════════════════════════════════════════");
}

/// Return the 19-cell hexagonal pattern cell centres (normalised by ISD).
///
/// Tier 0: 1 cell at (0, 0).
/// Tier 1: 6 cells at distance 1 × ISD.
/// Tier 2: 12 cells at distance √3 × ISD.
fn hexagonal_19_cell_positions() -> Vec<(f64, f64)> {
    let mut pos = vec![(0.0f64, 0.0f64)]; // centre

    // Tier 1: 6 cells at 60° intervals, distance 1.
    for k in 0..6 {
        let angle = k as f64 * std::f64::consts::PI / 3.0;
        pos.push((angle.cos(), angle.sin()));
    }

    // Tier 2: 12 cells.
    // Each tier-1 cell has two tier-2 neighbours at ±30° from the radial.
    let r2 = 3.0f64.sqrt();
    for k in 0..6 {
        let base_angle = k as f64 * std::f64::consts::PI / 3.0;
        let offset1 = base_angle + std::f64::consts::PI / 6.0;
        let offset2 = base_angle - std::f64::consts::PI / 6.0;
        pos.push((r2 * offset1.cos(), r2 * offset1.sin()));
        pos.push((r2 * offset2.cos(), r2 * offset2.sin()));
    }

    pos
}

fn distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt()
}

fn thermal_noise_dbm(bandwidth_hz: f64) -> f64 {
    // N₀ = kT = −174 dBm/Hz at 290 K.
    -174.0 + 10.0 * bandwidth_hz.log10()
}

fn dbm_to_linear(dbm: f64) -> f64 {
    10f64.powf((dbm - 30.0) / 10.0) // mW → W
}

fn median(values: &mut Vec<f64>) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = values.len();
    if n % 2 == 0 {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    } else {
        values[n / 2]
    }
}

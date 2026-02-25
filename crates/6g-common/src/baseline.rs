//! Comparison of simulation outputs against measurements from real systems.
//!
//! This module answers the question: *"How do we compare the 6G testbed
//! against actual systems that are already built?"*
//!
//! The approach mirrors the existing [`Validate`] / [`ValidationCheck`] framework
//! but operates on **external reference data** (published datasets, open-source
//! simulator outputs, or live-system measurement traces) rather than on
//! analytically known values.
//!
//! ## Workflow
//!
//! 1. Produce a `Vec<BaselinePoint>` from the simulation under test.
//! 2. Load a [`BaselineDataset`] from a two-column CSV file (or build it inline).
//! 3. Call [`BaselineDataset::compare`] with a percentage tolerance.
//! 4. Assert [`BaselineComparison::passed`] and print [`BaselineComparison::summary`].
//!
//! ## Example
//!
//! ```rust
//! use sixg_common::baseline::{BaselineDataset, BaselinePoint, BaselineSource};
//!
//! // Reference data (e.g., from srsRAN or Vienna 5G LLS)
//! let dataset = BaselineDataset {
//!     source: BaselineSource {
//!         system: "Vienna 5G LLS",
//!         metric: "BER",
//!         citation: "https://www.nt.tuwien.ac.at",
//!     },
//!     points: vec![
//!         BaselinePoint { input_parameter: 0.0,  reference_value: 0.50 },
//!         BaselinePoint { input_parameter: 10.0, reference_value: 0.10 },
//!         BaselinePoint { input_parameter: 20.0, reference_value: 0.01 },
//!     ],
//! };
//!
//! // Simulation outputs at the same operating points
//! let simulated = vec![
//!     (0.0_f64,  0.51_f64),
//!     (10.0, 0.10),
//!     (20.0, 0.0101),
//! ];
//!
//! let result = dataset.compare_values(&simulated, 5.0); // 5 % tolerance
//! assert!(result.passed(), "{}", result.summary());
//! ```
//!
//! See `docs/comparison-strategy.md` for the full comparison methodology.
//!
//! [`Validate`]: crate::validation::Validate
//! [`ValidationCheck`]: crate::validation::ValidationCheck

use crate::validation::{ValidationCheck, ValidationResult};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// BaselineSource
// ---------------------------------------------------------------------------

/// Provenance of an external reference dataset.
#[derive(Debug, Clone)]
pub struct BaselineSource {
    /// Name of the system that produced the reference data.
    ///
    /// Examples: `"srsRAN"`, `"OpenAirInterface5G"`, `"ns-3 NR"`,
    /// `"Vienna 5G LLS"`, `"NIST 28 GHz UMa"`, `"3GPP TR 38.901 CDL-C"`.
    pub system: &'static str,
    /// Name of the metric being compared.
    ///
    /// Examples: `"BER"`, `"BLER"`, `"throughput_bps"`, `"path_loss_db"`.
    pub metric: &'static str,
    /// URL or citation string for the source (for reproducibility).
    pub citation: &'static str,
}

// ---------------------------------------------------------------------------
// BaselinePoint
// ---------------------------------------------------------------------------

/// A single reference data point: one (input, value) pair from an external
/// system.
///
/// `input_parameter` is the x-axis (e.g. SNR in dB, distance in metres).
/// `reference_value` is the measured/simulated y-axis value from the real
/// system (e.g. BER, path loss in dB, throughput in bps).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselinePoint {
    /// X-axis value (e.g. SNR in dB, distance in metres).  Dimensionless
    /// for comparisons that are already in normalised units.
    pub input_parameter: f64,
    /// Measured or reference y-axis value from the external system.
    pub reference_value: f64,
}

// ---------------------------------------------------------------------------
// BaselineDataset
// ---------------------------------------------------------------------------

/// A named set of reference data points from an external system.
///
/// Build this from inline data or load it from a two-column CSV file
/// (`input_parameter,reference_value`) via [`BaselineDataset::from_csv_str`].
///
/// Compare against simulation outputs with [`BaselineDataset::compare_values`]
/// or [`BaselineDataset::compare`].
#[derive(Debug, Clone)]
pub struct BaselineDataset {
    /// Provenance of this dataset.
    pub source: BaselineSource,
    /// Reference data points, ordered by `input_parameter`.
    pub points: Vec<BaselinePoint>,
}

impl BaselineDataset {
    /// Parse a two-column CSV string into a `BaselineDataset`.
    ///
    /// Expected format (header row required):
    /// ```text
    /// input_parameter,reference_value
    /// 0.0,0.985
    /// 5.0,0.734
    /// ```
    ///
    /// Returns an error string if the CSV is malformed.
    pub fn from_csv_str(
        csv: &str,
        source: BaselineSource,
    ) -> Result<Self, String> {
        let mut points = Vec::new();
        let mut lines = csv.lines();
        // Skip header row.
        lines.next();
        for (line_no, line) in lines.enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.splitn(2, ',').collect();
            if parts.len() != 2 {
                return Err(format!(
                    "Line {}: expected 2 comma-separated fields, got: {:?}",
                    line_no + 2,
                    line
                ));
            }
            let x: f64 = parts[0].trim().parse().map_err(|e| {
                format!("Line {}: cannot parse input_parameter: {e}", line_no + 2)
            })?;
            let y: f64 = parts[1].trim().parse().map_err(|e| {
                format!("Line {}: cannot parse reference_value: {e}", line_no + 2)
            })?;
            points.push(BaselinePoint { input_parameter: x, reference_value: y });
        }
        Ok(Self { source, points })
    }

    /// Compare simulated `(input_parameter, simulated_value)` pairs against
    /// the reference dataset.
    ///
    /// For each simulated point the comparison finds the reference point with
    /// the **closest** `input_parameter` (nearest-neighbour match) and checks
    /// whether the relative error `|sim − ref| / |ref| × 100` is within
    /// `tolerance_pct`.
    ///
    /// Returns a [`BaselineComparison`] (analogous to [`ValidationResult`])
    /// that can be asserted and printed.
    pub fn compare_values(
        &self,
        simulated: &[(f64, f64)],
        tolerance_pct: f64,
    ) -> BaselineComparison {
        let checks: Vec<ValidationCheck> = simulated
            .iter()
            .map(|&(x, sim)| {
                let name_key = format!("{}@{:.3}", self.source.metric, x);
                // Find nearest reference point.
                let nearest = self
                    .points
                    .iter()
                    .min_by(|a, b| {
                        (a.input_parameter - x)
                            .abs()
                            .partial_cmp(&(b.input_parameter - x).abs())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|p| p.reference_value)
                    .unwrap_or(f64::NAN);
                // Leak a formatted string as a &'static str for ValidationCheck.
                // This is acceptable here because BaselineComparison is short-lived.
                let static_name: &'static str = Box::leak(name_key.into_boxed_str());
                ValidationCheck::new(static_name, sim, nearest, tolerance_pct)
            })
            .collect();

        BaselineComparison {
            system: self.source.system,
            metric: self.source.metric,
            citation: self.source.citation,
            checks,
        }
    }

    /// Compare using the reference dataset's own points as both the input
    /// parameter and the expected value (convenience wrapper).
    ///
    /// `sim_fn` maps each `input_parameter` to the simulated output value.
    pub fn compare<F>(&self, sim_fn: F, tolerance_pct: f64) -> BaselineComparison
    where
        F: Fn(f64) -> f64,
    {
        let pairs: Vec<(f64, f64)> = self
            .points
            .iter()
            .map(|p| (p.input_parameter, sim_fn(p.input_parameter)))
            .collect();
        self.compare_values(&pairs, tolerance_pct)
    }
}

// ---------------------------------------------------------------------------
// BaselineComparison
// ---------------------------------------------------------------------------

/// Result of comparing simulation outputs against an external reference system.
///
/// Mirrors [`ValidationResult`] so the same assertion / reporting pattern
/// works for both internal and external validation.
#[derive(Debug, Clone)]
pub struct BaselineComparison {
    /// Name of the external system (e.g. `"srsRAN"`).
    pub system: &'static str,
    /// Metric being compared (e.g. `"BER"`).
    pub metric: &'static str,
    /// Citation / URL for the reference data.
    pub citation: &'static str,
    /// Per-point validation checks.
    pub checks: Vec<ValidationCheck>,
}

impl BaselineComparison {
    /// Returns `true` if every point is within tolerance.
    pub fn passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }

    /// Returns a human-readable summary of the comparison.
    pub fn summary(&self) -> String {
        let total = self.checks.len();
        let failed: Vec<_> = self.checks.iter().filter(|c| !c.passed).collect();
        let header = format!(
            "[baseline: {} / {}]  citation: {}",
            self.system, self.metric, self.citation
        );
        if failed.is_empty() {
            format!("{header}\n  All {total} comparison points within tolerance ✓")
        } else {
            let lines: Vec<String> = failed
                .iter()
                .map(|c| {
                    format!(
                        "  FAIL {}: sim={:.4e}  ref={:.4e}  Δ={:.1}%  tol={:.1}%",
                        c.name,
                        c.actual,
                        c.expected,
                        if c.expected.abs() > 0.0 {
                            (c.actual - c.expected).abs() / c.expected.abs() * 100.0
                        } else {
                            f64::INFINITY
                        },
                        c.tolerance_pct
                    )
                })
                .collect();
            format!("{header}\n  {}/{total} points OUTSIDE tolerance:\n{}", failed.len(), lines.join("\n"))
        }
    }

    /// Convert into a [`ValidationResult`] for use with the standard CI
    /// validation pipeline.
    pub fn into_validation_result(self) -> ValidationResult {
        // Use a concatenated module name as the static str.
        let module: &'static str =
            Box::leak(format!("baseline/{}/{}", self.system, self.metric).into_boxed_str());
        ValidationResult { module, checks: self.checks }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn ber_dataset() -> BaselineDataset {
        BaselineDataset {
            source: BaselineSource {
                system: "Vienna 5G LLS",
                metric: "BER",
                citation: "https://www.nt.tuwien.ac.at",
            },
            points: vec![
                BaselinePoint { input_parameter: 0.0, reference_value: 0.5 },
                BaselinePoint { input_parameter: 10.0, reference_value: 0.1 },
                BaselinePoint { input_parameter: 20.0, reference_value: 0.01 },
            ],
        }
    }

    #[test]
    fn perfect_match_passes() {
        let ds = ber_dataset();
        let simulated = vec![(0.0, 0.5), (10.0, 0.1), (20.0, 0.01)];
        let result = ds.compare_values(&simulated, 1.0);
        assert!(result.passed(), "{}", result.summary());
    }

    #[test]
    fn within_tolerance_passes() {
        let ds = ber_dataset();
        // 4 % error — should pass with 5 % tolerance.
        let simulated = vec![(0.0, 0.52), (10.0, 0.104), (20.0, 0.0104)];
        let result = ds.compare_values(&simulated, 5.0);
        assert!(result.passed(), "{}", result.summary());
    }

    #[test]
    fn outside_tolerance_fails() {
        let ds = ber_dataset();
        // 20 % error — should fail with 5 % tolerance.
        let simulated = vec![(0.0, 0.6), (10.0, 0.1), (20.0, 0.01)];
        let result = ds.compare_values(&simulated, 5.0);
        assert!(!result.passed(), "20% deviation should fail");
    }

    #[test]
    fn compare_fn_matches_compare_values() {
        let ds = ber_dataset();
        let result_fn = ds.compare(|x| {
            // exact sim
            match x as i32 {
                0 => 0.5,
                10 => 0.1,
                _ => 0.01,
            }
        }, 1.0);
        assert!(result_fn.passed(), "{}", result_fn.summary());
    }

    #[test]
    fn summary_contains_system_name() {
        let ds = ber_dataset();
        let simulated = vec![(0.0, 0.5)];
        let result = ds.compare_values(&simulated, 1.0);
        assert!(result.summary().contains("Vienna 5G LLS"));
        assert!(result.summary().contains("BER"));
    }

    #[test]
    fn into_validation_result_passes_through() {
        let ds = ber_dataset();
        let simulated = vec![(0.0, 0.5), (10.0, 0.1), (20.0, 0.01)];
        let comparison = ds.compare_values(&simulated, 1.0);
        let vr = comparison.into_validation_result();
        assert!(vr.passed());
    }

    #[test]
    fn from_csv_str_parses_correctly() {
        let csv = "input_parameter,reference_value\n0.0,0.5\n10.0,0.1\n20.0,0.01\n";
        let source = BaselineSource {
            system: "test",
            metric: "BER",
            citation: "https://example.com",
        };
        let ds = BaselineDataset::from_csv_str(csv, source).expect("should parse");
        assert_eq!(ds.points.len(), 3);
        assert!((ds.points[0].input_parameter - 0.0).abs() < 1e-9);
        assert!((ds.points[1].reference_value - 0.1).abs() < 1e-9);
    }

    #[test]
    fn from_csv_str_rejects_malformed_line() {
        let csv = "input_parameter,reference_value\nno_comma_here\n";
        let source = BaselineSource { system: "t", metric: "m", citation: "c" };
        assert!(BaselineDataset::from_csv_str(csv, source).is_err());
    }

    #[test]
    fn nearest_neighbour_matching() {
        // Dataset only has SNR at 0, 10, 20 dB.
        let ds = ber_dataset();
        // Simulate at SNR = 9 dB — should match nearest reference point SNR=10.
        let simulated = vec![(9.0, 0.1)]; // exact match to SNR=10 ref
        let result = ds.compare_values(&simulated, 1.0);
        assert!(result.passed(), "{}", result.summary());
    }
}

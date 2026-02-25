//! Structured validation framework for physics and experiment modules.
//!
//! Every module that implements physics calculations must implement the
//! [`Validate`] trait so that numerical correctness can be verified
//! automatically by CI.
//!
//! # Example
//!
//! ```rust
//! use sixg_common::validation::{Validate, ValidationResult, ValidationCheck};
//!
//! struct MyModule;
//!
//! impl Validate for MyModule {
//!     fn validate() -> ValidationResult {
//!         let actual = 42.0_f64;
//!         let expected = 42.0_f64;
//!         ValidationResult {
//!             module: "my_module",
//!             checks: vec![ValidationCheck::new("answer", actual, expected, 0.01)],
//!         }
//!     }
//! }
//!
//! let result = MyModule::validate();
//! assert!(result.passed());
//! ```

/// Structured result from a module's self-validation.
///
/// Collect all [`ValidationCheck`]s for the module and call [`passed`](Self::passed)
/// to determine overall success.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Name of the module under validation (used in error messages).
    pub module: &'static str,
    /// Individual numerical checks performed by this module.
    pub checks: Vec<ValidationCheck>,
}

impl ValidationResult {
    /// Returns `true` if every check in this result passed.
    pub fn passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }

    /// Returns a human-readable summary of all checks.
    pub fn summary(&self) -> String {
        let total = self.checks.len();
        let failed: Vec<_> = self.checks.iter().filter(|c| !c.passed).collect();
        if failed.is_empty() {
            format!("[{}] All {total} checks passed", self.module)
        } else {
            let lines: Vec<String> = failed
                .iter()
                .map(|c| {
                    format!(
                        "  FAIL {}: expected {:.6e} ± {:.1}%, got {:.6e}",
                        c.name, c.expected, c.tolerance_pct, c.actual
                    )
                })
                .collect();
            format!(
                "[{}] {}/{total} checks FAILED\n{}",
                self.module,
                failed.len(),
                lines.join("\n")
            )
        }
    }
}

/// A single numerical validation check comparing an actual value to an expected value.
#[derive(Debug, Clone)]
pub struct ValidationCheck {
    /// Short descriptive name for this check (e.g. `"crb_at_20dB_snr"`).
    pub name: &'static str,
    /// Whether the check passed within the given tolerance.
    pub passed: bool,
    /// The value produced by the implementation.
    pub actual: f64,
    /// The reference value from a paper or analytical formula.
    pub expected: f64,
    /// Acceptable relative error in percent (e.g. `1.0` means ±1%).
    pub tolerance_pct: f64,
}

impl ValidationCheck {
    /// Create a new check, automatically computing pass/fail.
    ///
    /// Pass condition: `|actual − expected| / |expected| × 100 ≤ tolerance_pct`.
    /// If `expected` is zero, an exact equality check is used instead.
    pub fn new(name: &'static str, actual: f64, expected: f64, tolerance_pct: f64) -> Self {
        let passed = if expected == 0.0 {
            actual.abs() < f64::EPSILON * 1e6
        } else {
            ((actual - expected) / expected).abs() * 100.0 <= tolerance_pct
        };
        Self {
            name,
            passed,
            actual,
            expected,
            tolerance_pct,
        }
    }
}

/// Trait for modules that can self-validate their numerical outputs.
///
/// Implement this on any struct or unit struct that represents a physics module.
/// The `validate()` function should run known-good numerical checks against
/// reference values from published papers.
pub trait Validate {
    /// Run all numerical self-checks and return a structured result.
    fn validate() -> ValidationResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_passes_within_tolerance() {
        let check = ValidationCheck::new("test", 1.005, 1.0, 1.0);
        assert!(check.passed, "0.5% error should pass with 1% tolerance");
    }

    #[test]
    fn check_fails_outside_tolerance() {
        let check = ValidationCheck::new("test", 1.02, 1.0, 1.0);
        assert!(!check.passed, "2% error should fail with 1% tolerance");
    }

    #[test]
    fn check_zero_expected_exact() {
        let check = ValidationCheck::new("zero", 0.0, 0.0, 0.0);
        assert!(check.passed);
    }

    #[test]
    fn result_passed_when_all_checks_pass() {
        let result = ValidationResult {
            module: "test_module",
            checks: vec![
                ValidationCheck::new("a", 1.0, 1.0, 0.0),
                ValidationCheck::new("b", 2.0, 2.0, 0.0),
            ],
        };
        assert!(result.passed());
    }

    #[test]
    fn result_fails_when_any_check_fails() {
        let result = ValidationResult {
            module: "test_module",
            checks: vec![
                ValidationCheck::new("a", 1.0, 1.0, 0.0),
                ValidationCheck::new("b", 2.0, 3.0, 0.0), // fails
            ],
        };
        assert!(!result.passed());
    }

    #[test]
    fn summary_shows_failed_checks() {
        let result = ValidationResult {
            module: "test_module",
            checks: vec![ValidationCheck::new("bad", 5.0, 1.0, 1.0)],
        };
        let summary = result.summary();
        assert!(summary.contains("FAIL"));
        assert!(summary.contains("bad"));
    }

    struct DummyModule;

    impl Validate for DummyModule {
        fn validate() -> ValidationResult {
            ValidationResult {
                module: "dummy",
                checks: vec![ValidationCheck::new("one_plus_one", 2.0, 2.0, 0.0)],
            }
        }
    }

    #[test]
    fn validate_trait_impl() {
        let result = DummyModule::validate();
        assert!(result.passed());
        assert_eq!(result.module, "dummy");
    }
}

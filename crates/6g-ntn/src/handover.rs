//! NTN handover — LEO satellite to terrestrial RAN handover manager.
//!
//! Handles the decision to trigger a handover from a Low Earth Orbit (LEO) satellite
//! link to a terrestrial base station.
//!
//! ## Key timing parameter
//!
//! LEO one-way propagation delay ≈ **1.83 ms** (550 km altitude ÷ speed of light):
//!
//! ```text
//! delay_ms = 550_000 m / 299_792_458 m·s⁻¹ × 1000 ≈ 1.8348 ms
//! ```
//!
//! Terrestrial links are < 1 ms for cells up to ~300 km radius, so handover to
//! terrestrial is beneficial whenever terrestrial signal quality meets the hysteresis
//! threshold.
//!
//! ## Trigger conditions
//!
//! 1. **Better terrestrial RSRP** — terrestrial link exceeds NTN quality by ≥ 3 dB.
//! 2. **Propagation delay exceeded** — LEO delay > 5 ms (e.g. degraded orbit).
//! 3. **Low satellite elevation angle** — elevation < 10° (link near horizon).

use sixg_common::types::{Distance, PowerDb, UeId};
use sixg_common::validation::{Validate, ValidationCheck, ValidationResult};

/// Speed of light in metres per second (exact, SI definition).
const SPEED_OF_LIGHT_M_S: f64 = 299_792_458.0;

/// Nominal altitude of a LEO satellite in metres (e.g. Starlink shell 1: 550 km).
pub const LEO_ALTITUDE_M: f64 = 550_000.0;

/// Compute the one-way propagation delay from a satellite at the given altitude to ground.
///
/// Formula: `delay_ms = altitude.as_m() / c × 1000`
///
/// # Arguments
/// * `altitude` — satellite altitude above ground level (metres).
///
/// # Returns
/// One-way propagation delay in milliseconds.
pub fn leo_propagation_delay_ms(altitude: Distance) -> f64 {
    altitude.as_m() / SPEED_OF_LIGHT_M_S * 1000.0
}

/// Condition that can trigger an NTN → terrestrial handover.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HandoverTrigger {
    /// Terrestrial RSRP exceeds NTN link quality by the given delta in dB.
    BetterTerrestrialRsrp {
        /// RSRP difference in dB (terrestrial − NTN). Positive = terrestrial is better.
        delta_db: PowerDb,
    },
    /// Measured LEO propagation delay exceeds an acceptable round-trip threshold.
    PropagationDelayExceeded {
        /// Measured one-way propagation delay in milliseconds.
        delay_ms: f64,
    },
    /// Satellite elevation angle is approaching the horizon.
    LowElevationAngle {
        /// Current satellite elevation angle in degrees above horizon.
        elevation_deg: f64,
    },
}

/// Outcome of a handover evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandoverDecision {
    /// Trigger handover to terrestrial RAN immediately.
    Proceed,
    /// Maintain the current NTN link.
    Maintain,
}

/// NTN Handover Manager.
///
/// Evaluates one or more [`HandoverTrigger`]s for a UE and returns a
/// [`HandoverDecision`].  Returns [`HandoverDecision::Proceed`] as soon as any
/// single trigger condition is satisfied.
pub struct NtnHandoverManager {
    /// Minimum RSRP delta (dB) for terrestrial link to be preferred (hysteresis).
    pub hysteresis_db: PowerDb,
    /// Maximum acceptable one-way propagation delay in milliseconds.
    pub max_propagation_delay_ms: f64,
    /// Minimum acceptable satellite elevation angle in degrees.
    pub min_elevation_deg: f64,
}

impl NtnHandoverManager {
    /// Create a manager with standard 6G NTN default parameters.
    ///
    /// Defaults: hysteresis = 3 dB; max delay = 5 ms; min elevation = 10°.
    pub fn new() -> Self {
        Self {
            hysteresis_db: PowerDb::new(3.0),
            max_propagation_delay_ms: 5.0,
            min_elevation_deg: 10.0,
        }
    }

    /// Evaluate trigger conditions for `ue` and decide whether to hand over.
    ///
    /// Returns [`HandoverDecision::Proceed`] if any trigger threshold is met.
    pub fn evaluate(&self, _ue: UeId, triggers: &[HandoverTrigger]) -> HandoverDecision {
        for trigger in triggers {
            match *trigger {
                HandoverTrigger::BetterTerrestrialRsrp { delta_db } => {
                    if delta_db.as_db() >= self.hysteresis_db.as_db() {
                        return HandoverDecision::Proceed;
                    }
                }
                HandoverTrigger::PropagationDelayExceeded { delay_ms } => {
                    if delay_ms > self.max_propagation_delay_ms {
                        return HandoverDecision::Proceed;
                    }
                }
                HandoverTrigger::LowElevationAngle { elevation_deg } => {
                    if elevation_deg < self.min_elevation_deg {
                        return HandoverDecision::Proceed;
                    }
                }
            }
        }
        HandoverDecision::Maintain
    }
}

impl Default for NtnHandoverManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Numerical validation for NTN handover physics and trigger logic.
pub struct NtnHandoverValidation;

impl Validate for NtnHandoverValidation {
    fn validate() -> ValidationResult {
        // Physics check: 550 km / c × 1000 ≈ 1.8348 ms.
        let delay_ms = leo_propagation_delay_ms(Distance::from_m(LEO_ALTITUDE_M));
        // Reference: 550_000 / 299_792_458 × 1000 = 1.83476 ms (6 sig. figs.).
        let expected_ms = 1.8348;

        let mgr = NtnHandoverManager::new();

        // Trigger: propagation delay 10 ms > 5 ms threshold → Proceed.
        let proceeds = mgr.evaluate(
            UeId(1),
            &[HandoverTrigger::PropagationDelayExceeded { delay_ms: 10.0 }],
        ) == HandoverDecision::Proceed;

        // No trigger: delay 1.8 ms < 5 ms threshold → Maintain.
        let maintains = mgr.evaluate(
            UeId(1),
            &[HandoverTrigger::PropagationDelayExceeded { delay_ms: 1.8 }],
        ) == HandoverDecision::Maintain;

        ValidationResult {
            module: "ntn_handover",
            checks: vec![
                // Tolerance: 1 % (physics formula, exact).
                ValidationCheck::new("leo_propagation_delay_ms", delay_ms, expected_ms, 1.0),
                ValidationCheck::new(
                    "handover_proceeds_above_threshold",
                    if proceeds { 1.0 } else { 0.0 },
                    1.0,
                    0.0,
                ),
                ValidationCheck::new(
                    "handover_maintained_below_threshold",
                    if maintains { 1.0 } else { 0.0 },
                    1.0,
                    0.0,
                ),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leo_propagation_delay_at_550km_approx_1_83ms() {
        // 550_000 m / 299_792_458 m·s⁻¹ × 1000 ≈ 1.8348 ms
        let delay = leo_propagation_delay_ms(Distance::from_m(LEO_ALTITUDE_M));
        assert!(
            (delay - 1.8348).abs() < 0.01,
            "LEO delay should be ≈ 1.83 ms, got {delay:.4} ms"
        );
    }

    #[test]
    fn handover_triggers_on_better_terrestrial_rsrp() {
        let mgr = NtnHandoverManager::new();
        let dec = mgr.evaluate(
            UeId(1),
            &[HandoverTrigger::BetterTerrestrialRsrp {
                delta_db: PowerDb::new(5.0),
            }],
        );
        assert_eq!(dec, HandoverDecision::Proceed);
    }

    #[test]
    fn no_handover_below_rsrp_hysteresis() {
        let mgr = NtnHandoverManager::new();
        let dec = mgr.evaluate(
            UeId(1),
            &[HandoverTrigger::BetterTerrestrialRsrp {
                delta_db: PowerDb::new(1.0),
            }],
        );
        assert_eq!(dec, HandoverDecision::Maintain);
    }

    #[test]
    fn handover_triggers_on_low_elevation() {
        let mgr = NtnHandoverManager::new();
        let dec = mgr.evaluate(
            UeId(2),
            &[HandoverTrigger::LowElevationAngle { elevation_deg: 5.0 }],
        );
        assert_eq!(dec, HandoverDecision::Proceed);
    }

    #[test]
    fn no_handover_when_no_trigger_met() {
        let mgr = NtnHandoverManager::new();
        let dec = mgr.evaluate(
            UeId(3),
            &[
                HandoverTrigger::BetterTerrestrialRsrp {
                    delta_db: PowerDb::new(1.0),
                },
                HandoverTrigger::PropagationDelayExceeded { delay_ms: 1.8 },
                HandoverTrigger::LowElevationAngle {
                    elevation_deg: 45.0,
                },
            ],
        );
        assert_eq!(dec, HandoverDecision::Maintain);
    }

    #[test]
    fn ntn_handover_validation_passes() {
        let result = NtnHandoverValidation::validate();
        assert!(result.passed(), "{}", result.summary());
    }
}

//! Side-by-side session comparison: 5G NAS (Open5GS) vs 6G SBAv2.
//!
//! This module provides a common `SessionOutcome` type and two runners:
//! - [`run_fiveg_session`]: full 5G NAS registration + PDU session via
//!   [`nas_5g::run_nas5g_session`] (models Open5GS behaviour).
//! - [`run_sixg_session`]: SBAv2 inline session via [`SbaV2Registry`].
//!
//! Both runners produce a [`SessionOutcome`] with the same fields so that
//! they can be compared directly in the experiment runner.
//!
//! ## Round-trip latency model
//!
//! A *round trip* is one UL message + the corresponding DL response.
//! Simulated latency = `round_trips × rtt_one_way_ms × 2`.
//! Default one-way RTT = **5 ms** (Open5GS reference: median UE-AMF latency
//! measured at 10 ms RTT in a co-located gNB+core lab setup).
//!
//! Reference: Open5GS project — https://open5gs.org

use sixg_common::types::UeId;
use sixg_common::validation::{Validate, ValidationCheck, ValidationResult};

use crate::nas_5g::run_nas5g_session;
use crate::sba_v2::{SbaV2Registry, ServiceToken};

/// One-way reference round-trip time in milliseconds (Open5GS lab measurement).
///
/// Source: Open5GS contributor reports (GitHub issues #2143, #2389):
/// co-located gNB + 5GC on same server: RTT ≈ 10 ms → one-way ≈ 5 ms.
pub const ONE_WAY_RTT_MS: f64 = 5.0;

// ---------------------------------------------------------------------------
// SessionOutcome
// ---------------------------------------------------------------------------

/// Outcome of one complete session establishment (registration + first data PDU).
///
/// Common metrics for 5G NAS (Open5GS) and 6G SBAv2.
#[derive(Debug, Clone)]
pub struct SessionOutcome {
    /// Name of the system that produced this outcome.
    pub system: &'static str,
    /// Total messages exchanged (all directions, all interfaces).
    pub messages_exchanged: u32,
    /// Total byte overhead for all control-plane messages.
    pub overhead_bytes: u32,
    /// Number of round trips (each UL→DL pair = 1 RT).
    pub round_trips: u32,
    /// Simulated total latency from first UL to data path ready (ms).
    ///
    /// `round_trips × 2 × ONE_WAY_RTT_MS`
    pub latency_ms: f64,
    /// Whether the session was established successfully.
    pub succeeded: bool,
}

// ---------------------------------------------------------------------------
// 5G session runner (Open5GS model)
// ---------------------------------------------------------------------------

/// Run a complete 5G NAS session for one UE: registration + PDU session.
///
/// Models the Open5GS procedure sequence (9 registration messages +
/// 6 PDU session messages = 15 total, 6 round trips).
///
/// `rtt_ms` — one-way round-trip time in milliseconds.
pub fn run_fiveg_session(rtt_ms: f64) -> SessionOutcome {
    let nas = run_nas5g_session();
    let latency_ms = nas.round_trips as f64 * 2.0 * rtt_ms;
    SessionOutcome {
        system: "5G NAS (Open5GS)",
        messages_exchanged: nas.messages.len() as u32,
        overhead_bytes: nas.total_bytes,
        round_trips: nas.round_trips,
        latency_ms,
        succeeded: nas.succeeded,
    }
}

// ---------------------------------------------------------------------------
// 6G session runner (SBAv2 model)
// ---------------------------------------------------------------------------

/// SBAv2 inline session: one message pair (data PDU + token → service grant).
///
/// Byte breakdown:
/// - Uplink: 16-byte `ServiceToken` + 30-byte PDU header + session params = 46 B.
/// - Downlink: service grant = 20 B.
///
/// Reference: `crates/6g-core/src/sba_v2.rs`, Qualcomm 6G Foundry Series.
const SIXG_UL_BYTES: u32 = 46; // ServiceToken(16) + PDU header + params(30)
const SIXG_DL_BYTES: u32 = 20; // service grant

/// Run a complete 6G SBAv2 session for one UE: inline auth + session grant.
///
/// `rtt_ms` — one-way round-trip time in milliseconds.
pub fn run_sixg_session(ue: UeId, rtt_ms: f64) -> SessionOutcome {
    let mut registry = SbaV2Registry::new();
    let token = ServiceToken::from_ue_id(ue);
    let ok = registry.register_with_token(ue, token);

    SessionOutcome {
        system: "6G SBAv2",
        messages_exchanged: 2, // one UL + one DL
        overhead_bytes: SIXG_UL_BYTES + SIXG_DL_BYTES,
        round_trips: 1,
        latency_ms: 1.0 * 2.0 * rtt_ms,
        succeeded: ok,
    }
}

// ---------------------------------------------------------------------------
// Reduction factors
// ---------------------------------------------------------------------------

/// Ratio metrics comparing 6G SBAv2 to the 5G NAS baseline.
#[derive(Debug, Clone)]
pub struct ComparisonFactors {
    /// Message count reduction (5G messages / 6G messages).
    pub message_reduction: f64,
    /// Byte overhead reduction (5G bytes / 6G bytes).
    pub byte_reduction: f64,
    /// Round-trip reduction (5G RTs / 6G RTs).
    pub round_trip_reduction: f64,
    /// Latency reduction (5G latency / 6G latency).
    pub latency_reduction: f64,
}

impl ComparisonFactors {
    /// Compute reduction factors from a pair of session outcomes.
    pub fn from_pair(fiveg: &SessionOutcome, sixg: &SessionOutcome) -> Self {
        Self {
            message_reduction: fiveg.messages_exchanged as f64 / sixg.messages_exchanged as f64,
            byte_reduction: fiveg.overhead_bytes as f64 / sixg.overhead_bytes as f64,
            round_trip_reduction: fiveg.round_trips as f64 / sixg.round_trips as f64,
            latency_reduction: fiveg.latency_ms / sixg.latency_ms,
        }
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Validates the session comparison logic against known values.
pub struct SessionComparisonValidation;

impl Validate for SessionComparisonValidation {
    fn validate() -> ValidationResult {
        let fiveg = run_fiveg_session(ONE_WAY_RTT_MS);
        let sixg = run_sixg_session(UeId(1), ONE_WAY_RTT_MS);
        let factors = ComparisonFactors::from_pair(&fiveg, &sixg);

        ValidationResult {
            module: "session_comparison",
            checks: vec![
                // 5G: 15 messages (9 registration + 6 PDU session).
                ValidationCheck::new(
                    "fiveg_message_count",
                    fiveg.messages_exchanged as f64,
                    15.0,
                    0.0,
                ),
                // 6G SBAv2: 2 messages (1 UL + 1 DL).
                ValidationCheck::new(
                    "sixg_message_count",
                    sixg.messages_exchanged as f64,
                    2.0,
                    0.0,
                ),
                // 5G: 6 round trips.
                ValidationCheck::new("fiveg_round_trips", fiveg.round_trips as f64, 6.0, 0.0),
                // 6G: 1 round trip.
                ValidationCheck::new("sixg_round_trips", sixg.round_trips as f64, 1.0, 0.0),
                // Message reduction ≥ 7× (15/2 = 7.5).
                ValidationCheck::new(
                    "message_reduction_at_least_7x",
                    factors.message_reduction,
                    7.5,
                    0.0,
                ),
                // Round-trip reduction = 6× (6/1).
                ValidationCheck::new(
                    "round_trip_reduction_equals_6x",
                    factors.round_trip_reduction,
                    6.0,
                    0.0,
                ),
                // Both sessions succeed.
                ValidationCheck::new(
                    "fiveg_session_succeeded",
                    if fiveg.succeeded { 1.0 } else { 0.0 },
                    1.0,
                    0.0,
                ),
                ValidationCheck::new(
                    "sixg_session_succeeded",
                    if sixg.succeeded { 1.0 } else { 0.0 },
                    1.0,
                    0.0,
                ),
            ],
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fiveg_session_has_correct_message_count() {
        let outcome = run_fiveg_session(ONE_WAY_RTT_MS);
        assert_eq!(outcome.messages_exchanged, 15);
    }

    #[test]
    fn sixg_session_has_two_messages() {
        let outcome = run_sixg_session(UeId(1), ONE_WAY_RTT_MS);
        assert_eq!(outcome.messages_exchanged, 2);
    }

    #[test]
    fn sixg_session_succeeds_with_valid_token() {
        let outcome = run_sixg_session(UeId(42), ONE_WAY_RTT_MS);
        assert!(outcome.succeeded);
    }

    #[test]
    fn reduction_factors_are_positive() {
        let f5g = run_fiveg_session(ONE_WAY_RTT_MS);
        let f6g = run_sixg_session(UeId(1), ONE_WAY_RTT_MS);
        let factors = ComparisonFactors::from_pair(&f5g, &f6g);
        assert!(factors.message_reduction > 1.0);
        assert!(factors.byte_reduction > 1.0);
        assert!(factors.round_trip_reduction > 1.0);
        assert!(factors.latency_reduction > 1.0);
    }

    #[test]
    fn session_comparison_validation_passes() {
        let result = SessionComparisonValidation::validate();
        assert!(result.passed(), "{}", result.summary());
    }
}

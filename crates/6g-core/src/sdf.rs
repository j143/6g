//! Sensing Data Function (SDF) — 6G-new NF with no 5G equivalent.
//!
//! The SDF bridges the Integrated Sensing and Communication (ISAC) results
//! produced by the RAN layer (crate `6g-isac`) into the 6G core SBI so that
//! applications can subscribe to sensing results the same way they subscribe
//! to data traffic.
//!
//! ## Architecture
//!
//! ```text
//! 6g-isac (RAN)       6g-core (SDF)             Application
//!   IscScene::detect()  ──publish──► SensingDataFunction  ──notify──► SensingSubscription
//! ```
//!
//! The SDF does **not** depend on `6g-isac` directly (that would violate the
//! allowed dependency graph).  Instead, the top-level `sixg` binary / session
//! runner bridges them by calling `sdf.publish(DetectionEvent { ... })` with
//! data derived from `6g-isac` output types.  The SDF defines the contract;
//! the session runner performs the bridge.
//!
//! ## Reference
//!
//! Proposed in 3GPP TR 22.837 (Integrated Sensing and Communication for 5G/6G)
//! and Nokia Bell Labs *Sensing as a Service in 6G* (2021).

use std::collections::VecDeque;

use sixg_common::types::{Distance, NodeId, UeId, Velocity};
use sixg_common::validation::{Validate, ValidationCheck, ValidationResult};

/// A single sensing detection result published to the SDF.
///
/// Produced by the ISAC layer and forwarded to the SDF by the session runner.
/// Uses only `6g-common` types so that `6g-core` does not need to depend on
/// `6g-isac` directly.
#[derive(Debug, Clone)]
pub struct DetectionEvent {
    /// The RAN cell / ISAC node that produced this detection.
    pub cell_id: NodeId,
    /// Slant range from the ISAC TX to the detected object.
    pub range: Distance,
    /// Estimated radial velocity of the detected object (positive = moving away).
    pub velocity: Velocity,
    /// Identifier of the UE associated with the detection, if known.
    pub ue_id: Option<UeId>,
}

/// A subscription to sensing events from a specific cell.
///
/// Applications register a subscription and receive `DetectionEvent`s
/// whenever the SDF receives a matching detection from `cell_id` within
/// `max_range`.
#[derive(Debug)]
pub struct SensingSubscription {
    /// Cell whose detections are of interest.
    pub cell_id: NodeId,
    /// Maximum slant range — detections beyond this are filtered out.
    pub max_range: Distance,
    /// Number of matching detections delivered to this subscription.
    pub delivered_count: usize,
}

impl SensingSubscription {
    /// Create a new subscription for `cell_id` with `max_range`.
    pub fn new(cell_id: NodeId, max_range: Distance) -> Self {
        Self {
            cell_id,
            max_range,
            delivered_count: 0,
        }
    }

    /// Returns `true` if `event` matches this subscription's filter criteria.
    fn matches(&self, event: &DetectionEvent) -> bool {
        event.cell_id == self.cell_id && event.range.as_m() <= self.max_range.as_m()
    }
}

/// Sensing Data Function — ISAC-to-core bridge.
///
/// A 6G-new NF that exposes RAN sensing results as a core network service
/// over the Service Based Interface (SBI), with no 5G equivalent.
///
/// Applications call `subscribe(cell_id, max_range)` to register interest;
/// the session runner calls `publish(event)` after each ISAC radar sweep.
/// The SDF delivers each event to all matching subscriptions and returns
/// the number of subscriptions notified.
pub struct SensingDataFunction {
    subscriptions: Vec<SensingSubscription>,
    event_history: VecDeque<DetectionEvent>,
    max_history: usize,
    /// Total detection events published (diagnostic counter).
    pub published_count: usize,
}

impl SensingDataFunction {
    /// Create an empty SDF.
    pub fn new() -> Self {
        Self {
            subscriptions: Vec::new(),
            event_history: VecDeque::new(),
            max_history: 128,
            published_count: 0,
        }
    }

    /// Register a new sensing subscription.
    ///
    /// Returns the subscription index (stable as long as no subscriptions are
    /// removed).
    pub fn subscribe(&mut self, cell_id: NodeId, max_range: Distance) -> usize {
        let idx = self.subscriptions.len();
        let mut sub = SensingSubscription::new(cell_id, max_range);
        for event in &self.event_history {
            if sub.matches(event) {
                sub.delivered_count += 1;
            }
        }
        self.subscriptions.push(sub);
        idx
    }

    /// Remove (cancel) a subscription by index.
    ///
    /// Returns `true` if the subscription existed and was removed.
    pub fn unsubscribe(&mut self, index: usize) -> bool {
        if index < self.subscriptions.len() {
            self.subscriptions.remove(index);
            true
        } else {
            false
        }
    }

    /// Publish a detection event from the ISAC layer.
    ///
    /// Delivers the event to all subscriptions whose `cell_id` matches and
    /// whose `max_range` is not exceeded.
    ///
    /// Returns the number of subscriptions that received the event.
    pub fn publish(&mut self, event: &DetectionEvent) -> usize {
        self.published_count += 1;
        self.event_history.push_back(event.clone());
        if self.event_history.len() > self.max_history {
            self.event_history.pop_front();
        }
        let mut delivered = 0;
        for sub in &mut self.subscriptions {
            if sub.matches(event) {
                sub.delivered_count += 1;
                delivered += 1;
            }
        }
        delivered
    }

    /// Number of active subscriptions.
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Borrow the subscription at `index`, if it exists.
    pub fn subscription(&self, index: usize) -> Option<&SensingSubscription> {
        self.subscriptions.get(index)
    }

    /// Number of retained detection events in the replay ring buffer.
    pub fn history_len(&self) -> usize {
        self.event_history.len()
    }
}

impl Default for SensingDataFunction {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation for the SDF publish / subscribe logic.
///
/// Checks:
/// 1. Events within `max_range` are delivered.
/// 2. Events beyond `max_range` are filtered out.
/// 3. Events from a different cell are not delivered.
pub struct SdfValidation;

impl Validate for SdfValidation {
    fn validate() -> ValidationResult {
        let mut sdf = SensingDataFunction::new();
        let cell_a = NodeId(1);
        let cell_b = NodeId(2);
        let max_range = Distance::from_m(500.0);
        let _idx = sdf.subscribe(cell_a, max_range);

        // Event within range — must be delivered.
        let in_range = DetectionEvent {
            cell_id: cell_a,
            range: Distance::from_m(300.0),
            velocity: Velocity::from_m_per_s(10.0),
            ue_id: None,
        };
        let delivered_in = sdf.publish(&in_range);

        // Event out of range — must NOT be delivered.
        let out_of_range = DetectionEvent {
            cell_id: cell_a,
            range: Distance::from_m(600.0),
            velocity: Velocity::from_m_per_s(5.0),
            ue_id: None,
        };
        let delivered_out = sdf.publish(&out_of_range);

        // Event from wrong cell — must NOT be delivered.
        let wrong_cell = DetectionEvent {
            cell_id: cell_b,
            range: Distance::from_m(100.0),
            velocity: Velocity::from_m_per_s(0.0),
            ue_id: None,
        };
        let delivered_wrong = sdf.publish(&wrong_cell);

        ValidationResult {
            module: "sdf",
            checks: vec![
                ValidationCheck::new("in_range_event_delivered", delivered_in as f64, 1.0, 0.0),
                ValidationCheck::new(
                    "out_of_range_event_not_delivered",
                    delivered_out as f64,
                    0.0,
                    0.0,
                ),
                ValidationCheck::new(
                    "wrong_cell_event_not_delivered",
                    delivered_wrong as f64,
                    0.0,
                    0.0,
                ),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sixg_common::types::{Distance, NodeId, UeId, Velocity};

    fn make_event(cell_id: NodeId, range_m: f64) -> DetectionEvent {
        DetectionEvent {
            cell_id,
            range: Distance::from_m(range_m),
            velocity: Velocity::from_m_per_s(0.0),
            ue_id: None,
        }
    }

    #[test]
    fn publish_delivers_to_matching_subscription() {
        let mut sdf = SensingDataFunction::new();
        let cell = NodeId(1);
        sdf.subscribe(cell, Distance::from_m(500.0));

        let n = sdf.publish(&make_event(cell, 200.0));
        assert_eq!(n, 1, "one subscription must receive the event");
        assert_eq!(sdf.subscription(0).unwrap().delivered_count, 1);
        assert_eq!(sdf.published_count, 1);
    }

    #[test]
    fn publish_filters_events_beyond_max_range() {
        let mut sdf = SensingDataFunction::new();
        sdf.subscribe(NodeId(1), Distance::from_m(300.0));
        // Event at 400 m > 300 m max_range — must not be delivered.
        let n = sdf.publish(&make_event(NodeId(1), 400.0));
        assert_eq!(n, 0, "event beyond max_range must not be delivered");
        assert_eq!(sdf.subscription(0).unwrap().delivered_count, 0);
    }

    #[test]
    fn publish_filters_events_from_wrong_cell() {
        let mut sdf = SensingDataFunction::new();
        sdf.subscribe(NodeId(1), Distance::from_m(1_000.0));
        // Event from cell 2 — must not match subscription for cell 1.
        let n = sdf.publish(&make_event(NodeId(2), 50.0));
        assert_eq!(n, 0, "event from wrong cell must not be delivered");
    }

    #[test]
    fn multiple_subscriptions_receive_independently() {
        let mut sdf = SensingDataFunction::new();
        let cell = NodeId(1);
        // Sub 0: max 500 m; sub 1: max 100 m.
        sdf.subscribe(cell, Distance::from_m(500.0));
        sdf.subscribe(cell, Distance::from_m(100.0));

        // Range 200 m — within sub 0, beyond sub 1.
        let n = sdf.publish(&make_event(cell, 200.0));
        assert_eq!(n, 1, "only the wider-range subscription must receive event");
        assert_eq!(sdf.subscription(0).unwrap().delivered_count, 1);
        assert_eq!(sdf.subscription(1).unwrap().delivered_count, 0);
    }

    #[test]
    fn ue_id_is_propagated_in_event() {
        let mut sdf = SensingDataFunction::new();
        let cell = NodeId(1);
        sdf.subscribe(cell, Distance::from_m(1_000.0));
        let event = DetectionEvent {
            cell_id: cell,
            range: Distance::from_m(100.0),
            velocity: Velocity::from_m_per_s(15.0),
            ue_id: Some(UeId(42)),
        };
        let n = sdf.publish(&event);
        assert_eq!(n, 1);
    }

    #[test]
    fn unsubscribe_stops_delivery() {
        let mut sdf = SensingDataFunction::new();
        let cell = NodeId(1);
        let idx = sdf.subscribe(cell, Distance::from_m(1_000.0));
        assert_eq!(sdf.subscription_count(), 1);
        assert!(sdf.unsubscribe(idx));
        assert_eq!(sdf.subscription_count(), 0);
        let n = sdf.publish(&make_event(cell, 50.0));
        assert_eq!(n, 0, "unsubscribed sink must not receive events");
    }

    #[test]
    fn late_subscriber_replays_matching_history() {
        let mut sdf = SensingDataFunction::new();
        let cell = NodeId(99);
        let _ = sdf.publish(&make_event(cell, 80.0));
        let idx = sdf.subscribe(cell, Distance::from_m(100.0));
        assert_eq!(
            sdf.subscription(idx).unwrap().delivered_count,
            1,
            "late subscriber must receive replay from history"
        );
    }

    #[test]
    fn sdf_validation_passes() {
        let result = SdfValidation::validate();
        assert!(result.passed(), "{}", result.summary());
    }
}

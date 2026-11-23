//! NTN-aware data-call handover scenario.
//!
//! This module models a UE that is initially served by a LEO satellite and
//! later moves to a terrestrial gNB. It records the observable call-flow
//! events and guards the handover transition.
//!
//! This is a **simulated NTN data-session handover**, not an RRC-controlled
//! mobility procedure or a 3GPP NTN conformance test.

use crate::handover::{HandoverDecision, HandoverTrigger, NtnHandoverManager, NtnNode};
use sixg_common::types::{BearerId, NodeId, Position3D, UeId};
use thiserror::Error;

/// Serving node selected for the NTN call scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NtnServingNode {
    /// LEO satellite serving node.
    LeoSatellite,
    /// Terrestrial gNB serving node.
    TerrestrialGnb,
}

/// Observable events in the NTN call-handover flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NtnCallEvent {
    AttachedToLeo,
    PayloadSent,
    PayloadDelivered,
    HandoverEvaluated,
    HandoverCompleted,
    AttachedToTerrestrial,
}

/// Errors returned by the NTN call-handover scenario.
#[derive(Debug, Error)]
pub enum NtnCallError {
    #[error("handover has not been triggered")]
    HandoverNotTriggered,
    #[error("payload must not be empty")]
    InvalidPayload,
}

/// A minimal NTN-to-terrestrial data-call handover scenario.
///
/// The payload transport is currently a placeholder: `send_payload` returns an
/// exact copy of its input. It does not exercise a real PHY channel, CRC, or
/// radio link.
pub struct NtnCallScenario {
    ue: UeId,
    bearer: BearerId,
    satellite: NtnNode,
    terrestrial_node: NodeId,
    handover: NtnHandoverManager,
    serving_node: NtnServingNode,
    handover_triggered: bool,
    events: Vec<NtnCallEvent>,
}

impl NtnCallScenario {
    /// Create a scenario initially served by a 550 km LEO satellite.
    pub fn leo_to_terrestrial(ue: UeId, bearer: BearerId, terrestrial_node: NodeId) -> Self {
        let satellite = NtnNode::leo_satellite(
            1,
            Position3D::new(0.0, 0.0, 550_000.0),
        );

        let mut scenario = Self {
            ue,
            bearer,
            satellite,
            terrestrial_node,
            handover: NtnHandoverManager::new(),
            serving_node: NtnServingNode::LeoSatellite,
            handover_triggered: false,
            events: Vec::new(),
        };

        scenario.events.push(NtnCallEvent::AttachedToLeo);
        scenario
    }

    /// Return the currently selected serving node.
    pub fn serving_node(&self) -> NtnServingNode {
        self.serving_node
    }

    /// Return the recorded call-flow events.
    pub fn events(&self) -> &[NtnCallEvent] {
        &self.events
    }

    /// Return the modelled one-way LEO propagation delay in milliseconds.
    pub fn satellite_delay_ms(&self) -> f64 {
        self.satellite.propagation_delay_ms
    }

    /// Evaluate an NTN-to-terrestrial handover from satellite elevation.
    pub fn evaluate_handover(&mut self, elevation_deg: f64) -> HandoverDecision {
        let decision = self.handover.evaluate(
            self.ue,
            &[HandoverTrigger::LowElevationAngle { elevation_deg }],
        );

        self.handover_triggered = decision == HandoverDecision::Proceed;
        self.events.push(NtnCallEvent::HandoverEvaluated);
        decision
    }

    /// Complete the NTN-to-terrestrial handover.
    ///
    /// Returns an error unless a preceding handover evaluation returned
    /// [`HandoverDecision::Proceed`].
    pub fn handover_to_terrestrial(&mut self) -> Result<(), NtnCallError> {
        if self.serving_node == NtnServingNode::TerrestrialGnb {
            return Ok(());
        }

        if !self.handover_triggered {
            return Err(NtnCallError::HandoverNotTriggered);
        }

        self.serving_node = NtnServingNode::TerrestrialGnb;
        self.events.push(NtnCallEvent::HandoverCompleted);
        self.events.push(NtnCallEvent::AttachedToTerrestrial);
        Ok(())
    }

    /// Send a payload over the currently selected serving node.
    ///
    /// The returned payload is an exact copy of the input. This is a transport
    /// placeholder for this milestone; it does not exercise a real PHY/channel
    /// path.
    pub fn send_payload(&mut self, payload: &[u8]) -> Result<Vec<u8>, NtnCallError> {
        if payload.is_empty() {
            return Err(NtnCallError::InvalidPayload);
        }

        self.events.push(NtnCallEvent::PayloadSent);
        let delivered = payload.to_vec();
        self.events.push(NtnCallEvent::PayloadDelivered);
        Ok(delivered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_serving_node_is_leo() {
        let scenario = NtnCallScenario::leo_to_terrestrial(UeId(1), BearerId(1), NodeId(100));

        assert_eq!(scenario.serving_node(), NtnServingNode::LeoSatellite);
        assert_eq!(scenario.events(), &[NtnCallEvent::AttachedToLeo]);
    }

    #[test]
    fn leo_delay_is_about_1_835_ms() {
        let scenario = NtnCallScenario::leo_to_terrestrial(UeId(1), BearerId(1), NodeId(100));

        assert!((scenario.satellite_delay_ms() - 1.8348).abs() < 0.01);
    }

    #[test]
    fn empty_payload_is_rejected() {
        let mut scenario = NtnCallScenario::leo_to_terrestrial(UeId(1), BearerId(1), NodeId(100));

        assert_eq!(
            scenario.send_payload(&[]),
            Err(NtnCallError::InvalidPayload)
        );
    }
}
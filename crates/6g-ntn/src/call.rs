//! NTN-aware data-call handover scenario.
//!
//! This module models a UE that is initially served by a LEO satellite and
//! later moves to a terrestrial gNB. It records the observable call-flow
//! events, performs PDCP/RLC payload round trips, and guards the handover
//! transition.
//!
//! This is a **simulated NTN data-session handover**, not an RRC-controlled
//! mobility procedure or a 3GPP NTN conformance test.

use crate::NtnNode;
use crate::handover::{HandoverDecision, HandoverTrigger, NtnHandoverManager};
use sixg_common::types::{BearerId, NodeId, Payload, Position3D, UeId};
use sixg_pdcp::{CipheringAlgorithm, IntegrityAlgorithm, PdcpConfig, PdcpEntity};
use sixg_rlc::{RlcEntity, RlcMode};
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
#[derive(Debug, Error, PartialEq, Eq)]
pub enum NtnCallError {
    #[error("handover has not been triggered")]
    HandoverNotTriggered,
    #[error("payload must not be empty")]
    InvalidPayload,
    #[error("scenario requires an NTN payload and a terrestrial payload")]
    InsufficientPayloads,
    #[error("transport failed to deliver the payload")]
    TransportDeliveryFailed,
}

/// Transport boundary used by an NTN call scenario.
///
/// The first implementation performs a PDCP and RLC round trip. It does not
/// exercise a real PHY channel, CRC, or radio link.
pub trait NtnTransport {
    /// Send a payload through the transport and return the received payload.
    fn send(&mut self, payload: &[u8]) -> Result<Payload, NtnCallError>;
}

/// PDCP/RLC transport used for each serving leg.
///
/// The transmit and receive entities are independent, so delivery is a real
/// protocol round trip rather than an echo.
pub struct PdcpRlcTransport {
    bearer: BearerId,
    tx_pdcp: PdcpEntity,
    rx_pdcp: PdcpEntity,
    tx_rlc: RlcEntity,
    rx_rlc: RlcEntity,
}

impl PdcpRlcTransport {
    /// Create independent PDCP and RLC entities for one bearer.
    pub fn new(bearer: BearerId) -> Self {
        let config = PdcpConfig {
            bearer,
            ciphering: CipheringAlgorithm::Nea0,
            integrity: IntegrityAlgorithm::Nia0,
            rohc_enabled: false,
            sn_length: 12,
        };

        Self {
            bearer,
            tx_pdcp: PdcpEntity::new(config.clone()),
            rx_pdcp: PdcpEntity::new(config),
            tx_rlc: RlcEntity::new(bearer, RlcMode::Am),
            rx_rlc: RlcEntity::new(bearer, RlcMode::Am),
        }
    }

    /// Re-establish the transport with fresh PDCP and RLC state.
    pub fn reset(&mut self) {
        *self = Self::new(self.bearer);
    }
}

impl NtnTransport for PdcpRlcTransport {
    fn send(&mut self, payload: &[u8]) -> Result<Payload, NtnCallError> {
        if payload.is_empty() {
            return Err(NtnCallError::InvalidPayload);
        }

        let pdcp_pdu = self.tx_pdcp.process_tx(payload.to_vec());
        let rlc_pdus = self.tx_rlc.transmit(pdcp_pdu);

        let rlc_sdu = self
            .rx_rlc
            .receive(rlc_pdus)
            .ok_or(NtnCallError::TransportDeliveryFailed)?;

        self.rx_pdcp
            .process_rx(rlc_sdu)
            .ok_or(NtnCallError::TransportDeliveryFailed)
    }
}

/// Observable result of a complete NTN-to-terrestrial call scenario.
#[derive(Debug, Clone, PartialEq)]
pub struct NtnCallReport {
    /// Payload delivered through the initial LEO path.
    pub ntn_payload: Payload,
    /// Payload delivered through the terrestrial path after handover.
    pub terrestrial_payload: Payload,
    /// Handover decision produced by the NTN handover manager.
    pub handover_decision: HandoverDecision,
    /// Serving node after handover completion.
    pub serving_node_after_handover: NtnServingNode,
}

/// A minimal NTN-to-terrestrial data-call handover scenario.
pub struct NtnCallScenario {
    ue: UeId,
    bearer: BearerId,
    satellite: NtnNode,
    terrestrial_node: NodeId,
    handover: NtnHandoverManager,
    ntn_transport: PdcpRlcTransport,
    terrestrial_transport: PdcpRlcTransport,
    serving_node: NtnServingNode,
    handover_triggered: bool,
    events: Vec<NtnCallEvent>,
}

impl NtnCallScenario {
    /// Create a scenario initially served by a 550 km LEO satellite.
    pub fn leo_to_terrestrial(ue: UeId, bearer: BearerId, terrestrial_node: NodeId) -> Self {
        let satellite = NtnNode::leo_satellite(1, Position3D::new(0.0, 0.0, 550_000.0));

        let mut scenario = Self {
            ue,
            bearer,
            satellite,
            terrestrial_node,
            handover: NtnHandoverManager::new(),
            ntn_transport: PdcpRlcTransport::new(bearer),
            terrestrial_transport: PdcpRlcTransport::new(bearer),
            serving_node: NtnServingNode::LeoSatellite,
            handover_triggered: false,
            events: Vec::new(),
        };

        scenario.events.push(NtnCallEvent::AttachedToLeo);
        scenario
    }

    /// Return the UE associated with the scenario.
    pub fn ue(&self) -> UeId {
        self.ue
    }

    /// Return the bearer associated with the scenario.
    pub fn bearer(&self) -> BearerId {
        self.bearer
    }

    /// Return the terrestrial target node.
    pub fn terrestrial_node(&self) -> NodeId {
        self.terrestrial_node
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
    /// [`HandoverDecision::Proceed`]. The terrestrial transport is
    /// re-established with fresh PDCP and RLC state.
    pub fn handover_to_terrestrial(&mut self) -> Result<(), NtnCallError> {
        if self.serving_node == NtnServingNode::TerrestrialGnb {
            return Ok(());
        }

        if !self.handover_triggered {
            return Err(NtnCallError::HandoverNotTriggered);
        }

        self.terrestrial_transport.reset();
        self.serving_node = NtnServingNode::TerrestrialGnb;
        self.events.push(NtnCallEvent::HandoverCompleted);
        self.events.push(NtnCallEvent::AttachedToTerrestrial);
        Ok(())
    }

    /// Send a payload over the currently selected serving node.
    pub fn send_payload(&mut self, payload: &[u8]) -> Result<Payload, NtnCallError> {
        self.events.push(NtnCallEvent::PayloadSent);

        let delivered = match self.serving_node {
            NtnServingNode::LeoSatellite => self.ntn_transport.send(payload)?,
            NtnServingNode::TerrestrialGnb => self.terrestrial_transport.send(payload)?,
        };

        self.events.push(NtnCallEvent::PayloadDelivered);
        Ok(delivered)
    }

    /// Run the complete NTN-to-terrestrial call scenario.
    ///
    /// The first payload is delivered through the LEO path, the handover is
    /// evaluated and completed, and the second payload is delivered through
    /// the re-established terrestrial path.
    pub fn run(
        &mut self,
        payloads: &[&[u8]],
        elevation_deg: f64,
    ) -> Result<NtnCallReport, NtnCallError> {
        if payloads.len() < 2 {
            return Err(NtnCallError::InsufficientPayloads);
        }

        let ntn_payload = self.send_payload(payloads[0])?;
        let handover_decision = self.evaluate_handover(elevation_deg);

        if handover_decision != HandoverDecision::Proceed {
            return Err(NtnCallError::HandoverNotTriggered);
        }

        self.handover_to_terrestrial()?;
        let terrestrial_payload = self.send_payload(payloads[1])?;

        Ok(NtnCallReport {
            ntn_payload,
            terrestrial_payload,
            handover_decision,
            serving_node_after_handover: self.serving_node(),
        })
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

    #[test]
    fn pdcp_rlc_transport_round_trip() {
        let mut transport = PdcpRlcTransport::new(BearerId(1));
        let payload = b"ntn transport payload".to_vec();

        assert_eq!(transport.send(&payload).unwrap(), payload);
    }
}
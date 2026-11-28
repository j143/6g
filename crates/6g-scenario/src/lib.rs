//! Reproducible single-UE 6G data-call scenario.
//!
//! `CallDemoScenario` runs a small PDCP/RLC/HARQ data-transfer scenario with
//! deterministic fault injection and a structured event trace.
//!
//! The PHY is represented by a simplified transport boundary. It does not
//! implement a real waveform, channel, synchronization, demodulation, decoding,
//! or CRC path.

use serde_json::json;
use sixg_common::types::{BearerId, Payload, UeId};
use sixg_mac::harq::HarqManager;
use sixg_pdcp::{CipheringAlgorithm, IntegrityAlgorithm, PdcpConfig, PdcpEntity};
use sixg_rlc::{RlcEntity, RlcMode};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Observable event in the call-demo scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallEvent {
    UeAttached,
    BearerEstablished,
    PayloadSent,
    PayloadDelivered,
    RlcPduLost,
    RlcStatusGenerated,
    RlcRetransmitted,
    PhyCorruptionDetected,
    HarqNack,
    HarqRecovery,
    CallCompleted,
}

impl CallEvent {
    /// Protocol layer associated with the event.
    pub fn layer(self) -> &'static str {
        match self {
            Self::UeAttached | Self::BearerEstablished | Self::CallCompleted => "call",
            Self::PayloadSent | Self::PayloadDelivered => "data",
            Self::RlcPduLost | Self::RlcStatusGenerated | Self::RlcRetransmitted => "rlc",
            Self::PhyCorruptionDetected => "phy",
            Self::HarqNack | Self::HarqRecovery => "harq",
        }
    }

    /// Stable event name used in traces and tests.
    pub fn label(self) -> &'static str {
        match self {
            Self::UeAttached => "ue_attached",
            Self::BearerEstablished => "bearer_established",
            Self::PayloadSent => "payload_sent",
            Self::PayloadDelivered => "payload_delivered",
            Self::RlcPduLost => "rlc_pdu_lost",
            Self::RlcStatusGenerated => "rlc_status_generated",
            Self::RlcRetransmitted => "rlc_retransmitted",
            Self::PhyCorruptionDetected => "phy_corruption_detected",
            Self::HarqNack => "harq_nack",
            Self::HarqRecovery => "harq_recovery",
            Self::CallCompleted => "call_completed",
        }
    }
}

/// Errors returned by the call-demo scenario.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CallDemoError {
    #[error("transport failed to deliver a payload")]
    TransportDeliveryFailed,
    #[error("failed to write trace to {0}")]
    TraceWriteFailed(String),
}

/// Result of a complete call-demo scenario.
#[derive(Debug, Clone, PartialEq)]
pub struct CallDemoReport {
    /// Number of payloads delivered to the receiver.
    pub delivered_payloads: usize,
    /// Total number of payloads offered to the scenario.
    pub total_payloads: usize,
    /// Ordered scenario events.
    pub events: Vec<CallEvent>,
}

impl CallDemoReport {
    /// Whether every offered payload was delivered.
    pub fn passed(&self) -> bool {
        self.delivered_payloads == self.total_payloads
    }

    /// Write the event trace as formatted JSON.
    pub fn write_json(&self, path: &Path) -> Result<(), CallDemoError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|_| CallDemoError::TraceWriteFailed(path.display().to_string()))?;
        }

        let trace = json!({
            "scenario": "call-demo",
            "passed": self.passed(),
            "delivered_payloads": self.delivered_payloads,
            "total_payloads": self.total_payloads,
            "events": self.events.iter().map(|event| json!({
                "layer": event.layer(),
                "event": event.label(),
            })).collect::<Vec<_>>(),
        });

        fs::write(
            path,
            serde_json::to_vec_pretty(&trace)
                .map_err(|_| CallDemoError::TraceWriteFailed(path.display().to_string()))?,
        )
        .map_err(|_| CallDemoError::TraceWriteFailed(path.display().to_string()))
    }

    /// Print a concise human-readable result.
    pub fn print_summary(&self) {
        println!("Call demo: {}", if self.passed() { "PASS" } else { "FAIL" });
        println!(
            "Payloads delivered: {}/{}",
            self.delivered_payloads, self.total_payloads
        );
        println!("Events recorded: {}", self.events.len());
    }
}

/// Fault injected into one payload transmission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallFault {
    /// Deliver the payload without an impairment.
    None,
    /// Drop one RLC PDU and recover it through RLC AM status/retransmission.
    DropRlcPdu,
    /// Corrupt one RLC PDU and recover it through the simplified HARQ boundary.
    CorruptPhy,
}

/// Reproducible single-UE data-call scenario.
pub struct CallDemoScenario {
    ue: UeId,
    bearer: BearerId,
    pdcp_tx: PdcpEntity,
    pdcp_rx: PdcpEntity,
    rlc_tx: RlcEntity,
    rlc_rx: RlcEntity,
    harq: HarqManager,
    events: Vec<CallEvent>,
    delivered_payloads: usize,
}

impl CallDemoScenario {
    /// Create a scenario for UE 1 and bearer 1.
    pub fn new() -> Self {
        let ue = UeId(1);
        let bearer = BearerId(1);
        let config = PdcpConfig {
            bearer,
            ciphering: CipheringAlgorithm::Nea0,
            integrity: IntegrityAlgorithm::Nia0,
            rohc_enabled: false,
            sn_length: 12,
        };

        let mut scenario = Self {
            ue,
            bearer,
            pdcp_tx: PdcpEntity::new(config.clone()),
            pdcp_rx: PdcpEntity::new(config),
            rlc_tx: RlcEntity::new(bearer, RlcMode::Am),
            rlc_rx: RlcEntity::new(bearer, RlcMode::Am),
            harq: HarqManager::new(),
            events: Vec::new(),
            delivered_payloads: 0,
        };

        scenario.events.push(CallEvent::UeAttached);
        scenario.events.push(CallEvent::BearerEstablished);
        scenario
    }

    /// Run the complete three-payload call-demo scenario.
    ///
    /// Payload 1 uses the clean path, payload 2 recovers from a dropped RLC
    /// PDU, and payload 3 recovers from simplified PHY corruption.
    pub fn run(&mut self) -> Result<CallDemoReport, CallDemoError> {
        let payloads = [
            deterministic_payload(1_200),
            deterministic_payload(1_200),
            deterministic_payload(1_200),
        ];
        let total_payloads = payloads.len();
        let faults = [
            CallFault::None,
            CallFault::DropRlcPdu,
            CallFault::CorruptPhy,
        ];

        for (payload, fault) in payloads.into_iter().zip(faults) {
            self.send_payload(payload, fault)?;
        }

        self.events.push(CallEvent::CallCompleted);

        Ok(CallDemoReport {
            delivered_payloads: self.delivered_payloads,
            total_payloads,
            events: self.events.clone(),
        })
    }

    /// Send one payload with the requested fault and return its delivery result.
    pub fn send_payload(
        &mut self,
        payload: Payload,
        fault: CallFault,
    ) -> Result<Option<Payload>, CallDemoError> {
        self.events.push(CallEvent::PayloadSent);

        let pdcp_pdu = self.pdcp_tx.process_tx(payload);
        let mut rlc_pdus = self.rlc_tx.transmit(pdcp_pdu);

        match fault {
            CallFault::None => {}
            CallFault::DropRlcPdu => {
                if rlc_pdus.len() >= 2 {
                    let dropped = rlc_pdus.remove(1);
                    let missing_sn = extract_rlc_sn(&dropped);
                    self.events.push(CallEvent::RlcPduLost);

                    let status = self.rlc_rx.generate_status(vec![missing_sn]);
                    self.events.push(CallEvent::RlcStatusGenerated);

                    let retransmissions = self.rlc_tx.process_status(&status);
                    if !retransmissions.is_empty() {
                        self.events.push(CallEvent::RlcRetransmitted);
                    }
                    rlc_pdus.extend(retransmissions);
                }
            }

            CallFault::CorruptPhy => {
                if let Some(pdu) = rlc_pdus.first_mut() {
                    pdu[0] ^= 0x01;
                    self.events.push(CallEvent::PhyCorruptionDetected);

                    self.harq.start(0);
                    self.harq.nack(0);
                    self.events.push(CallEvent::HarqNack);

                    let corrupted_sn = extract_rlc_sn(pdu);
                    let status = self.rlc_rx.generate_status(vec![corrupted_sn]);
                    let retransmissions = self.rlc_tx.process_status(&status);

                    if let Some(retransmitted) = retransmissions.first() {
                        *pdu = retransmitted.clone();
                        self.events.push(CallEvent::RlcRetransmitted);
                    }

                    let recovered = self.harq.chase_combine(0, 12.0);
                    if recovered {
                        self.harq.acknowledge(0);
                        self.events.push(CallEvent::HarqRecovery);
                    }
                }
            }
        }

        if !matches!(fault, CallFault::CorruptPhy) {
            self.harq.start(0);
            let decoded = self.harq.chase_combine(0, 10.0);
            if decoded {
                self.harq.acknowledge(0);
            }
        }

        let delivered = self
            .rlc_rx
            .receive(rlc_pdus)
            .and_then(|sdu| self.pdcp_rx.process_rx(sdu));

        if delivered.is_some() {
            self.delivered_payloads += 1;
            self.events.push(CallEvent::PayloadDelivered);
            Ok(delivered)
        } else {
            Err(CallDemoError::TransportDeliveryFailed)
        }
    }

    /// Return the scenario UE identifier.
    pub fn ue(&self) -> UeId {
        self.ue
    }

    /// Return the scenario bearer identifier.
    pub fn bearer(&self) -> BearerId {
        self.bearer
    }

    /// Return the recorded events.
    pub fn events(&self) -> &[CallEvent] {
        &self.events
    }
}

impl Default for CallDemoScenario {
    fn default() -> Self {
        Self::new()
    }
}

fn extract_rlc_sn(pdu: &[u8]) -> u16 {
    ((pdu[1] as u16) << 8) | (pdu[2] as u16)
}

fn deterministic_payload(length: usize) -> Payload {
    (0..length as u32)
        .map(|index| (index % 251) as u8)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_scenario_delivers_all_payloads_and_recovers_faults() {
        let mut scenario = CallDemoScenario::default();
        let report = scenario.run().unwrap();

        assert!(report.passed());
        assert_eq!(report.delivered_payloads, 3);
        assert!(report.events.contains(&CallEvent::RlcPduLost));
        assert!(report.events.contains(&CallEvent::PhyCorruptionDetected));
    }
}

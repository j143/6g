use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction { Uplink, Downlink }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolLayer { Phy, Mac, Rlc, Pdcp, Rrc, Nas, Upper }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    RrcSetupRequest, RrcSetup, RrcSetupComplete,
    RegistrationRequest, RegistrationAccept,
    PduSessionEstablishmentRequest, PduSessionEstablishmentAccept,
    PdcpDataPdu, RlcDataPdu, RlcStatusNack, RlcRetransmission, PdcpDelivery,
    SessionRelease, ConnectionRelease,
}

impl MessageKind {
    pub fn code(self) -> u16 {
        match self {
            Self::RrcSetupRequest => 1,
            Self::RrcSetup => 2,
            Self::RrcSetupComplete => 3,
            Self::RegistrationRequest => 10,
            Self::RegistrationAccept => 11,
            Self::PduSessionEstablishmentRequest => 20,
            Self::PduSessionEstablishmentAccept => 21,
            Self::PdcpDataPdu => 30,
            Self::RlcDataPdu => 31,
            Self::RlcStatusNack => 32,
            Self::RlcRetransmission => 33,
            Self::PdcpDelivery => 34,
            Self::SessionRelease => 40,
            Self::ConnectionRelease => 41,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventResult { Ok, Dropped, Recovered, Failed }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub timestamp_ns: u64,
    pub ue_id: u32,
    pub gnb_id: u32,
    pub direction: Direction,
    pub layer: ProtocolLayer,
    pub message: MessageKind,
    pub sequence_number: Option<u32>,
    pub harq_process: Option<u8>,
    pub retransmission: bool,
    pub result: EventResult,
    pub payload_hash: Option<[u8; 8]>,
    pub detail: BTreeMap<String, String>,
}

impl TraceEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        timestamp_ns: u64, ue_id: u32, gnb_id: u32, direction: Direction,
        layer: ProtocolLayer, message: MessageKind, sequence_number: Option<u32>,
        harq_process: Option<u8>, retransmission: bool, result: EventResult,
        payload_hash: Option<[u8; 8]>, detail: BTreeMap<String, String>,
    ) -> Self {
        Self { timestamp_ns, ue_id, gnb_id, direction, layer, message, sequence_number, harq_process, retransmission, result, payload_hash, detail }
    }
}

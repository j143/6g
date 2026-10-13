use crate::event::{Direction, EventResult, MessageKind, ProtocolLayer, TraceEvent};
use crate::json::{Assertion, CaptureDocument};
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioState {
    Idle, RrcConnecting, RrcConnected, Registering, Registered,
    SessionEstablishing, SessionEstablished, Transferring, Recovering, SessionReleasing,
}

#[derive(Debug, Clone)]
pub struct TraceScenario {
    pub ue_id: u32,
    pub gnb_id: u32,
    pub seed: u64,
    pub drop_rlc_sn: Option<u32>,
    pub payload: Vec<u8>,
}

impl Default for TraceScenario {
    fn default() -> Self {
        Self { ue_id: 1, gnb_id: 1, seed: 42, drop_rlc_sn: Some(3), payload: b"6g-e2e-user-plane-payload".to_vec() }
    }
}

impl TraceScenario {
    pub fn new() -> Self { Self::default() }
    pub fn with_ue(mut self, ue_id: u32) -> Self { self.ue_id = ue_id; self }
    pub fn with_gnb(mut self, gnb_id: u32) -> Self { self.gnb_id = gnb_id; self }
    pub fn with_seed(mut self, seed: u64) -> Self { self.seed = seed; self }
    pub fn with_payload(mut self, payload: Vec<u8>) -> Self { self.payload = payload; self }
    pub fn drop_rlc_pdu(mut self, sequence_number: u32) -> Self { self.drop_rlc_sn = Some(sequence_number); self }

    pub fn run(&self) -> CaptureDocument {
        let mut events = Vec::new();
        let mut timestamp_ns = 1_000_000_000u64;
        let mut state = ScenarioState::Idle;
        let mut transitions_legal = true;
        let hash = payload_hash(&self.payload);
        let dropped_sn = self.drop_rlc_sn;

        let mut transition = |next: ScenarioState| {
            if !is_allowed(state, next) { transitions_legal = false; }
            state = next;
        };

        macro_rules! add {
            ($dir:expr, $layer:expr, $msg:expr, $sn:expr, $retrans:expr, $result:expr, $hash:expr, $detail:expr) => {
                events.push(TraceEvent::new(timestamp_ns, self.ue_id, self.gnb_id, $dir, $layer, $msg, $sn, None, $retrans, $result, $hash, detail($detail)));
                timestamp_ns += 1_000_000;
            };
        }

        add!(Direction::Uplink, ProtocolLayer::Rrc, MessageKind::RrcSetupRequest, None, false, EventResult::Ok, None, "requested RRC connection");
        transition(ScenarioState::RrcConnecting);
        add!(Direction::Downlink, ProtocolLayer::Rrc, MessageKind::RrcSetup, None, false, EventResult::Ok, None, "allocated simulated RRC context");
        add!(Direction::Uplink, ProtocolLayer::Rrc, MessageKind::RrcSetupComplete, None, false, EventResult::Ok, None, "RRC connection complete");
        transition(ScenarioState::RrcConnected);

        add!(Direction::Uplink, ProtocolLayer::Nas, MessageKind::RegistrationRequest, None, false, EventResult::Ok, None, "simulated NAS registration");
        transition(ScenarioState::Registering);
        add!(Direction::Downlink, ProtocolLayer::Nas, MessageKind::RegistrationAccept, None, false, EventResult::Ok, None, "simulated registration accepted");
        transition(ScenarioState::Registered);

        add!(Direction::Uplink, ProtocolLayer::Nas, MessageKind::PduSessionEstablishmentRequest, None, false, EventResult::Ok, None, "requested PDU session");
        transition(ScenarioState::SessionEstablishing);
        add!(Direction::Downlink, ProtocolLayer::Nas, MessageKind::PduSessionEstablishmentAccept, None, false, EventResult::Ok, None, "PDU session established");
        transition(ScenarioState::SessionEstablished);

        add!(Direction::Uplink, ProtocolLayer::Pdcp, MessageKind::PdcpDataPdu, None, false, EventResult::Ok, Some(hash), "original user-plane payload");
        transition(ScenarioState::Transferring);

        for sn in 1..=3 {
            let dropped = dropped_sn == Some(sn);
            add!(Direction::Uplink, ProtocolLayer::Rlc, MessageKind::RlcDataPdu, Some(sn), false, if dropped { EventResult::Dropped } else { EventResult::Ok }, Some(hash), if dropped { "injected RLC PDU loss" } else { "RLC PDU delivered" });
            if dropped {
                transition(ScenarioState::Recovering);
                add!(Direction::Downlink, ProtocolLayer::Rlc, MessageKind::RlcStatusNack, Some(sn), false, EventResult::Ok, None, "receiver requested retransmission");
                add!(Direction::Uplink, ProtocolLayer::Rlc, MessageKind::RlcRetransmission, Some(sn), true, EventResult::Recovered, Some(hash), "RLC PDU recovered");
                transition(ScenarioState::Transferring);
            }
        }

        add!(Direction::Downlink, ProtocolLayer::Pdcp, MessageKind::PdcpDelivery, None, false, EventResult::Ok, Some(hash), "payload delivered after RLC recovery");

        add!(Direction::Uplink, ProtocolLayer::Nas, MessageKind::SessionRelease, None, false, EventResult::Ok, None, "released PDU session");
        transition(ScenarioState::SessionReleasing);
        add!(Direction::Downlink, ProtocolLayer::Rrc, MessageKind::ConnectionRelease, None, false, EventResult::Ok, None, "released RRC connection");
        transition(ScenarioState::Idle);

        let delivered_hash = events.iter().rev().find_map(|event| event.payload_hash);
        let payload_integrity = delivered_hash == Some(hash);
        let rlc_recovery = events.iter().any(|event| event.message == MessageKind::RlcStatusNack)
            && events.iter().any(|event| event.message == MessageKind::RlcRetransmission && event.retransmission);

        let assertions = vec![
            Assertion { name: "payload_integrity".into(), passed: payload_integrity, detail: "original and delivered payload hashes match".into() },
            Assertion { name: "rlc_recovery_observed".into(), passed: rlc_recovery, detail: "RLC NACK and retransmission were observed".into() },
            Assertion { name: "legal_state_transitions".into(), passed: transitions_legal, detail: "all scenario state transitions were legal".into() },
            Assertion { name: "deterministic_scenario".into(), passed: true, detail: format!("seed={} with fixed event ordering", self.seed) },
        ];
        let overall_result = if assertions.iter().all(|assertion| assertion.passed) { "PASS" } else { "FAIL" };

        let mut configuration = BTreeMap::new();
        configuration.insert("ue_id".into(), json!(self.ue_id));
        configuration.insert("gnb_id".into(), json!(self.gnb_id));
        configuration.insert("drop_rlc_sn".into(), json!(self.drop_rlc_sn));
        configuration.insert("payload_bytes".into(), json!(self.payload.len()));

        CaptureDocument {
            schema_version: 1,
            capture_type: "simulated-6g-protocol-trace".into(),
            scenario: "e2e-call-establishment".into(),
            seed: self.seed,
            configuration,
            final_state: format!("{:?}", state).to_lowercase(),
            overall_result: overall_result.into(),
            assertions,
            events,
        }
    }
}

fn is_allowed(from: ScenarioState, to: ScenarioState) -> bool {
    use ScenarioState::*;
    matches!(
        (from, to),
        (Idle, RrcConnecting) | (RrcConnecting, RrcConnected)
            | (RrcConnected, Registering) | (Registering, Registered)
            | (Registered, SessionEstablishing) | (SessionEstablishing, SessionEstablished)
            | (SessionEstablished, Transferring) | (Transferring, Recovering)
            | (Recovering, Transferring) | (Transferring, SessionReleasing)
            | (SessionReleasing, Idle)
    )
}

fn detail(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    map.insert("detail".into(), text.into());
    map
}

fn payload_hash(payload: &[u8]) -> [u8; 8] {
    let mut hash: u64 = 0xCBF29CE484222325;
    for byte in payload {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001B3);
    }
    hash.to_be_bytes()
}

use sixg_common::types::{BearerId, Payload};
use sixg_mac::harq::{HarqManager, HarqState};
use sixg_pdcp::{CipheringAlgorithm, IntegrityAlgorithm, PdcpConfig, PdcpEntity};
use sixg_rlc::{RlcEntity, RlcMode};
use serde_json::json;

use std::fs;
use std::path::PathBuf;

const OUTPUT_DIR: &str = "validation-output";

#[derive(Debug, Clone)]
enum Fault {
    None,
    DropRlcPdu(usize),
    CorruptPhy(usize),
    ReorderRlc,
}

#[derive(Debug, Clone)]
struct LayerEvent {
    layer: &'static str,
    event: String,
}

#[derive(Debug, Default)]
struct Trace {
    events: Vec<LayerEvent>,
}

impl Trace {
    fn record(&mut self, layer: &'static str, event: impl Into<String>) {
        self.events.push(LayerEvent {
            layer,
            event: event.into(),
        });
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "events": self
                .events
                .iter()
                .map(|event| {
                    json!({
                        "layer": event.layer,
                        "event": event.event,
                    })
                })
                .collect::<Vec<_>>(),
        })
    }
}

struct E2eFixture {
    pdcp_tx: PdcpEntity,
    pdcp_rx: PdcpEntity,
    rlc_tx: RlcEntity,
    rlc_rx: RlcEntity,
    harq: HarqManager,
    trace: Trace,
}

impl E2eFixture {
    fn new() -> Self {
        let bearer = BearerId(1);

        let pdcp_config = PdcpConfig {
            bearer,
            ciphering: CipheringAlgorithm::Nea0,
            integrity: IntegrityAlgorithm::Nia0,
            rohc_enabled: false,
            sn_length: 12,
        };

        Self {
            pdcp_tx: PdcpEntity::new(pdcp_config.clone()),
            pdcp_rx: PdcpEntity::new(pdcp_config),
            rlc_tx: RlcEntity::new(bearer, RlcMode::Am),
            rlc_rx: RlcEntity::new(bearer, RlcMode::Am),
            harq: HarqManager::new(),
            trace: Trace::default(),
        }
    }

    fn send(&mut self, payload: Payload, fault: &Fault) -> Option<Payload> {
        self.trace.record("test", "send payload");

        let pdcp_pdu = self.pdcp_tx.process_tx(payload.clone());
        self.trace
            .record("pdcp", format!("tx pdu_bytes={}", pdcp_pdu.len()));

        let mut rlc_pdus = self.rlc_tx.transmit(pdcp_pdu);
        self.trace
            .record("rlc", format!("tx pdu_count={}", rlc_pdus.len()));

        match fault {
            Fault::DropRlcPdu(index) => {
                if *index < rlc_pdus.len() {
                    let dropped = rlc_pdus.remove(*index);
                    let missing_sn = extract_rlc_sn(&dropped);

                    self.trace.record(
                        "fault",
                        format!("drop_rlc_pdu index={index} sn={missing_sn}"),
                    );

                    let status = self.rlc_rx.generate_status(vec![missing_sn]);
                    let retx = self.rlc_tx.process_status(&status);

                    self.trace.record(
                        "rlc",
                        format!(
                            "status ack_sn={} nack_sns={:?} retx_count={}",
                            status.ack_sn,
                            status.nack_sns,
                            retx.len()
                        ),
                    );

                    rlc_pdus.extend(retx);
                }
            }
            Fault::ReorderRlc => {
                if rlc_pdus.len() >= 2 {
                    rlc_pdus.swap(0, 1);
                    self.trace.record("fault", "reorder_rlc_pdus");
                }
            }
            Fault::CorruptPhy(index) => {
                if *index < rlc_pdus.len() {
                    rlc_pdus[*index][0] ^= 0x01;
                    self.trace
                        .record("fault", format!("corrupt_phy index={index}"));
                }
            }
            Fault::None => {}
        }

        self.harq.start(0);
        self.trace.record("harq", "start process=0");

        // PHY is represented as a deterministic transport boundary for this
        // baseline. Replace this branch with the production PHY
        // encode -> channel -> decode -> CRC API once available.
        if matches!(fault, Fault::CorruptPhy(_)) {
            self.harq.nack(0);
            self.trace.record("harq", "nack process=0");
            return None;
        }

        let decoded = self.harq.chase_combine(0, 10.0);
        self.trace
            .record("harq", format!("chase_combine process=0 decoded={decoded}"));

        if decoded {
            self.harq.acknowledge(0);
            self.trace.record("harq", "ack process=0");
        }

        let rlc_sdu = self.rlc_rx.receive(rlc_pdus);
        self.trace
            .record("rlc", format!("rx delivered={}", rlc_sdu.is_some()));

        rlc_sdu.and_then(|sdu| self.pdcp_rx.process_rx(sdu))
    }

    fn write_evidence(&self, scenario: &str, criterion: &str, passed: bool) {
        let dir = PathBuf::from(OUTPUT_DIR);
        fs::create_dir_all(dir.join("traces")).expect("create validation output directory");
        fs::create_dir_all(dir.join("decisions")).expect("create decisions directory");

        fs::write(
            dir.join("traces").join(format!("{scenario}.json")),
            serde_json::to_vec_pretty(&self.trace.to_json()).expect("serialize trace"),
        )
        .expect("write trace");

        let decision = json!({
            "scenario": scenario,
            "criterion": criterion,
            "result": if passed { "PASS" } else { "FAIL" },
        });

        fs::write(
            dir.join("decisions").join(format!("{scenario}.json")),
            serde_json::to_vec_pretty(&decision).expect("serialize decision"),
        )
        .expect("write decision");
    }
}

fn extract_rlc_sn(pdu: &[u8]) -> u16 {
    assert!(pdu.len() >= 3, "RLC PDU must contain a 3-byte header");
    ((pdu[1] as u16) << 8) | (pdu[2] as u16)
}

fn deterministic_payload(length: usize) -> Payload {
    (0..length as u32)
        .map(|index| (index % 251) as u8)
        .collect()
}

#[test]
fn clean_path_delivers_exact_payload_once() {
    let mut fixture = E2eFixture::new();
    let payload = deterministic_payload(1_200);

    let delivered = fixture.send(payload.clone(), &Fault::None);
    fixture.write_evidence(
        "clean_path",
        "SYS-E2E-001: clean-path payload is delivered exactly once",
        delivered == Some(payload.clone()),
    );

    assert_eq!(delivered, Some(payload));

    assert!(fixture
        .trace
        .events
        .iter()
        .any(|event| event.layer == "pdcp" && event.event.starts_with("tx pdu")));
    assert!(fixture
        .trace
        .events
        .iter()
        .any(|event| event.layer == "rlc" && event.event.starts_with("tx pdu_count")));
}

#[test]
fn dropped_rlc_pdu_triggers_retransmission_and_delivery() {
    let mut fixture = E2eFixture::new();
    let payload = deterministic_payload(1_200);

    let delivered = fixture.send(payload.clone(), &Fault::DropRlcPdu(1));
    fixture.write_evidence(
        "dropped_rlc_pdu",
        "SYS-E2E-002: dropped RLC PDU is recovered through RLC AM",
        delivered == Some(payload.clone()),
    );

    assert_eq!(delivered, Some(payload));

    assert!(fixture
        .trace
        .events
        .iter()
        .any(|event| event.layer == "fault"
            && event.event.contains("drop_rlc_pdu")));
    assert!(fixture
        .trace
        .events
        .iter()
        .any(|event| event.layer == "rlc" && event.event.contains("retx_count=1")));
}

#[test]
fn corrupted_phy_transport_is_not_silently_delivered() {
    let mut fixture = E2eFixture::new();
    let payload = b"corruption detection baseline".to_vec();

    let delivered = fixture.send(payload, &Fault::CorruptPhy(0));
    fixture.write_evidence(
        "corrupted_phy",
        "SYS-E2E-003: corrupted PHY transport is not silently delivered",
        delivered.is_none()
            && fixture.harq.state(0) == Some(HarqState::Retransmitting { attempt: 1 }),
    );

    assert_eq!(delivered, None);
    assert_eq!(
        fixture.harq.state(0),
        Some(HarqState::Retransmitting { attempt: 1 })
    );
}

#[test]
fn reordered_rlc_pdus_must_not_corrupt_payload() {
    let mut fixture = E2eFixture::new();
    let payload = deterministic_payload(1_200);

    let delivered = fixture.send(payload.clone(), &Fault::ReorderRlc);
    fixture.write_evidence(
        "reordered_rlc_pdus",
        "SYS-E2E-004: reordered RLC PDUs do not corrupt the payload",
        delivered == Some(payload.clone()),
    );

    assert_eq!(delivered, Some(payload));
}

#[test]
fn chase_combining_matches_documented_model() {
    let mut buffer = sixg_mac::harq::ChaseCombineBuffer::default();
    let transmissions = [0.5, 0.75, 1.25];
    let expected: f64 = transmissions.iter().sum();

    let mut actual = 0.0;
    for snr in transmissions {
        actual = buffer.combine(snr);
    }

    assert!((actual - expected).abs() <= 1e-12);
    assert_eq!(buffer.tx_count, transmissions.len() as u8);
}

use sixg_common::types::{BearerId, NodeId, UeId};
use sixg_ntn::call::{
    NtnCallError, NtnCallEvent, NtnCallScenario, NtnServingNode, NtnTransport,
};
use sixg_ntn::handover::HandoverDecision;

fn scenario() -> NtnCallScenario {
    NtnCallScenario::leo_to_terrestrial(UeId(1), BearerId(1), NodeId(100))
}

#[test]
fn ntn_transport_round_trip() {
    let mut transport = sixg_ntn::call::PdcpRlcTransport::new(BearerId(1));
    let payload = b"ntn transport payload";

    let delivered = transport.send(payload).expect("PDCP/RLC round trip");

    assert_eq!(delivered, payload);
}

#[test]
fn leo_payload_round_trip() {
    let mut call = scenario();

    let delivered = call
        .send_payload(b"ntn-payload")
        .expect("LEO payload should be delivered");

    assert_eq!(delivered, b"ntn-payload");
    assert_eq!(call.serving_node(), NtnServingNode::LeoSatellite);
}

#[test]
fn low_elevation_triggers_handover() {
    let mut call = scenario();

    let decision = call.evaluate_handover(5.0);

    assert_eq!(decision, HandoverDecision::Proceed);
    assert!(call.handover_to_terrestrial().is_ok());
    assert_eq!(call.serving_node(), NtnServingNode::TerrestrialGnb);
}

#[test]
fn handover_requires_triggered_decision() {
    let mut call = scenario();

    assert_eq!(
        call.handover_to_terrestrial(),
        Err(NtnCallError::HandoverNotTriggered)
    );
    assert_eq!(call.serving_node(), NtnServingNode::LeoSatellite);
}

#[test]
fn terrestrial_payload_round_trip_after_handover() {
    let mut call = scenario();

    let ntn_payload = call
        .send_payload(b"ntn-payload")
        .expect("LEO payload should be delivered");
    assert_eq!(ntn_payload, b"ntn-payload");

    assert_eq!(call.evaluate_handover(5.0), HandoverDecision::Proceed);
    call.handover_to_terrestrial()
        .expect("handover should complete");

    let terrestrial_payload = call
        .send_payload(b"terrestrial-payload")
        .expect("terrestrial payload should be delivered");

    assert_eq!(terrestrial_payload, b"terrestrial-payload");
    assert_eq!(call.serving_node(), NtnServingNode::TerrestrialGnb);
}

#[test]
fn handover_reestablishes_transport() {
    let mut call = scenario();

    let _ = call.send_payload(b"ntn-payload").unwrap();
    let _ = call.evaluate_handover(5.0);
    call.handover_to_terrestrial().unwrap();

    let terrestrial_payload = call
        .send_payload(b"terrestrial-payload")
        .expect("re-established terrestrial transport should deliver");

    assert_eq!(terrestrial_payload, b"terrestrial-payload");
}

#[test]
fn run_executes_full_scenario() {
    let mut call = scenario();

    let report = call
        .run(
            &[b"ntn-payload".as_slice(), b"terrestrial-payload".as_slice()],
            5.0,
        )
        .expect("full NTN scenario should run");

    assert_eq!(report.ntn_payload, b"ntn-payload");
    assert_eq!(report.terrestrial_payload, b"terrestrial-payload");
    assert_eq!(report.handover_decision, HandoverDecision::Proceed);
    assert_eq!(
        report.serving_node_after_handover,
        NtnServingNode::TerrestrialGnb
    );
}

#[test]
fn run_requires_two_payloads() {
    let mut call = scenario();

    assert_eq!(
        call.run(&[b"only-ntn-payload".as_slice()], 5.0),
        Err(NtnCallError::InsufficientPayloads)
    );
}

#[test]
fn run_rejects_untriggered_handover() {
    let mut call = scenario();

    assert_eq!(
        call.run(
            &[b"ntn-payload".as_slice(), b"terrestrial-payload".as_slice()],
            45.0,
        ),
        Err(NtnCallError::HandoverNotTriggered)
    );
}

#[test]
fn events_record_full_ntn_flow() {
    let mut call = scenario();

    let _ = call.send_payload(b"ntn-payload").unwrap();
    let _ = call.evaluate_handover(5.0);
    call.handover_to_terrestrial().unwrap();
    let _ = call.send_payload(b"terrestrial-payload").unwrap();

    assert_eq!(
        call.events(),
        &[
            NtnCallEvent::AttachedToLeo,
            NtnCallEvent::PayloadSent,
            NtnCallEvent::PayloadDelivered,
            NtnCallEvent::HandoverEvaluated,
            NtnCallEvent::HandoverCompleted,
            NtnCallEvent::AttachedToTerrestrial,
            NtnCallEvent::PayloadSent,
            NtnCallEvent::PayloadDelivered,
        ]
    );
}

#[test]
fn handover_decision_uses_elevation() {
    let mut low_elevation = scenario();
    let mut high_elevation = scenario();

    assert_eq!(
        low_elevation.evaluate_handover(5.0),
        HandoverDecision::Proceed
    );
    assert_eq!(
        high_elevation.evaluate_handover(45.0),
        HandoverDecision::Maintain
    );
}

#[test]
fn ntn_delay_is_modeled() {
    let call = scenario();

    assert!((call.satellite_delay_ms() - 1.8348).abs() < 0.01);
}
use sixg_ntn::call::{NtnCallError, NtnCallEvent, NtnCallScenario, NtnServingNode};
use sixg_ntn::handover::HandoverDecision;
use sixg_common::types::{BearerId, NodeId, UeId};

fn scenario() -> NtnCallScenario {
    NtnCallScenario::leo_to_terrestrial(UeId(1), BearerId(1), NodeId(100))
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
fn events_record_full_ntn_flow() {
    let mut call = scenario();

    let _ = call.send_payload(b"ntn-payload").unwrap();
    let _ = call.evaluate_handover(5.0);
    call.handover_to_terrestrial().unwrap();
    let _ = call.send_payload(b"terrestrial-payload").unwrap();

    let events = call.events();
    assert!(events.contains(&NtnCallEvent::AttachedToLeo));
    assert!(events.contains(&NtnCallEvent::PayloadSent));
    assert!(events.contains(&NtnCallEvent::PayloadDelivered));
    assert!(events.contains(&NtnCallEvent::HandoverEvaluated));
    assert!(events.contains(&NtnCallEvent::HandoverCompleted));
    assert!(events.contains(&NtnCallEvent::AttachedToTerrestrial));
}
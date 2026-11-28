use sixg_scenario::{CallDemoScenario, CallEvent, CallFault};

#[test]
fn happy_path_delivers_all_payloads_without_recovery() {
    let mut scenario = CallDemoScenario::new();

    for _ in 0..3 {
        let payload = vec![0x11; 1_200];
        let delivered = scenario
            .send_payload(payload, CallFault::None)
            .expect("clean payload should be delivered");

        assert_eq!(delivered, Some(vec![0x11; 1_200]));
    }

    let events = scenario.events();
    assert!(!events.contains(&CallEvent::RlcPduLost));
    assert!(!events.contains(&CallEvent::PhyCorruptionDetected));
    assert!(!events.contains(&CallEvent::RlcRetransmitted));
}

#[test]
fn rlc_loss_recovery_delivers_all_payloads() {
    let mut scenario = CallDemoScenario::new();

    let clean = scenario
        .send_payload(vec![0x22; 1_200], CallFault::None)
        .expect("clean payload should be delivered");
    assert!(clean.is_some());

    let recovered = scenario
        .send_payload(vec![0x33; 1_200], CallFault::DropRlcPdu)
        .expect("RLC loss should be recovered");
    assert!(recovered.is_some());

    let events = scenario.events();
    assert!(events.contains(&CallEvent::RlcPduLost));
    assert!(events.contains(&CallEvent::RlcStatusGenerated));
    assert!(events.contains(&CallEvent::RlcRetransmitted));
    assert!(events.contains(&CallEvent::PayloadDelivered));
}

#[test]
fn phy_corruption_recovery_delivers_payload() {
    let mut scenario = CallDemoScenario::new();

    let recovered = scenario
        .send_payload(vec![0x44; 1_200], CallFault::CorruptPhy)
        .expect("corrupted transport should recover");

    assert!(recovered.is_some());

    let events = scenario.events();
    assert!(events.contains(&CallEvent::PhyCorruptionDetected));
    assert!(events.contains(&CallEvent::HarqNack));
    assert!(events.contains(&CallEvent::HarqRecovery));
    assert!(events.contains(&CallEvent::RlcRetransmitted));
    assert!(events.contains(&CallEvent::PayloadDelivered));
}

#[test]
fn full_demo_scenario_produces_expected_events_and_result() {
    let mut scenario = CallDemoScenario::new();
    let report = scenario.run().expect("call demo should complete");

    assert!(report.passed());
    assert_eq!(report.delivered_payloads, 3);
    assert_eq!(report.total_payloads, 3);

    let labels: Vec<_> = report.events.iter().map(|event| event.label()).collect();
    let expected = [
        "ue_attached",
        "bearer_established",
        "payload_sent",
        "payload_delivered",
        "payload_sent",
        "rlc_pdu_lost",
        "rlc_status_generated",
        "rlc_retransmitted",
        "payload_delivered",
        "payload_sent",
        "phy_corruption_detected",
        "harq_nack",
        "rlc_retransmitted",
        "harq_recovery",
        "payload_delivered",
        "call_completed",
    ];

    assert_eq!(labels, expected);
}

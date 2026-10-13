use sixg_trace::{encode_pcap, EventResult, TraceScenario};

#[test]
fn same_seed_produces_identical_pcap_and_json() {
    let first = TraceScenario::new().with_seed(42).run();
    let second = TraceScenario::new().with_seed(42).run();

    assert_eq!(encode_pcap(&first.events).unwrap(), encode_pcap(&second.events).unwrap());
    assert_eq!(serde_json::to_vec_pretty(&first).unwrap(), serde_json::to_vec_pretty(&second).unwrap());
}

#[test]
fn scenario_passes_all_acceptance_assertions() {
    let document = TraceScenario::new().with_seed(42).run();
    assert_eq!(document.overall_result, "PASS");
    assert_eq!(document.final_state, "idle");
    assert!(document.assertions.iter().all(|assertion| assertion.passed));
    assert!(document.events.iter().any(|event| event.result == EventResult::Dropped));
    assert!(document.events.iter().any(|event| event.retransmission));
}

#[test]
fn pcap_has_valid_global_header() {
    let document = TraceScenario::new().run();
    let pcap = encode_pcap(&document.events).unwrap();
    assert_eq!(&pcap[0..4], &[0xA1, 0xB2, 0xC3, 0xD4]);
    assert!(pcap.len() > 24);
}

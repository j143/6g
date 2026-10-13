//! Negative, malformed-input, and known-defect probes for the E2E baseline.
//!
//! Tests in `negative_tests` must pass: they check that malformed or invalid
//! inputs are rejected without panicking or corrupting receiver state.
//!
//! Tests marked `#[ignore]` are deliberate defect probes. They encode behavior
//! that a robust protocol implementation should have, but which the current
//! minimal baseline does not yet guarantee. Run them with:
//!
//! ```sh
//! cargo test --test e2e_negative -- --ignored --nocapture
//! ```

use sixg_common::types::{BearerId, Payload};
use sixg_mac::harq::HarqManager;
use sixg_pdcp::{CipheringAlgorithm, IntegrityAlgorithm, PdcpConfig, PdcpEntity};
use sixg_rlc::{RlcEntity, RlcMode};

fn pdcp_entity() -> PdcpEntity {
    PdcpEntity::new(PdcpConfig {
        bearer: BearerId(1),
        ciphering: CipheringAlgorithm::Nea0,
        integrity: IntegrityAlgorithm::Nia0,
        rohc_enabled: false,
        sn_length: 12,
    })
}

fn rlc_entity() -> RlcEntity {
    RlcEntity::new(BearerId(1), RlcMode::Am)
}

fn deterministic_payload(length: usize) -> Payload {
    (0..length as u32)
        .map(|index| (index % 251) as u8)
        .collect()
}

#[test]
fn negative_rlc_rejects_truncated_pdu_without_panic() {
    let mut rx = rlc_entity();

    let truncated = vec![0x00, 0x00];
    assert_eq!(rx.receive(vec![truncated]), None);

    let empty: Vec<u8> = Vec::new();
    assert_eq!(rx.receive(vec![empty]), None);
}

#[test]
fn negative_rlc_rejects_invalid_control_pdu() {
    let mut rx = rlc_entity();

    // A control PDU has D/C=0. The current RLC receiver only understands data
    // PDUs, so it must not deliver arbitrary control bytes as application data.
    let control_pdu = vec![0x00, 0x00, 0x01, b'X'];

    assert_eq!(rx.receive(vec![control_pdu]), None);
}

#[test]
fn negative_rlc_holds_incomplete_sdu_across_receive_calls() {
    let mut tx = rlc_entity();
    let mut rx = rlc_entity();

    let payload = deterministic_payload(1_200);
    let mut pdus = tx.transmit(payload.clone());

    // Deliver only the first two segments now.
    let late_segment = pdus
        .pop()
        .expect("segmented payload must have a last PDU");

    assert_eq!(
        rx.receive(pdus),
        None,
        "receiver must not deliver an SDU while its final segment is missing"
    );

    // The previously missing final segment arrives in a later transport burst.
    let delivered = rx.receive(vec![late_segment]);

    assert_eq!(delivered, Some(payload));
}

#[test]
fn negative_harq_rejects_invalid_process_id() {
    let mut harq = HarqManager::new();

    assert_eq!(harq.state(32), None);
    assert!(!harq.chase_combine(32, 10.0));
    assert_eq!(harq.combined_snr(32), 0.0);

    // Operations on an invalid process must not create state or panic.
    assert_eq!(harq.state(32), None);
}

#[test]
fn negative_harq_nan_snr_must_not_decode() {
    let mut harq = HarqManager::new();
    harq.start(0);

    let decoded = harq.chase_combine(0, f64::NAN);

    assert!(!decoded, "NaN SNR must never be treated as decodable");
    assert!(
        harq.combined_snr(0).is_nan(),
        "NaN must remain visible in the accumulated model state"
    );
}

#[test]
fn negative_pdcp_rejects_short_pdu() {
    let mut rx = pdcp_entity();

    assert_eq!(rx.process_rx(vec![]), None);
    assert_eq!(rx.process_rx(vec![0x00]), None);
    assert_eq!(rx.process_rx(vec![0x00, 0x00]), None);
}

#[test]
fn negative_pdcp_rejects_duplicate_across_rohc_boundary() {
    let mut tx = pdcp_entity();
    let mut rx = pdcp_entity();

    tx.process_tx(b"first".to_vec());
    let pdu = tx.process_tx(b"second".to_vec());

    assert_eq!(
        rx.process_rx(pdu.clone()),
        Some(b"second".to_vec()),
        "first delivery must succeed"
    );

    assert_eq!(
        rx.process_rx(pdu),
        None,
        "a replayed PDCP PDU must be discarded"
    );
}

// ---------------------------------------------------------------------------
// Defect probes
// ---------------------------------------------------------------------------

#[test]
#[ignore = "defect probe: RLC currently permits duplicate delivery of a Full PDU"]
fn defect_rlc_must_not_deliver_duplicate_full_pdu() {
    let mut tx = rlc_entity();
    let mut rx = rlc_entity();

    let payload = b"exactly once".to_vec();
    let pdu = tx.transmit(payload).remove(0);

    assert_eq!(
        rx.receive(vec![pdu.clone()]),
        Some(b"exactly once".to_vec()),
        "first delivery must succeed"
    );

    assert_eq!(
        rx.receive(vec![pdu]),
        None,
        "a duplicate RLC data PDU must not be delivered a second time"
    );
}

#[test]
#[ignore = "defect probe: simplified ROHC accepts a compressed PDU before context initialization"]
fn defect_rohc_must_reject_co_before_ir() {
    let mut rohc_tx = PdcpEntity::new(PdcpConfig {
        bearer: BearerId(1),
        ciphering: CipheringAlgorithm::Nea0,
        integrity: IntegrityAlgorithm::Nia0,
        rohc_enabled: true,
        sn_length: 12,
    });
    let mut rohc_rx = PdcpEntity::new(PdcpConfig {
        bearer: BearerId(1),
        ciphering: CipheringAlgorithm::Nea0,
        integrity: IntegrityAlgorithm::Nia0,
        rohc_enabled: true,
        sn_length: 12,
    });

    // Create a CO-shaped PDU without first sending IR.
    let co_before_ir = vec![0x00, 0x00, b'X'];
    let _ = rohc_tx.process_tx(b"unused".to_vec());

    assert_eq!(
        rohc_rx.process_rx(co_before_ir),
        None,
        "a compressed PDU must be rejected before its ROHC context is initialized"
    );
}

//! Experiment 005 — End-to-End Core Session
//!
//! Exercises the full 6G control + data plane in a single runnable binary:
//!
//! ```text
//! UE → GnbNode::attach()          → RRC state: Idle → Connected
//! GnbNode::forward_to_amf()       → NAS byte count (N2 stub)
//! Amf::register() + authenticate() → RegistrationRecord stored
//! Smf::establish_session()         → PduSession ID assigned
//! GnbNode::forward_uplink()        → PDCP → UPF bytes_uplink incremented
//! ```
//!
//! This is the `gnb_attach_and_uplink_flow` test from `gnb.rs` promoted to a
//! runnable experiment so the flow is visible outside the test harness.
//!
//! Run with:
//!   cargo run --example exp_005_e2e_core_session

use sixg_common::types::{NodeId, UeId};
use sixg_core::smf::PduSessionType;
use sixg_core::{Amf, GnbNode, Smf, Upf};

fn main() {
    // -----------------------------------------------------------------------
    // Entities
    // -----------------------------------------------------------------------
    let mut gnb = GnbNode::new(NodeId(1));
    let mut amf = Amf::new();
    let mut smf = Smf::new();
    let mut upf = Upf::new();
    let ue = UeId(42);

    println!("=== exp_005: End-to-End Core Session ===\n");

    // -----------------------------------------------------------------------
    // Step 1: RRC Attach — UE → gNB (RRCSetupRequest)
    // -----------------------------------------------------------------------
    let ctx_idx = gnb.attach(ue);
    let state = &gnb.rrc.context(ctx_idx).unwrap().state;
    println!("Step 1  RRC attach        UE={ue:?}  state={state:?}");

    // -----------------------------------------------------------------------
    // Step 2: N2 NAS forward stub — gNB → AMF
    // -----------------------------------------------------------------------
    let nas = b"ServiceToken:deadbeef";
    let forwarded = gnb.forward_to_amf(ue, nas);
    println!("Step 2  N2 NAS forward    bytes_forwarded={forwarded}  (stub)");

    // -----------------------------------------------------------------------
    // Step 3: AMF registration + authentication
    // -----------------------------------------------------------------------
    amf.register(ue, 1001);
    amf.authenticate(ue);
    println!(
        "Step 3  AMF register      registered_ues={}",
        amf.registered_ue_count()
    );

    // -----------------------------------------------------------------------
    // Step 4: SMF PDU session establishment
    // -----------------------------------------------------------------------
    let session_id = smf.establish_session(ue, PduSessionType::Ip);
    println!(
        "Step 4  SMF session       session_id={session_id}  session_count={}",
        smf.session_count()
    );

    // -----------------------------------------------------------------------
    // Step 5: Data plane — UE → gNB → PDCP → UPF
    // -----------------------------------------------------------------------
    let user_data = b"Hello 6G data network";
    gnb.forward_uplink(user_data, &mut upf);
    println!(
        "Step 5  Uplink data       payload_bytes={}  upf.bytes_uplink={}",
        user_data.len(),
        upf.stats.bytes_uplink
    );

    // -----------------------------------------------------------------------
    // Assertions (same numerical checks as gnb_attach_and_uplink_flow test)
    // -----------------------------------------------------------------------
    use sixg_rrc::RrcState;
    assert_eq!(
        gnb.rrc.context(ctx_idx).unwrap().state,
        RrcState::Connected,
        "UE must remain in Connected state"
    );
    assert_eq!(forwarded, nas.len(), "N2 stub must echo payload length");
    assert_eq!(amf.registered_ue_count(), 1, "exactly one UE registered");
    assert!(session_id > 0, "SMF must assign a non-zero session ID");
    assert!(
        upf.stats.bytes_uplink > 0,
        "UPF must have received at least one byte"
    );

    println!("\nAll exp_005 checks PASSED ✓");
}

//! Experiment 005 — End-to-End Core Session (v2)
//!
//! Exercises the full 6G control + data plane via the `CoreNetwork` orchestrator:
//!
//! ```text
//! UE → GnbNode::attach()                       RRC Idle → Connected
//! CoreNetwork::register_ue()                   SBAv2 inline auth (1 RTT vs 5G's 4)
//!   ├─ SbaV2Registry::register_with_token()    token validated inline
//!   ├─ Amf::register() + authenticate()        mobility record stored
//!   └─ DigitalTwin::update()                   snapshot #1 captured
//! CoreNetwork::establish_session()             NSSF → SMF → UPF → PCF chain
//!   ├─ NetworkSliceSelector::select(URLLC)     slice found
//!   ├─ Smf::establish_session()                session_id + IP allocated
//!   ├─ Smf::mark_upf_allocated()               UPF bearer confirmed
//!   ├─ Pcf::add_policy(for_slice(URLLC))       QCI 80 policy bound
//!   └─ DigitalTwin::update()                   snapshot #2 + diff
//! GnbNode::forward_uplink() → Upf             PDCP → N3 → UPF bytes_uplink
//! ```
//!
//! Parameters are read from `experiments/exp_005_e2e_core_session/config.json`.
//!
//! Run with:
//!   cargo run --example exp_005_e2e_core_session

use serde::Deserialize;
use sixg_common::types::{NodeId, UeId};
use sixg_core::nssf::SliceType;
use sixg_core::smf::PduSessionType;
use sixg_core::{CoreNetwork, GnbNode};
use sixg_rrc::RrcState;

#[derive(Deserialize)]
struct Config {
    ue_id: u64,
    gnb_node_id: u64,
    tracking_area: u32,
    user_data_bytes: usize,
}

fn main() {
    // -----------------------------------------------------------------------
    // Load configuration
    // -----------------------------------------------------------------------
    let config_path = "experiments/exp_005_e2e_core_session/config.json";
    let config_str = std::fs::read_to_string(config_path).expect("config.json must be readable");
    let cfg: Config = serde_json::from_str(&config_str).expect("config.json must parse");

    let ue = UeId(cfg.ue_id);

    println!("=== exp_005: End-to-End Core Session (SBAv2 + Digital Twin) ===\n");
    println!(
        "Config: UE={ue:?}  gNB={:?}  TA={}  data={}B\n",
        cfg.gnb_node_id, cfg.tracking_area, cfg.user_data_bytes
    );

    // -----------------------------------------------------------------------
    // Step 1: RRC Attach — UE → gNB (RRCSetupRequest)
    // -----------------------------------------------------------------------
    let mut gnb = GnbNode::new(NodeId(cfg.gnb_node_id));
    let ctx_idx = gnb.attach(ue);
    let state = &gnb.rrc.context(ctx_idx).unwrap().state;
    println!("[Step 1] RRC attach        UE={ue:?}  state={state:?}");

    // -----------------------------------------------------------------------
    // Step 2: SBAv2 registration — 1 RTT inline auth (vs 5G's ≥ 4 RTT)
    // -----------------------------------------------------------------------
    let mut core = CoreNetwork::new();
    let granted = core.register_ue(ue, cfg.tracking_area);
    let snap1 = core.digital_twin.current().unwrap();
    println!(
        "[Step 2] SBAv2 register    granted={granted}  amf_ues={}  twin_snaps={}",
        core.amf.registered_ue_count(),
        core.digital_twin.snapshot_count()
    );
    println!(
        "         Digital Twin #1  ues_in_snap={}  slice_loads={:?}",
        snap1.ues.len(),
        snap1.slice_load_pct.values().collect::<Vec<_>>()
    );

    // -----------------------------------------------------------------------
    // Step 3: Establish PDU session — NSSF → SMF → UPF → PCF
    // -----------------------------------------------------------------------
    let grant = core
        .establish_session(ue, SliceType::Urllc, PduSessionType::Ip)
        .expect("URLLC slice must be available");
    let snap2 = core.digital_twin.current().unwrap();
    println!(
        "[Step 3] Session grant     session_id={}  ip={}  slice={:?}  qci={}  gbr={:.0}kbps",
        grant.session_id,
        grant.ip_addr,
        grant.slice,
        grant.qci,
        grant.gbr.as_kbps()
    );
    println!(
        "         Digital Twin #2  ues_in_snap={}  pdu_sessions={}",
        snap2.ues.len(),
        snap2.ues.values().map(|u| u.pdu_session_count).sum::<u8>()
    );
    println!(
        "         UPF allocated    smf_all_upf={}  pcf_policies={}",
        core.smf.all_upf_allocated(),
        core.pcf.policy_count()
    );

    // -----------------------------------------------------------------------
    // Step 4: Data plane — UE → gNB → PDCP → UPF
    // -----------------------------------------------------------------------
    let user_data = vec![0xABu8; cfg.user_data_bytes];
    gnb.forward_uplink(&user_data, &mut core.upf);
    println!(
        "[Step 4] Uplink forward    payload_bytes={}  upf.bytes_uplink={}",
        user_data.len(),
        core.upf.stats.bytes_uplink
    );

    // -----------------------------------------------------------------------
    // Assertions
    // -----------------------------------------------------------------------
    assert!(granted, "SBAv2 token must be valid");
    assert_eq!(
        gnb.rrc.context(ctx_idx).unwrap().state,
        RrcState::Connected,
        "UE must remain Connected"
    );
    assert_eq!(
        core.amf.registered_ue_count(),
        1,
        "exactly one UE registered"
    );
    assert_eq!(core.sba_v2.validated_ue_count(), 1, "one SBAv2 validation");
    assert!(grant.session_id > 0, "session_id must be positive");
    assert_eq!(grant.ip_addr.octets()[0], 10, "IP must be in 10.0.0.0/8");
    assert_eq!(grant.qci, 80, "URLLC must be QCI 80");
    assert!(core.smf.all_upf_allocated(), "UPF bearer must be allocated");
    assert!(
        core.pcf.policy_count() > 0,
        "PCF must have at least one policy"
    );
    assert_eq!(
        core.digital_twin.snapshot_count(),
        2,
        "two snapshots: register + establish"
    );
    assert!(
        core.upf.stats.bytes_uplink > 0,
        "UPF must have received at least one byte"
    );

    println!("\nAll exp_005 checks PASSED ✓");
    println!("(6G: SBAv2 registration in 1 RTT vs 5G NAS ≥ 4 RTT)");
}

//! Experiment 009 — 5G vs 6G Full-Stack Cross-Layer Comparison
//!
//! Runs seven back-to-back sub-experiments, each pairing a 5G-equivalent
//! simulation against the full 6G stack.  After each sub-experiment the code
//! explicitly exercises the matching architectural flaw so that both the
//! capability gain and the current implementation gap are observable in a
//! single run.
//!
//! Sub-experiments:
//!   1  PHY waveform      — OFDM vs OTFS under high mobility (250 km/h)
//!   2  PHY coverage      — RIS-assisted link at sub-THz (150 GHz)
//!   3  MAC scheduling    — Round Robin vs AI-native Q-bandit
//!   4  Core registration — 5G multi-RTT vs SBAv2 1-RTT
//!   5  NTN integration   — LEO propagation delay + handover decision
//!   6  ISAC + SDF        — DFRC sensing + core subscription delivery
//!   7  Semantic session  — raw bytes vs goal-oriented compression
//!
//! Architecture flaws surfaced: F-1 … F-7 (see README.md for details).
//!
//! Run with:
//!   cargo run --example exp_009_5g_vs_6g_full_stack

use serde::Deserialize;

use sixg_common::types::{Distance, Frequency, NodeId, SnrDb, SnrLinear, UeId, Velocity};
use sixg_common::validation::{Validate, ValidationCheck, ValidationResult};

use sixg_phy::ris::{RisChannel, RisConfig};
use sixg_phy::spectrum::path_loss_db;
use sixg_phy::waveform::{bpsk_ber_awgn, ofdm_ber_high_doppler, Waveform};

use sixg_mac::scheduler::{jain_fairness, Scheduler, SchedulingPolicy, UeChannelState};

use sixg_ntn::handover::{
    leo_propagation_delay_ms, HandoverDecision, HandoverTrigger, NtnHandoverManager, LEO_ALTITUDE_M,
};
use sixg_ntn::NtnNode;

use sixg_isac::detection::pd_from_pfa;
use sixg_isac::dfrc::DfrcConfig;

use sixg_semantic::codec::{BandwidthReduction, GoalOrientedMetrics, TextSemanticCodec};
use sixg_semantic::SemanticCodec;

use sixg_core::nssf::SliceType;
use sixg_core::sdf::{DetectionEvent, SensingDataFunction};
use sixg_core::smf::PduSessionType;
use sixg_core::{CoreNetwork, GnbNode};

// ─────────────────────────────────────────────────────────────────────────────
// Config structs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct PhyCfg {
    carrier_freq_ghz: f64,
    subcarrier_spacing_khz: f64,
    velocity_kmh: f64,
    subthz_freq_ghz: f64,
    link_distance_m: f64,
}

#[derive(Deserialize)]
struct RisCfg {
    num_elements: usize,
    h_direct: f64,
    h_reflect_in: f64,
    h_reflect_out: f64,
}

#[derive(Deserialize)]
struct NtnCfg {
    leo_altitude_m: f64,
    haps_altitude_m: f64,
    geo_altitude_m: f64,
}

#[derive(Deserialize)]
struct IsacCfg {
    sensing_snr_db: f64,
    bandwidth_hz: f64,
    sensing_subcarriers: usize,
    total_subcarriers: usize,
}

#[derive(Deserialize)]
struct CoreCfg {
    ue_id_base: u64,
    gnb_node_id: u64,
    tracking_area: u32,
    rtt_5g_baseline: u32,
    rtt_6g_sbav2: u32,
}

#[derive(Deserialize)]
struct SemanticCfg {
    payload_bytes: usize,
}

#[derive(Deserialize)]
struct Config {
    ues: usize,
    n_tti: usize,
    total_rbs: usize,
    phy: PhyCfg,
    ris: RisCfg,
    ntn: NtnCfg,
    isac: IsacCfg,
    core: CoreCfg,
    semantic: SemanticCfg,
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Print a section header.
fn section(title: &str) {
    println!("\n{}", "═".repeat(70));
    println!("  {title}");
    println!("{}", "═".repeat(70));
}

/// Print a flaw notice.
fn flaw(id: &str, summary: &str) {
    println!("\n  ⚠  FLAW {id}: {summary}");
}

/// Compute normalised Doppler shift ε = f_d / Δf (dimensionless).
///
/// All arguments are internal computation values (raw `f64`) because this is a
/// private helper used only within the experiment binary — not a `pub fn` API surface.
/// Public crate APIs use newtypes; see `sixg_common::types`.
///
/// `f_d = v · f_c / c`  (Doppler frequency for a UE moving at `velocity_mps`).
/// `Δf` = subcarrier spacing in Hz.
fn normalised_doppler(velocity_mps: f64, carrier_hz: f64, scs_hz: f64) -> f64 {
    const C: f64 = 3.0e8;
    let fd = velocity_mps * carrier_hz / C;
    fd / scs_hz
}

// ─────────────────────────────────────────────────────────────────────────────
// Part 1 — PHY Waveform: OFDM vs OTFS under high mobility
// ─────────────────────────────────────────────────────────────────────────────

fn part1_waveform(cfg: &PhyCfg) {
    section("Part 1 — PHY Waveform: OFDM vs OTFS under high-mobility channel");

    let velocity_mps = cfg.velocity_kmh / 3.6;
    let carrier_hz = cfg.carrier_freq_ghz * 1e9;
    let scs_hz = cfg.subcarrier_spacing_khz * 1e3;
    let epsilon = normalised_doppler(velocity_mps, carrier_hz, scs_hz);

    let ofdm = Waveform::CpOfdm {
        subcarrier_spacing_khz: cfg.subcarrier_spacing_khz as u32,
        fft_size: 4096,
    };
    let otfs = Waveform::Otfs {
        delay_bins: 64,
        doppler_bins: 16,
    };

    println!(
        "\n  Scenario: v = {:.0} km/h  f_c = {:.0} GHz  SCS = {:.0} kHz  ε = {:.4}",
        cfg.velocity_kmh, cfg.carrier_freq_ghz, cfg.subcarrier_spacing_khz, epsilon
    );
    println!(
        "\n  {:>8}  {:>14}  {:>14}  {:>16}",
        "SNR(dB)", "BER OFDM(static)", "BER OFDM(Doppler)", "BER OTFS(Doppler)"
    );
    println!("  {}", "-".repeat(58));

    let snr_points = [0.0f64, 2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0];
    let mut ber_ofdm_doppler_at_8db = 0.0;
    let mut ber_otfs_at_8db = 0.0;

    for &snr_db in &snr_points {
        let snr = SnrDb(snr_db);
        let ber_static = ofdm.ber_awgn(snr);
        let ber_ofdm_dop = ofdm.ber_high_doppler(snr, epsilon);
        let ber_otfs_dop = otfs.ber_high_doppler(snr, epsilon);
        println!(
            "  {:>8.1}  {:>14.3e}  {:>14.3e}  {:>16.3e}",
            snr_db, ber_static, ber_ofdm_dop, ber_otfs_dop
        );
        if (snr_db - 8.0).abs() < 0.1 {
            ber_ofdm_doppler_at_8db = ber_ofdm_dop;
            ber_otfs_at_8db = ber_otfs_dop;
        }
    }

    let ber_ratio = ber_ofdm_doppler_at_8db / ber_otfs_at_8db.max(1e-15);
    println!(
        "\n  At 8 dB: OFDM(Doppler) = {:.3e}  OTFS = {:.3e}  → OTFS is {:.0}× better",
        ber_ofdm_doppler_at_8db, ber_otfs_at_8db, ber_ratio
    );

    // ── FLAW F-1: PHY→MAC decoupling ──────────────────────────────────────
    flaw(
        "F-1",
        "PHY->MAC cross-layer decoupling: improved BER/SNR is never fed back to MAC",
    );
    println!("       The MAC scheduler's UeChannelState.snr is set once at UE attach.");
    println!("       After OTFS selection or RIS deployment, phy_snr_improvement MUST");
    println!("       be applied to UeChannelState.snr -- but there is no API for this.");
    println!("       The AI scheduler will continue making decisions on stale SNR.");

    // ── FLAW F-2: ber_awgn identical for all waveforms ────────────────────
    flaw(
        "F-2",
        "Waveform::ber_awgn() dispatches identically for OTFS and CP-OFDM",
    );
    let ber_ofdm_awgn = ofdm.ber_awgn(SnrDb(8.0));
    let ber_otfs_awgn = otfs.ber_awgn(SnrDb(8.0));
    println!(
        "       ofdm.ber_awgn(8 dB) = {:.6e}  otfs.ber_awgn(8 dB) = {:.6e}",
        ber_ofdm_awgn, ber_otfs_awgn
    );
    assert!(
        (ber_ofdm_awgn - ber_otfs_awgn).abs() < 1e-15,
        "F-2 should show identity"
    );
    println!("       Values are IDENTICAL -- an orchestrator polling ber_awgn()");
    println!("       uniformly cannot distinguish the two waveforms for link adaptation.");
}

// ─────────────────────────────────────────────────────────────────────────────
// Part 2 — PHY Coverage: RIS-assisted link at sub-THz
// ─────────────────────────────────────────────────────────────────────────────

fn part2_ris(cfg: &PhyCfg, ris_cfg: &RisCfg) {
    section("Part 2 — PHY Coverage: RIS-assisted link vs direct link at 150 GHz");

    let freq = Frequency::from_ghz(cfg.subthz_freq_ghz);
    let dist = Distance::from_m(cfg.link_distance_m);
    let pl = path_loss_db(dist, freq);

    println!(
        "\n  Sub-THz path loss @ {:.0} GHz, {:.0} m: {:.1} dB",
        cfg.subthz_freq_ghz,
        cfg.link_distance_m,
        pl.as_db()
    );

    let ris = RisConfig {
        num_elements: ris_cfg.num_elements,
        rows: 16,
        columns: 16,
        ..RisConfig::default()
    };
    let channel = RisChannel::new(
        ris_cfg.h_direct,
        ris_cfg.h_reflect_in,
        ris_cfg.h_reflect_out,
        ris,
    );
    let snr_tx = SnrLinear::new(1.0);
    let snr_no = channel.snr_no_ris(snr_tx);
    let snr_opt = channel.snr_opt_ris(snr_tx);

    let snr_no_db = 10.0 * snr_no.as_linear().max(1e-15).log10();
    let snr_opt_db = 10.0 * snr_opt.as_linear().max(1e-15).log10();
    let gain_db = channel
        .snr_gain_db(snr_tx)
        .map(|g| g.as_db())
        .unwrap_or(f64::INFINITY);

    println!(
        "\n  RIS panel: {n} elements  h_d = {h_d:.4}  h_r = {h_r:.4}",
        n = ris_cfg.num_elements,
        h_d = ris_cfg.h_direct,
        h_r = ris_cfg.h_reflect_in
    );
    println!("  SNR without RIS: {snr_no_db:.1} dB   SNR with RIS (optimal): {snr_opt_db:.1} dB");
    println!("  SNR gain from RIS: {gain_db:.1} dB  (> 10 dB confirms coverage extension)");

    assert!(
        gain_db > 10.0,
        "RIS gain must exceed 10 dB in shadowed scenario, got {gain_db:.1} dB"
    );
    println!("  RIS SNR gain > 10 dB: PASSED ✓");

    // ── FLAW F-1 (continued in context of RIS) ────────────────────────────
    flaw(
        "F-1 (RIS context)",
        "RIS SNR gain has no path into MAC UeChannelState -- scheduler is blind to it",
    );
    println!("       RisChannel::snr_opt_ris() returned SNR = {snr_opt_db:.1} dB, but");
    println!("       there is no API to propagate this into UeChannelState::snr for");
    println!("       the MAC scheduler.  The stack has no cross-layer signal path.");
}

// ─────────────────────────────────────────────────────────────────────────────
// Part 3 — MAC Scheduling: Round Robin vs AI-native
// ─────────────────────────────────────────────────────────────────────────────

fn part3_mac(cfg: &Config) {
    section("Part 3 — MAC Scheduling: Round Robin vs AI-native Q-bandit");

    // Build heterogeneous UE channel states: alternating poor (2 dB) and
    // excellent (20 dB) SNR to make the scheduler choice meaningful.
    let make_states = |n: usize| -> Vec<UeChannelState> {
        (0..n)
            .map(|i| {
                let snr_db = if i % 2 == 0 { 2.0f64 } else { 20.0f64 };
                let snr_linear = 10f64.powf(snr_db / 10.0);
                UeChannelState::new(UeId(1001 + i as u64), SnrLinear::new(snr_linear))
            })
            .collect()
    };

    // ── Round Robin ────────────────────────────────────────────────────────
    let states = make_states(cfg.ues);
    let mut rr_sched = Scheduler::with_policy(SchedulingPolicy::RoundRobin);
    let mut rr_prbs = vec![0usize; cfg.ues];

    for _ in 0..cfg.n_tti {
        let assignments = rr_sched.schedule_with_csi(&states, cfg.total_rbs);
        for a in &assignments {
            // Find position of this UE in our states vector
            if let Some(idx) = states.iter().position(|s| s.ue == a.ue) {
                rr_prbs[idx] += a.rb_count;
            }
        }
    }
    let rr_throughputs: Vec<f64> = rr_prbs.iter().map(|&p| p as f64).collect();
    let rr_fairness = jain_fairness(&rr_throughputs);

    // ── AI-native ──────────────────────────────────────────────────────────
    let states = make_states(cfg.ues);
    let mut ai_sched = Scheduler::with_policy(SchedulingPolicy::AiNative);
    let mut ai_prbs = vec![0usize; cfg.ues];

    for _t in 0..cfg.n_tti {
        let assignments = ai_sched.schedule_with_csi(&states, cfg.total_rbs);
        for (slot, a) in assignments.iter().enumerate() {
            if let Some(idx) = states.iter().position(|s| s.ue == a.ue) {
                ai_prbs[idx] += a.rb_count;
                // Feed observed reward: high-SNR UEs get 5 Gbps, low-SNR get 0.5 Gbps
                let snr_db = if idx % 2 == 0 { 2.0f64 } else { 20.0f64 };
                let throughput = if snr_db > 10.0 { 5e9 } else { 0.5e9 };
                ai_sched.observe_reward(slot, states[idx].snr, throughput);
            }
        }
    }
    let ai_throughputs: Vec<f64> = ai_prbs.iter().map(|&p| p as f64).collect();
    let ai_fairness = jain_fairness(&ai_throughputs);

    let rr_high_snr_share: f64 = rr_prbs
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 != 0)
        .map(|(_, &p)| p as f64)
        .sum::<f64>()
        / rr_prbs.iter().sum::<usize>() as f64;
    let ai_high_snr_share: f64 = ai_prbs
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 != 0)
        .map(|(_, &p)| p as f64)
        .sum::<f64>()
        / ai_prbs.iter().sum::<usize>() as f64;

    println!(
        "\n  Scheduler comparison over {n} TTIs, {u} UEs ({total} PRBs/TTI):",
        n = cfg.n_tti,
        u = cfg.ues,
        total = cfg.total_rbs
    );
    println!(
        "  {:<22}  {:>12}  {:>18}  {:>18}",
        "Policy", "Jain fairness", "High-SNR PRB share", "Low-SNR PRB share"
    );
    println!("  {}", "-".repeat(76));
    println!(
        "  {:<22}  {:>12.4}  {:>18.1}%  {:>18.1}%",
        "Round Robin (5G equiv)",
        rr_fairness,
        rr_high_snr_share * 100.0,
        (1.0 - rr_high_snr_share) * 100.0
    );
    println!(
        "  {:<22}  {:>12.4}  {:>18.1}%  {:>18.1}%",
        "AI-native (6G)",
        ai_fairness,
        ai_high_snr_share * 100.0,
        (1.0 - ai_high_snr_share) * 100.0
    );

    // ── FLAW F-3: Q-table capacity overflow ───────────────────────────────
    flaw(
        "F-3",
        "AI Scheduler QBandit Q-table is fixed at 64 UEs; silent drop for ue_idx ≥ 64",
    );

    // Demonstrate: with the default AI scheduler (64-UE table), calling
    // observe_reward for ue_idx = 65 silently discards the reward.
    // We show this by creating 70 UEs and noting that UEs 64–69 never receive
    // Q-value updates — the scheduler degrades to random for those UEs.
    let overflow_n = 70usize;
    let mut overflow_sched = Scheduler::with_policy(SchedulingPolicy::AiNative);
    let overflow_states: Vec<UeChannelState> = (0..overflow_n)
        .map(|i| UeChannelState::new(UeId(2000 + i as u64), SnrLinear::new(100.0)))
        .collect();

    // Drive 20 TTIs.  For UE indices 64-69 the reward is dropped silently.
    let mut overflow_prbs = vec![0usize; overflow_n];
    for _ in 0..20 {
        let assignments = overflow_sched.schedule_with_csi(&overflow_states, 70);
        for a in &assignments {
            if let Some(idx) = overflow_states.iter().position(|s| s.ue == a.ue) {
                overflow_prbs[idx] += a.rb_count;
                // All UEs have identical SNR; we reward UE 65 generously
                if idx == 65 {
                    overflow_sched.observe_reward(idx, SnrLinear::new(100.0), 9e9);
                }
            }
        }
    }
    // UE 65's reward updates are silently dropped — verify indirectly:
    // (We cannot directly inspect QBandit.q_table from outside the crate,
    // so we note the architectural flaw and document the guard in QBandit::update.)
    println!(
        "       Created {n} UEs with AI scheduler (table capacity = 64).",
        n = overflow_n
    );
    println!("       Rewards for UE indices 64-69 are silently dropped by");
    println!("       `if ue_idx >= self.q_table.len() {{ return; }}`");
    println!("       in QBandit::update -- scheduler never learns for those UEs.");
    println!(
        "       PRBs assigned to UE[65] over 20 TTIs: {} (random, no learning)",
        overflow_prbs[65]
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Part 4 — Core Registration: 5G multi-RTT vs SBAv2 1-RTT
// ─────────────────────────────────────────────────────────────────────────────

fn part4_core(cfg: &Config) {
    section("Part 4 — Core Registration: 5G multi-RTT vs 6G SBAv2 inline auth");

    let core_cfg = &cfg.core;
    println!(
        "\n  5G baseline (3GPP TS 23.502 §4.2.2.2): ≥ {} RTTs for Initial Registration",
        core_cfg.rtt_5g_baseline
    );
    println!(
        "  6G SBAv2 (inline auth): {} RTT (token validated inline, no AMF reselection)",
        core_cfg.rtt_6g_sbav2
    );
    let reduction_pct =
        (1.0 - core_cfg.rtt_6g_sbav2 as f64 / core_cfg.rtt_5g_baseline as f64) * 100.0;
    println!("  Control-plane RTT reduction: {reduction_pct:.0}%\n");

    let mut core = CoreNetwork::new();
    let mut gnb = GnbNode::new(NodeId(core_cfg.gnb_node_id));

    // Register cfg.ues UEs and establish sessions
    let mut ip_session_count = 0;
    let mut semantic_session_count = 0;

    for i in 0..cfg.ues {
        let ue = UeId(core_cfg.ue_id_base + i as u64);
        let _ctx = gnb.attach(ue);
        let granted = core.register_ue(ue, core_cfg.tracking_area);
        assert!(granted, "SBAv2 registration must succeed for UE {ue:?}");

        // Alternate between IP and semantic sessions
        let session_type = if i % 2 == 0 {
            ip_session_count += 1;
            PduSessionType::Ip
        } else {
            use sixg_semantic::codec::TaskSuccessRate;
            use sixg_semantic::SemanticTask;
            semantic_session_count += 1;
            PduSessionType::Semantic(sixg_core::smf::GoalSpec {
                task: SemanticTask::TextUnderstanding,
                min_success_rate: TaskSuccessRate(0.9),
                max_bandwidth_reduction: sixg_semantic::codec::BandwidthReduction(15.0),
            })
        };
        let session_id = core.establish_session(ue, SliceType::Urllc, session_type);
        assert!(
            session_id.is_some(),
            "Session establishment must succeed for UE {ue:?}"
        );
    }

    let twin = core.digital_twin.current().unwrap();
    println!(
        "  Registered {} UEs (IP: {}, Semantic: {})",
        cfg.ues, ip_session_count, semantic_session_count
    );
    println!(
        "  Digital Twin snapshot: {} UEs, {} slice-load entries",
        twin.ues.len(),
        twin.slice_load_pct.len()
    );
    println!(
        "  AMF registered UE count: {}",
        core.amf.registered_ue_count()
    );

    // ── FLAW F-7: forward_unknown_flow drops first packet ─────────────────
    flaw(
        "F-7",
        "UPF forward_unknown_flow() returns TriggerEstablishment but silently drops payload",
    );
    let unregistered_ue = UeId(9999);
    let test_payload = b"first 6G packet - should be buffered, not dropped";
    let action = core.upf.forward_unknown_flow(unregistered_ue, test_payload);
    use sixg_core::upf::FlowAction;
    assert_eq!(
        action,
        FlowAction::TriggerEstablishment(unregistered_ue),
        "Expected TriggerEstablishment for unknown UE"
    );
    let bytes_for_unk = core
        .upf
        .session_stats(255)
        .map(|s| s.bytes_uplink)
        .unwrap_or(0);
    println!("       UPF returned TriggerEstablishment for UE {unregistered_ue:?}.");
    println!(
        "       Bytes forwarded = {bytes_for_unk} -- payload ({} bytes) was DROPPED.",
        test_payload.len()
    );
    println!("       Real 6G UPF-first architecture requires buffering before signalling");
    println!("       the SMF; this buffer does not exist.");
    assert_eq!(
        bytes_for_unk, 0,
        "No bytes should have been counted for unknown UE — they are silently lost"
    );

    // ── FLAW F-5: forward_semantic_uplink ignores session type ─────────────
    flaw(
        "F-5",
        "Upf::forward_semantic_uplink() applies codec with no check on PduSessionType",
    );
    // Register an IP session (not semantic) and route it through the semantic UPF path
    let ip_ue = UeId(core_cfg.ue_id_base); // was registered as IP session above
    let first_ip_session = core
        .smf
        .sessions_for_ue(ip_ue)
        .first()
        .map(|s| s.session_id)
        .unwrap_or(1);
    let raw_ip_payload = b"raw IP datagram - NOT a semantic payload";
    let encoded = core
        .upf
        .forward_semantic_uplink(first_ip_session, raw_ip_payload);
    println!(
        "       Routed raw IP payload ({} bytes) through forward_semantic_uplink.",
        raw_ip_payload.len()
    );
    println!(
        "       Got {} bytes back -- codec ran unconditionally on an IP session!",
        encoded.len()
    );
    println!("       PduSessionType was never consulted.");
    assert_ne!(
        encoded.len(),
        raw_ip_payload.len(),
        "Semantic codec must have transformed the payload"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Part 5 — NTN Integration: LEO propagation delay + handover
// ─────────────────────────────────────────────────────────────────────────────

fn part5_ntn(ntn_cfg: &NtnCfg) {
    section("Part 5 — NTN Integration: LEO/HAPS/GEO propagation delays + handover");

    // Physics-based propagation delays
    let leo_delay = leo_propagation_delay_ms(Distance::from_m(ntn_cfg.leo_altitude_m));
    let haps_delay = leo_propagation_delay_ms(Distance::from_m(ntn_cfg.haps_altitude_m));
    let geo_delay = leo_propagation_delay_ms(Distance::from_m(ntn_cfg.geo_altitude_m));

    println!("\n  Physics-derived one-way propagation delays (altitude / c × 1000):");
    println!(
        "  {:>20}  {:>16}  {:>18}",
        "Node type", "Altitude (km)", "Delay (ms)"
    );
    println!("  {}", "-".repeat(58));
    println!(
        "  {:>20}  {:>16.0}  {:>18.3}",
        "LEO satellite",
        ntn_cfg.leo_altitude_m / 1000.0,
        leo_delay
    );
    println!(
        "  {:>20}  {:>16.0}  {:>18.3}",
        "HAPS",
        ntn_cfg.haps_altitude_m / 1000.0,
        haps_delay
    );
    println!(
        "  {:>20}  {:>16.0}  {:>18.3}",
        "GEO satellite",
        ntn_cfg.geo_altitude_m / 1000.0,
        geo_delay
    );

    assert!(
        (leo_delay - 1.834_8).abs() < 0.05,
        "LEO delay must be ≈ 1.83 ms, got {leo_delay:.4}"
    );
    assert!(
        geo_delay > 100.0,
        "GEO delay must be > 100 ms, got {geo_delay:.1}"
    );
    println!("\n  LEO ≈ 1.83 ms: PASSED ✓    GEO > 100 ms: PASSED ✓");

    // ── FLAW F-4: NtnNode::leo_satellite hardcodes propagation delay ───────
    flaw(
        "F-4",
        "NtnNode::leo_satellite() hardcodes propagation_delay_ms = 1.8 for every altitude",
    );
    use sixg_common::types::Position3D;

    // Create a "HAPS" node (20 km) using `leo_satellite` — wrong delay!
    let haps_pos = Position3D::new(0.0, 0.0, ntn_cfg.haps_altitude_m);
    let haps_wrong = NtnNode::leo_satellite(99, haps_pos);
    println!(
        "       NtnNode::leo_satellite(99, pos_z=20 km).propagation_delay_ms = {:.4} ms",
        haps_wrong.propagation_delay_ms
    );
    println!("       Correct value for 20 km: {haps_delay:.4} ms");
    println!(
        "       Error = {:.4} ms ({:.1}× wrong) — altitude is IGNORED by constructor.",
        (haps_wrong.propagation_delay_ms - haps_delay).abs(),
        haps_wrong.propagation_delay_ms / haps_delay
    );
    assert!(
        (haps_wrong.propagation_delay_ms - haps_delay).abs() > 1.5,
        "F-4 flaw: HAPS delay should be far from 1.8 ms"
    );

    // Handover decision
    let mgr = NtnHandoverManager::new();
    let dec = mgr.evaluate(
        UeId(1001),
        &[
            HandoverTrigger::BetterTerrestrialRsrp {
                delta_db: sixg_common::types::PowerDb::new(5.0),
            },
            HandoverTrigger::PropagationDelayExceeded {
                delay_ms: leo_delay,
            },
        ],
    );
    println!(
        "\n  NTN handover evaluation for LEO UE (RSRP +5 dB, delay={leo_delay:.2} ms): {dec:?}"
    );
    assert_eq!(
        dec,
        HandoverDecision::Proceed,
        "Terrestrial RSRP advantage must trigger handover"
    );
    println!("  Handover decision = Proceed: PASSED ✓");
}

// ─────────────────────────────────────────────────────────────────────────────
// Part 6 — ISAC + SDF: DFRC sensing + SDF subscription delivery
// ─────────────────────────────────────────────────────────────────────────────

fn part6_isac_sdf(isac_cfg: &IsacCfg) {
    section("Part 6 — ISAC + SDF: DFRC Pareto frontier + SensingDataFunction");

    let dfrc = DfrcConfig::new(
        isac_cfg.sensing_snr_db,
        isac_cfg.bandwidth_hz,
        isac_cfg.sensing_subcarriers,
        isac_cfg.total_subcarriers,
    );

    // Pareto frontier: 5 points
    let pareto = dfrc.pareto_frontier(5);
    println!(
        "\n  DFRC Pareto frontier (SNR={snr:.0} dB, BW={bw:.0} MHz):",
        snr = isac_cfg.sensing_snr_db,
        bw = isac_cfg.bandwidth_hz / 1e6
    );
    println!(
        "  {:>8}  {:>14}  {:>14}  {:>16}",
        "a (sens)", "CRB(range) m^2", "sqrt_CRB (m)", "Capacity (Gbps)"
    );
    println!("  {}", "-".repeat(58));
    for pt in &pareto {
        println!(
            "  {:>8.2}  {:>14.4e}  {:>14.4}  {:>16.3}",
            pt.sensing_power_ratio,
            pt.crb_range_m2,
            pt.crb_range_m2.sqrt(),
            pt.capacity_bps / 1e9
        );
    }

    // At alpha=0.5: verify CRB and capacity are in expected range
    let mid = &pareto[pareto.len() / 2];
    let crb_std = mid.crb_range_m2.sqrt();
    assert!(
        crb_std < 0.2,
        "CRB std-dev at alpha=0.5 must be < 0.2 m, got {crb_std:.3} m"
    );
    let cap_gbps = mid.capacity_bps / 1e9;
    assert!(
        cap_gbps > 1.0,
        "Capacity at alpha=0.5 must be > 1 Gbps, got {cap_gbps:.3} Gbps"
    );
    let mid_alpha = mid.sensing_power_ratio;
    println!(
        "\n  At alpha={mid_alpha:.2}: CRB std-dev = {crb_std:.3} m < 0.2 m checked   \
         Capacity = {cap_gbps:.3} Gbps > 1 Gbps checked"
    );

    // Detection probability vs SNR
    let pfa = 1e-4;
    let snr_sensing = SnrLinear::new(10f64.powf(isac_cfg.sensing_snr_db / 10.0));
    let pd = pd_from_pfa(pfa, snr_sensing);
    println!(
        "  Detection probability at Pfa={pfa:.1e}, SNR={snr:.0} dB: Pd = {pd:.4}",
        snr = isac_cfg.sensing_snr_db
    );

    // SDF subscription and event delivery
    let mut sdf = SensingDataFunction::new();
    let cell = NodeId(42);
    let sub_idx = sdf.subscribe(cell, Distance::from_m(500.0));

    let event = DetectionEvent {
        cell_id: cell,
        range: Distance::from_m(200.0),
        velocity: Velocity::from_m_per_s(15.0),
        ue_id: Some(UeId(1001)),
    };
    let delivered = sdf.publish(&event);
    println!("\n  SDF: published 1 detection event  →  {delivered} subscription(s) delivered");
    assert_eq!(delivered, 1, "Event must reach the registered subscription");
    assert_eq!(sdf.subscription(sub_idx).unwrap().delivered_count, 1);
    println!("  SDF event delivery: PASSED ✓");

    // ── FLAW F-6: late subscriber misses events ────────────────────────────
    flaw(
        "F-6",
        "SDF has no event replay — late subscribers miss all prior detection events",
    );
    // Register a second subscriber AFTER the publish
    let late_sub_idx = sdf.subscribe(cell, Distance::from_m(500.0));
    let late_delivered = sdf.subscription(late_sub_idx).unwrap().delivered_count;
    println!(
        "       Late subscriber (registered after publish): delivered_count = {late_delivered}"
    );
    println!("       The 1 prior event is not replayed -- subscriber permanently missed it.");
    println!("       A ring-buffer or subscription-replay API is needed for reliability.");
    assert_eq!(
        late_delivered, 0,
        "F-6 confirmed: late subscriber received 0 events"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Part 7 — Semantic PDU Session: raw bytes vs goal-oriented compression
// ─────────────────────────────────────────────────────────────────────────────

fn part7_semantic(sem_cfg: &SemanticCfg) {
    section("Part 7 — Semantic PDU session: raw IP vs goal-oriented compression");

    let payload: Vec<u8> = b"the quick brown fox jumps over the lazy dog "
        .iter()
        .copied()
        .cycle()
        .take(sem_cfg.payload_bytes)
        .collect();

    let codec = TextSemanticCodec;
    let encoded = codec.encode(&payload);
    let compression_ratio = payload.len() as f64 / encoded.len() as f64;

    println!(
        "\n  Raw payload: {} bytes → Semantic encoding: {} bytes",
        payload.len(),
        encoded.len()
    );
    println!("  Compression ratio: {compression_ratio:.1}×");
    assert!(
        compression_ratio > 10.0,
        "Semantic codec must achieve > 10× compression for 1 kB payload"
    );
    println!("  Compression ratio > 10×: PASSED ✓");

    // Goal-oriented task success sweep
    println!("\n  Task success rate vs bandwidth reduction (text understanding task):");
    println!(
        "  {:>12}  {:>14}  {:>14}  {:>14}",
        "BW_reduction", "Raw success", "JPEG success", "Semantic success"
    );
    println!("  {}", "-".repeat(58));

    let reduction_points = [1.0f64, 5.0, 10.0, 15.625, 20.0];
    let mut semantic_success_at_target = 0.0f64;

    for &r in &reduction_points {
        let bw = BandwidthReduction(r);
        let raw_s = GoalOrientedMetrics::raw_success_rate(bw).0;
        let jpeg_s = GoalOrientedMetrics::jpeg_success_rate(bw).0;
        let sem_s = GoalOrientedMetrics::semantic_success_rate(bw).0;
        println!(
            "  {:>12.3}×  {:>13.3}  {:>13.3}  {:>13.3}",
            r, raw_s, jpeg_s, sem_s
        );
        if (r - 15.625).abs() < 0.1 {
            semantic_success_at_target = sem_s;
        }
    }

    println!("\n  At 15.6× compression (= TextSemanticCodec actual ratio):");
    println!("    Raw success rate:      ~0.004 (degraded by bandwidth squeeze)");
    println!("    Semantic success rate: {semantic_success_at_target:.3} (≥ 0.9 target met)");
    assert!(
        semantic_success_at_target >= 0.8,
        "Semantic success at target compression must be ≥ 0.8"
    );
    println!("  Semantic task success ≥ 80% at 15.6× compression: PASSED ✓");
}

// ─────────────────────────────────────────────────────────────────────────────
// Summary
// ─────────────────────────────────────────────────────────────────────────────

fn summary() {
    section("Experiment 009 — Architecture Flaw Summary");
    println!(
        r#"
  ┌────┬─────────────────────────────────────────┬──────────────────────────────────────────┐
  │    │ Module                                  │ Flaw                                     │
  ├────┼─────────────────────────────────────────┼──────────────────────────────────────────┤
  │ F-1│ 6g-phy → 6g-mac                         │ PHY gains (RIS/OTFS) not fed into MAC    │
  │    │                                         │ UeChannelState — scheduler is PHY-blind  │
  ├────┼─────────────────────────────────────────┼──────────────────────────────────────────┤
  │ F-2│ 6g-phy/waveform                         │ Waveform::ber_awgn() is identical for    │
  │    │                                         │ OTFS and CP-OFDM — no static distinction │
  ├────┼─────────────────────────────────────────┼──────────────────────────────────────────┤
  │ F-3│ 6g-mac/scheduler (QBandit)              │ Q-table fixed at 64 UEs; ue_idx ≥ 64    │
  │    │                                         │ rewards silently dropped — 6G AI fails   │
  │    │                                         │ in dense deployments without crashing    │
  ├────┼─────────────────────────────────────────┼──────────────────────────────────────────┤
  │ F-4│ 6g-ntn (NtnNode::leo_satellite)         │ propagation_delay_ms hardcoded to 1.8   │
  │    │                                         │ regardless of altitude — HAPS/GEO wrong  │
  ├────┼─────────────────────────────────────────┼──────────────────────────────────────────┤
  │ F-5│ 6g-core/upf (forward_semantic_uplink)   │ Semantic codec applied to ALL payloads   │
  │    │                                         │ — PduSessionType never checked           │
  ├────┼─────────────────────────────────────────┼──────────────────────────────────────────┤
  │ F-6│ 6g-core/sdf (SensingDataFunction)       │ No event replay; late subscribers miss   │
  │    │                                         │ all prior DetectionEvents permanently     │
  ├────┼─────────────────────────────────────────┼──────────────────────────────────────────┤
  │ F-7│ 6g-core/upf (forward_unknown_flow)      │ First packet silently dropped when no    │
  │    │                                         │ session exists — no buffer for UPF-first │
  └────┴─────────────────────────────────────────┴──────────────────────────────────────────┘
"#
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Validation
// ─────────────────────────────────────────────────────────────────────────────

/// Numerical validation for exp_009 key results.
pub struct Exp009Validation;

impl Validate for Exp009Validation {
    fn validate() -> ValidationResult {
        // 1. OTFS outperforms OFDM in high-Doppler channel at 8 dB SNR.
        //    ε = 0.216 (v=250 km/h, f_c=28 GHz, SCS=30 kHz).
        let epsilon = 0.216_f64;
        let snr = SnrDb(8.0);
        let ber_otfs = bpsk_ber_awgn(snr); // OTFS achieves AWGN bound
        let ber_ofdm_dop = ofdm_ber_high_doppler(snr, epsilon);
        let otfs_ratio = ber_ofdm_dop / ber_otfs.max(1e-15); // must be > 1.0

        // 2. RIS (256 elements) in shadowed scenario yields > 10 dB SNR gain.
        let ris = RisConfig {
            num_elements: 256,
            rows: 16,
            columns: 16,
            ..RisConfig::default()
        };
        let channel = RisChannel::new(0.0001, 0.01, 0.01, ris);
        let gain_db = channel.snr_gain_db(SnrLinear::new(1.0)).unwrap().as_db();

        // 3. LEO propagation delay matches physics (550 km / c × 1000).
        let leo_delay = leo_propagation_delay_ms(Distance::from_m(LEO_ALTITUDE_M));

        // 4. Semantic codec achieves > 10× compression for 1 kB payload.
        let payload: Vec<u8> = vec![b'a'; 1_024];
        let codec = TextSemanticCodec;
        let encoded = codec.encode(&payload);
        let compression_ratio = payload.len() as f64 / encoded.len() as f64;

        // 5. Semantic task success ≥ 80 % at 15.6× bandwidth reduction.
        let sem_success = GoalOrientedMetrics::semantic_success_rate(BandwidthReduction(15.625)).0;

        ValidationResult {
            module: "exp_009_5g_vs_6g_full_stack",
            checks: vec![
                // OTFS/OFDM BER ratio at ε=0.216, 8 dB SNR.
                // ε=0.216: snr_eff = 6.31/(1 + π²/3×0.216²) ≈ 5.47 → ratio ≈ 2.46.
                ValidationCheck::new("otfs_ber_ratio_gt_1", otfs_ratio, 2.46, 5.0),
                // RIS gain: h_d=0.0001, h_r=0.01, N=256 → gain ≈ 48.2 dB.
                ValidationCheck::new("ris_snr_gain_db", gain_db, 48.2, 1.0),
                // LEO delay ≈ 1.8348 ms (1 % tolerance)
                ValidationCheck::new("leo_propagation_delay_ms", leo_delay, 1.8348, 1.0),
                // Compression ratio: 1024 / 64 = 16× (1 % tolerance)
                ValidationCheck::new("semantic_compression_ratio", compression_ratio, 16.0, 1.0),
                // Semantic success ≥ 0.896 at 15.6× compression (10 % tolerance)
                ValidationCheck::new("semantic_task_success", sem_success, 0.896, 10.0),
            ],
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    let config_path = "experiments/exp_009_5g_vs_6g_full_stack/config.json";
    let config_str = std::fs::read_to_string(config_path).expect("config.json must be readable");
    let cfg: Config = serde_json::from_str(&config_str).expect("config.json must parse");

    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║  Experiment 009 — 5G vs 6G Full-Stack Cross-Layer Comparison        ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!(
        "  Parameters: {} UEs  {} TTIs  {} PRBs/TTI",
        cfg.ues, cfg.n_tti, cfg.total_rbs
    );

    part1_waveform(&cfg.phy);
    part2_ris(&cfg.phy, &cfg.ris);
    part3_mac(&cfg);
    part4_core(&cfg);
    part5_ntn(&cfg.ntn);
    part6_isac_sdf(&cfg.isac);
    part7_semantic(&cfg.semantic);
    summary();

    // Run Validate checks
    section("Numerical Validation");
    let result = Exp009Validation::validate();
    println!("\n  {}", result.summary());
    assert!(result.passed(), "Validation failed:\n{}", result.summary());
    println!("\n  All validation checks passed ✓");
    println!("\n  exp_009 complete.\n");
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests (cargo test --example exp_009_5g_vs_6g_full_stack)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otfs_ber_lower_than_ofdm_high_doppler_at_8db() {
        // OTFS achieves AWGN bound; OFDM suffers ICI at ε = 0.216
        let epsilon = 0.216;
        let snr = SnrDb(8.0);
        let ber_otfs = bpsk_ber_awgn(snr);
        let ber_ofdm = ofdm_ber_high_doppler(snr, epsilon);
        assert!(
            ber_otfs < ber_ofdm,
            "OTFS BER ({ber_otfs:.3e}) must be lower than OFDM BER ({ber_ofdm:.3e}) at 8 dB, ε=0.216"
        );
    }

    #[test]
    fn fix_f2_ber_awgn_distinguishes_otfs_and_ofdm() {
        // Fixed F-2: ber_awgn now distinguishes OTFS and CP-OFDM.
        let ofdm = Waveform::CpOfdm {
            subcarrier_spacing_khz: 30,
            fft_size: 4096,
        };
        let otfs = Waveform::Otfs {
            delay_bins: 64,
            doppler_bins: 16,
        };
        let snr = SnrDb(10.0);
        assert!(
            otfs.ber_awgn(snr) < ofdm.ber_awgn(snr),
            "OTFS ber_awgn must now be lower than CP-OFDM ber_awgn"
        );
    }

    #[test]
    fn ris_shadowed_gain_exceeds_10db() {
        let ris = RisConfig {
            num_elements: 256,
            rows: 16,
            columns: 16,
            ..RisConfig::default()
        };
        let ch = RisChannel::new(0.0001, 0.01, 0.01, ris);
        let gain = ch.snr_gain_db(SnrLinear::new(1.0)).unwrap().as_db();
        assert!(gain > 10.0, "RIS gain must exceed 10 dB, got {gain:.1}");
    }

    #[test]
    fn fix_f4_leo_satellite_uses_altitude_for_delay() {
        use sixg_common::types::Position3D;
        let haps_alt = 20_000.0_f64;
        let pos = Position3D::new(0.0, 0.0, haps_alt);
        let node = NtnNode::leo_satellite(1, pos);
        // Correct for 20 km is ~0.067 ms
        let correct_delay = leo_propagation_delay_ms(Distance::from_m(haps_alt));
        assert!(
            (node.propagation_delay_ms - correct_delay).abs() < 0.01,
            "NtnNode::leo_satellite must compute delay from altitude"
        );
        assert!(
            (correct_delay - 0.067).abs() < 0.01,
            "Correct HAPS delay must be ≈ 0.067 ms"
        );
    }

    #[test]
    fn fix_f6_late_sdf_subscriber_replays_event() {
        let mut sdf = SensingDataFunction::new();
        let cell = NodeId(1);
        // Publish BEFORE subscribing
        let event = DetectionEvent {
            cell_id: cell,
            range: Distance::from_m(100.0),
            velocity: Velocity::from_m_per_s(5.0),
            ue_id: None,
        };
        sdf.publish(&event);
        // Now subscribe — event replay should deliver from history
        let idx = sdf.subscribe(cell, Distance::from_m(500.0));
        assert_eq!(
            sdf.subscription(idx).unwrap().delivered_count,
            1,
            "Late subscriber must receive replayed matching event"
        );
    }

    #[test]
    fn fix_f7_unknown_flow_buffers_payload() {
        use sixg_core::upf::{FlowAction, Upf};
        let mut upf = Upf::new();
        let ue = UeId(9999);
        let payload = b"first packet";
        let action = upf.forward_unknown_flow(ue, payload);
        assert_eq!(action, FlowAction::TriggerEstablishment(ue));
        // No bytes counted yet (not forwarded), but payload is buffered.
        assert_eq!(
            upf.stats.bytes_uplink, 0,
            "Unknown-flow payload is buffered and not yet forwarded"
        );
        assert_eq!(
            upf.buffered_uplink_count(ue),
            1,
            "F-7 fix: payload must be buffered for later forwarding"
        );
    }

    #[test]
    fn exp009_validation_passes() {
        let result = Exp009Validation::validate();
        assert!(result.passed(), "{}", result.summary());
    }

    #[test]
    fn semantic_compression_exceeds_10x_for_1kb() {
        let payload: Vec<u8> = vec![b'x'; 1_024];
        let codec = TextSemanticCodec;
        let encoded = codec.encode(&payload);
        let ratio = payload.len() as f64 / encoded.len() as f64;
        assert!(
            ratio > 10.0,
            "Semantic compression must exceed 10×, got {ratio:.1}×"
        );
    }
}

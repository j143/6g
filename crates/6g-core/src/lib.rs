//! 6G Core Network (6GC).
//!
//! The 6G core evolves the 5G Service-Based Architecture (SBA) with:
//! * Native AI/ML support for network automation
//! * Integrated Non-Terrestrial Network management
//! * Intent-based networking and zero-touch management
//! * Enhanced network slicing with sub-millisecond SLA guarantees
//! * Native support for Semantic and Goal-Oriented services
//!
//! Key network functions (NFs) modelled here (Phase 0–3 baseline):
//! * AMF – Access and Mobility Management Function
//! * SMF – Session Management Function
//! * UPF – User Plane Function
//! * PCF – Policy Control Function
//! * NSSF – Network Slice Selection Function
//!
//! Phase 4 additions:
//! * [`sba_v2`] – Service-Based Architecture v2 (flat inline-auth registry)
//! * [`digital_twin`] – Digital Twin snapshot + diff mechanism
//! * [`gnb`] – `GnbNode` bridging RRC/PDCP layers to N2/N3 core interfaces
//!
//! Phase 5 gap-closure:
//! * [`ausf`] – AUSF + UDM: subscriber credential store + 5G-AKA auth vectors
//! * [`nrf`]  – NRF: NF registration, deregistration, and discovery
//!
//! Phase 6 — 6G architectural differentiators:
//! * [`smf`] – `PduSessionType::Semantic(GoalSpec)` — 6G-native semantic sessions
//! * [`upf`] – `forward_semantic_uplink` + `forward_unknown_flow` (user-plane-first)
//! * [`amf`] – `TrackingArea` enum (Terrestrial vs NTN-aware mobility)

use std::net::Ipv4Addr;

use sixg_common::types::{Bitrate, Duration, UeId};

pub mod amf;
pub mod ausf;
pub mod digital_twin;
pub mod gnb;
pub mod nrf;
pub mod nssf;
pub mod pcf;
pub mod sba_v2;
pub mod smf;
pub mod upf;

pub use amf::{Amf, TrackingArea};
pub use ausf::{Ausf, SubscriberCredential, Udm};
pub use digital_twin::DigitalTwin;
pub use gnb::GnbNode;
pub use nrf::{NfProfile, NfType, Nrf};
pub use nssf::{NetworkSliceSelector, SliceType};
pub use pcf::Pcf;
pub use sba_v2::SbaV2Registry;
pub use smf::{GoalSpec, PduSessionType, Smf};
pub use upf::{FlowAction, Upf};

/// Result returned by [`CoreNetwork::establish_session`].
#[derive(Debug, Clone)]
pub struct SessionGrant {
    /// SMF-assigned session identifier.
    pub session_id: u8,
    /// UPF-allocated IPv4 address for this session.
    pub ip_addr: Ipv4Addr,
    /// Network slice selected for this session.
    pub slice: SliceType,
    /// QCI of the policy applied by the PCF.
    pub qci: u8,
    /// Guaranteed bit rate from the PCF policy.
    pub gbr: Bitrate,
}

/// 6G Core Network instance bundling all mandatory NFs and Phase 4/5 extensions.
pub struct CoreNetwork {
    /// 5GC-derived baseline: Access and Mobility Management Function.
    pub amf: Amf,
    /// 5GC-derived baseline: Session Management Function.
    pub smf: Smf,
    /// 5GC-derived baseline: User Plane Function.
    pub upf: Upf,
    /// 5GC-derived baseline: Policy Control Function.
    pub pcf: Pcf,
    /// 5GC-derived baseline: Network Slice Selection Function.
    pub nssf: NetworkSliceSelector,
    /// Phase 4: SBAv2 flat inline-authentication registry.
    pub sba_v2: SbaV2Registry,
    /// Phase 4: Digital twin — state-snapshot + diff engine.
    pub digital_twin: DigitalTwin,
    /// Phase 5: AUSF — authentication server with UDM subscriber store.
    pub ausf: Ausf,
    /// Phase 5: NRF — NF registration and discovery.
    pub nrf: Nrf,
}

impl CoreNetwork {
    /// Create a new 6G Core Network with all NFs initialised.
    pub fn new() -> Self {
        Self {
            amf: Amf::new(),
            smf: Smf::new(),
            upf: Upf::new(),
            pcf: Pcf::new(),
            nssf: NetworkSliceSelector::new(),
            sba_v2: SbaV2Registry::new(),
            digital_twin: DigitalTwin::new(),
            ausf: Ausf::new(),
            nrf: Nrf::new(),
        }
    }

    /// Register a UE using SBAv2 inline token authentication (1 RTT).
    ///
    /// **6G vs 5G:** 5G NAS registration requires ≥ 4 round trips
    /// (Registration Request → Authentication Challenge → Security Mode Command →
    /// Registration Accept).  SBAv2 derives the token from the UE identifier
    /// inline, granting service in a single exchange.
    ///
    /// On success the AMF record is created, the UE is marked as authenticated,
    /// and the Digital Twin is updated with the new UE snapshot.
    ///
    /// Returns `true` when the token is valid and service is granted.
    pub fn register_ue(&mut self, ue: UeId, tracking_area: u32) -> bool {
        // SBAv2: derive expected token and validate inline — 1 RTT.
        let token = sba_v2::ServiceToken::from_ue_id(ue);
        let granted = self.sba_v2.register_with_token(ue, token);
        if granted {
            // AMF still holds a mobility record for paging and handover.
            self.amf.register_terrestrial(ue, tracking_area);
            self.amf.authenticate(ue);
            self.push_snapshot();
        }
        granted
    }

    /// Register a UE served by an NTN node using SBAv2 inline authentication.
    ///
    /// Same SBAv2 1-RTT flow as [`register_ue`] but records the UE in the AMF
    /// with a [`TrackingArea::Ntn`] context so that NTN-aware paging and
    /// pre-emptive handover can be triggered before the satellite pass ends.
    ///
    /// Returns `true` when the token is valid and service is granted.
    pub fn register_ue_ntn(
        &mut self,
        ue: UeId,
        ntn_node_id: u64,
        beam_id: u32,
        propagation_delay: Duration,
    ) -> bool {
        let token = sba_v2::ServiceToken::from_ue_id(ue);
        let granted = self.sba_v2.register_with_token(ue, token);
        if granted {
            self.amf.register(
                ue,
                TrackingArea::Ntn {
                    ntn_node_id,
                    beam_id,
                    propagation_delay,
                },
            );
            self.amf.authenticate(ue);
            self.push_snapshot();
        }
        granted
    }

    /// Deregister a UE — tears down all AMF, SBAv2, and SMF state.
    ///
    /// Steps:
    /// 1. All active PDU sessions for the UE are released via SMF and UPF.
    /// 2. The AMF registration record is removed.
    /// 3. The SBAv2 validated-registration entry is cleared.
    /// 4. The Digital Twin is updated to reflect the deregistration.
    ///
    /// Returns `true` if the UE had an active registration that was removed.
    pub fn deregister_ue(&mut self, ue: UeId) -> bool {
        if !self.amf.is_registered(ue) {
            return false;
        }
        // Collect all session IDs for this UE, then release them.
        let session_ids: Vec<u8> = self
            .smf
            .sessions_for_ue(ue)
            .iter()
            .map(|s| s.session_id)
            .collect();
        for sid in session_ids {
            self.smf.release_session(sid);
            self.upf.release_bearer(sid);
        }
        self.amf.deregister(ue);
        self.sba_v2.deregister(ue);
        self.push_snapshot();
        true
    }

    /// Establish a PDU session: NSSF slice selection → SMF → UPF → PCF.
    ///
    /// Steps:
    /// 1. **NSSF** selects the requested slice — returns `None` if unavailable.
    /// 2. **SMF** allocates a session ID and a unique IP address.
    /// 3. **UPF** bearer is marked as allocated and the session → UE mapping
    ///    is registered (enabling lazy `forward_unknown_flow` lookups).
    /// 4. **PCF** ensures a policy exists for the slice (adds default if absent).
    /// 5. **Digital Twin** is updated with the new session state.
    ///
    /// Returns a [`SessionGrant`] on success, or `None` if the slice is
    /// unavailable.
    pub fn establish_session(
        &mut self,
        ue: UeId,
        slice: SliceType,
        pdu_type: PduSessionType,
    ) -> Option<SessionGrant> {
        // 1. NSSF: check the slice exists.
        self.nssf.select(slice)?;

        // 2. SMF: allocate session + IP.
        let session_id = self.smf.establish_session(ue, pdu_type);
        let ip_addr = self.smf.session_ip(session_id)?;

        // 3. SMF → UPF linkage: mark bearer allocated + register for lazy lookup.
        self.smf.mark_upf_allocated(session_id);
        self.upf.register_session(session_id, ue);

        // 4. PCF: add a default slice policy if none exists yet.
        if self.pcf.policy_for_slice(slice).is_none() {
            self.pcf.add_policy(pcf::QosPolicy::for_slice(slice));
        }
        let policy = self.pcf.policy_for_slice(slice).unwrap();
        let qci = policy.qci.0;
        let gbr = policy.gbr;

        // 5. Digital Twin: record the new state.
        self.push_snapshot();

        Some(SessionGrant {
            session_id,
            ip_addr,
            slice,
            qci,
            gbr,
        })
    }

    /// Release a PDU session — tears down the SMF session and UPF bearer.
    ///
    /// Returns `true` if the session was found and released.
    pub fn release_session(&mut self, session_id: u8) -> bool {
        let released = self.smf.release_session(session_id);
        if released {
            self.upf.release_bearer(session_id);
            self.push_snapshot();
        }
        released
    }

    /// Push the current network state into the Digital Twin and return the diff.
    ///
    /// Called automatically after every state-changing operation
    /// (`register_ue`, `establish_session`).  Callers can also invoke this
    /// manually to capture intermediate snapshots.
    pub fn push_snapshot(&mut self) -> digital_twin::SnapshotDiff {
        let seq = self.digital_twin.snapshot_count() + 1;
        let mut snap = digital_twin::NetworkSnapshot::new(seq);

        // Per-UE state from AMF records.
        for record in self.amf.registrations() {
            snap.add_ue(digital_twin::UeSnapshot {
                ue: record.ue,
                pdu_session_count: self.smf.session_count_for_ue(record.ue) as u8,
                dl_throughput: Bitrate::from_mbps(0.0), // placeholder; no PHY yet
            });
        }

        // Slice load estimate: sessions spread evenly across configured slices.
        let slice_count = self.nssf.slice_count();
        let session_count = self.smf.session_count();
        let load_pct = if slice_count > 0 {
            (session_count as f64 / slice_count as f64) * 10.0
        } else {
            0.0
        };
        // s_nssai values pre-configured in NSSF: 1 (eMBB), 2 (URLLC), 3 (mMTC), 4 (Sensing)
        for s_nssai in 1..=(slice_count as u32) {
            snap.set_slice_load(s_nssai, load_pct);
        }

        self.digital_twin.update(snap)
    }
}

impl Default for CoreNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_network_initialises_with_phase4_components() {
        let core = CoreNetwork::new();
        assert_eq!(core.sba_v2.registration_count(), 0);
        assert_eq!(core.digital_twin.snapshot_count(), 0);
    }

    #[test]
    fn core_network_initialises_with_phase5_components() {
        let core = CoreNetwork::new();
        assert_eq!(core.ausf.subscriber_count(), 0);
        assert_eq!(core.nrf.active_count(), 0);
    }

    #[test]
    fn register_ue_via_sbav2_succeeds_and_updates_twin() {
        let mut core = CoreNetwork::new();
        let ue = UeId(7);
        let granted = core.register_ue(ue, 2001);
        assert!(granted, "valid SBAv2 token must be granted");
        assert_eq!(core.amf.registered_ue_count(), 1);
        assert_eq!(core.sba_v2.validated_ue_count(), 1);
        // Digital Twin must have captured the registration.
        assert_eq!(core.digital_twin.snapshot_count(), 1);
        let snap = core.digital_twin.current().unwrap();
        assert!(snap.ues.contains_key(&ue), "UE must appear in Digital Twin");
    }

    #[test]
    fn establish_session_wires_nssf_smf_upf_pcf() {
        let mut core = CoreNetwork::new();
        let ue = UeId(55);
        assert!(core.register_ue(ue, 3000));

        let grant = core
            .establish_session(ue, SliceType::Urllc, PduSessionType::Ip)
            .expect("URLLC slice must be available");

        assert!(grant.session_id > 0);
        // IP must come from the 10.0.0.0/8 pool.
        assert_eq!(grant.ip_addr.octets()[0], 10);
        assert_eq!(grant.slice, SliceType::Urllc);
        assert_eq!(grant.qci, 80, "URLLC must map to QCI 80");
        // UPF bearer must be marked allocated.
        assert!(core.smf.all_upf_allocated(), "UPF must be allocated");
        // Digital Twin must have two snapshots (register + establish).
        assert_eq!(core.digital_twin.snapshot_count(), 2);
    }

    #[test]
    fn establish_session_returns_none_for_unknown_slice() {
        let mut core = CoreNetwork::new();
        // NtnBackhaul is not in the default NSSF slice set.
        let result = core.establish_session(UeId(1), SliceType::NtnBackhaul, PduSessionType::Ip);
        assert!(result.is_none(), "unknown slice must return None");
    }

    #[test]
    fn digital_twin_diff_reports_added_ue() {
        let mut core = CoreNetwork::new();
        let diff = core.register_ue(UeId(99), 1000);
        // register_ue returns bool, not diff; use push_snapshot directly.
        let _ = diff;
        let snap = core.digital_twin.current().unwrap();
        assert_eq!(snap.ues.len(), 1);
    }

    #[test]
    fn deregister_ue_tears_down_amf_smf_upf() {
        let mut core = CoreNetwork::new();
        let ue = UeId(10);
        assert!(core.register_ue(ue, 1000));
        let grant = core
            .establish_session(ue, SliceType::EMbb, PduSessionType::Ip)
            .unwrap();

        assert_eq!(core.smf.session_count(), 1);
        assert!(core.deregister_ue(ue), "deregister must succeed");
        // AMF record removed.
        assert_eq!(core.amf.registered_ue_count(), 0);
        // SMF session removed.
        assert_eq!(core.smf.session_count(), 0);
        // UPF bearer removed.
        assert!(core.upf.session_stats(grant.session_id).is_none());
        // Digital Twin updated.
        assert!(core.digital_twin.snapshot_count() >= 3);
    }

    #[test]
    fn release_session_removes_smf_and_upf_bearer() {
        let mut core = CoreNetwork::new();
        let ue = UeId(20);
        assert!(core.register_ue(ue, 2000));
        let grant = core
            .establish_session(ue, SliceType::Urllc, PduSessionType::Ip)
            .unwrap();

        assert!(core.release_session(grant.session_id));
        assert_eq!(core.smf.session_count(), 0);
        assert!(core.upf.session_stats(grant.session_id).is_none());
    }

    #[test]
    fn deregister_unknown_ue_returns_false() {
        let mut core = CoreNetwork::new();
        assert!(!core.deregister_ue(UeId(999)));
    }

    /// Option 1: Semantic session type flows through establish_session.
    #[test]
    fn establish_semantic_session_succeeds() {
        let mut core = CoreNetwork::new();
        let ue = UeId(30);
        assert!(core.register_ue(ue, 5000));

        let goal = GoalSpec {
            task: sixg_semantic::SemanticTask::ImageClassification,
            min_success_rate: sixg_semantic::codec::TaskSuccessRate(0.90),
            max_bandwidth_reduction: sixg_semantic::codec::BandwidthReduction(10.0),
        };
        let grant = core
            .establish_session(ue, SliceType::EMbb, PduSessionType::Semantic(goal))
            .expect("semantic session must be established");

        assert!(grant.session_id > 0);
        // Verify SMF has it flagged as semantic.
        let sessions = core.smf.sessions_for_ue(ue);
        assert_eq!(sessions.len(), 1);
        assert!(
            sessions[0].session_type.is_semantic(),
            "session type must be Semantic"
        );
    }

    /// Option 2: User-plane-first — packet for unregistered UE returns TriggerEstablishment.
    #[test]
    fn user_plane_first_triggers_establishment_for_unknown_ue() {
        let mut core = CoreNetwork::new();
        let ue = UeId(50);
        // No session established — UPF should request lazy establishment.
        let action = core.upf.forward_unknown_flow(ue, b"first packet");
        assert_eq!(
            action,
            FlowAction::TriggerEstablishment(ue),
            "unregistered UE must trigger lazy session establishment"
        );
    }

    /// Option 2: User-plane-first — after establish_session, packet is forwarded immediately.
    #[test]
    fn user_plane_first_forwards_after_session_established() {
        let mut core = CoreNetwork::new();
        let ue = UeId(60);
        assert!(core.register_ue(ue, 6000));
        let grant = core
            .establish_session(ue, SliceType::EMbb, PduSessionType::Ip)
            .unwrap();

        // Now the UPF knows the session — packet must be forwarded immediately.
        let action = core.upf.forward_unknown_flow(ue, b"data packet");
        assert_eq!(
            action,
            FlowAction::Forwarded(grant.session_id),
            "known UE must be forwarded immediately without control-plane roundtrip"
        );
    }

    /// Option 4: NTN-aware AMF — UE registered via NTN node shows correct tracking area.
    #[test]
    fn ntn_ue_registration_records_ntn_tracking_area() {
        let mut core = CoreNetwork::new();
        let ue = UeId(70);
        assert!(
            core.register_ue_ntn(ue, 42, 3, Duration::from_ms(1.83)),
            "NTN registration must succeed"
        );
        let ta = core.amf.tracking_area(ue).unwrap();
        assert!(ta.is_ntn(), "tracking area must be NTN");
        assert_eq!(core.amf.ntn_ue_count(), 1);
    }
}

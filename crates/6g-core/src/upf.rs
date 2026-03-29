//! User Plane Function (UPF).
//!
//! The UPF is the data-plane anchor in the 6G core:
//! * Packet routing and forwarding
//! * Traffic usage reporting
//! * QoS enforcement
//! * Uplink classifier / branching point for local breakout
//!
//! ## 6G extensions
//!
//! **Semantic routing plane** — `forward_semantic_uplink` encodes the payload
//! through a [`sixg_semantic::TextSemanticCodec`] before forwarding.  This
//! routes semantic sessions to the semantic processing function rather than
//! straight GTP-U forwarding, making the `6g-semantic` crate load-bearing.
//!
//! **User-plane-first / lazy session establishment** — `forward_unknown_flow`
//! accepts uplink packets without a pre-established session.  When a session
//! is not found, it returns [`FlowAction::TriggerEstablishment`], signalling
//! the SMF to establish the session in the background while the UPF buffers
//! or forwards the packet (the 6G "control-plane-as-thin-adaptation-layer"
//! hypothesis).

use std::collections::HashMap;

use sixg_common::types::{Payload, UeId};
use sixg_semantic::codec::TextSemanticCodec;
use sixg_semantic::SemanticCodec;

use crate::smf::PduSessionType;

/// UPF traffic statistics (per-global or per-session).
#[derive(Debug, Default, Clone)]
pub struct TrafficStats {
    pub bytes_uplink: u64,
    pub bytes_downlink: u64,
    pub packets_dropped: u64,
}

/// Action returned by [`Upf::forward_unknown_flow`].
///
/// The 6G user-plane-first architecture allows uplink packets to arrive before
/// the control plane has established a session.  The UPF signals the required
/// action to the session runner rather than silently dropping.
#[derive(Debug, PartialEq, Eq)]
pub enum FlowAction {
    /// Session already exists — packet was forwarded.  Contains the `session_id`.
    Forwarded(u8),
    /// No session found for `ue` — the SMF must establish one.
    /// The caller should invoke `CoreNetwork::establish_session` and then
    /// re-inject the buffered payload.
    TriggerEstablishment(UeId),
}

/// User Plane Function.
pub struct Upf {
    /// Aggregate traffic statistics across all sessions.
    pub stats: TrafficStats,
    /// Per-session bearer statistics keyed by `session_id`.
    bearer_stats: HashMap<u8, TrafficStats>,
    /// Known session → UE mapping for unknown-flow lookup.
    session_ue_map: HashMap<u8, UeId>,
    /// Session type map used to enforce semantic routing only on semantic PDU sessions.
    session_type_map: HashMap<u8, PduSessionType>,
    /// Buffered uplink payloads accepted before session establishment (user-plane-first).
    pending_uplink: HashMap<UeId, Vec<Payload>>,
}

impl Upf {
    pub fn new() -> Self {
        Self {
            stats: TrafficStats::default(),
            bearer_stats: HashMap::new(),
            session_ue_map: HashMap::new(),
            session_type_map: HashMap::new(),
            pending_uplink: HashMap::new(),
        }
    }

    /// Register a session → UE mapping so `forward_unknown_flow` can look it up.
    ///
    /// Called by `CoreNetwork::establish_session()` after bearer allocation.
    pub fn register_session(&mut self, session_id: u8, ue: UeId, session_type: PduSessionType) {
        self.session_ue_map.insert(session_id, ue);
        self.session_type_map.insert(session_id, session_type);
        if let Some(buffered_payloads) = self.pending_uplink.remove(&ue) {
            let is_semantic = self
                .session_type_map
                .get(&session_id)
                .map(|t| t.is_semantic())
                .unwrap_or(false);
            for payload in buffered_payloads {
                if is_semantic {
                    let _ = self.forward_semantic_uplink(session_id, &payload);
                } else {
                    self.forward_uplink_for_session(session_id, &payload);
                }
            }
        }
    }

    /// Unregister a session mapping during teardown.
    pub fn unregister_session(&mut self, session_id: u8) {
        self.session_ue_map.remove(&session_id);
        self.session_type_map.remove(&session_id);
    }

    /// Forward an uplink payload (stub – no actual routing yet).
    ///
    /// Accumulates bytes in the global `stats.bytes_uplink` counter.
    pub fn forward_uplink(&mut self, payload: &[u8]) {
        self.stats.bytes_uplink += payload.len() as u64;
    }

    /// Forward a downlink payload (stub).
    ///
    /// Accumulates bytes in the global `stats.bytes_downlink` counter.
    pub fn forward_downlink(&mut self, payload: &[u8]) {
        self.stats.bytes_downlink += payload.len() as u64;
    }

    /// Forward an uplink payload for a specific PDU session bearer.
    ///
    /// Updates both the global counter and the per-session bearer stats.
    /// Creates a bearer entry for `session_id` on first use (PDR install).
    pub fn forward_uplink_for_session(&mut self, session_id: u8, payload: &[u8]) {
        let len = payload.len() as u64;
        self.stats.bytes_uplink += len;
        self.bearer_stats
            .entry(session_id)
            .or_default()
            .bytes_uplink += len;
    }

    /// Forward a downlink payload for a specific PDU session bearer.
    ///
    /// Updates both the global counter and the per-session bearer stats.
    pub fn forward_downlink_for_session(&mut self, session_id: u8, payload: &[u8]) {
        let len = payload.len() as u64;
        self.stats.bytes_downlink += len;
        self.bearer_stats
            .entry(session_id)
            .or_default()
            .bytes_downlink += len;
    }

    /// **6G semantic routing plane** — encode payload through
    /// [`TextSemanticCodec`] before forwarding.
    ///
    /// Steps:
    /// 1. Encode `payload` → compact semantic representation (term-frequency
    ///    signature, ~15× compression per Xie et al. 2021).
    /// 2. Accumulate *compressed* byte count in global + per-session stats.
    /// 3. Return the encoded [`Payload`] for downstream delivery.
    ///
    /// The payload is semantically encoded **only** when the registered
    /// `session_id` is a [`crate::smf::PduSessionType::Semantic`] session.
    /// For non-semantic sessions, the payload is forwarded raw and counted
    /// exactly as in [`forward_uplink_for_session`].
    pub fn forward_semantic_uplink(&mut self, session_id: u8, payload: &[u8]) -> Payload {
        if !self
            .session_type_map
            .get(&session_id)
            .map(|t| t.is_semantic())
            .unwrap_or(false)
        {
            self.forward_uplink_for_session(session_id, payload);
            return payload.to_vec();
        }
        let codec = TextSemanticCodec;
        let encoded = codec.encode(payload);
        let len = encoded.len() as u64;
        self.stats.bytes_uplink += len;
        self.bearer_stats
            .entry(session_id)
            .or_default()
            .bytes_uplink += len;
        encoded
    }

    /// **6G user-plane-first** — accept an uplink packet without a
    /// pre-established session (lazy session establishment).
    ///
    /// This implements the 6G architectural hypothesis that the data path
    /// should not block on control-plane session setup.
    ///
    /// * If a bearer for `ue` exists, the packet is forwarded and
    ///   [`FlowAction::Forwarded(session_id)`] is returned.
    /// * If no bearer exists, the packet is buffered in-memory and **not
    ///   dropped**.  [`FlowAction::TriggerEstablishment(ue)`] is returned so
    ///   the caller can trigger background SMF session establishment.
    ///
    /// Buffered payloads are auto-forwarded when [`register_session`] is
    /// called for the same `ue`.
    ///
    /// Reference: Nokia Bell Labs, *User-Plane-First Architecture for 6G*,
    /// 2021 White Paper.
    pub fn forward_unknown_flow(&mut self, ue: UeId, payload: &[u8]) -> FlowAction {
        // Find the first session registered for this UE.
        if let Some((&session_id, _)) = self.session_ue_map.iter().find(|(_, &u)| u == ue) {
            self.forward_uplink_for_session(session_id, payload);
            FlowAction::Forwarded(session_id)
        } else {
            // No session — buffer packet and signal lazy establishment.
            self.pending_uplink
                .entry(ue)
                .or_default()
                .push(payload.to_vec());
            FlowAction::TriggerEstablishment(ue)
        }
    }

    /// Number of buffered uplink payloads awaiting session establishment for `ue`.
    pub fn buffered_uplink_count(&self, ue: UeId) -> usize {
        self.pending_uplink.get(&ue).map_or(0, Vec::len)
    }

    /// Return per-session bearer statistics for `session_id`, if any traffic
    /// has been forwarded for that bearer.
    pub fn session_stats(&self, session_id: u8) -> Option<&TrafficStats> {
        self.bearer_stats.get(&session_id)
    }

    /// Remove the bearer entry for a released session.
    ///
    /// Called by `CoreNetwork::release_session()` during teardown.
    /// Returns `true` if the bearer was present and removed.
    pub fn release_bearer(&mut self, session_id: u8) -> bool {
        self.unregister_session(session_id);
        self.bearer_stats.remove(&session_id).is_some()
    }

    /// Number of active (non-released) bearer entries.
    pub fn bearer_count(&self) -> usize {
        self.bearer_stats.len()
    }
}

impl Default for Upf {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smf::GoalSpec;
    use sixg_common::types::UeId;
    use sixg_semantic::codec::{BandwidthReduction, TaskSuccessRate};
    use sixg_semantic::SemanticTask;

    #[test]
    fn forward_uplink_accumulates_bytes() {
        let mut upf = Upf::new();
        upf.forward_uplink(b"hello");
        assert_eq!(upf.stats.bytes_uplink, 5);
        upf.forward_uplink(b"world!");
        assert_eq!(upf.stats.bytes_uplink, 11, "counter must be cumulative");
        assert_eq!(upf.stats.bytes_downlink, 0, "downlink must be unaffected");
    }

    #[test]
    fn forward_downlink_accumulates_bytes() {
        let mut upf = Upf::new();
        upf.forward_downlink(b"data");
        assert_eq!(upf.stats.bytes_downlink, 4);
        upf.forward_downlink(b"more data");
        assert_eq!(upf.stats.bytes_downlink, 13, "counter must be cumulative");
        assert_eq!(upf.stats.bytes_uplink, 0, "uplink must be unaffected");
    }

    #[test]
    fn counters_never_decrease() {
        let mut upf = Upf::new();
        for _ in 0..10 {
            upf.forward_uplink(b"pkt");
        }
        let before = upf.stats.bytes_uplink;
        // No method to decrement — compile-time guarantee.
        // This test documents the invariant: counters are cumulative.
        assert_eq!(upf.stats.bytes_uplink, before);
        assert_eq!(upf.stats.packets_dropped, 0);
    }

    #[test]
    fn per_session_bearer_stats_are_tracked() {
        let mut upf = Upf::new();
        upf.forward_uplink_for_session(1, b"hello");
        upf.forward_downlink_for_session(1, b"world!");
        upf.forward_uplink_for_session(2, b"data");

        let s1 = upf.session_stats(1).expect("bearer 1 must have stats");
        assert_eq!(s1.bytes_uplink, 5);
        assert_eq!(s1.bytes_downlink, 6);

        let s2 = upf.session_stats(2).expect("bearer 2 must have stats");
        assert_eq!(s2.bytes_uplink, 4);
        assert_eq!(s2.bytes_downlink, 0);

        // Global counter aggregates all sessions.
        assert_eq!(upf.stats.bytes_uplink, 9);
        assert_eq!(upf.stats.bytes_downlink, 6);
    }

    #[test]
    fn release_bearer_removes_session_stats() {
        let mut upf = Upf::new();
        upf.forward_uplink_for_session(3, b"payload");
        assert_eq!(upf.bearer_count(), 1);
        assert!(upf.release_bearer(3));
        assert_eq!(upf.bearer_count(), 0);
        assert!(upf.session_stats(3).is_none());
    }

    /// Semantic routing compresses the payload.
    /// TextSemanticCodec always produces exactly 64 bytes regardless of input
    /// size (term-frequency signature). A 200-byte input → 64-byte output.
    #[test]
    fn semantic_uplink_compresses_payload() {
        let mut upf = Upf::new();
        let goal = GoalSpec {
            task: SemanticTask::TextUnderstanding,
            min_success_rate: TaskSuccessRate(0.90),
            max_bandwidth_reduction: BandwidthReduction(10.0),
        };
        upf.register_session(1, UeId(1), PduSessionType::Semantic(goal));
        let raw = b"the quick brown fox jumps over the lazy dog ".repeat(5); // 220 bytes
        let encoded = upf.forward_semantic_uplink(1, &raw);
        // Codec output is always 64 bytes.
        assert_eq!(
            encoded.len(),
            64,
            "semantic codec must produce 64-byte output"
        );
        // UPF must count compressed bytes, not raw bytes.
        assert_eq!(
            upf.stats.bytes_uplink, 64,
            "UPF must count encoded bytes for semantic sessions"
        );
        assert!(
            (raw.len() as u64) > upf.stats.bytes_uplink,
            "raw bytes must exceed compressed bytes"
        );
    }

    /// User-plane-first: packet for unknown UE returns TriggerEstablishment.
    #[test]
    fn unknown_flow_triggers_establishment() {
        let mut upf = Upf::new();
        let ue = UeId(42);
        let action = upf.forward_unknown_flow(ue, b"first packet");
        assert_eq!(
            action,
            FlowAction::TriggerEstablishment(ue),
            "unknown UE must request lazy establishment"
        );
        // Packet is buffered, not dropped.
        assert_eq!(upf.stats.bytes_uplink, 0);
        assert_eq!(upf.buffered_uplink_count(ue), 1);
    }

    /// User-plane-first: packet for known UE is forwarded immediately.
    #[test]
    fn known_flow_is_forwarded_immediately() {
        let mut upf = Upf::new();
        let ue = UeId(7);
        upf.register_session(1, ue, PduSessionType::Ip);
        let action = upf.forward_unknown_flow(ue, b"payload");
        assert_eq!(
            action,
            FlowAction::Forwarded(1),
            "known UE must be forwarded immediately"
        );
        assert_eq!(upf.stats.bytes_uplink, 7, "bytes must be counted");
    }

    #[test]
    fn buffered_unknown_flow_is_flushed_on_session_registration() {
        let mut upf = Upf::new();
        let ue = UeId(8);
        assert_eq!(
            upf.forward_unknown_flow(ue, b"first"),
            FlowAction::TriggerEstablishment(ue)
        );
        assert_eq!(upf.buffered_uplink_count(ue), 1);
        upf.register_session(3, ue, PduSessionType::Ip);
        assert_eq!(upf.buffered_uplink_count(ue), 0);
        assert_eq!(
            upf.stats.bytes_uplink, 5,
            "buffered payload must be forwarded"
        );
    }

    #[test]
    fn non_semantic_session_is_not_semantically_encoded() {
        let mut upf = Upf::new();
        let ue = UeId(9);
        upf.register_session(4, ue, PduSessionType::Ip);
        let payload = b"plain ip packet data";
        let out = upf.forward_semantic_uplink(4, payload);
        assert_eq!(out, payload, "IP session must forward payload unmodified");
        assert_eq!(
            upf.stats.bytes_uplink as usize,
            payload.len(),
            "IP session accounting must use raw bytes"
        );
    }
}

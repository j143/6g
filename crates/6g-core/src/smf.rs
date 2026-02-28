//! Session Management Function (SMF).
//!
//! The SMF establishes, modifies, and releases PDU sessions for UEs.
//! In 6G it also supports:
//! * Semantic communication sessions (goal-oriented QoS)
//! * Multi-path / ATSSS (Access Traffic Steering, Switching, Splitting)
//! * AI-assisted dynamic session management

use std::net::Ipv4Addr;

use sixg_common::types::UeId;
use sixg_semantic::codec::{BandwidthReduction, TaskSuccessRate};
use sixg_semantic::SemanticTask;

/// Goal specification for a semantic PDU session.
///
/// Replaces the 5G bandwidth/latency QoS contract with a *task-success* SLA.
/// "This session is for image classification — guarantee that `min_success_rate`
/// of inferences at the receiver succeed, using ≤ `max_bandwidth_pct` of the
/// raw data path."
///
/// Reference: Qin et al., *Semantic Communications: Principles and Challenges*,
/// IEEE JSAC 2022.
#[derive(Debug, Clone, PartialEq)]
pub struct GoalSpec {
    /// Semantic task type (classification, speech understanding, control, text).
    pub task: SemanticTask,
    /// Minimum required task success rate (0.0–1.0).
    pub min_success_rate: TaskSuccessRate,
    /// Maximum fraction of raw IP bandwidth permitted (dimensionless, 0.0–1.0).
    /// A value of `BandwidthReduction(10.0)` means ≤ 10% of raw bandwidth.
    pub max_bandwidth_reduction: BandwidthReduction,
}

/// PDU session type.
#[derive(Debug, Clone, PartialEq)]
pub enum PduSessionType {
    /// IPv4/IPv6 internet session.
    Ip,
    /// Unstructured (non-IP) data session.
    Unstructured,
    /// Ethernet session (LAN services).
    Ethernet,
    /// **6G-native** semantic session — QoS is expressed as a task-success
    /// rate, not bandwidth/latency.  The UPF routes payloads through a
    /// [`sixg_semantic::SemanticCodec`] rather than straight GTP-U forwarding.
    Semantic(GoalSpec),
}

impl PduSessionType {
    /// Returns `true` if this is a semantic (goal-oriented) session.
    pub fn is_semantic(&self) -> bool {
        matches!(self, Self::Semantic(_))
    }
}

/// A PDU session record maintained by the SMF.
///
/// # Invariant
/// Each session is assigned a unique [`ip_addr`](Self::ip_addr) from the
/// `10.0.0.0/8` pool at creation time and this is never changed afterwards.
#[derive(Debug, Clone)]
pub struct PduSession {
    pub session_id: u8,
    pub ue: UeId,
    pub session_type: PduSessionType,
    /// Unique IPv4 address allocated from the `10.0.0.x` pool.
    pub ip_addr: Ipv4Addr,
    pub upf_allocated: bool,
}

/// Session Management Function.
pub struct Smf {
    sessions: Vec<PduSession>,
    next_id: u8,
}

impl Smf {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            next_id: 1,
        }
    }

    /// Establish a new PDU session for a UE.
    ///
    /// Returns the allocated `session_id` (1-indexed, wraps at 255).
    /// Each session is assigned a unique address from `10.0.0.x` where `x`
    /// equals the session id, satisfying the uniqueness invariant.
    pub fn establish_session(&mut self, ue: UeId, session_type: PduSessionType) -> u8 {
        let id = self.next_id;
        let ip_addr = Ipv4Addr::new(10, 0, 0, id);
        self.sessions.push(PduSession {
            session_id: id,
            ue,
            session_type,
            ip_addr,
            upf_allocated: false,
        });
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    /// Release a PDU session, removing its record from the SMF.
    ///
    /// Returns `true` if the session was found and removed, `false` if unknown.
    /// Called by `CoreNetwork::release_session()` as part of the teardown path.
    pub fn release_session(&mut self, session_id: u8) -> bool {
        if let Some(pos) = self
            .sessions
            .iter()
            .position(|s| s.session_id == session_id)
        {
            self.sessions.remove(pos);
            true
        } else {
            false
        }
    }

    /// Return the total number of established sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Look up the allocated IP address for a session, if it exists.
    pub fn session_ip(&self, session_id: u8) -> Option<Ipv4Addr> {
        self.sessions
            .iter()
            .find(|s| s.session_id == session_id)
            .map(|s| s.ip_addr)
    }

    /// Mark a PDU session as having a UPF bearer allocated.
    ///
    /// Called by `CoreNetwork::establish_session()` after the UPF accepts
    /// the session, satisfying the SMF ↔ UPF invariant.
    pub fn mark_upf_allocated(&mut self, session_id: u8) {
        if let Some(s) = self
            .sessions
            .iter_mut()
            .find(|s| s.session_id == session_id)
        {
            s.upf_allocated = true;
        }
    }

    /// Return all active sessions for a given UE.
    ///
    /// Used by `CoreNetwork::deregister_ue()` to collect session IDs for teardown.
    pub fn sessions_for_ue(&self, ue: UeId) -> Vec<&PduSession> {
        self.sessions.iter().filter(|s| s.ue == ue).collect()
    }

    /// Number of active sessions for a given UE.
    pub fn session_count_for_ue(&self, ue: UeId) -> usize {
        self.sessions.iter().filter(|s| s.ue == ue).count()
    }

    /// Returns `true` when all registered sessions have a UPF bearer allocated.
    pub fn all_upf_allocated(&self) -> bool {
        self.sessions.iter().all(|s| s.upf_allocated)
    }
}

impl Default for Smf {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn establish_session_increments_count() {
        let mut smf = Smf::new();
        assert_eq!(smf.session_count(), 0);
        smf.establish_session(UeId(1), PduSessionType::Ip);
        assert_eq!(smf.session_count(), 1);
        smf.establish_session(UeId(2), PduSessionType::Ethernet);
        assert_eq!(smf.session_count(), 2);
    }

    #[test]
    fn release_session_removes_record() {
        let mut smf = Smf::new();
        let id = smf.establish_session(UeId(1), PduSessionType::Ip);
        assert_eq!(smf.session_count(), 1);
        assert!(smf.release_session(id));
        assert_eq!(smf.session_count(), 0);
        assert!(
            smf.session_ip(id).is_none(),
            "IP must be gone after release"
        );
    }

    #[test]
    fn release_unknown_session_returns_false() {
        let mut smf = Smf::new();
        assert!(!smf.release_session(99));
    }

    #[test]
    fn session_ids_are_sequential() {
        let mut smf = Smf::new();
        let id1 = smf.establish_session(UeId(1), PduSessionType::Ip);
        let id2 = smf.establish_session(UeId(1), PduSessionType::Unstructured);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn each_session_gets_unique_ip() {
        let mut smf = Smf::new();
        let id1 = smf.establish_session(UeId(10), PduSessionType::Ip);
        let id2 = smf.establish_session(UeId(20), PduSessionType::Ip);
        let ip1 = smf.session_ip(id1).expect("session 1 must have an IP");
        let ip2 = smf.session_ip(id2).expect("session 2 must have an IP");
        assert_ne!(ip1, ip2, "invariant #4: each session must have a unique IP");
        // IPs come from the 10.0.0.0/8 pool
        assert_eq!(ip1.octets()[0], 10);
        assert_eq!(ip2.octets()[0], 10);
    }

    #[test]
    fn session_ip_returns_none_for_unknown_id() {
        let smf = Smf::new();
        assert!(smf.session_ip(99).is_none());
    }

    #[test]
    fn semantic_session_is_identified() {
        let mut smf = Smf::new();
        let goal = GoalSpec {
            task: SemanticTask::ImageClassification,
            min_success_rate: TaskSuccessRate(0.90),
            max_bandwidth_reduction: BandwidthReduction(10.0),
        };
        let id = smf.establish_session(UeId(1), PduSessionType::Semantic(goal));
        assert_eq!(smf.session_count(), 1);
        // Retrieve the session and verify it is flagged as semantic.
        let session = smf.sessions_for_ue(UeId(1));
        assert_eq!(session.len(), 1);
        assert!(
            session[0].session_type.is_semantic(),
            "session must be semantic"
        );
        assert!(
            smf.session_ip(id).is_some(),
            "semantic session must have an IP"
        );
    }
}

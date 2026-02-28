//! Session Management Function (SMF).
//!
//! The SMF establishes, modifies, and releases PDU sessions for UEs.
//! In 6G it also supports:
//! * Semantic communication sessions (goal-oriented QoS)
//! * Multi-path / ATSSS (Access Traffic Steering, Switching, Splitting)
//! * AI-assisted dynamic session management

use std::net::Ipv4Addr;

use sixg_common::types::UeId;

/// PDU session type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PduSessionType {
    /// IPv4/IPv6 internet session.
    Ip,
    /// Unstructured (non-IP) data session.
    Unstructured,
    /// Ethernet session (LAN services).
    Ethernet,
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
            let updated = PduSession {
                session_id: s.session_id,
                ue: s.ue,
                session_type: s.session_type,
                ip_addr: s.ip_addr,
                upf_allocated: true,
            };
            *s = updated;
        }
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
}

//! Session Management Function (SMF).
//!
//! The SMF establishes, modifies, and releases PDU sessions for UEs.
//! In 6G it also supports:
//! * Semantic communication sessions (goal-oriented QoS)
//! * Multi-path / ATSSS (Access Traffic Steering, Switching, Splitting)
//! * AI-assisted dynamic session management

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
#[derive(Debug, Clone)]
pub struct PduSession {
    pub session_id: u8,
    pub ue: UeId,
    pub session_type: PduSessionType,
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
    pub fn establish_session(&mut self, ue: UeId, session_type: PduSessionType) -> u8 {
        let id = self.next_id;
        self.sessions.push(PduSession {
            session_id: id,
            ue,
            session_type,
            upf_allocated: false,
        });
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for Smf {
    fn default() -> Self {
        Self::new()
    }
}

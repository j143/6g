//! Radio Resource Control (RRC) layer for 6G.
//!
//! RRC manages the control-plane between the UE and the RAN:
//! * RRC connection establishment, modification, and release
//! * Broadcast of system information (SIBs)
//! * Measurement configuration and reporting
//! * Mobility (handover, conditional handover, DAPS)
//! * AI/ML model provisioning from network to UE

use sixg_common::types::{NodeId, UeId};

/// RRC connection state of a UE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RrcState {
    /// UE has no RRC context; completely idle.
    Idle,
    /// UE is reachable but has suspended its connection (new in NR, retained in 6G).
    Inactive,
    /// Full RRC connection established; data transfer possible.
    Connected,
}

/// An RRC context maintained per UE.
#[derive(Debug, Clone)]
pub struct UeContext {
    pub ue: UeId,
    pub serving_node: NodeId,
    pub state: RrcState,
}

impl UeContext {
    pub fn new(ue: UeId, serving_node: NodeId) -> Self {
        Self {
            ue,
            serving_node,
            state: RrcState::Idle,
        }
    }

    /// Move to Connected state.
    pub fn connect(&mut self) {
        self.state = RrcState::Connected;
    }

    /// Suspend the connection (Idle ← Inactive ← Connected).
    pub fn suspend(&mut self) {
        if self.state == RrcState::Connected {
            self.state = RrcState::Inactive;
        }
    }

    /// Release the RRC connection entirely.
    pub fn release(&mut self) {
        self.state = RrcState::Idle;
    }
}

/// RRC layer – manages all UE contexts.
pub struct RrcLayer {
    contexts: Vec<UeContext>,
}

impl RrcLayer {
    pub fn new() -> Self {
        Self {
            contexts: Vec::new(),
        }
    }

    /// Register a new UE and return its index.
    pub fn add_ue(&mut self, ue: UeId, node: NodeId) -> usize {
        let idx = self.contexts.len();
        self.contexts.push(UeContext::new(ue, node));
        idx
    }

    pub fn ue_count(&self) -> usize {
        self.contexts.len()
    }

    pub fn context(&self, idx: usize) -> Option<&UeContext> {
        self.contexts.get(idx)
    }

    pub fn context_mut(&mut self, idx: usize) -> Option<&mut UeContext> {
        self.contexts.get_mut(idx)
    }
}

impl Default for RrcLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ue_state_transitions() {
        let mut ctx = UeContext::new(UeId(1), NodeId(100));
        assert_eq!(ctx.state, RrcState::Idle);
        ctx.connect();
        assert_eq!(ctx.state, RrcState::Connected);
        ctx.suspend();
        assert_eq!(ctx.state, RrcState::Inactive);
        ctx.release();
        assert_eq!(ctx.state, RrcState::Idle);
    }
}

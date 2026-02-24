//! HARQ (Hybrid Automatic Repeat reQuest) process management.

/// Number of HARQ processes per UE (6G can support more than 5G's 16).
pub const MAX_HARQ_PROCESSES: usize = 32;

/// State of a single HARQ process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarqState {
    Idle,
    WaitingAck,
    Retransmitting { attempt: u8 },
}

/// Manages the pool of HARQ processes.
pub struct HarqManager {
    processes: [HarqState; MAX_HARQ_PROCESSES],
}

impl HarqManager {
    pub fn new() -> Self {
        Self {
            processes: [HarqState::Idle; MAX_HARQ_PROCESSES],
        }
    }

    /// Return the state of HARQ process `id`.
    pub fn state(&self, id: usize) -> Option<HarqState> {
        self.processes.get(id).copied()
    }

    /// Mark a process as waiting for an ACK/NACK.
    pub fn start(&mut self, id: usize) {
        if let Some(p) = self.processes.get_mut(id) {
            *p = HarqState::WaitingAck;
        }
    }

    /// Acknowledge a successful transmission and free the process.
    pub fn acknowledge(&mut self, id: usize) {
        if let Some(p) = self.processes.get_mut(id) {
            *p = HarqState::Idle;
        }
    }
}

impl Default for HarqManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harq_process_lifecycle() {
        let mut mgr = HarqManager::new();
        assert_eq!(mgr.state(0), Some(HarqState::Idle));
        mgr.start(0);
        assert_eq!(mgr.state(0), Some(HarqState::WaitingAck));
        mgr.acknowledge(0);
        assert_eq!(mgr.state(0), Some(HarqState::Idle));
    }
}

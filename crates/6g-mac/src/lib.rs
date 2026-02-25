//! Medium Access Control (MAC) layer for 6G.
//!
//! The MAC layer manages:
//! * Uplink and downlink scheduling (AI-native)
//! * Multiple access schemes (OFDMA, NOMA, grant-free)
//! * HARQ process management
//! * Random access procedures

pub mod access;
pub mod harq;
pub mod scheduler;

pub use access::AccessScheme;
pub use harq::{HarqManager, HarqState, ProactiveHarq};
pub use scheduler::{
    jain_fairness, ResourceAssignment, Scheduler, SchedulingPolicy, UeChannelState,
};

/// MAC layer entry point.
pub struct MacLayer {
    scheduler: Scheduler,
    harq: HarqManager,
}

impl MacLayer {
    pub fn new() -> Self {
        Self {
            scheduler: Scheduler::new(),
            harq: HarqManager::new(),
        }
    }

    pub fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }

    pub fn harq(&self) -> &HarqManager {
        &self.harq
    }
}

impl Default for MacLayer {
    fn default() -> Self {
        Self::new()
    }
}

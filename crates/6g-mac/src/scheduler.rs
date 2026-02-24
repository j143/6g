//! AI-native downlink/uplink scheduler.
//!
//! The 6G MAC scheduler is AI-native: it uses a learned policy to map UE
//! channel state information (CSI) to resource block (RB) assignments,
//! optimising for throughput, fairness, and energy efficiency simultaneously.

use sixg_common::types::UeId;

/// A resource block assignment produced by the scheduler.
#[derive(Debug, Clone)]
pub struct ResourceAssignment {
    pub ue: UeId,
    /// Starting resource block index.
    pub rb_start: usize,
    /// Number of allocated resource blocks.
    pub rb_count: usize,
    /// Modulation and Coding Scheme index.
    pub mcs: u8,
}

/// Scheduling policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    /// Round-robin (baseline, no AI).
    RoundRobin,
    /// Proportional fair.
    ProportionalFair,
    /// AI-native policy (RL-trained).
    AiNative,
}

/// The MAC scheduler.
pub struct Scheduler {
    policy: SchedulingPolicy,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            policy: SchedulingPolicy::AiNative,
        }
    }

    pub fn policy(&self) -> SchedulingPolicy {
        self.policy
    }

    /// Produce resource assignments for the provided set of UEs.
    ///
    /// This is a stub – a real implementation will call the AI engine to
    /// obtain per-UE predictions and translate them into RB allocations.
    pub fn schedule(&self, ues: &[UeId], total_rbs: usize) -> Vec<ResourceAssignment> {
        if ues.is_empty() || total_rbs == 0 {
            return vec![];
        }
        let rbs_per_ue = (total_rbs / ues.len()).max(1);
        ues.iter()
            .enumerate()
            .map(|(i, &ue)| ResourceAssignment {
                ue,
                rb_start: i * rbs_per_ue,
                rb_count: rbs_per_ue,
                mcs: 27, // placeholder: highest 5G NR MCS
            })
            .collect()
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_assigns_all_rbs() {
        let sched = Scheduler::new();
        let ues = vec![UeId(1), UeId(2), UeId(4)];
        let assignments = sched.schedule(&ues, 99);
        assert_eq!(assignments.len(), 3);
        // Each UE gets 33 RBs (99 / 3).
        for a in &assignments {
            assert_eq!(a.rb_count, 33);
        }
    }

    #[test]
    fn scheduler_handles_empty_ue_list() {
        let sched = Scheduler::new();
        assert!(sched.schedule(&[], 100).is_empty());
    }
}

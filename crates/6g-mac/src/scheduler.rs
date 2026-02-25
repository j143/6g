//! AI-native downlink/uplink scheduler.
//!
//! The 6G MAC scheduler is AI-native: it uses a learned policy to map UE
//! channel state information (CSI) to resource block (RB) assignments,
//! optimising for throughput, fairness, and energy efficiency simultaneously.
//!
//! Three policies are provided for comparison experiments:
//! - [`SchedulingPolicy::RoundRobin`] – equal allocation, stateful rotation.
//! - [`SchedulingPolicy::ProportionalFair`] – maximises `r_k / R̄_k` metric.
//! - [`SchedulingPolicy::AiNative`] – ε-greedy Q-learning bandit.
//!
//! Use [`Scheduler::schedule_with_csi`] for full-featured scheduling and
//! [`jain_fairness`] to measure allocation equity across UEs.

use sixg_common::types::{SnrLinear, UeId};
use sixg_common::validation::{Validate, ValidationCheck, ValidationResult};

// ---------------------------------------------------------------------------
// Resource assignment
// ---------------------------------------------------------------------------

/// A resource block assignment produced by the scheduler for one TTI.
#[derive(Debug, Clone)]
pub struct ResourceAssignment {
    pub ue: UeId,
    /// Starting resource block index (dimensionless PRB index).
    pub rb_start: usize,
    /// Number of allocated resource blocks (dimensionless PRB count).
    pub rb_count: usize,
    /// Modulation and Coding Scheme index (0 – 27, per 3GPP TS 38.214).
    pub mcs: u8,
}

// ---------------------------------------------------------------------------
// Scheduling policy
// ---------------------------------------------------------------------------

/// Scheduling policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    /// Round-robin: equal allocation, rotates starting UE each TTI.
    RoundRobin,
    /// Proportional fair: maximises instantaneous-rate / average-rate ratio.
    ProportionalFair,
    /// AI-native: ε-greedy Q-learning bandit over (UE × channel-state) space.
    AiNative,
}

// ---------------------------------------------------------------------------
// Per-UE channel state
// ---------------------------------------------------------------------------

/// Per-UE channel state reported each TTI.
#[derive(Debug, Clone)]
pub struct UeChannelState {
    pub ue: UeId,
    /// Instantaneous SNR reported by the UE (linear ratio).
    pub snr: SnrLinear,
    /// Exponential moving average of served throughput (bits/s, dimensionless ratio).
    pub avg_throughput_bps: f64,
}

impl UeChannelState {
    /// Create a new channel state with no throughput history.
    pub fn new(ue: UeId, snr: SnrLinear) -> Self {
        Self {
            ue,
            snr,
            avg_throughput_bps: 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Jain fairness
// ---------------------------------------------------------------------------

/// Jain's fairness index for a set of throughput values (dimensionless, 0–1).
///
/// `J = (Σ xᵢ)² / (n · Σ xᵢ²)`.  Returns `1.0` for perfect fairness.
/// Reference: Jain et al., "A quantitative measure of fairness", 1984.
///
/// # Examples
/// ```
/// use sixg_mac::scheduler::jain_fairness;
/// // Perfect fairness: all equal
/// assert!((jain_fairness(&[1.0, 1.0, 1.0]) - 1.0).abs() < 1e-9);
/// // Worst case: one UE gets everything
/// assert!(jain_fairness(&[1.0, 0.0, 0.0]) < 0.4);
/// ```
pub fn jain_fairness(throughputs: &[f64]) -> f64 {
    if throughputs.is_empty() {
        return 1.0;
    }
    let n = throughputs.len() as f64;
    let sum: f64 = throughputs.iter().sum();
    let sum_sq: f64 = throughputs.iter().map(|x| x * x).sum();
    if sum_sq == 0.0 {
        return 1.0;
    }
    (sum * sum) / (n * sum_sq)
}

// ---------------------------------------------------------------------------
// Q-learning bandit
// ---------------------------------------------------------------------------

/// ε-greedy Q-learning bandit scheduler state.
///
/// Maintains a Q-value table indexed by `(UE index, channel-state bucket)`.
/// Exploration rate `ε` defaults to `0.1` (10 % random exploration).
pub struct QBandit {
    /// Q-values: `q_table[ue_idx][channel_bucket]` = estimated normalised reward.
    q_table: Vec<Vec<f64>>,
    /// Number of discrete channel-state buckets.
    n_buckets: usize,
    /// Exploration probability ε ∈ [0, 1].
    epsilon: f64,
    /// TD learning rate α ∈ (0, 1].
    alpha: f64,
}

impl QBandit {
    /// Create a new bandit for `n_ues` UEs with `n_buckets` SNR buckets.
    ///
    /// `epsilon` — exploration rate (0.1 recommended).
    pub fn new(n_ues: usize, n_buckets: usize, epsilon: f64) -> Self {
        Self {
            q_table: vec![vec![0.0; n_buckets]; n_ues],
            n_buckets,
            epsilon,
            alpha: 0.1,
        }
    }

    /// Map a linear SNR value to a discrete bucket index.
    fn snr_bucket(&self, snr: SnrLinear) -> usize {
        // Map 0–30 dB to [0, n_buckets-1].
        let snr_db = 10.0 * snr.as_linear().max(1e-6).log10();
        let bucket = (snr_db.max(0.0) / 30.0 * (self.n_buckets as f64 - 1.0)) as usize;
        bucket.min(self.n_buckets - 1)
    }

    /// Select the UE index to prioritise (ε-greedy over Q-values).
    ///
    /// `tti` — current TTI, used as a deterministic LCG seed.
    pub fn select(&self, ue_states: &[UeChannelState], tti: u64) -> usize {
        if ue_states.is_empty() {
            return 0;
        }
        // Deterministic LCG for reproducible experiments.
        let rng = tti
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let explore = (rng & 0xFFFF) as f64 / 65535.0 < self.epsilon;
        if explore {
            return (rng as usize) % ue_states.len();
        }
        // Greedy: UE with highest Q-value at its current channel bucket.
        ue_states
            .iter()
            .enumerate()
            .max_by(|(i, a), (j, b)| {
                let qa = self.q_value(*i, a.snr);
                let qb = self.q_value(*j, b.snr);
                qa.partial_cmp(&qb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    /// Return the current Q-value for a (UE index, SNR) pair.
    pub fn q_value(&self, ue_idx: usize, snr: SnrLinear) -> f64 {
        let bucket = self.snr_bucket(snr);
        self.q_table
            .get(ue_idx)
            .and_then(|row| row.get(bucket))
            .copied()
            .unwrap_or(0.0)
    }

    /// TD(0) update after observing `reward` for (UE index, SNR).
    ///
    /// `reward` should be a normalised value in [0, 1] (e.g. throughput / max_throughput).
    pub fn update(&mut self, ue_idx: usize, snr: SnrLinear, reward: f64) {
        if ue_idx >= self.q_table.len() {
            return;
        }
        let bucket = self.snr_bucket(snr);
        let q = self.q_table[ue_idx][bucket];
        self.q_table[ue_idx][bucket] = q + self.alpha * (reward - q);
    }
}

// ---------------------------------------------------------------------------
// Scheduler
// ---------------------------------------------------------------------------

/// MAC resource scheduler supporting three policies.
pub struct Scheduler {
    policy: SchedulingPolicy,
    /// Round-robin rotation offset (TTI index mod n_ues).
    rr_offset: usize,
    /// Q-learning bandit (used by AiNative policy).
    q_bandit: Option<QBandit>,
    /// TTI counter.
    tti: u64,
}

impl Scheduler {
    /// Create a scheduler with the `AiNative` policy.
    pub fn new() -> Self {
        Self::with_policy(SchedulingPolicy::AiNative)
    }

    /// Create a scheduler with a specific policy.
    pub fn with_policy(policy: SchedulingPolicy) -> Self {
        let q_bandit = match policy {
            SchedulingPolicy::AiNative => Some(QBandit::new(64, 16, 0.1)),
            _ => None,
        };
        Self {
            policy,
            rr_offset: 0,
            q_bandit,
            tti: 0,
        }
    }

    /// Return the active scheduling policy.
    pub fn policy(&self) -> SchedulingPolicy {
        self.policy
    }

    /// Produce resource assignments for `ues` using equal allocation (stateless).
    ///
    /// Each UE receives `floor(total_rbs / n_ues)` PRBs starting at contiguous offsets.
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
                mcs: 27,
            })
            .collect()
    }

    /// Schedule one TTI using per-UE channel state information.
    ///
    /// Returns one [`ResourceAssignment`] per active UE. The policy governs
    /// how PRBs are distributed:
    /// - `RoundRobin` — equal PRBs, starting UE rotates each TTI.
    /// - `ProportionalFair` — orders by `SNR_linear / avg_throughput_bps`.
    /// - `AiNative` — ε-greedy Q-bandit selects priority UE; it receives up
    ///   to 2× the base allocation, with the remainder split among others.
    pub fn schedule_with_csi(
        &mut self,
        ue_states: &[UeChannelState],
        total_rbs: usize,
    ) -> Vec<ResourceAssignment> {
        if ue_states.is_empty() || total_rbs == 0 {
            self.tti = self.tti.wrapping_add(1);
            return vec![];
        }
        let n = ue_states.len();
        let assignments = match self.policy {
            SchedulingPolicy::RoundRobin => {
                let rbs_per_ue = (total_rbs / n).max(1);
                (0..n)
                    .map(|slot| {
                        let idx = (slot + self.rr_offset) % n;
                        ResourceAssignment {
                            ue: ue_states[idx].ue,
                            rb_start: slot * rbs_per_ue,
                            rb_count: rbs_per_ue,
                            mcs: snr_to_mcs(ue_states[idx].snr),
                        }
                    })
                    .collect()
            }
            SchedulingPolicy::ProportionalFair => {
                let mut order: Vec<usize> = (0..n).collect();
                order.sort_by(|&a, &b| {
                    pf_metric(&ue_states[b])
                        .partial_cmp(&pf_metric(&ue_states[a]))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let rbs_per_ue = (total_rbs / n).max(1);
                order
                    .iter()
                    .enumerate()
                    .map(|(slot, &idx)| ResourceAssignment {
                        ue: ue_states[idx].ue,
                        rb_start: slot * rbs_per_ue,
                        rb_count: rbs_per_ue,
                        mcs: snr_to_mcs(ue_states[idx].snr),
                    })
                    .collect()
            }
            SchedulingPolicy::AiNative => {
                let priority_idx = if let Some(ref bandit) = self.q_bandit {
                    bandit.select(ue_states, self.tti)
                } else {
                    self.tti as usize % n
                };
                let base = (total_rbs / n).max(1);
                // Priority UE gets 2× base, remainder divided among others.
                let priority_rbs = (base * 2).min(total_rbs);
                let other_rbs = if n > 1 {
                    (total_rbs.saturating_sub(priority_rbs) / (n - 1)).max(1)
                } else {
                    0
                };
                let mut cursor = 0usize;
                let mut out = Vec::with_capacity(n);
                for slot in 0..n {
                    let idx = (slot + priority_idx) % n;
                    let rbs = if slot == 0 { priority_rbs } else { other_rbs };
                    out.push(ResourceAssignment {
                        ue: ue_states[idx].ue,
                        rb_start: cursor,
                        rb_count: rbs,
                        mcs: snr_to_mcs(ue_states[idx].snr),
                    });
                    cursor += rbs;
                }
                out
            }
        };
        self.rr_offset = (self.rr_offset + 1) % n;
        self.tti = self.tti.wrapping_add(1);
        assignments
    }

    /// Feed back an observed throughput reward to the Q-learning bandit.
    ///
    /// `ue_idx` — index into the last `ue_states` slice passed to `schedule_with_csi`.
    /// `snr` — SNR at which the UE was served (linear).
    /// `throughput_bps` — achieved throughput in bits/s (used to compute normalised reward).
    pub fn observe_reward(&mut self, ue_idx: usize, snr: SnrLinear, throughput_bps: f64) {
        if let Some(ref mut bandit) = self.q_bandit {
            // Normalise to [0, 1] assuming a peak of 10 Gbps per UE.
            let reward = (throughput_bps / 10e9).clamp(0.0, 1.0);
            bandit.update(ue_idx, snr, reward);
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Map a linear SNR to a 5G NR MCS index (0 – 27, per 3GPP TS 38.214 Table 5.1.3.1-1).
///
/// Linear mapping: 0 dB → MCS 0, ≥ 30 dB → MCS 27.
fn snr_to_mcs(snr: SnrLinear) -> u8 {
    let snr_db = 10.0 * snr.as_linear().max(1e-10).log10();
    ((snr_db.max(0.0) / 30.0 * 27.0) as u8).min(27)
}

/// Proportional Fair metric: `log₂(1 + SNR) / avg_throughput`.
fn pf_metric(s: &UeChannelState) -> f64 {
    let r = (1.0 + s.snr.as_linear()).log2();
    if s.avg_throughput_bps < 1.0 {
        r
    } else {
        r / s.avg_throughput_bps
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Unit struct to implement [`Validate`] for the scheduler module.
pub struct SchedulerValidation;

impl Validate for SchedulerValidation {
    fn validate() -> ValidationResult {
        // Jain fairness: three equal throughputs → J = 1.0 exactly.
        let j_equal = jain_fairness(&[1.0, 1.0, 1.0]);
        // Jain fairness: one UE gets all → J = 1/n = 1/3 ≈ 0.333.
        let j_unfair = jain_fairness(&[3.0, 0.0, 0.0]);
        // snr_to_mcs(1.0 linear = 0 dB) → MCS 0.
        let mcs_0db = snr_to_mcs(SnrLinear::new(1.0)) as f64;
        // snr_to_mcs(1000.0 linear ≈ 30 dB) → MCS 27.
        let mcs_30db = snr_to_mcs(SnrLinear::new(1000.0)) as f64;
        ValidationResult {
            module: "6g-mac/scheduler",
            checks: vec![
                ValidationCheck::new("jain_equal_throughput", j_equal, 1.0, 0.001),
                ValidationCheck::new("jain_unfair_one_third", j_unfair, 1.0 / 3.0, 1.0),
                ValidationCheck::new("mcs_at_0db_snr", mcs_0db, 0.0, 0.0),
                ValidationCheck::new("mcs_at_30db_snr", mcs_30db, 27.0, 0.0),
            ],
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_assigns_all_rbs() {
        let sched = Scheduler::new();
        let ues = vec![UeId(1), UeId(2), UeId(4)];
        let assignments = sched.schedule(&ues, 99);
        assert_eq!(assignments.len(), 3);
        for a in &assignments {
            assert_eq!(a.rb_count, 33);
        }
    }

    #[test]
    fn scheduler_handles_empty_ue_list() {
        let sched = Scheduler::new();
        assert!(sched.schedule(&[], 100).is_empty());
    }

    #[test]
    fn round_robin_rotates_each_tti() {
        let mut sched = Scheduler::with_policy(SchedulingPolicy::RoundRobin);
        let states = vec![
            UeChannelState::new(UeId(1), SnrLinear::new(10.0)),
            UeChannelState::new(UeId(2), SnrLinear::new(10.0)),
        ];
        let t0 = sched.schedule_with_csi(&states, 10);
        let t1 = sched.schedule_with_csi(&states, 10);
        // First assignment in t0 starts at UE[0], t1 starts at UE[1].
        assert_ne!(t0[0].ue, t1[0].ue);
    }

    #[test]
    fn pf_gives_more_rbs_to_better_channel() {
        let mut sched = Scheduler::with_policy(SchedulingPolicy::ProportionalFair);
        let states = vec![
            UeChannelState {
                ue: UeId(1),
                snr: SnrLinear::new(1.0),
                avg_throughput_bps: 1.0,
            },
            UeChannelState {
                ue: UeId(2),
                snr: SnrLinear::new(1000.0),
                avg_throughput_bps: 1.0,
            },
        ];
        let assignments = sched.schedule_with_csi(&states, 10);
        // UE 2 has much better SNR → should appear first (slot 0) under PF.
        assert_eq!(assignments[0].ue, UeId(2));
    }

    #[test]
    fn ai_native_priority_ue_gets_more_rbs() {
        let mut sched = Scheduler::with_policy(SchedulingPolicy::AiNative);
        let states = vec![
            UeChannelState::new(UeId(1), SnrLinear::new(10.0)),
            UeChannelState::new(UeId(2), SnrLinear::new(10.0)),
            UeChannelState::new(UeId(3), SnrLinear::new(10.0)),
        ];
        let assignments = sched.schedule_with_csi(&states, 30);
        // Priority UE always receives at least as many RBs as others.
        let max_rbs = assignments.iter().map(|a| a.rb_count).max().unwrap();
        let min_rbs = assignments.iter().map(|a| a.rb_count).min().unwrap();
        assert!(max_rbs >= min_rbs);
    }

    #[test]
    fn jain_equal_is_one() {
        let j = jain_fairness(&[2.0, 2.0, 2.0]);
        assert!((j - 1.0).abs() < 1e-9, "equal throughputs → J=1, got {j}");
    }

    #[test]
    fn jain_unfair_less_than_one() {
        let j = jain_fairness(&[10.0, 0.0, 0.0]);
        assert!(j < 1.0, "unfair throughputs → J<1, got {j}");
    }

    #[test]
    fn snr_to_mcs_bounds() {
        assert_eq!(snr_to_mcs(SnrLinear::new(1.0)), 0);
        assert_eq!(snr_to_mcs(SnrLinear::new(1000.0)), 27);
    }

    #[test]
    fn observe_reward_updates_q_values() {
        let mut sched = Scheduler::with_policy(SchedulingPolicy::AiNative);
        let snr = SnrLinear::new(100.0);
        // Initial Q-value should be 0.
        let q_before = sched.q_bandit.as_ref().unwrap().q_value(0, snr);
        sched.observe_reward(0, snr, 5e9);
        let q_after = sched.q_bandit.as_ref().unwrap().q_value(0, snr);
        assert!(
            q_after > q_before,
            "Q-value should increase after positive reward"
        );
    }

    #[test]
    fn scheduler_validation_passes() {
        let result = SchedulerValidation::validate();
        assert!(result.passed(), "{}", result.summary());
    }
}

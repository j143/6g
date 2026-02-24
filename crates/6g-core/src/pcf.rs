//! Policy Control Function (PCF).
//!
//! The PCF provides network policies to other NFs:
//! * QoS policies per PDU session / service data flow
//! * Network slice admission control
//! * Charging rules

/// QoS class identifier (simplified).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Qci(pub u8);

/// A QoS policy rule.
#[derive(Debug, Clone)]
pub struct QosPolicy {
    pub qci: Qci,
    /// Guaranteed Bit Rate in kbps (0 = non-GBR).
    pub gbr_kbps: u64,
    /// Maximum Bit Rate in kbps.
    pub max_br_kbps: u64,
    /// Packet Delay Budget in milliseconds.
    pub delay_budget_ms: u32,
}

impl QosPolicy {
    /// Standard URLLC policy: low latency, guaranteed rate.
    pub fn urllc() -> Self {
        Self {
            qci: Qci(80),
            gbr_kbps: 100_000,
            max_br_kbps: 1_000_000,
            delay_budget_ms: 1,
        }
    }

    /// Enhanced Mobile Broadband policy.
    pub fn embb() -> Self {
        Self {
            qci: Qci(9),
            gbr_kbps: 0,
            max_br_kbps: 100_000_000,
            delay_budget_ms: 100,
        }
    }
}

/// Policy Control Function.
pub struct Pcf {
    policies: Vec<QosPolicy>,
}

impl Pcf {
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
        }
    }

    pub fn add_policy(&mut self, policy: QosPolicy) {
        self.policies.push(policy);
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }
}

impl Default for Pcf {
    fn default() -> Self {
        Self::new()
    }
}

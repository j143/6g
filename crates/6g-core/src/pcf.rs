//! Policy Control Function (PCF).
//!
//! The PCF provides network policies to other NFs:
//! * QoS policies per PDU session / service data flow
//! * Network slice admission control
//! * Charging rules

use sixg_common::types::Bitrate;

use crate::nssf::SliceType;

/// QoS class identifier — maps to 3GPP TS 23.501 Table 5.7.4-1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Qci(pub u8);

/// A QoS policy rule.
#[derive(Debug, Clone)]
pub struct QosPolicy {
    /// QoS class identifier (3GPP TS 23.501 Table 5.7.4-1).
    pub qci: Qci,
    /// Guaranteed Bit Rate (0 for non-GBR bearers).
    pub gbr: Bitrate,
    /// Maximum Bit Rate.
    pub max_br: Bitrate,
    /// Packet Delay Budget in milliseconds.
    pub delay_budget_ms: u32,
    /// Network slice this policy is bound to (`None` = generic / unbound).
    pub slice_type: Option<SliceType>,
}

impl QosPolicy {
    /// Standard URLLC policy: QCI 80, low latency, guaranteed rate.
    ///
    /// QCI 80 per 3GPP TS 23.501 Table 5.7.4-1: GBR = 100 Mbps,
    /// max-BR = 1 Gbps, delay budget = 1 ms.
    pub fn urllc() -> Self {
        Self {
            qci: Qci(80),
            gbr: Bitrate::from_mbps(100.0),
            max_br: Bitrate::from_gbps(1.0),
            delay_budget_ms: 1,
            slice_type: Some(SliceType::Urllc),
        }
    }

    /// Enhanced Mobile Broadband policy: QCI 9, non-GBR, 100 Gbps max.
    ///
    /// QCI 9 per 3GPP TS 23.501 Table 5.7.4-1: non-GBR (gbr = 0),
    /// max-BR = 100 Gbps, delay budget = 100 ms.
    pub fn embb() -> Self {
        Self {
            qci: Qci(9),
            gbr: Bitrate::from_bps(0.0),
            max_br: Bitrate::from_gbps(100.0),
            delay_budget_ms: 100,
            slice_type: Some(SliceType::EMbb),
        }
    }

    /// Return the default policy for a given slice type.
    ///
    /// Maps each [`SliceType`] to a sensible QCI and rate profile:
    /// - `Urllc`      → QCI 80, 100 Mbps GBR, 1 ms delay budget  
    /// - `EMbb`       → QCI 9,  non-GBR, 100 Gbps max, 100 ms  
    /// - `MMtc`       → QCI 70, 1 kbps GBR, 1 000 ms delay budget  
    /// - `Sensing`    → QCI 65, 10 Mbps GBR, 5 ms delay budget  
    /// - `NtnBackhaul`→ QCI 5,  non-GBR, 10 Gbps max, 600 ms
    pub fn for_slice(slice: SliceType) -> Self {
        match slice {
            SliceType::Urllc => Self::urllc(),
            SliceType::EMbb => Self::embb(),
            SliceType::MMtc => Self {
                qci: Qci(70),
                gbr: Bitrate::from_kbps(1.0),
                max_br: Bitrate::from_mbps(1.0),
                delay_budget_ms: 1_000,
                slice_type: Some(SliceType::MMtc),
            },
            SliceType::Sensing => Self {
                qci: Qci(65),
                gbr: Bitrate::from_mbps(10.0),
                max_br: Bitrate::from_mbps(500.0),
                delay_budget_ms: 5,
                slice_type: Some(SliceType::Sensing),
            },
            SliceType::NtnBackhaul => Self {
                qci: Qci(5),
                gbr: Bitrate::from_bps(0.0),
                max_br: Bitrate::from_gbps(10.0),
                delay_budget_ms: 600,
                slice_type: Some(SliceType::NtnBackhaul),
            },
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

    /// Add a QoS policy rule.
    pub fn add_policy(&mut self, policy: QosPolicy) {
        self.policies.push(policy);
    }

    /// Number of policies currently registered.
    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    /// Return the policy bound to a specific slice type, if any.
    pub fn policy_for_slice(&self, slice: SliceType) -> Option<&QosPolicy> {
        self.policies.iter().find(|p| p.slice_type == Some(slice))
    }
}

impl Default for Pcf {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// QCI 80 maps to URLLC per 3GPP TS 23.501 Table 5.7.4-1.
    #[test]
    fn urllc_qci_matches_3gpp_table() {
        let p = QosPolicy::urllc();
        assert_eq!(p.qci, Qci(80), "URLLC must be QCI 80");
        assert_eq!(p.delay_budget_ms, 1, "URLLC delay budget must be 1 ms");
        assert!(
            p.gbr.as_mbps() > 0.0,
            "URLLC must have a non-zero guaranteed bit rate"
        );
    }

    /// QCI 9 maps to eMBB (non-GBR) per 3GPP TS 23.501 Table 5.7.4-1.
    #[test]
    fn embb_qci_matches_3gpp_table() {
        let p = QosPolicy::embb();
        assert_eq!(p.qci, Qci(9), "eMBB must be QCI 9");
        assert_eq!(p.delay_budget_ms, 100);
        assert_eq!(p.gbr.as_bps(), 0.0, "eMBB is non-GBR: gbr must be 0");
    }

    #[test]
    fn urllc_max_br_greater_than_gbr() {
        let p = QosPolicy::urllc();
        assert!(p.max_br.as_bps() > p.gbr.as_bps(), "max-BR must exceed GBR");
    }

    #[test]
    fn policy_count_increments_on_add() {
        let mut pcf = Pcf::new();
        assert_eq!(pcf.policy_count(), 0);
        pcf.add_policy(QosPolicy::urllc());
        assert_eq!(pcf.policy_count(), 1);
        pcf.add_policy(QosPolicy::embb());
        assert_eq!(pcf.policy_count(), 2);
    }
}

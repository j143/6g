//! Network Repository Function (NRF).
//!
//! The NRF provides NF discovery and registration services in the 5G/6G SBA:
//! * NFs register their profile (type, address, capacity) on startup.
//! * Consumer NFs query `discover(nf_type)` to locate provider endpoints.
//! * NFs call `deregister(instance_id)` on shutdown (record retained for audit).
//!
//! Reference: 3GPP TS 29.510 (Nnrf_NFDiscovery service).

use std::collections::HashMap;

use sixg_common::types::NodeId;
use sixg_common::validation::{Validate, ValidationCheck, ValidationResult};

/// NF type identifiers per 3GPP TS 29.510 §6.1.6.3.3 `NfType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NfType {
    /// Access and Mobility Management Function.
    Amf,
    /// Session Management Function.
    Smf,
    /// User Plane Function.
    Upf,
    /// Policy Control Function.
    Pcf,
    /// Network Slice Selection Function.
    Nssf,
    /// Authentication Server Function.
    Ausf,
    /// User Data Management function.
    Udm,
    /// Network Exposure Function.
    Nef,
    /// gNB / RAN node.
    Gnb,
}

/// An NF instance profile registered in the NRF.
#[derive(Debug, Clone)]
pub struct NfProfile {
    /// Unique NF instance identifier.
    pub instance_id: NodeId,
    /// NF type used for discovery queries.
    pub nf_type: NfType,
    /// Nominal capacity score 1–100 (higher → preferred by discovery).
    pub capacity: u8,
    /// `true` while the NF is active; `false` after deregistration.
    pub active: bool,
}

impl NfProfile {
    /// Create a new active NF profile.
    ///
    /// `capacity` is clamped to 100 if larger.
    pub fn new(instance_id: NodeId, nf_type: NfType, capacity: u8) -> Self {
        Self {
            instance_id,
            nf_type,
            capacity: capacity.min(100),
            active: true,
        }
    }
}

/// Network Repository Function — NF registration and discovery.
pub struct Nrf {
    profiles: HashMap<NodeId, NfProfile>,
}

impl Nrf {
    /// Create an empty NRF.
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
        }
    }

    /// Register (or update) an NF profile.
    ///
    /// Returns `true` if this is a new registration, `false` if an existing
    /// profile was updated in place.
    pub fn register(&mut self, profile: NfProfile) -> bool {
        self.profiles.insert(profile.instance_id, profile).is_none()
    }

    /// Deregister an NF instance (marks `active = false`; record is retained).
    ///
    /// Returns `true` if the instance was found and deactivated.
    pub fn deregister(&mut self, instance_id: NodeId) -> bool {
        if let Some(p) = self.profiles.get_mut(&instance_id) {
            p.active = false;
            true
        } else {
            false
        }
    }

    /// Discover all active NF instances of `nf_type`, ordered by capacity descending.
    pub fn discover(&self, nf_type: NfType) -> Vec<&NfProfile> {
        let mut matches: Vec<&NfProfile> = self
            .profiles
            .values()
            .filter(|p| p.nf_type == nf_type && p.active)
            .collect();
        matches.sort_by(|a, b| b.capacity.cmp(&a.capacity));
        matches
    }

    /// Number of currently active NF registrations.
    pub fn active_count(&self) -> usize {
        self.profiles.values().filter(|p| p.active).count()
    }

    /// Total registrations (active + deregistered).
    pub fn total_count(&self) -> usize {
        self.profiles.len()
    }
}

impl Default for Nrf {
    fn default() -> Self {
        Self::new()
    }
}

/// Numerical validation for the NRF registration and discovery logic.
pub struct NrfValidation;

impl Validate for NrfValidation {
    fn validate() -> ValidationResult {
        let mut nrf = Nrf::new();

        // Register three UPF instances with different capacities.
        nrf.register(NfProfile::new(NodeId(1), NfType::Upf, 30));
        nrf.register(NfProfile::new(NodeId(2), NfType::Upf, 90));
        nrf.register(NfProfile::new(NodeId(3), NfType::Upf, 60));

        let found_before = nrf.discover(NfType::Upf);
        let order_ok = found_before.len() == 3 && found_before[0].capacity == 90;

        // Deregister the highest-capacity instance.
        nrf.deregister(NodeId(2));
        let found_after = nrf.discover(NfType::Upf);
        let deregister_ok = found_after.len() == 2 && nrf.total_count() == 3;

        ValidationResult {
            module: "nrf",
            checks: vec![
                ValidationCheck::new(
                    "discovery_ordered_by_capacity_desc",
                    if order_ok { 1.0 } else { 0.0 },
                    1.0,
                    0.0,
                ),
                ValidationCheck::new(
                    "deregister_removes_from_discovery",
                    if deregister_ok { 1.0 } else { 0.0 },
                    1.0,
                    0.0,
                ),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sixg_common::types::NodeId;

    #[test]
    fn register_and_discover_nf() {
        let mut nrf = Nrf::new();
        assert!(nrf.register(NfProfile::new(NodeId(1), NfType::Smf, 80)));
        assert_eq!(nrf.active_count(), 1);
        let found = nrf.discover(NfType::Smf);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].instance_id, NodeId(1));
    }

    #[test]
    fn deregister_removes_from_discovery_retains_record() {
        let mut nrf = Nrf::new();
        nrf.register(NfProfile::new(NodeId(2), NfType::Amf, 90));
        assert!(nrf.deregister(NodeId(2)));
        assert_eq!(nrf.active_count(), 0);
        assert_eq!(nrf.total_count(), 1, "record must be retained for audit");
        assert!(nrf.discover(NfType::Amf).is_empty());
    }

    #[test]
    fn discovery_orders_by_capacity_descending() {
        let mut nrf = Nrf::new();
        nrf.register(NfProfile::new(NodeId(3), NfType::Upf, 30));
        nrf.register(NfProfile::new(NodeId(4), NfType::Upf, 90));
        nrf.register(NfProfile::new(NodeId(5), NfType::Upf, 60));
        let found = nrf.discover(NfType::Upf);
        assert_eq!(found.len(), 3);
        assert_eq!(found[0].capacity, 90, "highest capacity must come first");
        assert_eq!(found[1].capacity, 60);
        assert_eq!(found[2].capacity, 30);
    }

    #[test]
    fn capacity_is_clamped_to_100() {
        let p = NfProfile::new(NodeId(1), NfType::Amf, 200);
        assert_eq!(p.capacity, 100);
    }

    #[test]
    fn nrf_validation_passes() {
        let result = NrfValidation::validate();
        assert!(result.passed(), "{}", result.summary());
    }
}

//! Network Repository Function (NRF).
//!
//! The NRF provides NF discovery and registration services in the 5G/6G SBA:
//! * NFs register their profile (type, address, capacity, capabilities) on startup.
//! * Consumer NFs query `discover(nf_type)` to locate provider endpoints.
//! * Consumer NFs query `discover_by_capability(cap)` to find NFs that support
//!   a specific 6G capability — replacing the static type-only lookup with a
//!   knowledge-graph-style query.
//! * NFs call `deregister(instance_id)` on shutdown (record retained for audit).
//!
//! ## 6G capability graph
//!
//! In 5G, NFs register by type and consumers query "give me any SMF".  In 6G,
//! NFs register **capabilities** — "I can handle sub-THz sessions", "I support
//! semantic sessions", "I can serve NTN-connected UEs".  Other NFs query
//! "find me an SMF that can handle semantic sessions for UEs in an NTN cell."
//!
//! This is implemented here as a `Vec<NfCapability>` on each `NfProfile`
//! combined with `Nrf::discover_by_capability`, without requiring an external
//! graph library.
//!
//! Reference: 3GPP TS 29.510 (Nnrf_NFDiscovery service); Nokia Bell Labs
//! *6G SBA as a Knowledge Graph*, 2022.

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
    /// **6G-new**: Sensing Data Function (ISAC-to-core bridge).
    Sdf,
}

/// 6G-specific NF capability labels.
///
/// NFs register capabilities alongside their type so that consumers can
/// perform **capability-based discovery** rather than just type-based.
/// This replaces the 5G static endpoint table with a lightweight knowledge
/// graph: nodes are NF instances, edges are "supports capability X".
///
/// Reference: Nokia Bell Labs *6G SBA as a Knowledge Graph* (2022).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NfCapability {
    /// NF can handle sub-THz (above 100 GHz) sessions.
    SubTHz,
    /// NF supports non-terrestrial network (LEO/HAPS/UAV) UE contexts.
    NtnHandover,
    /// NF supports 6G semantic (goal-oriented) PDU sessions.
    SemanticSession,
    /// NF can process integrated sensing and communication (ISAC) data.
    IsacProcessing,
    /// NF implements post-quantum cryptographic algorithms (e.g. CRYSTALS-Kyber).
    PostQuantumAuth,
    /// NF supports in-network AI inference (edge compute plane).
    InNetworkAi,
    /// NF can expose sensing results over the SBI northbound (SDF capability).
    SensingExposure,
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
    /// 6G capability labels supported by this NF instance.
    pub capabilities: Vec<NfCapability>,
}

impl NfProfile {
    /// Create a new active NF profile with no capabilities.
    ///
    /// `capacity` is clamped to 100 if larger.
    pub fn new(instance_id: NodeId, nf_type: NfType, capacity: u8) -> Self {
        Self {
            instance_id,
            nf_type,
            capacity: capacity.min(100),
            active: true,
            capabilities: Vec::new(),
        }
    }

    /// Create a new active NF profile with the given capabilities.
    pub fn with_capabilities(
        instance_id: NodeId,
        nf_type: NfType,
        capacity: u8,
        capabilities: Vec<NfCapability>,
    ) -> Self {
        Self {
            instance_id,
            nf_type,
            capacity: capacity.min(100),
            active: true,
            capabilities,
        }
    }

    /// Returns `true` if this profile advertises `capability`.
    pub fn has_capability(&self, capability: NfCapability) -> bool {
        self.capabilities.contains(&capability)
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

    /// **6G capability-based discovery** — find all active NF instances that
    /// advertise `capability`, ordered by capacity descending.
    ///
    /// Example: `nrf.discover_by_capability(NfCapability::SemanticSession)`
    /// returns every active NF (of any type) that declared
    /// `NfCapability::SemanticSession` at registration time.
    pub fn discover_by_capability(&self, capability: NfCapability) -> Vec<&NfProfile> {
        let mut matches: Vec<&NfProfile> = self
            .profiles
            .values()
            .filter(|p| p.active && p.has_capability(capability))
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

        // 6G capability discovery: register an SMF with SemanticSession capability.
        nrf.register(NfProfile::with_capabilities(
            NodeId(10),
            NfType::Smf,
            80,
            vec![NfCapability::SemanticSession, NfCapability::NtnHandover],
        ));
        let semantic_smfs = nrf.discover_by_capability(NfCapability::SemanticSession);
        let cap_discovery_ok = semantic_smfs.len() == 1
            && semantic_smfs[0].instance_id == NodeId(10)
            && semantic_smfs[0].nf_type == NfType::Smf;

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
                ValidationCheck::new(
                    "capability_based_discovery_returns_correct_nf",
                    if cap_discovery_ok { 1.0 } else { 0.0 },
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
    fn capability_based_discovery_finds_correct_nf() {
        let mut nrf = Nrf::new();
        // Register two SMFs: one with SemanticSession, one without.
        nrf.register(NfProfile::with_capabilities(
            NodeId(10),
            NfType::Smf,
            80,
            vec![NfCapability::SemanticSession],
        ));
        nrf.register(NfProfile::new(NodeId(11), NfType::Smf, 70));

        let semantic_smfs = nrf.discover_by_capability(NfCapability::SemanticSession);
        assert_eq!(semantic_smfs.len(), 1);
        assert_eq!(semantic_smfs[0].instance_id, NodeId(10));
    }

    #[test]
    fn capability_discovery_returns_empty_when_none_match() {
        let mut nrf = Nrf::new();
        nrf.register(NfProfile::new(NodeId(1), NfType::Amf, 90));
        assert!(nrf.discover_by_capability(NfCapability::SubTHz).is_empty());
    }

    #[test]
    fn capability_discovery_respects_deregistration() {
        let mut nrf = Nrf::new();
        nrf.register(NfProfile::with_capabilities(
            NodeId(5),
            NfType::Upf,
            75,
            vec![NfCapability::InNetworkAi],
        ));
        assert_eq!(
            nrf.discover_by_capability(NfCapability::InNetworkAi).len(),
            1
        );
        nrf.deregister(NodeId(5));
        assert!(
            nrf.discover_by_capability(NfCapability::InNetworkAi)
                .is_empty(),
            "deregistered NF must not appear in capability discovery"
        );
    }

    #[test]
    fn nrf_validation_passes() {
        let result = NrfValidation::validate();
        assert!(result.passed(), "{}", result.summary());
    }
}

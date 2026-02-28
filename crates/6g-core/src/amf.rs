//! Access and Mobility Management Function (AMF).
//!
//! The AMF is the primary control-plane NF for UE connectivity in 6G.
//! Responsibilities:
//! * NAS (Non-Access Stratum) signalling termination
//! * UE registration and de-registration
//! * Mobility management (tracking area management)
//! * Authentication and authorisation (via AUSF/UDM)
//! * Paging for UEs in Idle/Inactive state
//!
//! ## 6G extension: NTN-aware tracking areas
//!
//! `RegistrationRecord.tracking_area` is replaced by the [`TrackingArea`]
//! enum.  The AMF can now distinguish whether a UE is served by a
//! terrestrial cell or a Non-Terrestrial Network (NTN) node and exposes
//! the upcoming handover deadline so that mobility can be pre-triggered.

use sixg_common::types::{Duration, NodeId, UeId};

/// Identifies whether a UE is currently served by a terrestrial or an NTN cell.
///
/// This is the 6G extension that makes NTN-aware mobility a first-class AMF
/// function, as described in 3GPP TR 38.821 and Nokia Bell Labs NTN
/// architecture white paper (2022).
#[derive(Debug, Clone, PartialEq)]
pub enum TrackingArea {
    /// UE is served by a terrestrial base station.
    Terrestrial {
        /// Tracking Area Code (24-bit, 3GPP TS 23.003 §19.4.2.3).
        tac: u32,
        /// Serving cell / NodeB identifier.
        cell_id: NodeId,
    },
    /// UE is served by a Non-Terrestrial Network node (LEO/MEO/GEO/HAPS/UAV).
    Ntn {
        /// NTN node identifier (matches [`sixg_ntn::NtnNode::id`]).
        ntn_node_id: u64,
        /// Beam (spot-beam) identifier within the NTN node.
        beam_id: u32,
        /// One-way propagation delay to the NTN node.
        propagation_delay: Duration,
    },
}

impl TrackingArea {
    /// Returns `true` if this is an NTN-served tracking area.
    pub fn is_ntn(&self) -> bool {
        matches!(self, Self::Ntn { .. })
    }
}

/// AMF-maintained UE registration record.
#[derive(Debug, Clone)]
pub struct RegistrationRecord {
    pub ue: UeId,
    pub tracking_area: TrackingArea,
    pub authenticated: bool,
}

/// The Access and Mobility Management Function.
pub struct Amf {
    registrations: Vec<RegistrationRecord>,
}

impl Amf {
    pub fn new() -> Self {
        Self {
            registrations: Vec::new(),
        }
    }

    /// Register a UE in the given tracking area.
    pub fn register(&mut self, ue: UeId, tracking_area: TrackingArea) {
        self.registrations.push(RegistrationRecord {
            ue,
            tracking_area,
            authenticated: false,
        });
    }

    /// Register a UE using a simple numeric TAC (terrestrial, backward-compatible).
    ///
    /// Convenience wrapper: creates a [`TrackingArea::Terrestrial`] with
    /// `cell_id = NodeId(0)`.  Existing call sites that pass a bare `u32` TAC
    /// can use this to avoid breaking changes.
    pub fn register_terrestrial(&mut self, ue: UeId, tac: u32) {
        self.register(
            ue,
            TrackingArea::Terrestrial {
                tac,
                cell_id: NodeId(0),
            },
        );
    }

    /// Mark a UE as authenticated.
    pub fn authenticate(&mut self, ue: UeId) {
        if let Some(rec) = self.registrations.iter_mut().find(|r| r.ue == ue) {
            rec.authenticated = true;
        }
    }

    /// Deregister a UE — removes its registration record from the AMF.
    ///
    /// Returns `true` if the UE was found and removed, `false` if unknown.
    pub fn deregister(&mut self, ue: UeId) -> bool {
        if let Some(pos) = self.registrations.iter().position(|r| r.ue == ue) {
            self.registrations.remove(pos);
            true
        } else {
            false
        }
    }

    /// Page a UE — returns `true` if the UE has a registration record.
    ///
    /// A real AMF would send a paging message over N2 to all cells in the UE's
    /// tracking area.  Here we return whether the UE is currently registered,
    /// which is the precondition for any paging attempt.
    pub fn page_ue(&self, ue: UeId) -> bool {
        self.registrations.iter().any(|r| r.ue == ue)
    }

    /// Return `true` if the UE has an active (authenticated) registration.
    pub fn is_registered(&self, ue: UeId) -> bool {
        self.registrations
            .iter()
            .any(|r| r.ue == ue && r.authenticated)
    }

    /// Return the [`TrackingArea`] for a registered UE, if any.
    pub fn tracking_area(&self, ue: UeId) -> Option<&TrackingArea> {
        self.registrations
            .iter()
            .find(|r| r.ue == ue)
            .map(|r| &r.tracking_area)
    }

    /// Count UEs currently served via NTN nodes.
    pub fn ntn_ue_count(&self) -> usize {
        self.registrations
            .iter()
            .filter(|r| r.tracking_area.is_ntn())
            .count()
    }

    pub fn registered_ue_count(&self) -> usize {
        self.registrations.len()
    }

    /// Borrow all registration records — used by `CoreNetwork::push_snapshot()`.
    pub fn registrations(&self) -> &[RegistrationRecord] {
        &self.registrations
    }
}

impl Default for Amf {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terrestrial(tac: u32) -> TrackingArea {
        TrackingArea::Terrestrial {
            tac,
            cell_id: NodeId(1),
        }
    }

    fn ntn(ntn_node_id: u64) -> TrackingArea {
        TrackingArea::Ntn {
            ntn_node_id,
            beam_id: 0,
            propagation_delay: Duration::from_ms(1.83),
        }
    }

    #[test]
    fn register_and_authenticate_ue() {
        let mut amf = Amf::new();
        amf.register(UeId(42), terrestrial(1001));
        assert_eq!(amf.registered_ue_count(), 1);
        amf.authenticate(UeId(42));
        assert!(amf.registrations[0].authenticated);
    }

    #[test]
    fn register_terrestrial_convenience() {
        let mut amf = Amf::new();
        amf.register_terrestrial(UeId(1), 2000);
        let ta = amf.tracking_area(UeId(1)).unwrap();
        assert!(!ta.is_ntn(), "must be terrestrial");
    }

    #[test]
    fn ntn_tracking_area_is_identified() {
        let mut amf = Amf::new();
        amf.register(UeId(5), ntn(99));
        assert!(amf.tracking_area(UeId(5)).unwrap().is_ntn());
        assert_eq!(amf.ntn_ue_count(), 1);
    }

    #[test]
    fn ntn_ue_count_excludes_terrestrial() {
        let mut amf = Amf::new();
        amf.register(UeId(1), terrestrial(1));
        amf.register(UeId(2), ntn(10));
        amf.register(UeId(3), ntn(11));
        assert_eq!(amf.ntn_ue_count(), 2);
        assert_eq!(amf.registered_ue_count(), 3);
    }

    #[test]
    fn deregister_removes_ue() {
        let mut amf = Amf::new();
        amf.register(UeId(1), terrestrial(1001));
        amf.authenticate(UeId(1));
        assert!(amf.is_registered(UeId(1)));
        assert!(amf.deregister(UeId(1)));
        assert_eq!(amf.registered_ue_count(), 0);
        assert!(!amf.is_registered(UeId(1)));
    }

    #[test]
    fn page_ue_returns_true_for_known_ue() {
        let mut amf = Amf::new();
        amf.register(UeId(5), terrestrial(2000));
        assert!(amf.page_ue(UeId(5)));
        assert!(!amf.page_ue(UeId(99)));
    }

    #[test]
    fn deregister_unknown_ue_returns_false() {
        let mut amf = Amf::new();
        assert!(!amf.deregister(UeId(42)));
    }
}

//! Access and Mobility Management Function (AMF).
//!
//! The AMF is the primary control-plane NF for UE connectivity in 6G.
//! Responsibilities:
//! * NAS (Non-Access Stratum) signalling termination
//! * UE registration and de-registration
//! * Mobility management (tracking area management)
//! * Authentication and authorisation (via AUSF/UDM)
//! * Paging for UEs in Idle/Inactive state

use sixg_common::types::UeId;

/// AMF-maintained UE registration record.
#[derive(Debug, Clone)]
pub struct RegistrationRecord {
    pub ue: UeId,
    pub tracking_area: u32,
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
    pub fn register(&mut self, ue: UeId, tracking_area: u32) {
        self.registrations.push(RegistrationRecord {
            ue,
            tracking_area,
            authenticated: false,
        });
    }

    /// Mark a UE as authenticated.
    ///
    /// Replaces the existing [`RegistrationRecord`] with a new one that has
    /// `authenticated = true`, preserving the invariant that a record is never
    /// mutated in place after creation.
    pub fn authenticate(&mut self, ue: UeId) {
        if let Some(pos) = self.registrations.iter().position(|r| r.ue == ue) {
            let old = &self.registrations[pos];
            let updated = RegistrationRecord {
                ue: old.ue,
                tracking_area: old.tracking_area,
                authenticated: true,
            };
            self.registrations[pos] = updated;
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

    #[test]
    fn register_and_authenticate_ue() {
        let mut amf = Amf::new();
        amf.register(UeId(42), 1001);
        assert_eq!(amf.registered_ue_count(), 1);
        amf.authenticate(UeId(42));
        assert!(amf.registrations[0].authenticated);
    }

    #[test]
    fn deregister_removes_ue() {
        let mut amf = Amf::new();
        amf.register(UeId(1), 1001);
        amf.authenticate(UeId(1));
        assert!(amf.is_registered(UeId(1)));
        assert!(amf.deregister(UeId(1)));
        assert_eq!(amf.registered_ue_count(), 0);
        assert!(!amf.is_registered(UeId(1)));
    }

    #[test]
    fn page_ue_returns_true_for_known_ue() {
        let mut amf = Amf::new();
        amf.register(UeId(5), 2000);
        assert!(amf.page_ue(UeId(5)));
        assert!(!amf.page_ue(UeId(99)));
    }

    #[test]
    fn deregister_unknown_ue_returns_false() {
        let mut amf = Amf::new();
        assert!(!amf.deregister(UeId(42)));
    }
}

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
    pub fn authenticate(&mut self, ue: UeId) {
        if let Some(r) = self.registrations.iter_mut().find(|r| r.ue == ue) {
            r.authenticated = true;
        }
    }

    pub fn registered_ue_count(&self) -> usize {
        self.registrations.len()
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
}

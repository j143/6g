//! Service-Based Architecture v2 (SBAv2) — flat inline-authentication registry.
//!
//! **Research hypothesis:** replace the 5G NAS multi-message registration exchange
//! (≥ 4 round trips: Registration Request → Authentication → Security Mode Command →
//! Registration Accept) with a **single round trip** where the UE embeds a
//! pre-provisioned [`ServiceToken`] in its first data-path PDU.
//!
//! Flatter hierarchy vs 5GC:
//! - No AMF ↔ AUSF ↔ UDM authentication chain.
//! - No separate NAS Security Mode Command.
//! - Core validates the token inline and grants service in the same exchange.
//!
//! Reference: Qualcomm, *Rethinking the Control Plane* (6G Foundry Series).

use sixg_common::types::UeId;
use sixg_common::validation::{Validate, ValidationCheck, ValidationResult};

/// A pre-provisioned 16-byte service token used for inline authentication.
///
/// Replaces the multi-message NAS authentication chain.  In a real deployment
/// this would be a cryptographically derived credential (e.g. AKMA token from
/// 3GPP TS 33.535); here it is a deterministic function of the UE identifier
/// sufficient for experiment-level validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceToken(pub [u8; 16]);

impl ServiceToken {
    /// Derive a token deterministically from a UE identifier.
    ///
    /// The UE id is written into the first 8 bytes; the remaining bytes are
    /// zero.  This is not cryptographically secure — it exists to give each
    /// UE a unique, reproducible credential for simulation purposes.
    pub fn from_ue_id(ue: UeId) -> Self {
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&ue.0.to_le_bytes());
        Self(bytes)
    }
}

/// A registration record in the SBAv2 flat registry.
#[derive(Debug, Clone)]
pub struct SbaRegistration {
    /// The registering UE.
    pub ue: UeId,
    /// Token presented by the UE.
    pub token: ServiceToken,
    /// `true` when the presented token matched the expected credential.
    pub token_validated: bool,
}

/// SBAv2 flat registry — replaces the AMF/AUSF/UDM chain with inline auth.
///
/// Round-trip comparison:
/// - **5G NAS**: ≥ 4 round trips (Registration Request, Authentication Challenge,
///   Security Mode Command, Registration Accept).
/// - **SBAv2**: 1 round trip (token embedded in first data PDU → service grant).
pub struct SbaV2Registry {
    registrations: Vec<SbaRegistration>,
}

impl SbaV2Registry {
    /// Create an empty SBAv2 registry.
    pub fn new() -> Self {
        Self {
            registrations: Vec::new(),
        }
    }

    /// Attempt to register a UE using inline token authentication.
    ///
    /// Returns `true` when the presented token is valid and service is granted.
    /// The registration record is stored regardless of outcome so that failed
    /// attempts can be audited.
    pub fn register_with_token(&mut self, ue: UeId, token: ServiceToken) -> bool {
        let expected = ServiceToken::from_ue_id(ue);
        let valid = token == expected;
        self.registrations.push(SbaRegistration {
            ue,
            token,
            token_validated: valid,
        });
        valid
    }

    /// Number of UEs that have passed inline token validation.
    pub fn validated_ue_count(&self) -> usize {
        self.registrations
            .iter()
            .filter(|r| r.token_validated)
            .count()
    }

    /// Total number of registration attempts (including rejected ones).
    pub fn registration_count(&self) -> usize {
        self.registrations.len()
    }
}

impl Default for SbaV2Registry {
    fn default() -> Self {
        Self::new()
    }
}

/// Numerical validation for the SBAv2 inline authentication logic.
pub struct SbaV2Validation;

impl Validate for SbaV2Validation {
    fn validate() -> ValidationResult {
        let mut registry = SbaV2Registry::new();

        // Valid token — should be accepted.
        let ue_good = UeId(1);
        let good_token = ServiceToken::from_ue_id(ue_good);
        let accepted = registry.register_with_token(ue_good, good_token);

        // Invalid token — should be rejected.
        let bad_token = ServiceToken([0xFF; 16]);
        let rejected = !registry.register_with_token(UeId(2), bad_token);

        ValidationResult {
            module: "sba_v2",
            checks: vec![
                ValidationCheck::new(
                    "valid_token_accepted",
                    if accepted { 1.0 } else { 0.0 },
                    1.0,
                    0.0,
                ),
                ValidationCheck::new(
                    "invalid_token_rejected",
                    if rejected { 1.0 } else { 0.0 },
                    1.0,
                    0.0,
                ),
                ValidationCheck::new(
                    "validated_ue_count_equals_one",
                    registry.validated_ue_count() as f64,
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

    #[test]
    fn inline_token_auth_accepted() {
        let mut registry = SbaV2Registry::new();
        let ue = UeId(42);
        let token = ServiceToken::from_ue_id(ue);
        assert!(registry.register_with_token(ue, token));
        assert_eq!(registry.validated_ue_count(), 1);
    }

    #[test]
    fn wrong_token_rejected() {
        let mut registry = SbaV2Registry::new();
        let ue = UeId(42);
        let bad = ServiceToken([0xAB; 16]);
        assert!(!registry.register_with_token(ue, bad));
        assert_eq!(registry.validated_ue_count(), 0);
    }

    #[test]
    fn registration_count_includes_rejected_attempts() {
        let mut registry = SbaV2Registry::new();
        let ue = UeId(1);
        registry.register_with_token(ue, ServiceToken::from_ue_id(ue)); // valid
        registry.register_with_token(UeId(2), ServiceToken([0x00; 16])); // invalid
        assert_eq!(registry.registration_count(), 2);
        assert_eq!(registry.validated_ue_count(), 1);
    }

    #[test]
    fn sba_v2_validation_passes() {
        let result = SbaV2Validation::validate();
        assert!(result.passed(), "{}", result.summary());
    }
}

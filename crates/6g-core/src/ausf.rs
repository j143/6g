//! Authentication Server Function (AUSF) and User Data Management (UDM).
//!
//! In 5G/6G SA, the AMF delegates authentication to the AUSF, which fetches
//! authentication vectors from the UDM's subscriber database.
//!
//! * [`Udm`] — subscriber credential store (SUPI → long-term key K).
//! * [`Ausf`] — generates and verifies 5G-AKA authentication vectors.
//!
//! Authentication flow (3GPP TS 33.501 §6.1):
//! 1. AMF calls `Ausf::initiate_auth(ue)` → AUSF fetches K from UDM, derives
//!    an auth vector, and returns `RAND` (the challenge sent to the UE).
//! 2. UE computes `RES* = K XOR RAND` and sends it back.
//! 3. AMF calls `Ausf::verify_response(ue, res)` → `true` on match.
//!
//! The XRES derivation (`K XOR RAND`) is a conceptual stand-in for the full
//! MILENAGE/TUAK function defined in 3GPP TS 35.205.

use std::collections::HashMap;

use sixg_common::types::UeId;
use sixg_common::validation::{Validate, ValidationCheck, ValidationResult};

/// A subscriber's long-term credentials stored in the UDM.
#[derive(Debug, Clone)]
pub struct SubscriberCredential {
    /// Subscriber unique permanent identifier.
    pub ue: UeId,
    /// 128-bit long-term key K (16 bytes).
    pub k: [u8; 16],
}

impl SubscriberCredential {
    /// Create a new credential with a deterministic key derived from `ue`.
    ///
    /// Key layout: bytes 0–7 = `ue_id` in little-endian; bytes 8–15 = bitwise
    /// complement of `ue_id` in little-endian.
    ///
    /// # ⚠ Simulation only
    /// This derivation is **not** cryptographically secure.  It exists for
    /// reproducible simulation results only.  Do **not** use with real subscriber
    /// data or in production systems.
    pub fn new(ue: UeId) -> Self {
        let mut k = [0u8; 16];
        k[..8].copy_from_slice(&ue.0.to_le_bytes());
        k[8..].copy_from_slice(&(!ue.0).to_le_bytes());
        Self { ue, k }
    }
}

/// Simulation-only offset added to each key byte when deriving RAND.
///
/// In 3GPP TS 33.501 RAND must be a cryptographically random 128-bit value.
/// This deterministic offset exists solely for reproducible simulation results.
const SIMULATION_RAND_OFFSET: u8 = 0x5A;

/// User Data Management function — subscriber credential store.
pub struct Udm {
    subscribers: HashMap<UeId, SubscriberCredential>,
}

impl Udm {
    /// Create an empty UDM.
    pub fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
        }
    }

    /// Provision a subscriber credential into the UDM.
    ///
    /// Returns `true` if this is a new subscriber, `false` if overwritten.
    pub fn provision(&mut self, credential: SubscriberCredential) -> bool {
        self.subscribers.insert(credential.ue, credential).is_none()
    }

    /// Look up the long-term credential for a subscriber SUPI.
    ///
    /// Returns `None` if the SUPI is not provisioned.
    pub fn get_credential(&self, ue: UeId) -> Option<&SubscriberCredential> {
        self.subscribers.get(&ue)
    }

    /// Number of provisioned subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }
}

impl Default for Udm {
    fn default() -> Self {
        Self::new()
    }
}

/// A 5G-AKA authentication vector (simplified).
///
/// In 3GPP TS 33.501 §6.1, a full auth vector contains RAND, AUTN, HXRES\*,
/// and K_AUSF.  Here we use `XRES = K XOR RAND` as a conceptual equivalent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthVector {
    /// Random challenge (16 bytes) sent to the UE.
    pub rand: [u8; 16],
    /// Expected response: `XRES = K XOR RAND`.
    pub xres: [u8; 16],
}

impl AuthVector {
    /// Derive an auth vector from long-term key `k` and random challenge `rand`.
    ///
    /// `xres[i] = k[i] XOR rand[i]` for all 16 bytes.
    ///
    /// # Known-value test
    /// `K = [0x01; 16]`, `RAND = [0x02; 16]` → `XRES = [0x03; 16]`.
    pub fn derive(k: &[u8; 16], rand: &[u8; 16]) -> Self {
        let mut xres = [0u8; 16];
        for i in 0..16 {
            xres[i] = k[i] ^ rand[i];
        }
        Self { rand: *rand, xres }
    }
}

/// Authentication Server Function.
///
/// Mediates between the AMF (which forwards UE responses) and the UDM
/// (which holds subscriber long-term keys).
pub struct Ausf {
    udm: Udm,
    /// Pending authentication vectors keyed by UE identifier.
    pending: HashMap<UeId, AuthVector>,
}

impl Ausf {
    /// Create an AUSF with an empty subscriber store.
    pub fn new() -> Self {
        Self {
            udm: Udm::new(),
            pending: HashMap::new(),
        }
    }

    /// Borrow the underlying UDM mutably for subscriber provisioning.
    pub fn udm_mut(&mut self) -> &mut Udm {
        &mut self.udm
    }

    /// Initiate 5G-AKA authentication for a UE (AMF → AUSF step 1).
    ///
    /// Fetches the subscriber's key K from the UDM, derives a deterministic
    /// RAND, generates the auth vector, stores it as pending, and returns
    /// `rand` (the challenge to forward to the UE).
    ///
    /// Returns `None` if the SUPI is not provisioned in the UDM.
    pub fn initiate_auth(&mut self, ue: UeId) -> Option<[u8; 16]> {
        let cred = self.udm.get_credential(ue)?;
        // Deterministic RAND: apply SIMULATION_RAND_OFFSET to each key byte.
        // In 3GPP TS 33.501 RAND must be a 128-bit cryptographically random value;
        // this offset is used solely for reproducible simulation results.
        let mut rand = [0u8; 16];
        for (i, &b) in cred.k.iter().enumerate() {
            rand[i] = b.wrapping_add(SIMULATION_RAND_OFFSET);
        }
        let av = AuthVector::derive(&cred.k, &rand);
        let challenge = av.rand;
        self.pending.insert(ue, av);
        Some(challenge)
    }

    /// Verify the UE's authentication response (AMF → AUSF step 2).
    ///
    /// Returns `true` if `res` matches the stored expected response (`xres`).
    /// The pending vector is removed regardless of the outcome.
    pub fn verify_response(&mut self, ue: UeId, res: &[u8; 16]) -> bool {
        if let Some(av) = self.pending.remove(&ue) {
            av.xres == *res
        } else {
            false
        }
    }

    /// Number of subscribers provisioned in the UDM.
    pub fn subscriber_count(&self) -> usize {
        self.udm.subscriber_count()
    }
}

impl Default for Ausf {
    fn default() -> Self {
        Self::new()
    }
}

/// Numerical validation for the AUSF/UDM authentication logic.
pub struct AusfValidation;

impl Validate for AusfValidation {
    fn validate() -> ValidationResult {
        // Known-value test: K = [0x01; 16], RAND = [0x02; 16] → XRES = [0x03; 16].
        let k = [0x01u8; 16];
        let rand = [0x02u8; 16];
        let av = AuthVector::derive(&k, &rand);
        let xres_matches = av.xres == [0x03u8; 16];

        // End-to-end auth flow for UeId(1).
        let mut ausf = Ausf::new();
        let ue = UeId(1);
        ausf.udm_mut().provision(SubscriberCredential::new(ue));
        let challenge = ausf.initiate_auth(ue).unwrap();
        let cred = SubscriberCredential::new(ue);
        let mut res = [0u8; 16];
        for i in 0..16 {
            res[i] = cred.k[i] ^ challenge[i];
        }
        let auth_ok = ausf.verify_response(ue, &res);

        // Unknown subscriber must return None.
        let unknown_none = ausf.initiate_auth(UeId(999)).is_none();

        ValidationResult {
            module: "ausf",
            checks: vec![
                ValidationCheck::new(
                    "xres_equals_k_xor_rand",
                    if xres_matches { 1.0 } else { 0.0 },
                    1.0,
                    0.0,
                ),
                ValidationCheck::new(
                    "valid_subscriber_auth_succeeds",
                    if auth_ok { 1.0 } else { 0.0 },
                    1.0,
                    0.0,
                ),
                ValidationCheck::new(
                    "unknown_supi_returns_none",
                    if unknown_none { 1.0 } else { 0.0 },
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

    /// Known-value test: K XOR RAND derivation matches 3GPP TS 33.501 §A.5 concept.
    /// K = [0x01; 16], RAND = [0x02; 16] → XRES = [0x03; 16].
    #[test]
    fn auth_vector_derive_xor_known_value() {
        let k = [0x01u8; 16];
        let rand = [0x02u8; 16];
        let av = AuthVector::derive(&k, &rand);
        assert_eq!(av.xres, [0x03u8; 16], "XRES must equal K XOR RAND");
        assert_eq!(av.rand, rand);
    }

    #[test]
    fn provisioned_subscriber_authenticates_successfully() {
        let mut ausf = Ausf::new();
        let ue = UeId(42);
        ausf.udm_mut().provision(SubscriberCredential::new(ue));
        assert_eq!(ausf.subscriber_count(), 1);

        let rand = ausf
            .initiate_auth(ue)
            .expect("provisioned UE must get a challenge");
        // UE computes RES = K XOR RAND.
        let cred = SubscriberCredential::new(ue);
        let mut res = [0u8; 16];
        for i in 0..16 {
            res[i] = cred.k[i] ^ rand[i];
        }
        assert!(
            ausf.verify_response(ue, &res),
            "correct RES must be accepted"
        );
    }

    #[test]
    fn unknown_subscriber_returns_none() {
        let mut ausf = Ausf::new();
        assert!(ausf.initiate_auth(UeId(99)).is_none());
    }

    #[test]
    fn wrong_response_is_rejected() {
        let mut ausf = Ausf::new();
        let ue = UeId(7);
        ausf.udm_mut().provision(SubscriberCredential::new(ue));
        ausf.initiate_auth(ue).unwrap();
        assert!(!ausf.verify_response(ue, &[0xFFu8; 16]));
    }

    #[test]
    fn ausf_validation_passes() {
        let result = AusfValidation::validate();
        assert!(result.passed(), "{}", result.summary());
    }
}

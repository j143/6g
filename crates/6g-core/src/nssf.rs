//! Network Slice Selection Function (NSSF).
//!
//! Network slicing allows operators to partition the 6G network into
//! independent logical networks ("slices"), each tailored to a specific
//! service type (eMBB, URLLC, mMTC, sensing, etc.).

use serde::{Deserialize, Serialize};

use sixg_common::types::SliceId;

/// Standardised Slice/Service Type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SliceType {
    /// Enhanced Mobile Broadband.
    EMbb,
    /// Ultra-Reliable Low-Latency Communication.
    Urllc,
    /// Massive Machine-Type Communication (IoT).
    MMtc,
    /// Network Sensing slice (6G new).
    Sensing,
    /// Satellite / NTN backhaul slice (6G new).
    NtnBackhaul,
}

/// A network slice descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSlice {
    pub slice_type: SliceType,
    /// Single Network Slice Selection Assistance Information.
    pub s_nssai: u32,
    /// Maximum number of UEs admitted to this slice.
    pub max_ues: usize,
}

impl NetworkSlice {
    /// Return the [`SliceId`] corresponding to this slice's S-NSSAI.
    pub fn slice_id(&self) -> SliceId {
        SliceId(self.s_nssai as u16)
    }
}

/// Network Slice Selection Function.
pub struct NetworkSliceSelector {
    slices: Vec<NetworkSlice>,
    /// Per-slice current admitted UE count keyed by S-NSSAI.
    admitted: std::collections::HashMap<u32, usize>,
}

impl NetworkSliceSelector {
    pub fn new() -> Self {
        // Pre-configure standard slices.
        Self {
            slices: vec![
                NetworkSlice {
                    slice_type: SliceType::EMbb,
                    s_nssai: 1,
                    max_ues: 500_000,
                },
                NetworkSlice {
                    slice_type: SliceType::Urllc,
                    s_nssai: 2,
                    max_ues: 100_000,
                },
                NetworkSlice {
                    slice_type: SliceType::MMtc,
                    s_nssai: 3,
                    max_ues: 1_000_000,
                },
                NetworkSlice {
                    slice_type: SliceType::Sensing,
                    s_nssai: 4,
                    max_ues: 50_000,
                },
            ],
            admitted: std::collections::HashMap::new(),
        }
    }

    /// Create a `NetworkSliceSelector` with a custom slice set.
    ///
    /// Useful for testing admission control with small `max_ues` values.
    pub fn with_slices(slices: Vec<NetworkSlice>) -> Self {
        Self {
            slices,
            admitted: std::collections::HashMap::new(),
        }
    }

    pub fn slice_count(&self) -> usize {
        self.slices.len()
    }

    /// Select a slice for the given service type.
    ///
    /// Returns `None` if the slice type is not configured.
    pub fn select(&self, slice_type: SliceType) -> Option<&NetworkSlice> {
        self.slices.iter().find(|s| s.slice_type == slice_type)
    }

    /// Admit a UE into the given slice (admission control).
    ///
    /// Returns `true` if the slice exists and has capacity (`current < max_ues`),
    /// and increments the per-slice UE count.  Returns `false` if the slice is
    /// unknown or at capacity (load-based rejection).
    pub fn admit_ue(&mut self, slice_type: SliceType) -> bool {
        if let Some(slice) = self.slices.iter().find(|s| s.slice_type == slice_type) {
            let s_nssai = slice.s_nssai;
            let max = slice.max_ues;
            let current = self.admitted.entry(s_nssai).or_insert(0);
            if *current < max {
                *current += 1;
                return true;
            }
        }
        false
    }

    /// Release a UE from the given slice, decrementing the admission counter.
    ///
    /// Returns `true` if the slice exists and had at least one admitted UE.
    pub fn release_ue(&mut self, slice_type: SliceType) -> bool {
        if let Some(slice) = self.slices.iter().find(|s| s.slice_type == slice_type) {
            let s_nssai = slice.s_nssai;
            if let Some(count) = self.admitted.get_mut(&s_nssai) {
                if *count > 0 {
                    *count -= 1;
                    return true;
                }
            }
        }
        false
    }

    /// Current admitted UE count for the given slice type.
    ///
    /// Returns `0` if the slice is unconfigured or has no admitted UEs.
    pub fn admitted_count(&self, slice_type: SliceType) -> usize {
        self.slices
            .iter()
            .find(|s| s.slice_type == slice_type)
            .and_then(|s| self.admitted.get(&s.s_nssai).copied())
            .unwrap_or(0)
    }
}

impl Default for NetworkSliceSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urllc_slice_exists() {
        let nssf = NetworkSliceSelector::new();
        let s = nssf.select(SliceType::Urllc);
        assert!(s.is_some());
        assert_eq!(s.unwrap().s_nssai, 2);
    }

    #[test]
    fn slice_id_matches_s_nssai() {
        let nssf = NetworkSliceSelector::new();
        // EMbb = s_nssai 1, Urllc = 2, MMtc = 3, Sensing = 4
        let cases = [
            (SliceType::EMbb, 1u32),
            (SliceType::Urllc, 2),
            (SliceType::MMtc, 3),
            (SliceType::Sensing, 4),
        ];
        for (slice_type, expected_nssai) in cases {
            let slice = nssf.select(slice_type).expect("slice must exist");
            assert_eq!(
                slice.slice_id(),
                SliceId(expected_nssai as u16),
                "slice_id() must equal SliceId(s_nssai) for {:?}",
                slice_type
            );
        }
    }

    #[test]
    fn admit_and_release_ue_tracks_count() {
        let mut nssf = NetworkSliceSelector::new();
        assert_eq!(nssf.admitted_count(SliceType::EMbb), 0);
        assert!(nssf.admit_ue(SliceType::EMbb));
        assert!(nssf.admit_ue(SliceType::EMbb));
        assert_eq!(nssf.admitted_count(SliceType::EMbb), 2);
        assert!(nssf.release_ue(SliceType::EMbb));
        assert_eq!(nssf.admitted_count(SliceType::EMbb), 1);
    }

    #[test]
    fn admission_control_rejects_at_capacity() {
        // Use a custom selector with max_ues = 2 to test capacity enforcement.
        let mut nssf = NetworkSliceSelector::with_slices(vec![NetworkSlice {
            slice_type: SliceType::Urllc,
            s_nssai: 2,
            max_ues: 2,
        }]);
        assert!(nssf.admit_ue(SliceType::Urllc), "first UE must be admitted");
        assert!(
            nssf.admit_ue(SliceType::Urllc),
            "second UE must be admitted"
        );
        assert!(
            !nssf.admit_ue(SliceType::Urllc),
            "third UE must be rejected — slice is at capacity"
        );
        assert_eq!(nssf.admitted_count(SliceType::Urllc), 2);
    }

    #[test]
    fn admission_control_rejects_unconfigured_slice() {
        let mut nssf = NetworkSliceSelector::new();
        assert!(
            !nssf.admit_ue(SliceType::NtnBackhaul),
            "unconfigured slice must be rejected"
        );
    }

    #[test]
    fn release_without_admission_returns_false() {
        let mut nssf = NetworkSliceSelector::new();
        assert!(
            !nssf.release_ue(SliceType::Urllc),
            "releasing with 0 admitted must return false"
        );
    }
}

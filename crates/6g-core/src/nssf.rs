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
        }
    }

    pub fn slice_count(&self) -> usize {
        self.slices.len()
    }

    /// Select a slice for the given service type.
    pub fn select(&self, slice_type: SliceType) -> Option<&NetworkSlice> {
        self.slices.iter().find(|s| s.slice_type == slice_type)
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
}

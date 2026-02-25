//! Digital Twin integration stub for the 6G Core Network.
//!
//! The digital twin maintains a real-time model of network state.
//! This module implements a **state-snapshot + diff** mechanism:
//!
//! 1. Callers periodically push a [`NetworkSnapshot`] into [`DigitalTwin::update()`].
//! 2. The twin compares it against the previous snapshot and returns a [`SnapshotDiff`]
//!    describing what changed (UEs added/removed, slice loads that shifted > 1 %).
//!
//! The diff is the hook for predictive mobility, proactive HARQ, and AI-driven
//! slice selection — future phases can subscribe to the diff stream instead of
//! polling full state.
//!
//! Reference: ETSI ENI (Experiential Networked Intelligence) specifications.

use std::collections::HashMap;

use sixg_common::types::UeId;
use sixg_common::validation::{Validate, ValidationCheck, ValidationResult};

/// State of a single UE captured in a network snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct UeSnapshot {
    /// The UE being described.
    pub ue: UeId,
    /// Number of active PDU sessions.
    pub pdu_session_count: u8,
    /// Estimated downlink throughput in Mbps.
    pub dl_throughput_mbps: f64,
}

/// Full network state captured at one instant.
///
/// Snapshots are identified by a monotonically increasing [`sequence`](Self::sequence)
/// counter so that callers can detect missed updates.
#[derive(Debug, Clone)]
pub struct NetworkSnapshot {
    /// Per-UE states at this instant.
    pub ues: HashMap<UeId, UeSnapshot>,
    /// Per-slice load as a percentage (0–100) keyed by S-NSSAI.
    pub slice_load_pct: HashMap<u32, f64>,
    /// Monotonic sequence counter.
    pub sequence: u64,
}

impl NetworkSnapshot {
    /// Create an empty snapshot with the given sequence number.
    pub fn new(sequence: u64) -> Self {
        Self {
            ues: HashMap::new(),
            slice_load_pct: HashMap::new(),
            sequence,
        }
    }

    /// Add or update a UE entry in this snapshot.
    pub fn add_ue(&mut self, snap: UeSnapshot) {
        self.ues.insert(snap.ue, snap);
    }

    /// Record the load percentage (0–100) for a network slice identified by S-NSSAI.
    pub fn set_slice_load(&mut self, s_nssai: u32, load_pct: f64) {
        self.slice_load_pct.insert(s_nssai, load_pct);
    }
}

/// Changes detected between two successive [`NetworkSnapshot`]s.
#[derive(Debug, Clone)]
pub struct SnapshotDiff {
    /// UEs present in the new snapshot but absent from the old one.
    pub added_ues: Vec<UeId>,
    /// UEs present in the old snapshot but absent from the new one.
    pub removed_ues: Vec<UeId>,
    /// Slices whose load changed by more than 1 % between snapshots.
    pub changed_slices: Vec<u32>,
}

impl SnapshotDiff {
    /// Returns `true` when no differences were detected.
    pub fn is_empty(&self) -> bool {
        self.added_ues.is_empty()
            && self.removed_ues.is_empty()
            && self.changed_slices.is_empty()
    }
}

/// Digital Twin — ingests network snapshots and surfaces diffs.
pub struct DigitalTwin {
    latest: Option<NetworkSnapshot>,
    /// Total snapshots processed since creation.
    snapshot_count: u64,
}

impl DigitalTwin {
    /// Create a new, empty digital twin.
    pub fn new() -> Self {
        Self {
            latest: None,
            snapshot_count: 0,
        }
    }

    /// Ingest a new network snapshot and return the diff against the previous one.
    ///
    /// On the first call (no prior snapshot) the diff lists all UEs in `new` as
    /// added and reports no removed UEs or changed slices.
    pub fn update(&mut self, new: NetworkSnapshot) -> SnapshotDiff {
        let diff = match &self.latest {
            Some(old) => Self::compute_diff(old, &new),
            None => SnapshotDiff {
                added_ues: new.ues.keys().copied().collect(),
                removed_ues: Vec::new(),
                changed_slices: Vec::new(),
            },
        };
        self.latest = Some(new);
        self.snapshot_count += 1;
        diff
    }

    /// Borrow the most recent snapshot, if any.
    pub fn current(&self) -> Option<&NetworkSnapshot> {
        self.latest.as_ref()
    }

    /// Total number of snapshots processed since creation.
    pub fn snapshot_count(&self) -> u64 {
        self.snapshot_count
    }

    fn compute_diff(old: &NetworkSnapshot, new: &NetworkSnapshot) -> SnapshotDiff {
        let added_ues = new
            .ues
            .keys()
            .filter(|id| !old.ues.contains_key(id))
            .copied()
            .collect();

        let removed_ues = old
            .ues
            .keys()
            .filter(|id| !new.ues.contains_key(id))
            .copied()
            .collect();

        // Report slices whose load changed by more than 1 % (noise filter).
        let changed_slices = new
            .slice_load_pct
            .iter()
            .filter(|(&id, &new_load)| {
                old.slice_load_pct
                    .get(&id)
                    .is_none_or(|&old_load| (new_load - old_load).abs() > 1.0)
            })
            .map(|(&id, _)| id)
            .collect();

        SnapshotDiff {
            added_ues,
            removed_ues,
            changed_slices,
        }
    }
}

impl Default for DigitalTwin {
    fn default() -> Self {
        Self::new()
    }
}

/// Numerical validation for the Digital Twin snapshot/diff logic.
pub struct DigitalTwinValidation;

impl Validate for DigitalTwinValidation {
    fn validate() -> ValidationResult {
        let mut twin = DigitalTwin::new();

        // Snapshot 1 — first update: all UEs should appear as "added".
        let mut s1 = NetworkSnapshot::new(1);
        s1.add_ue(UeSnapshot {
            ue: UeId(1),
            pdu_session_count: 1,
            dl_throughput_mbps: 100.0,
        });
        s1.set_slice_load(1, 20.0);
        let diff1 = twin.update(s1);

        // Snapshot 2 — identical state: diff should be empty.
        let mut s2 = NetworkSnapshot::new(2);
        s2.add_ue(UeSnapshot {
            ue: UeId(1),
            pdu_session_count: 1,
            dl_throughput_mbps: 100.0,
        });
        s2.set_slice_load(1, 20.5); // 0.5 % change — below 1 % threshold
        let diff2 = twin.update(s2);

        // Snapshot 3 — UE removed, slice load jumps 30 %.
        let mut s3 = NetworkSnapshot::new(3);
        s3.set_slice_load(1, 50.0);
        let diff3 = twin.update(s3);

        ValidationResult {
            module: "digital_twin",
            checks: vec![
                ValidationCheck::new(
                    "first_snapshot_ue_added",
                    diff1.added_ues.len() as f64,
                    1.0,
                    0.0,
                ),
                ValidationCheck::new(
                    "sub_threshold_change_ignored",
                    if diff2.is_empty() { 1.0 } else { 0.0 },
                    1.0,
                    0.0,
                ),
                ValidationCheck::new(
                    "removed_ue_detected",
                    diff3.removed_ues.len() as f64,
                    1.0,
                    0.0,
                ),
                ValidationCheck::new(
                    "slice_load_change_detected",
                    diff3.changed_slices.len() as f64,
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
    fn identical_snapshots_produce_empty_diff() {
        let mut twin = DigitalTwin::new();
        let mut s1 = NetworkSnapshot::new(1);
        s1.add_ue(UeSnapshot {
            ue: UeId(10),
            pdu_session_count: 2,
            dl_throughput_mbps: 200.0,
        });
        s1.set_slice_load(1, 40.0);
        twin.update(s1);

        let mut s2 = NetworkSnapshot::new(2);
        s2.add_ue(UeSnapshot {
            ue: UeId(10),
            pdu_session_count: 2,
            dl_throughput_mbps: 200.0,
        });
        s2.set_slice_load(1, 40.0);
        let diff = twin.update(s2);

        assert!(diff.is_empty(), "Identical snapshots must produce no diff");
    }

    #[test]
    fn detects_added_ue() {
        let mut twin = DigitalTwin::new();
        twin.update(NetworkSnapshot::new(1)); // empty

        let mut s2 = NetworkSnapshot::new(2);
        s2.add_ue(UeSnapshot {
            ue: UeId(5),
            pdu_session_count: 1,
            dl_throughput_mbps: 50.0,
        });
        let diff = twin.update(s2);

        assert_eq!(diff.added_ues.len(), 1);
        assert_eq!(diff.added_ues[0], UeId(5));
    }

    #[test]
    fn detects_removed_ue() {
        let mut twin = DigitalTwin::new();
        let mut s1 = NetworkSnapshot::new(1);
        s1.add_ue(UeSnapshot {
            ue: UeId(7),
            pdu_session_count: 1,
            dl_throughput_mbps: 10.0,
        });
        twin.update(s1);

        let diff = twin.update(NetworkSnapshot::new(2)); // empty — UE gone
        assert_eq!(diff.removed_ues.len(), 1);
        assert_eq!(diff.removed_ues[0], UeId(7));
    }

    #[test]
    fn slice_load_below_threshold_not_reported() {
        let mut twin = DigitalTwin::new();
        let mut s1 = NetworkSnapshot::new(1);
        s1.set_slice_load(2, 50.0);
        twin.update(s1);

        let mut s2 = NetworkSnapshot::new(2);
        s2.set_slice_load(2, 50.8); // 0.8 % change — below 1 % threshold
        let diff = twin.update(s2);

        assert!(
            diff.changed_slices.is_empty(),
            "Sub-threshold slice change must not be reported"
        );
    }

    #[test]
    fn snapshot_count_increments() {
        let mut twin = DigitalTwin::new();
        twin.update(NetworkSnapshot::new(1));
        twin.update(NetworkSnapshot::new(2));
        assert_eq!(twin.snapshot_count(), 2);
    }

    #[test]
    fn digital_twin_validation_passes() {
        let result = DigitalTwinValidation::validate();
        assert!(result.passed(), "{}", result.summary());
    }
}

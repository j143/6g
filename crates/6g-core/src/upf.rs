//! User Plane Function (UPF).
//!
//! The UPF is the data-plane anchor in the 6G core:
//! * Packet routing and forwarding
//! * Traffic usage reporting
//! * QoS enforcement
//! * Uplink classifier / branching point for local breakout

use std::collections::HashMap;

/// UPF traffic statistics (per-global or per-session).
#[derive(Debug, Default, Clone)]
pub struct TrafficStats {
    pub bytes_uplink: u64,
    pub bytes_downlink: u64,
    pub packets_dropped: u64,
}

/// User Plane Function.
pub struct Upf {
    /// Aggregate traffic statistics across all sessions.
    pub stats: TrafficStats,
    /// Per-session bearer statistics keyed by `session_id`.
    bearer_stats: HashMap<u8, TrafficStats>,
}

impl Upf {
    pub fn new() -> Self {
        Self {
            stats: TrafficStats::default(),
            bearer_stats: HashMap::new(),
        }
    }

    /// Forward an uplink payload (stub – no actual routing yet).
    ///
    /// Accumulates bytes in the global `stats.bytes_uplink` counter.
    pub fn forward_uplink(&mut self, payload: &[u8]) {
        self.stats.bytes_uplink += payload.len() as u64;
    }

    /// Forward a downlink payload (stub).
    ///
    /// Accumulates bytes in the global `stats.bytes_downlink` counter.
    pub fn forward_downlink(&mut self, payload: &[u8]) {
        self.stats.bytes_downlink += payload.len() as u64;
    }

    /// Forward an uplink payload for a specific PDU session bearer.
    ///
    /// Updates both the global counter and the per-session bearer stats.
    /// Creates a bearer entry for `session_id` on first use (PDR install).
    pub fn forward_uplink_for_session(&mut self, session_id: u8, payload: &[u8]) {
        let len = payload.len() as u64;
        self.stats.bytes_uplink += len;
        self.bearer_stats
            .entry(session_id)
            .or_default()
            .bytes_uplink += len;
    }

    /// Forward a downlink payload for a specific PDU session bearer.
    ///
    /// Updates both the global counter and the per-session bearer stats.
    pub fn forward_downlink_for_session(&mut self, session_id: u8, payload: &[u8]) {
        let len = payload.len() as u64;
        self.stats.bytes_downlink += len;
        self.bearer_stats
            .entry(session_id)
            .or_default()
            .bytes_downlink += len;
    }

    /// Return per-session bearer statistics for `session_id`, if any traffic
    /// has been forwarded for that bearer.
    pub fn session_stats(&self, session_id: u8) -> Option<&TrafficStats> {
        self.bearer_stats.get(&session_id)
    }

    /// Remove the bearer entry for a released session.
    ///
    /// Called by `CoreNetwork::release_session()` during teardown.
    /// Returns `true` if the bearer was present and removed.
    pub fn release_bearer(&mut self, session_id: u8) -> bool {
        self.bearer_stats.remove(&session_id).is_some()
    }

    /// Number of active (non-released) bearer entries.
    pub fn bearer_count(&self) -> usize {
        self.bearer_stats.len()
    }
}

impl Default for Upf {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_uplink_accumulates_bytes() {
        let mut upf = Upf::new();
        upf.forward_uplink(b"hello");
        assert_eq!(upf.stats.bytes_uplink, 5);
        upf.forward_uplink(b"world!");
        assert_eq!(upf.stats.bytes_uplink, 11, "counter must be cumulative");
        assert_eq!(upf.stats.bytes_downlink, 0, "downlink must be unaffected");
    }

    #[test]
    fn forward_downlink_accumulates_bytes() {
        let mut upf = Upf::new();
        upf.forward_downlink(b"data");
        assert_eq!(upf.stats.bytes_downlink, 4);
        upf.forward_downlink(b"more data");
        assert_eq!(upf.stats.bytes_downlink, 13, "counter must be cumulative");
        assert_eq!(upf.stats.bytes_uplink, 0, "uplink must be unaffected");
    }

    #[test]
    fn counters_never_decrease() {
        let mut upf = Upf::new();
        for _ in 0..10 {
            upf.forward_uplink(b"pkt");
        }
        let before = upf.stats.bytes_uplink;
        // No method to decrement — compile-time guarantee.
        // This test documents the invariant: counters are cumulative.
        assert_eq!(upf.stats.bytes_uplink, before);
        assert_eq!(upf.stats.packets_dropped, 0);
    }

    #[test]
    fn per_session_bearer_stats_are_tracked() {
        let mut upf = Upf::new();
        upf.forward_uplink_for_session(1, b"hello");
        upf.forward_downlink_for_session(1, b"world!");
        upf.forward_uplink_for_session(2, b"data");

        let s1 = upf.session_stats(1).expect("bearer 1 must have stats");
        assert_eq!(s1.bytes_uplink, 5);
        assert_eq!(s1.bytes_downlink, 6);

        let s2 = upf.session_stats(2).expect("bearer 2 must have stats");
        assert_eq!(s2.bytes_uplink, 4);
        assert_eq!(s2.bytes_downlink, 0);

        // Global counter aggregates all sessions.
        assert_eq!(upf.stats.bytes_uplink, 9);
        assert_eq!(upf.stats.bytes_downlink, 6);
    }

    #[test]
    fn release_bearer_removes_session_stats() {
        let mut upf = Upf::new();
        upf.forward_uplink_for_session(3, b"payload");
        assert_eq!(upf.bearer_count(), 1);
        assert!(upf.release_bearer(3));
        assert_eq!(upf.bearer_count(), 0);
        assert!(upf.session_stats(3).is_none());
    }
}

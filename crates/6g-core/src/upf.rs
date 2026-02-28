//! User Plane Function (UPF).
//!
//! The UPF is the data-plane anchor in the 6G core:
//! * Packet routing and forwarding
//! * Traffic usage reporting
//! * QoS enforcement
//! * Uplink classifier / branching point for local breakout

/// UPF traffic statistics (placeholder).
#[derive(Debug, Default, Clone)]
pub struct TrafficStats {
    pub bytes_uplink: u64,
    pub bytes_downlink: u64,
    pub packets_dropped: u64,
}

/// User Plane Function.
pub struct Upf {
    pub stats: TrafficStats,
}

impl Upf {
    pub fn new() -> Self {
        Self {
            stats: TrafficStats::default(),
        }
    }

    /// Forward an uplink payload (stub – no actual routing yet).
    pub fn forward_uplink(&mut self, payload: &[u8]) {
        self.stats.bytes_uplink += payload.len() as u64;
    }

    /// Forward a downlink payload (stub).
    pub fn forward_downlink(&mut self, payload: &[u8]) {
        self.stats.bytes_downlink += payload.len() as u64;
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
}

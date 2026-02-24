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

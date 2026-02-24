//! Non-Terrestrial Networks (NTN) integration layer.
//!
//! 6G natively integrates terrestrial, aerial, and space-based nodes:
//! * LEO / MEO / GEO satellites
//! * High-Altitude Platform Stations (HAPS)
//! * Unmanned Aerial Vehicles (UAV / drones)
//!
//! Key challenges addressed here:
//! * Very long propagation delays (Doppler compensation, timing advance)
//! * Dynamic topology management
//! * Seamless handover between NTN and terrestrial segments

use serde::{Deserialize, Serialize};
use sixg_common::types::Position3D;

/// Category of a non-terrestrial node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NtnNodeType {
    /// Low Earth Orbit satellite (~550 km altitude).
    LeoSatellite,
    /// Medium Earth Orbit satellite (~2 000 – 20 000 km).
    MeoSatellite,
    /// Geostationary satellite (~35 786 km).
    GeoSatellite,
    /// High-Altitude Platform Station (~20 km).
    Haps,
    /// Unmanned Aerial Vehicle (< 1 km).
    Uav,
}

/// A node in the non-terrestrial network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtnNode {
    pub id: u64,
    pub node_type: NtnNodeType,
    pub position: Position3D,
    /// Altitude in metres.
    pub altitude_m: f64,
    /// One-way propagation delay to ground (milliseconds).
    pub propagation_delay_ms: f64,
}

impl NtnNode {
    pub fn leo_satellite(id: u64, position: Position3D) -> Self {
        Self {
            id,
            node_type: NtnNodeType::LeoSatellite,
            position,
            altitude_m: 550_000.0,
            propagation_delay_ms: 1.8, // ~550 km / c
        }
    }
}

/// NTN layer managing the fleet of non-terrestrial nodes.
pub struct NtnLayer {
    nodes: Vec<NtnNode>,
}

impl NtnLayer {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_node(&mut self, node: NtnNode) {
        self.nodes.push(node);
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn nodes(&self) -> &[NtnNode] {
        &self.nodes
    }
}

impl Default for NtnLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_leo_satellite() {
        let mut ntn = NtnLayer::new();
        let pos = Position3D::new(0.0, 0.0, 550_000.0);
        ntn.add_node(NtnNode::leo_satellite(1, pos));
        assert_eq!(ntn.node_count(), 1);
        assert_eq!(ntn.nodes()[0].node_type, NtnNodeType::LeoSatellite);
    }
}

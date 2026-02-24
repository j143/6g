//! Radio Link Control (RLC) layer for 6G.
//!
//! The RLC layer provides:
//! * Segmentation and reassembly of PDCP SDUs
//! * ARQ error correction
//! * Three operating modes: Transparent (TM), Unacknowledged (UM),
//!   Acknowledged (AM)

use sixg_common::types::{BearerId, Payload};

/// RLC operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlcMode {
    /// Transparent Mode – no header, no error correction.
    Tm,
    /// Unacknowledged Mode – best-effort delivery.
    Um,
    /// Acknowledged Mode – reliable delivery with ARQ.
    Am,
}

/// A single RLC entity (one per radio bearer).
pub struct RlcEntity {
    pub bearer: BearerId,
    pub mode: RlcMode,
}

impl RlcEntity {
    pub fn new(bearer: BearerId, mode: RlcMode) -> Self {
        Self { bearer, mode }
    }

    /// Receive a PDCP PDU, segment it, and pass PDUs to MAC.
    /// (Stub – segmentation logic to be implemented.)
    pub fn transmit(&self, data: Payload) -> Vec<Payload> {
        // TODO: segment data according to RLC PDU size limits.
        vec![data]
    }

    /// Receive RLC PDUs from MAC, reassemble, and deliver to PDCP.
    /// (Stub – ARQ and reassembly logic to be implemented.)
    pub fn receive(&self, pdus: Vec<Payload>) -> Option<Payload> {
        pdus.into_iter().next()
    }
}

/// RLC layer managing all bearer entities.
pub struct RlcLayer {
    entities: Vec<RlcEntity>,
}

impl RlcLayer {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    /// Add a new RLC entity for the given bearer.
    pub fn add_entity(&mut self, bearer: BearerId, mode: RlcMode) {
        self.entities.push(RlcEntity::new(bearer, mode));
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
}

impl Default for RlcLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rlc_transmit_passthrough() {
        let e = RlcEntity::new(BearerId(1), RlcMode::Am);
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let pdus = e.transmit(data.clone());
        assert_eq!(pdus, vec![data]);
    }
}

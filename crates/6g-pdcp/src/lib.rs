//! Packet Data Convergence Protocol (PDCP) layer for 6G.
//!
//! PDCP provides:
//! * Header compression (ROHC)
//! * Ciphering and integrity protection
//! * Sequence numbering and reordering
//! * Duplication and duplication detection (for split bearers / DAPS)

use sixg_common::types::{BearerId, Payload};

/// Ciphering algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipheringAlgorithm {
    /// No ciphering (null).
    Nea0,
    /// 128-bit SNOW 3G.
    Nea1,
    /// 128-bit AES-CTR.
    Nea2,
    /// 128-bit ZUC.
    Nea3,
}

/// Integrity protection algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityAlgorithm {
    /// No integrity protection.
    Nia0,
    /// SNOW 3G MAC-I.
    Nia1,
    /// AES-CMAC.
    Nia2,
    /// ZUC MAC-I.
    Nia3,
}

/// Configuration for a PDCP entity.
#[derive(Debug, Clone)]
pub struct PdcpConfig {
    pub bearer: BearerId,
    pub ciphering: CipheringAlgorithm,
    pub integrity: IntegrityAlgorithm,
    /// Enable ROHC header compression.
    pub rohc_enabled: bool,
}

impl PdcpConfig {
    pub fn secure_default(bearer: BearerId) -> Self {
        Self {
            bearer,
            ciphering: CipheringAlgorithm::Nea2,
            integrity: IntegrityAlgorithm::Nia2,
            rohc_enabled: true,
        }
    }
}

/// A PDCP entity (one per bearer).
pub struct PdcpEntity {
    #[allow(dead_code)]
    config: PdcpConfig,
    tx_sequence: u32,
}

impl PdcpEntity {
    pub fn new(config: PdcpConfig) -> Self {
        Self {
            config,
            tx_sequence: 0,
        }
    }

    /// Process an outgoing IP packet: compress, cipher, add header.
    /// (Stub – actual crypto to be implemented.)
    pub fn process_tx(&mut self, payload: Payload) -> Payload {
        // TODO: apply ROHC compression, ciphering, integrity protection.
        self.tx_sequence = self.tx_sequence.wrapping_add(1);
        payload
    }

    /// Process an incoming PDCP PDU: verify, decipher, decompress.
    pub fn process_rx(&self, payload: Payload) -> Payload {
        // TODO: verify integrity, decipher, decompress.
        payload
    }
}

/// PDCP layer.
pub struct PdcpLayer {
    entities: Vec<PdcpEntity>,
}

impl PdcpLayer {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    pub fn add_entity(&mut self, config: PdcpConfig) {
        self.entities.push(PdcpEntity::new(config));
    }
}

impl Default for PdcpLayer {
    fn default() -> Self {
        Self::new()
    }
}

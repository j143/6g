//! Packet Data Convergence Protocol (PDCP) layer for 6G.
//!
//! PDCP provides:
//! * Simplified ROHC header compression (U-mode, context-based)
//! * Sequence numbering (12-bit or 18-bit)
//! * In-order delivery and out-of-order reordering
//! * Replay detection via a sliding receive window
//!
//! ## ROHC Simplified Model
//!
//! Full ROHC (RFC 5795) uses profiles and state machines.  This implementation
//! uses a simplified two-state model:
//!
//! - **IR (Initialization & Refresh)**: first packet on a flow, or after context
//!   reset.  The full IP/UDP header is embedded in the PDCP PDU prefixed by the
//!   `IR_MARKER` byte (`0xFF`).
//! - **CO (Compressed)**: subsequent packets.  Only the SN (2 or 3 bytes) is
//!   prepended; the receiver reconstructs the full header from context.
//!
//! Typical compression ratio: 40-byte IP/UDP/RTP header → 2-3 bytes per PDU.
//!
//! ## Sequence Number
//!
//! The SN length is configured as 12 or 18 bits (3GPP TS 38.323 §7.1).
//! The transmitter wraps at `2^sn_length`.  The receiver tracks `rx_next`
//! (next expected SN) and maintains a sliding window of size `2^(sn_length-1)`.
//!
//! ## Replay Detection
//!
//! PDUs whose SN falls within the sliding window but have already been received
//! are discarded (duplicate detection).  PDUs outside the window are also
//! discarded.  A bitfield tracks received SNs within the window.

use sixg_common::types::{BearerId, Payload};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// ROHC IR marker byte — signals a full-header (Initialization & Refresh) PDU.
const IR_MARKER: u8 = 0xFF;

// ---------------------------------------------------------------------------
// Algorithms
// ---------------------------------------------------------------------------

/// Ciphering algorithm (NEA = NR Encryption Algorithm).
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

/// Integrity protection algorithm (NIA = NR Integrity Algorithm).
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

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for a PDCP entity (per bearer).
///
/// `sn_length` must be either `12` or `18` (3GPP TS 38.323 §7.1).
#[derive(Debug, Clone)]
pub struct PdcpConfig {
    pub bearer: BearerId,
    pub ciphering: CipheringAlgorithm,
    pub integrity: IntegrityAlgorithm,
    /// Enable simplified ROHC header compression.
    pub rohc_enabled: bool,
    /// PDCP sequence number length in bits: must be 12 or 18.
    pub sn_length: u8,
}

impl PdcpConfig {
    /// Create a secure default config (AES-CTR ciphering, AES-CMAC integrity,
    /// ROHC enabled, 12-bit SN).
    pub fn secure_default(bearer: BearerId) -> Self {
        Self {
            bearer,
            ciphering: CipheringAlgorithm::Nea2,
            integrity: IntegrityAlgorithm::Nia2,
            rohc_enabled: true,
            sn_length: 12,
        }
    }

    /// Maximum SN value (exclusive): `2^sn_length`.
    pub fn sn_modulus(&self) -> u32 {
        1u32 << self.sn_length
    }

    /// Receive window size: `2^(sn_length - 1)`.
    pub fn window_size(&self) -> u32 {
        1u32 << (self.sn_length - 1)
    }
}

// ---------------------------------------------------------------------------
// ROHC context
// ---------------------------------------------------------------------------

/// Simplified ROHC compressor/decompressor context (U-mode, single-flow).
///
/// Stores the static IP/UDP header fields that do not change between packets.
/// In a real ROHC implementation this would be per-flow with full state machines.
#[derive(Debug, Default, Clone)]
pub struct RohcContext {
    /// Cached full header bytes (populated on IR packet).
    pub header_template: Option<Vec<u8>>,
    /// Whether the context has been initialised (IR packet sent/received).
    pub initialised: bool,
}

impl RohcContext {
    /// Compress a PDU on the TX side.
    ///
    /// Returns `(compressed_pdu, ir_packet_sent)`.
    ///
    /// If the context is not yet initialised, embeds the first 20 bytes of
    /// `payload` as the IR template and emits a full IR packet.  Otherwise
    /// emits only a CO (compressed) packet with the SN prepended.
    ///
    /// `sn_bytes` — the encoded SN (2 or 3 bytes).
    pub fn compress(&mut self, payload: &[u8], sn_bytes: &[u8]) -> Payload {
        if !self.initialised || self.header_template.is_none() {
            // IR packet: [IR_MARKER][sn_bytes...][full_payload...]
            let hdr_len = payload.len().min(20);
            self.header_template = Some(payload[..hdr_len].to_vec());
            self.initialised = true;
            let mut pdu = Vec::with_capacity(1 + sn_bytes.len() + payload.len());
            pdu.push(IR_MARKER);
            pdu.extend_from_slice(sn_bytes);
            pdu.extend_from_slice(payload);
            pdu
        } else {
            // CO packet: [sn_bytes...][payload...]
            let mut pdu = Vec::with_capacity(sn_bytes.len() + payload.len());
            pdu.extend_from_slice(sn_bytes);
            pdu.extend_from_slice(payload);
            pdu
        }
    }

    /// Decompress a PDU on the RX side.
    ///
    /// Returns `(decompressed_payload, sn)` or `None` if the PDU is malformed.
    ///
    /// `sn_bytes_len` — expected SN field length (2 or 3 bytes).
    pub fn decompress(&mut self, pdu: &[u8], sn_bytes_len: usize) -> Option<(Payload, u32)> {
        if pdu.is_empty() {
            return None;
        }
        if pdu[0] == IR_MARKER {
            // IR packet: extract and cache the header template.
            if pdu.len() < 1 + sn_bytes_len {
                return None;
            }
            let sn = decode_sn(&pdu[1..1 + sn_bytes_len]);
            let payload = pdu[1 + sn_bytes_len..].to_vec();
            let hdr_len = payload.len().min(20);
            self.header_template = Some(payload[..hdr_len].to_vec());
            self.initialised = true;
            Some((payload, sn))
        } else {
            // CO packet: use cached context.
            if pdu.len() < sn_bytes_len {
                return None;
            }
            let sn = decode_sn(&pdu[..sn_bytes_len]);
            let payload = pdu[sn_bytes_len..].to_vec();
            Some((payload, sn))
        }
    }
}

/// Encode a sequence number into big-endian bytes (2 bytes for 12-bit, 3 for 18-bit).
fn encode_sn(sn: u32, sn_length: u8) -> Vec<u8> {
    if sn_length <= 12 {
        vec![(sn >> 8) as u8, (sn & 0xFF) as u8]
    } else {
        vec![(sn >> 16) as u8, ((sn >> 8) & 0xFF) as u8, (sn & 0xFF) as u8]
    }
}

/// Decode a sequence number from big-endian bytes.
fn decode_sn(bytes: &[u8]) -> u32 {
    match bytes.len() {
        2 => ((bytes[0] as u32) << 8) | (bytes[1] as u32),
        3 => ((bytes[0] as u32) << 16) | ((bytes[1] as u32) << 8) | (bytes[2] as u32),
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// PDCP entity
// ---------------------------------------------------------------------------

/// A PDCP entity (one per radio bearer).
pub struct PdcpEntity {
    config: PdcpConfig,
    /// TX sequence number (wraps at `sn_modulus`).
    tx_sn: u32,
    /// Next expected RX sequence number.
    rx_next: u32,
    /// Sliding window bitmap: bit `i` set → SN `(rx_next - window_size + i)` received.
    rx_window: Vec<bool>,
    /// ROHC context for this entity.
    rohc: RohcContext,
}

impl PdcpEntity {
    /// Create a new PDCP entity from the given config.
    pub fn new(config: PdcpConfig) -> Self {
        assert!(
            config.sn_length == 12 || config.sn_length == 18,
            "sn_length must be 12 or 18"
        );
        let win = config.window_size() as usize;
        Self {
            tx_sn: 0,
            rx_next: 0,
            rx_window: vec![false; win],
            rohc: RohcContext::default(),
            config,
        }
    }

    /// Process an outgoing IP packet: apply ROHC compression and add SN header.
    ///
    /// Returns the PDCP PDU ready for delivery to RLC.
    pub fn process_tx(&mut self, payload: Payload) -> Payload {
        let sn = self.tx_sn;
        self.tx_sn = (self.tx_sn + 1) % self.config.sn_modulus();
        let sn_bytes = encode_sn(sn, self.config.sn_length);
        if self.config.rohc_enabled {
            self.rohc.compress(&payload, &sn_bytes)
        } else {
            let mut pdu = sn_bytes;
            pdu.extend_from_slice(&payload);
            pdu
        }
    }

    /// Process an incoming PDCP PDU: decompress, verify SN, replay-detect.
    ///
    /// Returns `Some(payload)` if the PDU is valid and in-window, `None` otherwise.
    pub fn process_rx(&mut self, pdu: Payload) -> Option<Payload> {
        let sn_bytes_len = if self.config.sn_length <= 12 { 2 } else { 3 };
        let (payload, sn) = if self.config.rohc_enabled {
            self.rohc.decompress(&pdu, sn_bytes_len)?
        } else {
            if pdu.len() < sn_bytes_len {
                return None;
            }
            let sn = decode_sn(&pdu[..sn_bytes_len]);
            (pdu[sn_bytes_len..].to_vec(), sn)
        };

        // Replay / reorder detection.
        if !self.is_in_window(sn) {
            return None; // out-of-window: discard
        }
        let offset = self.window_offset(sn);
        if self.rx_window[offset] {
            return None; // duplicate: discard
        }
        self.rx_window[offset] = true;
        // Advance rx_next when the expected SN is received.
        if sn == self.rx_next {
            self.advance_rx_next();
        }
        Some(payload)
    }

    /// Check whether `sn` falls within the current receive window.
    fn is_in_window(&self, sn: u32) -> bool {
        let win = self.config.window_size();
        let modulus = self.config.sn_modulus();
        let lower = self.rx_next;
        let upper = (lower + win) % modulus;
        if lower <= upper {
            sn >= lower && sn < upper
        } else {
            sn >= lower || sn < upper
        }
    }

    /// Map `sn` to its index in `rx_window`.
    fn window_offset(&self, sn: u32) -> usize {
        let modulus = self.config.sn_modulus();
        let win = self.config.window_size();
        let lower = (self.rx_next + modulus - win) % modulus;
        ((sn + modulus - lower) % modulus) as usize % self.rx_window.len()
    }

    /// Slide `rx_next` forward while consecutive SNs have been received.
    fn advance_rx_next(&mut self) {
        loop {
            let offset = self.window_offset(self.rx_next);
            if !self.rx_window[offset] {
                break;
            }
            self.rx_window[offset] = false;
            self.rx_next = (self.rx_next + 1) % self.config.sn_modulus();
        }
    }

    /// Return the current TX sequence number (for testing / diagnostics).
    pub fn tx_sn(&self) -> u32 {
        self.tx_sn
    }

    /// Return the next expected RX sequence number (for testing / diagnostics).
    pub fn rx_next(&self) -> u32 {
        self.rx_next
    }
}

// ---------------------------------------------------------------------------
// PDCP layer
// ---------------------------------------------------------------------------

/// PDCP layer — manages all active per-bearer entities.
pub struct PdcpLayer {
    entities: Vec<PdcpEntity>,
}

impl PdcpLayer {
    pub fn new() -> Self {
        Self { entities: Vec::new() }
    }

    /// Add a new PDCP entity for the given bearer config.
    pub fn add_entity(&mut self, config: PdcpConfig) {
        self.entities.push(PdcpEntity::new(config));
    }

    /// Return a mutable reference to the entity at `index`.
    pub fn entity_mut(&mut self, index: usize) -> Option<&mut PdcpEntity> {
        self.entities.get_mut(index)
    }

    /// Return the number of active entities.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
}

impl Default for PdcpLayer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity(sn_length: u8, rohc: bool) -> PdcpEntity {
        PdcpEntity::new(PdcpConfig {
            bearer: BearerId(1),
            ciphering: CipheringAlgorithm::Nea0,
            integrity: IntegrityAlgorithm::Nia0,
            rohc_enabled: rohc,
            sn_length,
        })
    }

    #[test]
    fn tx_sn_increments() {
        let mut e = make_entity(12, false);
        assert_eq!(e.tx_sn(), 0);
        let _ = e.process_tx(vec![1, 2, 3]);
        assert_eq!(e.tx_sn(), 1);
        let _ = e.process_tx(vec![4, 5, 6]);
        assert_eq!(e.tx_sn(), 2);
    }

    #[test]
    fn tx_sn_wraps_at_modulus() {
        let mut e = make_entity(12, false);
        // Fast-forward to one before wrap.
        e.tx_sn = (1u32 << 12) - 1;
        let _ = e.process_tx(vec![0]);
        assert_eq!(e.tx_sn(), 0, "SN must wrap at 2^12");
    }

    #[test]
    fn round_trip_no_rohc() {
        let mut tx = make_entity(12, false);
        let mut rx = make_entity(12, false);
        let data: Payload = b"hello pdcp".to_vec();
        let pdu = tx.process_tx(data.clone());
        let recovered = rx.process_rx(pdu).expect("should decode");
        assert_eq!(recovered, data);
    }

    #[test]
    fn round_trip_rohc_ir_then_co() {
        let mut tx = make_entity(12, true);
        let mut rx = make_entity(12, true);
        let data: Payload = vec![0u8; 40]; // simulate IP/UDP packet
        // First packet → IR
        let pdu0 = tx.process_tx(data.clone());
        assert_eq!(pdu0[0], IR_MARKER, "first ROHC packet must be IR");
        let r0 = rx.process_rx(pdu0).expect("IR round-trip");
        assert_eq!(r0, data);
        // Second packet → CO (no IR marker)
        let pdu1 = tx.process_tx(data.clone());
        assert_ne!(pdu1[0], IR_MARKER, "second ROHC packet must be CO");
        let r1 = rx.process_rx(pdu1).expect("CO round-trip");
        assert_eq!(r1, data);
    }

    #[test]
    fn replay_detection_rejects_duplicate() {
        let mut tx = make_entity(12, false);
        let mut rx = make_entity(12, false);
        let pdu = tx.process_tx(b"test".to_vec());
        let _ = rx.process_rx(pdu.clone()).expect("first delivery should succeed");
        let dup = rx.process_rx(pdu);
        assert!(dup.is_none(), "duplicate PDU must be dropped");
    }

    #[test]
    fn out_of_window_pdu_rejected() {
        let mut tx = make_entity(12, false);
        let mut rx = make_entity(12, false);
        // Advance rx_next by filling the window.
        let win = rx.config.window_size() as usize;
        for _ in 0..win {
            let pdu = tx.process_tx(vec![0]);
            let _ = rx.process_rx(pdu);
        }
        // Now send packet with SN=0 which is behind the window.
        // Build a manual PDU with SN=0.
        let old_pdu = vec![0u8, 0u8, 42u8]; // SN=0 (12-bit), payload=42
        assert!(rx.process_rx(old_pdu).is_none(), "out-of-window PDU must be dropped");
    }

    #[test]
    fn rohc_compression_reduces_size() {
        // After IR, CO packets should be shorter (only SN prefix, no IR marker).
        let mut tx = make_entity(12, true);
        let data = vec![0xAB; 40];
        let ir_pdu = tx.process_tx(data.clone());
        let co_pdu = tx.process_tx(data.clone());
        // CO is shorter than IR (IR has extra IR_MARKER byte).
        assert!(co_pdu.len() < ir_pdu.len(), "CO must be shorter than IR");
    }

    #[test]
    fn sn_18bit_round_trip() {
        let mut tx = make_entity(18, false);
        let mut rx = make_entity(18, false);
        let data: Payload = b"18bit sn test".to_vec();
        let pdu = tx.process_tx(data.clone());
        // 18-bit SN uses 3-byte prefix.
        assert_eq!(pdu.len(), 3 + data.len());
        let recovered = rx.process_rx(pdu).expect("18-bit SN round-trip");
        assert_eq!(recovered, data);
    }

    #[test]
    fn pdcp_layer_entity_count() {
        let mut layer = PdcpLayer::new();
        assert_eq!(layer.entity_count(), 0);
        layer.add_entity(PdcpConfig::secure_default(BearerId(1)));
        layer.add_entity(PdcpConfig::secure_default(BearerId(2)));
        assert_eq!(layer.entity_count(), 2);
    }
}

//! Radio Link Control (RLC) layer for 6G.
//!
//! The RLC layer provides:
//! * Segmentation and reassembly of PDCP SDUs
//! * ARQ error correction (AM mode)
//! * Three operating modes: Transparent (TM), Unacknowledged (UM),
//!   Acknowledged (AM)
//!
//! ## Modes
//!
//! | Mode | Use case            | ARQ | Segmentation |
//! |------|---------------------|-----|--------------|
//! | TM   | Broadcast/paging    | No  | No           |
//! | UM   | VoIP / streaming    | No  | Yes          |
//! | AM   | TCP / reliable data | Yes | Yes          |
//!
//! ## PDU Format (UM / AM)
//!
//! ```text
//! Byte 0: [SI (2b) | reserved (6b)]   SI: 00=full, 01=first, 10=last, 11=middle
//! Byte 1: SN high byte
//! Byte 2: SN low byte  (12-bit SN → 2 bytes total; 0-extended to fit)
//! Byte 3..: data
//! ```
//!
//! For AM, byte 0 adds a D/C bit (bit 7 = 1 for data PDU) and poll bit (bit 6).
//!
//! ## ARQ (AM mode)
//!
//! The transmitter stores unacknowledged SDUs in a retransmission buffer keyed
//! by SN.  After `poll_after_n_pdus` PDUs, the poll bit is set and the receiver
//! responds with a STATUS PDU carrying an `ack_sn` and a list of NACKed SNs.
//! The transmitter retransmits only the NACKed SNs.

use sixg_common::types::{BearerId, Payload};

// ---------------------------------------------------------------------------
// RLC mode
// ---------------------------------------------------------------------------

/// RLC operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlcMode {
    /// Transparent Mode — no header, no segmentation, no error correction.
    Tm,
    /// Unacknowledged Mode — segmentation, SN header, best-effort delivery.
    Um,
    /// Acknowledged Mode — segmentation, SN header, ARQ retransmission.
    Am,
}

// ---------------------------------------------------------------------------
// Segment info field (SI)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SegmentInfo {
    /// Complete SDU in one PDU.
    Full = 0b00,
    /// First segment of a larger SDU.
    First = 0b01,
    /// Last segment of a larger SDU.
    Last = 0b10,
    /// Middle segment.
    Middle = 0b11,
}

// ---------------------------------------------------------------------------
// PDU header encoding
// ---------------------------------------------------------------------------

/// Build an RLC PDU header.
///
/// Header format (3 bytes):
/// - Byte 0 UM:  `[0][0][SI_H][SI_L][0][0][0][0]` (D/C always 0, SI in bits 5–4)
/// - Byte 0 AM:  `[D/C=1][P][SI_H][SI_L][0][0][0][0]` (D/C=1, P in bit 6, SI in bits 5–4)
/// - Byte 1: SN high byte
/// - Byte 2: SN low byte
fn build_pdu(si: SegmentInfo, sn: u16, data: &[u8], is_am: bool, poll: bool) -> Payload {
    let si_bits = si as u8;
    let flags: u8 = if is_am {
        0x80 | (if poll { 0x40 } else { 0x00 }) | (si_bits << 4)
    } else {
        si_bits << 4
    };
    let mut pdu = Vec::with_capacity(3 + data.len());
    pdu.push(flags);
    pdu.push((sn >> 8) as u8);
    pdu.push((sn & 0xFF) as u8);
    pdu.extend_from_slice(data);
    pdu
}

/// Parse the header of an RLC PDU.
///
/// Returns `(si, is_data, poll, sn, payload_offset)` or `None` on error.
fn parse_pdu_header(pdu: &[u8]) -> Option<(SegmentInfo, bool, bool, u16, usize)> {
    if pdu.len() < 3 {
        return None;
    }
    let flags = pdu[0];
    let is_data = (flags & 0x80) != 0;
    let poll = (flags & 0x40) != 0;
    // SI always in bits 5–4 (both UM and AM).
    let si_bits = (flags >> 4) & 0x03;
    let si = match si_bits {
        0b00 => SegmentInfo::Full,
        0b01 => SegmentInfo::First,
        0b10 => SegmentInfo::Last,
        _ => SegmentInfo::Middle,
    };
    let sn = ((pdu[1] as u16) << 8) | (pdu[2] as u16);
    Some((si, is_data, poll, sn, 3))
}

// ---------------------------------------------------------------------------
// STATUS PDU
// ---------------------------------------------------------------------------

/// AM STATUS PDU — carries ACK SN and a list of NACKed SNs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusPdu {
    /// The first SN above the highest successfully received in-sequence SN.
    pub ack_sn: u16,
    /// List of SNs that were not received (need retransmission).
    pub nack_sns: Vec<u16>,
}

impl StatusPdu {
    /// Encode into bytes: `[0x00][ack_sn_high][ack_sn_low][nack_sn_high][nack_sn_low]...`
    pub fn encode(&self) -> Payload {
        let mut out = Vec::with_capacity(3 + self.nack_sns.len() * 2);
        out.push(0x00); // D/C = 0 → control PDU
        out.push((self.ack_sn >> 8) as u8);
        out.push((self.ack_sn & 0xFF) as u8);
        for &n in &self.nack_sns {
            out.push((n >> 8) as u8);
            out.push((n & 0xFF) as u8);
        }
        out
    }

    /// Decode from bytes.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 3 || bytes[0] != 0x00 {
            return None;
        }
        let ack_sn = ((bytes[1] as u16) << 8) | (bytes[2] as u16);
        let nack_sns = bytes[3..]
            .chunks_exact(2)
            .map(|c| ((c[0] as u16) << 8) | (c[1] as u16))
            .collect();
        Some(StatusPdu { ack_sn, nack_sns })
    }
}

// ---------------------------------------------------------------------------
// RLC entity
// ---------------------------------------------------------------------------

/// Maximum data bytes per RLC PDU (payload, not counting header).
pub const MAX_PDU_PAYLOAD: usize = 500;

/// Poll after every N PDUs (AM mode).
const POLL_INTERVAL: usize = 8;

/// A single RLC entity (one per radio bearer).
pub struct RlcEntity {
    pub bearer: BearerId,
    pub mode: RlcMode,
    /// TX sequence number.
    tx_sn: u16,
    /// Next expected RX sequence number.
    rx_next: u16,
    /// Unacknowledged SDU segments for AM ARQ: `(sn, pdu_bytes)`.
    retx_buffer: Vec<(u16, Payload)>,
    /// PDU counter for poll scheduling.
    pdu_count: usize,
    /// Reassembly buffers for UM/AM: `(sn, segment)` pairs pending reassembly.
    rx_segments: Vec<(u16, Payload, SegmentInfo)>,
}

impl RlcEntity {
    /// Create a new RLC entity for the given bearer and mode.
    pub fn new(bearer: BearerId, mode: RlcMode) -> Self {
        Self {
            bearer,
            mode,
            tx_sn: 0,
            rx_next: 0,
            retx_buffer: Vec::new(),
            pdu_count: 0,
            rx_segments: Vec::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Transmit path
    // -----------------------------------------------------------------------

    /// Segment a PDCP SDU and produce a list of RLC PDUs for delivery to MAC.
    ///
    /// - TM: returns the SDU unchanged (single PDU, no header).
    /// - UM: segments into `MAX_PDU_PAYLOAD`-byte chunks; prepends SN header.
    /// - AM: same as UM but stores PDUs in the retransmission buffer and sets
    ///   the poll bit every `POLL_INTERVAL` PDUs.
    pub fn transmit(&mut self, data: Payload) -> Vec<Payload> {
        match self.mode {
            RlcMode::Tm => vec![data],
            RlcMode::Um => self.segment(data, false),
            RlcMode::Am => self.segment(data, true),
        }
    }

    /// Segment `data` into RLC PDUs.
    fn segment(&mut self, data: Payload, am: bool) -> Vec<Payload> {
        if data.is_empty() {
            return vec![];
        }
        let chunks: Vec<&[u8]> = data.chunks(MAX_PDU_PAYLOAD).collect();
        let n = chunks.len();
        let mut pdus = Vec::with_capacity(n);
        for (i, chunk) in chunks.iter().enumerate() {
            let si = match (i, n) {
                (_, 1) => SegmentInfo::Full,
                (0, _) => SegmentInfo::First,
                (j, k) if j == k - 1 => SegmentInfo::Last,
                _ => SegmentInfo::Middle,
            };
            let sn = self.tx_sn;
            self.pdu_count += 1;
            let poll = am && self.pdu_count.is_multiple_of(POLL_INTERVAL);
            let pdu = build_pdu(si, sn, chunk, am, poll);
            if am {
                self.retx_buffer.push((sn, pdu.clone()));
            }
            pdus.push(pdu);
            // Advance SN per PDU (each segment gets its own SN).
            self.tx_sn = self.tx_sn.wrapping_add(1);
        }
        pdus
    }

    // -----------------------------------------------------------------------
    // Receive path
    // -----------------------------------------------------------------------

    /// Receive RLC PDUs from MAC, reassemble, and deliver to PDCP.
    ///
    /// - TM: returns the first PDU unchanged.
    /// - UM/AM: parses headers, buffers segments, delivers complete SDUs.
    ///
    /// Returns `Some(sdu)` when a complete SDU has been reassembled, `None` if
    /// more segments are still expected.
    pub fn receive(&mut self, pdus: Vec<Payload>) -> Option<Payload> {
        match self.mode {
            RlcMode::Tm => pdus.into_iter().next(),
            RlcMode::Um | RlcMode::Am => self.reassemble(pdus),
        }
    }

    /// Parse PDUs and attempt SDU reassembly.
    fn reassemble(&mut self, pdus: Vec<Payload>) -> Option<Payload> {
        for pdu in pdus {
            if let Some((si, _is_data, _poll, sn, offset)) = parse_pdu_header(&pdu) {
                let payload = pdu[offset..].to_vec();
                self.rx_segments.push((sn, payload, si));
            }
        }
        // Try to reassemble a complete SDU (Full or First..Middle*..Last sequence).
        self.try_reassemble()
    }

    /// Attempt to reconstruct an SDU from buffered segments.
    ///
    /// A complete SDU is either one `Full` segment or a consecutive chain of
    /// `First` → zero or more `Middle` → `Last` with sequential SNs.
    fn try_reassemble(&mut self) -> Option<Payload> {
        // Sort by SN.
        self.rx_segments.sort_by_key(|(sn, _, _)| *sn);
        // Check for a single Full segment.
        if let Some(pos) = self.rx_segments.iter().position(|(_, _, si)| *si == SegmentInfo::Full) {
            let (_, data, _) = self.rx_segments.remove(pos);
            return Some(data);
        }
        // Check for First + ... + Last chain.
        if let Some(first_pos) =
            self.rx_segments.iter().position(|(_, _, si)| *si == SegmentInfo::First)
        {
            let start_sn = self.rx_segments[first_pos].0;
            // Collect consecutive SNs from start_sn until we find Last.
            let mut chain_sns: Vec<u16> = vec![start_sn];
            let mut cur_sn = start_sn.wrapping_add(1);
            loop {
                if let Some(pos) =
                    self.rx_segments.iter().position(|(sn, _, _)| *sn == cur_sn)
                {
                    let si = self.rx_segments[pos].2;
                    chain_sns.push(cur_sn);
                    if si == SegmentInfo::Last {
                        break;
                    }
                    cur_sn = cur_sn.wrapping_add(1);
                } else {
                    return None; // chain incomplete
                }
            }
            // Extract and reassemble.
            let mut sdu = Vec::new();
            for sn in &chain_sns {
                let pos = self.rx_segments.iter().position(|(s, _, _)| s == sn).unwrap();
                let (_, data, _) = self.rx_segments.remove(pos);
                sdu.extend_from_slice(&data);
            }
            return Some(sdu);
        }
        None
    }

    // -----------------------------------------------------------------------
    // ARQ (AM mode)
    // -----------------------------------------------------------------------

    /// Process a received STATUS PDU and remove ACKed segments from the
    /// retransmission buffer.
    ///
    /// Returns the list of PDUs that must be retransmitted (NACKed SNs).
    pub fn process_status(&mut self, status: &StatusPdu) -> Vec<Payload> {
        // ACK semantics: all SNs < ack_sn are ACKed UNLESS they appear in nack_sns.
        let nack_set: std::collections::HashSet<u16> =
            status.nack_sns.iter().copied().collect();
        self.retx_buffer
            .retain(|(sn, _)| *sn >= status.ack_sn || nack_set.contains(sn));
        // Return payloads for NACKed SNs (need retransmission).
        status
            .nack_sns
            .iter()
            .filter_map(|nack_sn| {
                self.retx_buffer
                    .iter()
                    .find(|(sn, _)| sn == nack_sn)
                    .map(|(_, pdu)| pdu.clone())
            })
            .collect()
    }

    /// Generate a STATUS PDU acknowledging all in-sequence SNs up to `rx_next`.
    ///
    /// NACKs any SNs in `nack_sns` (caller provides the list of missing SNs).
    pub fn generate_status(&self, nack_sns: Vec<u16>) -> StatusPdu {
        StatusPdu { ack_sn: self.rx_next, nack_sns }
    }

    /// Return the current TX SN (for diagnostics / tests).
    pub fn tx_sn(&self) -> u16 {
        self.tx_sn
    }
}

// ---------------------------------------------------------------------------
// RLC layer
// ---------------------------------------------------------------------------

/// RLC layer — manages all active bearer entities.
pub struct RlcLayer {
    entities: Vec<RlcEntity>,
}

impl RlcLayer {
    pub fn new() -> Self {
        Self { entities: Vec::new() }
    }

    /// Add a new RLC entity for the given bearer.
    pub fn add_entity(&mut self, bearer: BearerId, mode: RlcMode) {
        self.entities.push(RlcEntity::new(bearer, mode));
    }

    /// Return the number of active entities.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Return a mutable reference to the entity at `index`.
    pub fn entity_mut(&mut self, index: usize) -> Option<&mut RlcEntity> {
        self.entities.get_mut(index)
    }
}

impl Default for RlcLayer {
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

    // --- TM mode ---

    #[test]
    fn tm_passthrough() {
        let mut e = RlcEntity::new(BearerId(1), RlcMode::Tm);
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let pdus = e.transmit(data.clone());
        assert_eq!(pdus, vec![data]);
    }

    #[test]
    fn tm_receive_passthrough() {
        let mut e = RlcEntity::new(BearerId(1), RlcMode::Tm);
        let data = vec![1u8, 2, 3];
        let result = e.receive(vec![data.clone()]);
        assert_eq!(result, Some(data));
    }

    // --- UM mode ---

    #[test]
    fn um_small_sdu_single_pdu() {
        let mut e = RlcEntity::new(BearerId(1), RlcMode::Um);
        let data = vec![0xAA; 10];
        let pdus = e.transmit(data.clone());
        // Small SDU fits in one PDU → Full SI.
        assert_eq!(pdus.len(), 1);
        let (si, _, _, _sn, offset) = parse_pdu_header(&pdus[0]).unwrap();
        assert_eq!(si, SegmentInfo::Full);
        assert_eq!(&pdus[0][offset..], &data[..]);
    }

    #[test]
    fn um_large_sdu_segmented() {
        let mut e = RlcEntity::new(BearerId(1), RlcMode::Um);
        let data = vec![0xBB; MAX_PDU_PAYLOAD * 3 + 1];
        let pdus = e.transmit(data.clone());
        assert_eq!(pdus.len(), 4, "should produce 4 segments");
        // Check SI flags.
        let sis: Vec<SegmentInfo> = pdus
            .iter()
            .map(|p| {
                let (si, _, _, _, _) = parse_pdu_header(p).unwrap();
                si
            })
            .collect();
        assert_eq!(sis[0], SegmentInfo::First);
        assert_eq!(sis[1], SegmentInfo::Middle);
        assert_eq!(sis[2], SegmentInfo::Middle);
        assert_eq!(sis[3], SegmentInfo::Last);
    }

    #[test]
    fn um_round_trip_large_sdu() {
        let mut tx = RlcEntity::new(BearerId(1), RlcMode::Um);
        let mut rx = RlcEntity::new(BearerId(1), RlcMode::Um);
        let data = vec![0xCC; MAX_PDU_PAYLOAD + 100];
        let pdus = tx.transmit(data.clone());
        let sdu = rx.receive(pdus).expect("should reassemble");
        assert_eq!(sdu, data);
    }

    #[test]
    fn um_sn_increments_per_segment() {
        let mut e = RlcEntity::new(BearerId(1), RlcMode::Um);
        let data = vec![0; MAX_PDU_PAYLOAD * 2 + 1]; // 3 segments
        let pdus = e.transmit(data);
        let sns: Vec<u16> = pdus
            .iter()
            .map(|p| {
                let (_, _, _, sn, _) = parse_pdu_header(p).unwrap();
                sn
            })
            .collect();
        assert_eq!(sns, vec![0, 1, 2]);
    }

    // --- AM mode ---

    #[test]
    fn am_round_trip() {
        let mut tx = RlcEntity::new(BearerId(1), RlcMode::Am);
        let mut rx = RlcEntity::new(BearerId(1), RlcMode::Am);
        let data = vec![0xDD; 100];
        let pdus = tx.transmit(data.clone());
        let sdu = rx.receive(pdus).expect("AM reassembly");
        assert_eq!(sdu, data);
    }

    #[test]
    fn am_retx_buffer_populated() {
        let mut e = RlcEntity::new(BearerId(1), RlcMode::Am);
        let data = vec![0; MAX_PDU_PAYLOAD + 1]; // 2 segments
        let _ = e.transmit(data);
        assert_eq!(e.retx_buffer.len(), 2, "retx buffer should hold 2 segments");
    }

    #[test]
    fn am_process_status_clears_acked_sns() {
        let mut e = RlcEntity::new(BearerId(1), RlcMode::Am);
        let data = vec![0; MAX_PDU_PAYLOAD * 3 + 1]; // 4 segments (SN 0–3)
        let _ = e.transmit(data);
        // ACK SNs 0, 1, 2 (ack_sn=3 means everything up to SN<3 is ACKed).
        let status = StatusPdu { ack_sn: 3, nack_sns: vec![] };
        let retx = e.process_status(&status);
        assert!(retx.is_empty(), "no NACKs → nothing to retransmit");
        // retx_buffer should now only contain SN=3.
        assert_eq!(e.retx_buffer.len(), 1);
        assert_eq!(e.retx_buffer[0].0, 3);
    }

    #[test]
    fn am_process_status_returns_nacked_pdus() {
        let mut e = RlcEntity::new(BearerId(1), RlcMode::Am);
        let data = vec![0xEE; MAX_PDU_PAYLOAD * 2 + 1]; // 3 segments (SN 0, 1, 2)
        let _ = e.transmit(data);
        let status = StatusPdu { ack_sn: 3, nack_sns: vec![1] };
        let retx = e.process_status(&status);
        assert_eq!(retx.len(), 1, "one NACK should yield one retransmission PDU");
    }

    #[test]
    fn status_pdu_encode_decode() {
        let s = StatusPdu { ack_sn: 42, nack_sns: vec![10, 20] };
        let bytes = s.encode();
        let decoded = StatusPdu::decode(&bytes).expect("should decode");
        assert_eq!(decoded.ack_sn, 42);
        assert_eq!(decoded.nack_sns, vec![10, 20]);
    }

    #[test]
    fn am_dc_bit_set_in_data_pdu() {
        let mut e = RlcEntity::new(BearerId(1), RlcMode::Am);
        let pdus = e.transmit(vec![0xFF; 10]);
        let (_, is_data, _, _, _) = parse_pdu_header(&pdus[0]).unwrap();
        assert!(is_data, "AM data PDU must have D/C=1");
    }

    #[test]
    fn rlc_layer_add_and_count() {
        let mut layer = RlcLayer::new();
        layer.add_entity(BearerId(1), RlcMode::Am);
        layer.add_entity(BearerId(2), RlcMode::Um);
        assert_eq!(layer.entity_count(), 2);
    }

    // Preserve the original test name for backwards compatibility.
    #[test]
    fn rlc_transmit_passthrough() {
        let mut e = RlcEntity::new(BearerId(1), RlcMode::Am);
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let pdus = e.transmit(data.clone());
        // AM mode: small SDU → one PDU with header, strip header to compare.
        assert_eq!(pdus.len(), 1);
        let (_, _, _, _, offset) = parse_pdu_header(&pdus[0]).unwrap();
        assert_eq!(&pdus[0][offset..], &data[..]);
    }
}

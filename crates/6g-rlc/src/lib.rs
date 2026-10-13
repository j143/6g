//! Radio Link Control (RLC) layer for 6G.
//!
//! The RLC layer provides segmentation and reassembly of PDCP SDUs, ARQ error
//! correction in acknowledged mode, and three operating modes: transparent,
//! unacknowledged, and acknowledged.

use sixg_common::types::{BearerId, Payload};

/// RLC operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlcMode {
    /// Transparent Mode: no header, segmentation, or error correction.
    Tm,
    /// Unacknowledged Mode: segmentation, sequence numbers, best-effort delivery.
    Um,
    /// Acknowledged Mode: segmentation, sequence numbers, and ARQ.
    Am,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SegmentInfo {
    Full = 0b00,
    First = 0b01,
    Last = 0b10,
    Middle = 0b11,
}

fn build_pdu(
    si: SegmentInfo,
    sn: u16,
    data: &[u8],
    is_am: bool,
    poll: bool,
) -> Payload {
    let si_bits = si as u8;

    let flags = if is_am {
        let poll_bit = if poll { 0x40 } else { 0x00 };
        0x80 | poll_bit | (si_bits << 4)
    } else {
        si_bits << 4
    };

    let mut pdu = Vec::with_capacity(3 + data.len());
    pdu.push(flags);
    pdu.push((sn >> 8) as u8);
    pdu.push((sn & 0xff) as u8);
    pdu.extend_from_slice(data);
    pdu
}

fn parse_pdu_header(pdu: &[u8]) -> Option<(SegmentInfo, bool, bool, u16, usize)> {
    if pdu.len() < 3 {
        return None;
    }

    let flags = pdu[0];
    let is_data = (flags & 0x80) != 0;
    let poll = (flags & 0x40) != 0;
    let si = match (flags >> 4) & 0x03 {
        0b00 => SegmentInfo::Full,
        0b01 => SegmentInfo::First,
        0b10 => SegmentInfo::Last,
        _ => SegmentInfo::Middle,
    };
    let sn = ((pdu[1] as u16) << 8) | (pdu[2] as u16);

    Some((si, is_data, poll, sn, 3))
}

/// AM STATUS PDU carrying an ACK SN and NACKed SNs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusPdu {
    pub ack_sn: u16,
    pub nack_sns: Vec<u16>,
}

impl StatusPdu {
    pub fn encode(&self) -> Payload {
        let mut bytes = Vec::with_capacity(3 + 2 * self.nack_sns.len());
        bytes.push(0x00);
        bytes.push((self.ack_sn >> 8) as u8);
        bytes.push((self.ack_sn & 0xff) as u8);

        for sn in &self.nack_sns {
            bytes.push((sn >> 8) as u8);
            bytes.push((sn & 0xff) as u8);
        }

        bytes
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 3 || bytes[0] != 0x00 {
            return None;
        }

        let ack_sn = ((bytes[1] as u16) << 8) | (bytes[2] as u16);
        let nack_sns = bytes[3..]
            .chunks(2)
            .filter(|chunk| chunk.len() == 2)
            .map(|chunk| ((chunk[0] as u16) << 8) | (chunk[1] as u16))
            .collect();

        Some(Self { ack_sn, nack_sns })
    }
}

/// Maximum payload bytes in one RLC data PDU.
pub const MAX_PDU_PAYLOAD: usize = 500;

const POLL_INTERVAL: usize = 8;

/// One RLC entity for a radio bearer.
pub struct RlcEntity {
    pub bearer: BearerId,
    pub mode: RlcMode,
    tx_sn: u16,
    rx_next: u16,
    retx_buffer: Vec<(u16, Payload)>,
    pdu_count: usize,
    rx_segments: Vec<(u16, Payload, SegmentInfo)>,
}

impl RlcEntity {
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

    pub fn transmit(&mut self, data: Payload) -> Vec<Payload> {
        match self.mode {
            RlcMode::Tm => vec![data],
            RlcMode::Um => self.segment(data, false),
            RlcMode::Am => self.segment(data, true),
        }
    }

    fn segment(&mut self, data: Payload, is_am: bool) -> Vec<Payload> {
        if data.is_empty() {
            return Vec::new();
        }

        let chunks: Vec<&[u8]> = data.chunks(MAX_PDU_PAYLOAD).collect();
        let segment_count = chunks.len();
        let mut pdus = Vec::with_capacity(segment_count);

        for (index, chunk) in chunks.into_iter().enumerate() {
            let si = match (index, segment_count) {
                (_, 1) => SegmentInfo::Full,
                (0, _) => SegmentInfo::First,
                (index, count) if index + 1 == count => SegmentInfo::Last,
                _ => SegmentInfo::Middle,
            };

            let sn = self.tx_sn;
            self.pdu_count += 1;

            let poll = is_am && self.pdu_count % POLL_INTERVAL == 0;
            let pdu = build_pdu(si, sn, chunk, is_am, poll);

            if is_am {
                self.retx_buffer.push((sn, pdu.clone()));
            }

            pdus.push(pdu);
            self.tx_sn = self.tx_sn.wrapping_add(1);
        }

        pdus
    }

    pub fn receive(&mut self, pdus: Vec<Payload>) -> Option<Payload> {
        match self.mode {
            RlcMode::Tm => pdus.into_iter().next(),
            RlcMode::Um | RlcMode::Am => self.reassemble(pdus),
        }
    }

    fn reassemble(&mut self, pdus: Vec<Payload>) -> Option<Payload> {
        for pdu in pdus {
            let Some((si, is_data, _, sn, offset)) = parse_pdu_header(&pdu) else {
                continue;
            };

            // Only AM defines a D/C bit. In UM, D/C=0 is normal for a data PDU.
            let is_control = self.mode == RlcMode::Am && !is_data;
            if is_control {
                continue;
            }

            let payload = pdu[offset..].to_vec();
            self.rx_segments.push((sn, payload, si));
        }

        self.try_reassemble()
    }

    fn try_reassemble(&mut self) -> Option<Payload> {
        self.rx_segments
            .sort_by_key(|(sequence_number, _, _)| *sequence_number);

        if let Some(position) = self
            .rx_segments
            .iter()
            .position(|(_, _, si)| *si == SegmentInfo::Full)
        {
            let (_, payload, _) = self.rx_segments.remove(position);
            return Some(payload);
        }

        let Some(first_position) = self
            .rx_segments
            .iter()
            .position(|(_, _, si)| *si == SegmentInfo::First)
        else {
            return None;
        };

        let first_sn = self.rx_segments[first_position].0;
        let mut chain = vec![first_sn];
        let mut next_sn = first_sn.wrapping_add(1);

        loop {
            let Some(position) = self
                .rx_segments
                .iter()
                .position(|(sequence_number, _, _)| *sequence_number == next_sn)
            else {
                return None;
            };

            let is_last = self.rx_segments[position].2 == SegmentInfo::Last;
            chain.push(next_sn);

            if is_last {
                break;
            }

            next_sn = next_sn.wrapping_add(1);
        }

        let mut sdu = Vec::new();

        for sequence_number in chain {
            let position = self
                .rx_segments
                .iter()
                .position(|(candidate, _, _)| *candidate == sequence_number)
                .expect("segment must remain buffered while its chain is assembled");

            let (_, payload, _) = self.rx_segments.remove(position);
            sdu.extend_from_slice(&payload);
        }

        Some(sdu)
    }

    pub fn process_status(&mut self, status: &StatusPdu) -> Vec<Payload> {
        let nacked: std::collections::HashSet<u16> =
            status.nack_sns.iter().copied().collect();

        self.retx_buffer.retain(|(sequence_number, _)| {
            *sequence_number >= status.ack_sn || nacked.contains(sequence_number)
        });

        status
            .nack_sns
            .iter()
            .filter_map(|nacked_sn| {
                self.retx_buffer
                    .iter()
                    .find(|(sequence_number, _)| sequence_number == nacked_sn)
                    .map(|(_, pdu)| pdu.clone())
            })
            .collect()
    }

    pub fn generate_status(&self, nack_sns: Vec<u16>) -> StatusPdu {
        StatusPdu {
            ack_sn: self.rx_next,
            nack_sns,
        }
    }

    pub fn tx_sn(&self) -> u16 {
        self.tx_sn
    }
}

/// Manages RLC entities for active radio bearers.
pub struct RlcLayer {
    entities: Vec<RlcEntity>,
}

impl RlcLayer {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    pub fn add_entity(&mut self, bearer: BearerId, mode: RlcMode) {
        self.entities.push(RlcEntity::new(bearer, mode));
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub fn entity_mut(&mut self, index: usize) -> Option<&mut RlcEntity> {
        self.entities.get_mut(index)
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
    fn tm_passthrough() {
        let mut entity = RlcEntity::new(BearerId(1), RlcMode::Tm);
        let payload = vec![0xde, 0xad, 0xbe, 0xef];

        assert_eq!(entity.transmit(payload.clone()), vec![payload]);
    }

    #[test]
    fn tm_receive_passthrough() {
        let mut entity = RlcEntity::new(BearerId(1), RlcMode::Tm);
        let payload = vec![1, 2, 3];

        assert_eq!(entity.receive(vec![payload.clone()]), Some(payload));
    }

    #[test]
    fn um_small_sdu_single_pdu() {
        let mut entity = RlcEntity::new(BearerId(1), RlcMode::Um);
        let payload = vec![0xaa; 10];
        let pdus = entity.transmit(payload.clone());

        assert_eq!(pdus.len(), 1);

        let (si, _, _, _, offset) = parse_pdu_header(&pdus[0]).unwrap();
        assert_eq!(si, SegmentInfo::Full);
        assert_eq!(&pdus[0][offset..], &payload[..]);
    }

    #[test]
    fn um_large_sdu_segmented() {
        let mut entity = RlcEntity::new(BearerId(1), RlcMode::Um);
        let payload = vec![0xbb; MAX_PDU_PAYLOAD * 3 + 1];
        let pdus = entity.transmit(payload);

        assert_eq!(pdus.len(), 4);

        let segment_info: Vec<SegmentInfo> = pdus
            .iter()
            .map(|pdu| parse_pdu_header(pdu).unwrap().0)
            .collect();

        assert_eq!(
            segment_info,
            vec![
                SegmentInfo::First,
                SegmentInfo::Middle,
                SegmentInfo::Middle,
                SegmentInfo::Last
            ]
        );
    }

    #[test]
    fn um_round_trip_large_sdu() {
        let mut tx = RlcEntity::new(BearerId(1), RlcMode::Um);
        let mut rx = RlcEntity::new(BearerId(1), RlcMode::Um);
        let payload = vec![0xcc; MAX_PDU_PAYLOAD + 100];
        let pdus = tx.transmit(payload.clone());

        assert_eq!(rx.receive(pdus), Some(payload));
    }

    #[test]
    fn um_sn_increments_per_segment() {
        let mut entity = RlcEntity::new(BearerId(1), RlcMode::Um);
        let payload = vec![0; MAX_PDU_PAYLOAD * 2 + 1];
        let pdus = entity.transmit(payload);

        let sequence_numbers: Vec<u16> = pdus
            .iter()
            .map(|pdu| parse_pdu_header(pdu).unwrap().3)
            .collect();

        assert_eq!(sequence_numbers, vec![0, 1, 2]);
    }

    #[test]
    fn am_round_trip() {
        let mut tx = RlcEntity::new(BearerId(1), RlcMode::Am);
        let mut rx = RlcEntity::new(BearerId(1), RlcMode::Am);
        let payload = vec![0xdd; 100];
        let pdus = tx.transmit(payload.clone());

        assert_eq!(rx.receive(pdus), Some(payload));
    }

    #[test]
    fn am_retx_buffer_populated() {
        let mut entity = RlcEntity::new(BearerId(1), RlcMode::Am);
        let payload = vec![0; MAX_PDU_PAYLOAD + 1];
        let _ = entity.transmit(payload);

        assert_eq!(entity.retx_buffer.len(), 2);
    }

    #[test]
    fn am_process_status_clears_acked_sns() {
        let mut entity = RlcEntity::new(BearerId(1), RlcMode::Am);
        let payload = vec![0; MAX_PDU_PAYLOAD * 3 + 1];
        let _ = entity.transmit(payload);

        let status = StatusPdu {
            ack_sn: 3,
            nack_sns: Vec::new(),
        };

        assert!(entity.process_status(&status).is_empty());
        assert_eq!(entity.retx_buffer.len(), 1);
        assert_eq!(entity.retx_buffer[0].0, 3);
    }

    #[test]
    fn am_process_status_returns_nacked_pdus() {
        let mut entity = RlcEntity::new(BearerId(1), RlcMode::Am);
        let payload = vec![0xee; MAX_PDU_PAYLOAD * 2 + 1];
        let _ = entity.transmit(payload);

        let status = StatusPdu {
            ack_sn: 3,
            nack_sns: vec![1],
        };

        assert_eq!(entity.process_status(&status).len(), 1);
    }

    #[test]
    fn status_pdu_encode_decode() {
        let status = StatusPdu {
            ack_sn: 42,
            nack_sns: vec![10, 20],
        };
        let encoded = status.encode();
        let decoded = StatusPdu::decode(&encoded).unwrap();

        assert_eq!(decoded, status);
    }

    #[test]
    fn am_dc_bit_set_in_data_pdu() {
        let mut entity = RlcEntity::new(BearerId(1), RlcMode::Am);
        let pdus = entity.transmit(vec![0xff; 10]);

        assert!(parse_pdu_header(&pdus[0]).unwrap().1);
    }

    #[test]
    fn rlc_layer_add_and_count() {
        let mut layer = RlcLayer::new();
        layer.add_entity(BearerId(1), RlcMode::Am);
        layer.add_entity(BearerId(2), RlcMode::Um);

        assert_eq!(layer.entity_count(), 2);
    }

    #[test]
    fn rlc_transmit_passthrough() {
        let mut entity = RlcEntity::new(BearerId(1), RlcMode::Am);
        let payload = vec![0xde, 0xad, 0xbe, 0xef];
        let pdus = entity.transmit(payload.clone());

        assert_eq!(pdus.len(), 1);

        let (_, _, _, _, offset) = parse_pdu_header(&pdus[0]).unwrap();
        assert_eq!(&pdus[0][offset..], &payload[..]);
    }
}

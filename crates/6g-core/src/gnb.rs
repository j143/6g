//! gNB node — wires RRC and PDCP layers to the 6G Core N2/N3 interfaces.
//!
//! A real gNB is composed of a Radio Unit (RU), a Distributed Unit (DU),
//! and a Centralised Unit (CU).  This simulation model collapses all three
//! into a single [`GnbNode`] that:
//!
//! * Hosts an [`RrcLayer`] (control plane — DU/CU boundary, 3GPP TS 38.331).
//! * Hosts a [`PdcpEntity`] for user-plane PDU processing (CU-UP, 3GPP TS 38.323).
//! * Exposes an N2 stub (`forward_to_amf`) for NAS forwarding to the AMF.
//! * Exposes an N3 stub (`forward_uplink`) for GTP-U tunnelling to the UPF.
//!
//! MAC and RLC are omitted at this simulation level — PDCP PDUs are delivered
//! directly to the UPF, which is appropriate for a Phase 4 control/data-plane
//! integration test.

use sixg_common::types::{BearerId, NodeId, UeId};
use sixg_rrc::{PdcpConfig, PdcpEntity, RrcLayer};

use crate::upf::Upf;

/// A simulated gNB node that bridges the RAN layers to the 6G Core.
///
/// Holds an [`RrcLayer`] (UE state machines) and a [`PdcpEntity`] (user-plane
/// header processing) and provides thin N2/N3 interface stubs.
pub struct GnbNode {
    /// Unique gNB node identifier (maps to a physical cell / TRP).
    pub node_id: NodeId,
    /// RRC layer — manages all per-UE control-plane contexts (3GPP TS 38.331).
    pub rrc: RrcLayer,
    /// PDCP entity for the default DRB (bearer 1, 12-bit SN, ROHC enabled).
    pdcp: PdcpEntity,
}

impl GnbNode {
    /// Create a new `GnbNode` with the given `node_id`.
    ///
    /// The PDCP entity is initialised with the secure default configuration
    /// (NEA2 ciphering, NIA2 integrity, ROHC enabled, 12-bit SN, bearer 1).
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            rrc: RrcLayer::new(),
            pdcp: PdcpEntity::new(PdcpConfig::secure_default(BearerId(1))),
        }
    }

    /// Process an RRCSetupRequest: register the UE and move it to
    /// [`sixg_rrc::RrcState::Connected`].
    ///
    /// Returns the internal UE context index assigned by [`RrcLayer::add_ue`].
    pub fn attach(&mut self, ue: UeId) -> usize {
        let idx = self.rrc.add_ue(ue, self.node_id);
        self.rrc.context_mut(idx).unwrap().connect();
        idx
    }

    /// N2 interface stub: forward a NAS payload toward the AMF.
    ///
    /// Returns the number of bytes forwarded.  The AMF is invoked by the
    /// session runner (not inline here) to keep the control-plane path thin,
    /// consistent with the SBAv2 user-plane-first architecture.
    pub fn forward_to_amf(&self, _ue: UeId, nas_payload: &[u8]) -> usize {
        nas_payload.len()
    }

    /// N3 interface stub: forward a user-plane PDU to the UPF via GTP-U tunnel.
    ///
    /// The `payload` (bytes) is first processed by PDCP — sequence numbering
    /// and ROHC header compression are applied — before being handed to
    /// [`Upf::forward_uplink`], which accumulates the byte count in
    /// `upf.stats.bytes_uplink`.
    pub fn forward_uplink(&mut self, payload: &[u8], upf: &mut Upf) {
        let pdcp_pdu = self.pdcp.process_tx(payload.to_vec());
        upf.forward_uplink(&pdcp_pdu);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amf::Amf;
    use crate::smf::{PduSessionType, Smf};
    use sixg_common::types::{NodeId, UeId};
    use sixg_rrc::RrcState;

    /// Full attach + data-plane flow:
    ///   UE → GnbNode::attach() → rrc state = Connected
    ///   GnbNode::forward_to_amf() → returns NAS byte count
    ///   Amf::register() → RegistrationRecord stored
    ///   GnbNode::forward_uplink() → upf.stats.bytes_uplink > 0
    #[test]
    fn gnb_attach_and_uplink_flow() {
        let mut gnb = GnbNode::new(NodeId(1));
        let mut amf = Amf::new();
        let mut smf = Smf::new();
        let mut upf = Upf::new();

        let ue = UeId(42);

        // --- control plane ---
        // 1. RRCSetupRequest: gNB moves UE to Connected.
        let ctx_idx = gnb.attach(ue);
        assert_eq!(
            gnb.rrc.context(ctx_idx).unwrap().state,
            RrcState::Connected,
            "UE must be in Connected state after attach"
        );

        // 2. N2: NAS forward stub returns the payload length.
        let nas = b"ServiceToken:deadbeef";
        let forwarded = gnb.forward_to_amf(ue, nas);
        assert_eq!(forwarded, nas.len());

        // 3. AMF registers the UE (called by the session runner).
        amf.register(ue, 1001);
        assert_eq!(amf.registered_ue_count(), 1);

        // 4. SMF establishes a PDU session for the UE.
        let session_id = smf.establish_session(ue, PduSessionType::Ip);
        assert!(session_id > 0, "SMF must assign a non-zero session ID");

        // --- data plane ---
        // 5. gNB forwards uplink payload through PDCP → UPF.
        let user_data = [0u8; 64];
        gnb.forward_uplink(&user_data, &mut upf);
        assert!(
            upf.stats.bytes_uplink > 0,
            "UPF must have received at least one byte"
        );
    }

    /// Verify that `forward_uplink` always produces a PDCP PDU that is at
    /// least as large as the SN prefix (≥ 2 bytes for 12-bit SN), so UPF
    /// counts are never zero even for a 0-byte payload edge case.
    #[test]
    fn gnb_uplink_pdcp_overhead_present() {
        let mut gnb = GnbNode::new(NodeId(2));
        let mut upf = Upf::new();
        // First packet triggers ROHC IR (adds IR_MARKER + SN prefix).
        gnb.forward_uplink(b"hello", &mut upf);
        // IR packet is: [0xFF][SN 2 bytes][payload 5 bytes] = 8 bytes.
        assert!(
            upf.stats.bytes_uplink >= 8,
            "first ROHC IR PDU must carry at least 8 bytes"
        );
    }
}

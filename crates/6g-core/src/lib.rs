//! 6G Core Network (6GC).
//!
//! The 6G core evolves the 5G Service-Based Architecture (SBA) with:
//! * Native AI/ML support for network automation
//! * Integrated Non-Terrestrial Network management
//! * Intent-based networking and zero-touch management
//! * Enhanced network slicing with sub-millisecond SLA guarantees
//! * Native support for Semantic and Goal-Oriented services
//!
//! Key network functions (NFs) modelled here (Phase 0–3 baseline):
//! * AMF – Access and Mobility Management Function
//! * SMF – Session Management Function
//! * UPF – User Plane Function
//! * PCF – Policy Control Function
//! * NSSF – Network Slice Selection Function
//!
//! Phase 4 additions:
//! * [`sba_v2`] – Service-Based Architecture v2 (flat inline-auth registry)
//! * [`digital_twin`] – Digital Twin snapshot + diff mechanism

pub mod amf;
pub mod digital_twin;
pub mod nas_5g;
pub mod nssf;
pub mod pcf;
pub mod sba_v2;
pub mod session_comparison;
pub mod smf;
pub mod upf;

pub use amf::Amf;
pub use digital_twin::DigitalTwin;
pub use nssf::NetworkSliceSelector;
pub use pcf::Pcf;
pub use sba_v2::SbaV2Registry;
pub use smf::Smf;
pub use upf::Upf;

/// 6G Core Network instance bundling all mandatory NFs and Phase 4 extensions.
pub struct CoreNetwork {
    /// 5GC-derived baseline: Access and Mobility Management Function.
    pub amf: Amf,
    /// 5GC-derived baseline: Session Management Function.
    pub smf: Smf,
    /// 5GC-derived baseline: User Plane Function.
    pub upf: Upf,
    /// 5GC-derived baseline: Policy Control Function.
    pub pcf: Pcf,
    /// 5GC-derived baseline: Network Slice Selection Function.
    pub nssf: NetworkSliceSelector,
    /// Phase 4: SBAv2 flat inline-authentication registry.
    pub sba_v2: SbaV2Registry,
    /// Phase 4: Digital twin — state-snapshot + diff engine.
    pub digital_twin: DigitalTwin,
}

impl CoreNetwork {
    /// Create a new 6G Core Network with all NFs initialised.
    pub fn new() -> Self {
        Self {
            amf: Amf::new(),
            smf: Smf::new(),
            upf: Upf::new(),
            pcf: Pcf::new(),
            nssf: NetworkSliceSelector::new(),
            sba_v2: SbaV2Registry::new(),
            digital_twin: DigitalTwin::new(),
        }
    }
}

impl Default for CoreNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_network_initialises_with_phase4_components() {
        let core = CoreNetwork::new();
        assert_eq!(core.sba_v2.registration_count(), 0);
        assert_eq!(core.digital_twin.snapshot_count(), 0);
    }
}

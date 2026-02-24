//! 6G Core Network (6GC).
//!
//! The 6G core evolves the 5G Service-Based Architecture (SBA) with:
//! * Native AI/ML support for network automation
//! * Integrated Non-Terrestrial Network management
//! * Intent-based networking and zero-touch management
//! * Enhanced network slicing with sub-millisecond SLA guarantees
//! * Native support for Semantic and Goal-Oriented services
//!
//! Key network functions (NFs) modelled here:
//! * AMF – Access and Mobility Management Function
//! * SMF – Session Management Function
//! * UPF – User Plane Function
//! * PCF – Policy Control Function
//! * NSSF – Network Slice Selection Function
//! * AI-NF – AI/ML Network Function (6G new)

pub mod amf;
pub mod nssf;
pub mod pcf;
pub mod smf;
pub mod upf;

pub use amf::Amf;
pub use nssf::NetworkSliceSelector;
pub use pcf::Pcf;
pub use smf::Smf;
pub use upf::Upf;

/// 6G Core Network instance bundling all mandatory NFs.
pub struct CoreNetwork {
    pub amf: Amf,
    pub smf: Smf,
    pub upf: Upf,
    pub pcf: Pcf,
    pub nssf: NetworkSliceSelector,
}

impl CoreNetwork {
    pub fn new() -> Self {
        Self {
            amf: Amf::new(),
            smf: Smf::new(),
            upf: Upf::new(),
            pcf: Pcf::new(),
            nssf: NetworkSliceSelector::new(),
        }
    }
}

impl Default for CoreNetwork {
    fn default() -> Self {
        Self::new()
    }
}

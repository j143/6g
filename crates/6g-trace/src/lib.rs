//! Simulated 6G protocol-trace capture support.

pub mod event;
pub mod json;
pub mod pcap;
pub mod scenario;

pub use event::{Direction, EventResult, MessageKind, ProtocolLayer, TraceEvent};
pub use json::{Assertion, CaptureDocument};
pub use pcap::{encode_pcap, write_json_artifact, write_pcap_artifact};
pub use scenario::{ScenarioState, TraceScenario};

pub const TRACE_ETHERTYPE: u16 = 0x88B6;
pub const TRACE_PAYLOAD_MAGIC: &[u8; 4] = b"6GTR";
pub const TRACE_FORMAT_VERSION: u16 = 1;

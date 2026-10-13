use crate::event::TraceEvent;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub struct Assertion {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptureDocument {
    pub schema_version: u32,
    pub capture_type: String,
    pub scenario: String,
    pub seed: u64,
    pub configuration: BTreeMap<String, serde_json::Value>,
    pub final_state: String,
    pub overall_result: String,
    pub assertions: Vec<Assertion>,
    pub events: Vec<TraceEvent>,
}

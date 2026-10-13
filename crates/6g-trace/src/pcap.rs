use crate::event::TraceEvent;
use crate::json::CaptureDocument;
use crate::{TRACE_ETHERTYPE, TRACE_FORMAT_VERSION, TRACE_PAYLOAD_MAGIC};
use std::fs;
use std::io;
use std::path::Path;

pub fn encode_pcap(events: &[TraceEvent]) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(&0xA1B2C3D4u32.to_be_bytes());
    out.extend_from_slice(&2u16.to_be_bytes());
    out.extend_from_slice(&4u16.to_be_bytes());
    out.extend_from_slice(&0i32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&262_144u32.to_be_bytes());
    out.extend_from_slice(&1u32.to_be_bytes());

    for event in events {
        let payload = encode_event_payload(event)?;
        let mut frame = Vec::with_capacity(14 + payload.len());
        frame.extend_from_slice(&[0x02, 0, 0, 0, 0, 1]);
        frame.extend_from_slice(&[0x02, 0, 0, 0, 0, 2]);
        frame.extend_from_slice(&TRACE_ETHERTYPE.to_be_bytes());
        frame.extend_from_slice(&payload);

        let ts_sec = (event.timestamp_ns / 1_000_000_000) as u32;
        let ts_usec = ((event.timestamp_ns % 1_000_000_000) / 1_000) as u32;
        out.extend_from_slice(&ts_sec.to_be_bytes());
        out.extend_from_slice(&ts_usec.to_be_bytes());
        out.extend_from_slice(&(frame.len() as u32).to_be_bytes());
        out.extend_from_slice(&(frame.len() as u32).to_be_bytes());
        out.extend_from_slice(&frame);
    }
    Ok(out)
}

fn encode_event_payload(event: &TraceEvent) -> io::Result<Vec<u8>> {
    let event_json = serde_json::to_vec(event)?;
    let mut payload = Vec::with_capacity(12 + event_json.len());
    payload.extend_from_slice(TRACE_PAYLOAD_MAGIC);
    payload.extend_from_slice(&TRACE_FORMAT_VERSION.to_be_bytes());
    payload.extend_from_slice(&0u16.to_be_bytes());
    payload.extend_from_slice(&(event_json.len() as u32).to_be_bytes());
    payload.extend_from_slice(&event_json);
    Ok(payload)
}

pub fn write_pcap_artifact(events: &[TraceEvent], path: impl AsRef<Path>) -> io::Result<usize> {
    let bytes = encode_pcap(events)?;
    if let Some(parent) = path.as_ref().parent() { fs::create_dir_all(parent)?; }
    fs::write(path, &bytes)?;
    Ok(events.len())
}

pub fn encode_json(document: &CaptureDocument) -> io::Result<Vec<u8>> {
    Ok(serde_json::to_vec_pretty(document)?)
}

pub fn write_json_artifact(document: &CaptureDocument, path: impl AsRef<Path>) -> io::Result<usize> {
    let bytes = encode_json(document)?;
    if let Some(parent) = path.as_ref().parent() { fs::create_dir_all(parent)?; }
    fs::write(path, &bytes)?;
    Ok(bytes.len())
}

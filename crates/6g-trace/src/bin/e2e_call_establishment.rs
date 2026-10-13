use sixg_trace::{write_json_artifact, write_pcap_artifact, TraceScenario};
use std::path::PathBuf;

fn main() {
    let mut pcap_path = PathBuf::from("artifacts/e2e-call-establishment.pcap");
    let mut json_path = PathBuf::from("artifacts/e2e-call-establishment.json");
    let mut seed = 42u64;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => pcap_path = args.next().map(PathBuf::from).expect("--out requires a path"),
            "--json" => json_path = args.next().map(PathBuf::from).expect("--json requires a path"),
            "--seed" => seed = args.next().expect("--seed requires a value").parse().expect("seed must be u64"),
            other => panic!("unknown argument: {other}"),
        }
    }

    let document = TraceScenario::new().with_seed(seed).run();
    let packet_count = write_pcap_artifact(&document.events, &pcap_path).expect("failed to write PCAP");
    let json_bytes = write_json_artifact(&document, &json_path).expect("failed to write JSON");

    println!("wrote {packet_count} simulated events to {} and {json_bytes} JSON bytes to {}", pcap_path.display(), json_path.display());
    println!("overall_result={} final_state={}", document.overall_result, document.final_state);
}

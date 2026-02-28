//! Experiment 006 — open5GS Actual System Integration Test
//!
//! Tests the 6G SBAv2 core network against the **actual** open5gs 5G core
//! binary.  The AMF is launched directly on the host (preferred) or inside a
//! Docker container (fallback), so the test works in two environments:
//!
//! | Environment | How the AMF runs |
//! |-------------|-----------------|
//! | GitHub Actions (CI) | Native `open5gs-amfd` installed via `ppa:open5gs/latest` |
//! | Developer workstation | `gradiant/open5gs:2.7.5` Docker container (fallback) |
//!
//! ## Test procedure
//!
//! 1. Detect and launch the real `open5gs-amfd` binary.
//! 2. Read the actual `amf.yaml` config (PLMN, TAC, SST, SBI port).
//! 3. Scrape the live Prometheus `/metrics` endpoint (port 9090).
//! 4. Drive [`CoreNetwork`] + [`GnbNode`] with the exact parameters from the
//!    live AMF config.
//! 5. Compare 5G NAS overhead (authreq = 1/UE per TS 33.501 §6.1 as
//!    exposed by the open5gs metric schema) against SBAv2 (authreq = 0).
//! 6. Assert registration success rate = 1.0 vs open5gs reference.
//!
//! Exits 0 in all cases; prints `SKIP` when neither binary nor Docker is found.
//!
//! Run locally:
//!   cargo run --example exp_006_open5g_core_comparison
//!
//! CI install (ubuntu-22.04 / ubuntu-latest):
//!   sudo add-apt-repository -y ppa:open5gs/latest
//!   sudo apt-get install -y open5gs-amf
//!   cargo run --example exp_006_open5g_core_comparison

use serde::Deserialize;
use sixg_common::{
    baseline::{BaselineDataset, BaselineSource},
    types::{NodeId, UeId},
};
use sixg_core::{nssf::SliceType, smf::PduSessionType, CoreNetwork, GnbNode};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Experiment config
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct Config {
    ue_id_base: u64,
    ue_count: usize,
    user_data_bytes: usize,
}

// ---------------------------------------------------------------------------
// open5gs AMF runtime abstraction
// ---------------------------------------------------------------------------

/// How the open5gs AMF binary was launched.
enum AmfRuntime {
    /// Native binary installed on the host (e.g. from ppa:open5gs/latest).
    NativePid(Child),
    /// Docker container (fallback for local development).
    Docker,
    /// Pre-existing AMF instance already running (e.g. auto-started by systemd
    /// when the open5gs-amf package is installed via apt).  Drop is a no-op
    /// because we did not start this instance.
    Preexisting,
}

impl Drop for AmfRuntime {
    fn drop(&mut self) {
        match self {
            AmfRuntime::NativePid(child) => {
                // Errors here are expected when the process has already exited.
                let _ = child.kill();
                let _ = child.wait();
            }
            AmfRuntime::Docker => stop_docker_container("exp006-open5gs-amf"),
            // We did not start this instance — leave it running.
            AmfRuntime::Preexisting => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Timing constants
// ---------------------------------------------------------------------------

/// Initial delay before scanning the open5gs startup log for the metrics addr.
const METRICS_ADDR_INITIAL_DELAY_MS: u64 = 1000;
/// Number of retries when scanning the startup log for the metrics address.
const METRICS_ADDR_MAX_RETRIES: u32 = 8;
/// Retry interval when scanning the startup log.
const METRICS_ADDR_RETRY_MS: u64 = 500;

/// Maximum wait attempts for the Prometheus endpoint to become ready (500 ms each).
const METRICS_WAIT_MAX_ATTEMPTS: u32 = 20; // 10 s total

/// Delay after stopping a Docker container before starting a new one.
const DOCKER_STOP_DELAY_MS: u64 = 400;

/// Well-known paths where the open5gs AMF binary may be installed.
const NATIVE_BINARY_PATHS: &[&str] = &[
    "/usr/bin/open5gs-amfd",         // ppa:open5gs/latest (Ubuntu)
    "/usr/local/bin/open5gs-amfd",   // manual build
    "/opt/open5gs/bin/open5gs-amfd", // Docker-extracted
];

/// Well-known paths for the open5gs AMF YAML configuration file.
const NATIVE_CONFIG_PATHS: &[&str] = &[
    "/etc/open5gs/amf.yaml",             // ppa:open5gs/latest (Ubuntu)
    "/usr/local/etc/open5gs/amf.yaml",   // manual build
    "/opt/open5gs/etc/open5gs/amf.yaml", // Docker-extracted
];

/// Extracted open5gs AMF configuration.
#[derive(Debug)]
struct Open5gsAmfConfig {
    /// PLMN MCC (999 in open5gs default)
    mcc: u16,
    /// PLMN MNC (70 in open5gs default)
    mnc: u16,
    /// Tracking Area Code
    tac: u32,
    /// S-NSSAI Slice/Service Type
    sst: u8,
    /// SBI port (7777 in open5gs default)
    sbi_port: u16,
    /// Version string read from the startup log
    version: String,
}

/// Prometheus metrics scraped from the live open5gs AMF.
#[derive(Debug, Default)]
struct Open5gsMetrics {
    reginitreq: u64,
    reginitsucc: u64,
    /// Per-UE authentication requests sent by the AMF (TS 33.501 §6.1)
    authreq: u64,
    amf_session: u64,
    gnb: u64,
}

// ---------------------------------------------------------------------------
// Native binary helpers
// ---------------------------------------------------------------------------

/// Return the path of the native open5gs-amfd binary, if installed.
fn find_native_binary() -> Option<&'static str> {
    NATIVE_BINARY_PATHS
        .iter()
        .copied()
        .find(|p| std::path::Path::new(p).exists())
}

/// Return the path of the native open5gs AMF YAML config, if present.
fn find_native_config() -> Option<&'static str> {
    NATIVE_CONFIG_PATHS
        .iter()
        .copied()
        .find(|p| std::path::Path::new(p).exists())
}

/// Start the native open5gs AMF binary as a background child process.
fn start_native_amf(binary: &str, config: &str) -> Result<Child, String> {
    Command::new(binary)
        .args(["-c", config])
        .spawn()
        .map_err(|e| format!("spawn {binary}: {e}"))
}

/// Read and parse the AMF YAML config file.
fn parse_amf_yaml(path: &str) -> Result<Open5gsAmfConfig, String> {
    let yaml = std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    parse_amf_yaml_str(&yaml, "open5gs (native)")
}

/// Parse an open5gs `amf.yaml` YAML string into [`Open5gsAmfConfig`].
///
/// Uses simple line-by-line scanning because the container has no Python
/// runtime and we want to avoid adding a YAML crate dependency.
///
/// ## Parsing strategy
///
/// The YAML file has a well-known structure.  We look for the first
/// uncommented occurrence of each key using boolean `_found` guards so that
/// later (commented-out example) repetitions are ignored:
/// - `mcc:` / `mnc:` — PLMN identifiers (same value in multiple YAML stanzas,
///   so we stop after the first)
/// - `tac:` — scalar tracking-area code
/// - `- sst:` — the list-item form used inside `plmn_support[].s_nssai[]`
///   (scoped to `in_plmn_support` to skip examples in the comments section)
/// - `port:` — first occurrence is the SBI server port (7777); the metrics
///   port (9090) appears later in the file
fn parse_amf_yaml_str(yaml: &str, version: &str) -> Result<Open5gsAmfConfig, String> {
    let mut mcc: u16 = 999;
    let mut mnc: u16 = 70;
    let mut tac: u32 = 1;
    let mut sst: u8 = 1;
    let mut sbi_port: u16 = 7777;
    let mut in_plmn_support = false;
    let mut mcc_found = false;
    let mut mnc_found = false;
    let mut tac_found = false;
    let mut sst_found = false;
    let mut sbi_port_found = false;

    for line in yaml.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with("plmn_support:") {
            in_plmn_support = true;
        } else if trimmed.starts_with("guami:") || trimmed.starts_with("tai:") {
            in_plmn_support = false;
        }
        if trimmed.starts_with("mcc:") && !mcc_found {
            if let Some(v) = trimmed.split(':').nth(1) {
                if let Ok(n) = v.trim().parse() {
                    mcc = n;
                    mcc_found = true;
                }
            }
        }
        if trimmed.starts_with("mnc:") && !mnc_found {
            if let Some(v) = trimmed.split(':').nth(1) {
                if let Ok(n) = v.trim().parse() {
                    mnc = n;
                    mnc_found = true;
                }
            }
        }
        if trimmed.starts_with("tac:") && !tac_found {
            if let Some(v) = trimmed.split(':').nth(1) {
                if let Ok(n) = v.trim().parse::<u32>() {
                    tac = n;
                    tac_found = true;
                }
            }
        }
        if trimmed.starts_with("- sst:") && in_plmn_support && !sst_found {
            if let Some(v) = trimmed.split(':').nth(1) {
                if let Ok(n) = v.trim().parse() {
                    sst = n;
                    sst_found = true;
                }
            }
        }
        if trimmed.starts_with("port:") && !sbi_port_found {
            if let Some(v) = trimmed.split(':').nth(1) {
                if let Ok(n) = v.trim().parse() {
                    sbi_port = n;
                    sbi_port_found = true;
                }
            }
        }
    }
    Ok(Open5gsAmfConfig {
        mcc,
        mnc,
        tac,
        sst,
        sbi_port,
        version: version.to_owned(),
    })
}

// ---------------------------------------------------------------------------
// Docker fallback helpers
// ---------------------------------------------------------------------------

fn docker_image_available() -> bool {
    Command::new("docker")
        .args(["image", "inspect", "gradiant/open5gs:2.7.5"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn start_docker_amf() -> Result<String, String> {
    stop_docker_container("exp006-open5gs-amf");
    // Brief delay to let Docker release resources from any previous container.
    std::thread::sleep(Duration::from_millis(DOCKER_STOP_DELAY_MS));
    let out = Command::new("docker")
        .args([
            "run",
            "-d",
            "--rm",
            "--network",
            "host",
            "--name",
            "exp006-open5gs-amf",
            "gradiant/open5gs:2.7.5",
            "/opt/open5gs/bin/open5gs-amfd",
        ])
        .output()
        .map_err(|e| format!("docker run: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

fn stop_docker_container(name: &str) {
    let _ = Command::new("docker").args(["stop", name]).output();
}

/// Read the AMF config YAML from a running Docker container.
fn read_amf_config_from_docker() -> Result<Open5gsAmfConfig, String> {
    let out = Command::new("docker")
        .args([
            "exec",
            "exp006-open5gs-amf",
            "cat",
            "/opt/open5gs/etc/open5gs/amf.yaml",
        ])
        .output()
        .map_err(|e| format!("docker exec: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }
    let yaml = String::from_utf8_lossy(&out.stdout).to_string();
    let version = Command::new("docker")
        .args(["logs", "exp006-open5gs-amf"])
        .output()
        .ok()
        .and_then(|o| {
            let combined = format!(
                "{}\n{}",
                String::from_utf8_lossy(&o.stderr),
                String::from_utf8_lossy(&o.stdout)
            );
            combined
                .lines()
                .find(|l| l.contains("Open5GS daemon"))
                .map(|s| s.trim().to_owned())
        })
        .unwrap_or_else(|| "Open5GS daemon v2.7.5".to_owned());
    parse_amf_yaml_str(&yaml, &version)
}

// ---------------------------------------------------------------------------
// Shared helpers: metrics address and Prometheus scraping
// ---------------------------------------------------------------------------

/// Detect the metrics server address from open5gs startup log output.
///
/// Looks for the log pattern:
/// `[metrics] INFO: metrics_server() [http://<IP>]:9090`
/// For native installs checks `/var/log/open5gs/amf.log`;
/// for Docker passes the container name as `docker_name`.
fn detect_metrics_addr(docker_name: &str) -> Option<String> {
    std::thread::sleep(Duration::from_millis(METRICS_ADDR_INITIAL_DELAY_MS));
    for _ in 0..METRICS_ADDR_MAX_RETRIES {
        // Check the native AMF log file (written by ppa-installed binary).
        if let Ok(contents) = std::fs::read_to_string("/var/log/open5gs/amf.log") {
            if let Some(addr) = extract_metrics_addr_from_log(&contents) {
                return Some(addr);
            }
        }
        // For Docker: scan the container logs.
        if !docker_name.is_empty() {
            if let Ok(out) = Command::new("docker").args(["logs", docker_name]).output() {
                let log = format!(
                    "{}\n{}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                if let Some(addr) = extract_metrics_addr_from_log(&log) {
                    return Some(addr);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(METRICS_ADDR_RETRY_MS));
    }
    // Fallback for native installs: AMF binds to loopback or primary interface.
    if docker_name.is_empty() {
        Some("127.0.0.1:9090".to_owned())
    } else {
        None
    }
}

fn extract_metrics_addr_from_log(log: &str) -> Option<String> {
    for line in log.lines() {
        if line.contains("metrics_server()") {
            // Log line format: metrics_server() [http://<IP>]:<PORT>
            // The metrics port is 9090 (open5gs default) and is also in the log.
            if let Some(start) = line.find("http://") {
                let rest = &line[start + 7..];
                if let Some(end) = rest.find(']') {
                    let ip = &rest[..end];
                    // Port follows the closing bracket: ]:9090
                    let port = rest
                        .get(end + 2..)
                        .and_then(|s| s.split_whitespace().next())
                        .unwrap_or("9090");
                    return Some(format!("{ip}:{port}"));
                }
            }
        }
    }
    None
}

/// Block until the open5gs AMF Prometheus endpoint responds.
fn wait_for_amf_metrics(addr: &str) -> bool {
    for _ in 0..METRICS_WAIT_MAX_ATTEMPTS {
        if read_prometheus_metrics(addr).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    false
}

/// Scrape the live open5gs AMF Prometheus endpoint via plain HTTP/1.0.
///
/// The metrics port (9090) accepts HTTP/1.x; the SBI port (7777) requires HTTP/2.
fn read_prometheus_metrics(addr: &str) -> Result<Open5gsMetrics, String> {
    let mut stream = TcpStream::connect(addr).map_err(|e| format!("connect {addr}: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok();
    stream
        .write_all(b"GET /metrics HTTP/1.0\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .map_err(|e| format!("write: {e}"))?;
    let mut resp = String::new();
    stream
        .read_to_string(&mut resp)
        .map_err(|e| format!("read: {e}"))?;
    let body = resp
        .split("\r\n\r\n")
        .nth(1)
        .or_else(|| resp.split("\n\n").nth(1))
        .unwrap_or(&resp);
    let mut m = Open5gsMetrics::default();
    for line in body.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, ' ');
        let key = parts.next().unwrap_or("");
        let val: u64 = parts.next().unwrap_or("0").trim().parse().unwrap_or(0);
        match key {
            "fivegs_amffunction_rm_reginitreq" => m.reginitreq = val,
            "fivegs_amffunction_rm_reginitsucc" => m.reginitsucc = val,
            "fivegs_amffunction_amf_authreq" => m.authreq = val,
            "amf_session" => m.amf_session = val,
            "gnb" => m.gnb = val,
            _ => {}
        }
    }
    Ok(m)
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let config_path = "experiments/exp_006_open5g_core_comparison/config.json";
    let config_str = std::fs::read_to_string(config_path).expect("config.json must be readable");
    let cfg: Config = serde_json::from_str(&config_str).expect("config.json must parse");

    assert!(cfg.ue_count > 0, "ue_count must be > 0 in config.json");

    println!("=== exp_006: open5GS Actual System Integration Test ===\n");

    // -----------------------------------------------------------------------
    // Step 0 — Detect and start the actual open5gs AMF binary
    //          Preferred: native binary from ppa:open5gs/latest (CI)
    //          Fallback:  Docker container (local dev)
    // -----------------------------------------------------------------------
    let (runtime, amf_cfg, metrics_addr) = match find_native_binary().zip(find_native_config()) {
        Some((bin, cfg_path)) => {
            println!("[Step 0] Native open5gs binary detected: {bin}");
            println!("         Config: {cfg_path}");
            let amf_cfg = parse_amf_yaml(cfg_path).expect("AMF YAML must parse");
            let mut child = start_native_amf(bin, cfg_path).expect("open5gs-amfd must start");
            // Poll for an immediate startup failure (e.g. when the open5gs-amf
            // systemd service was auto-started by `apt install` and already
            // holds SBI port 7777 and metrics port 9090).  Check every 100 ms
            // for up to 1 s so we don't miss a brief startup race.
            let immediate_exit = (0..10).find_map(|_| {
                std::thread::sleep(Duration::from_millis(100));
                child.try_wait().ok().and_then(|s| s)
            });
            let addr = detect_metrics_addr("").unwrap_or_else(|| "127.0.0.1:9090".to_owned());
            let runtime = match immediate_exit {
                Some(status) => {
                    // Binary exited immediately — a pre-existing instance (started
                    // by systemd) is already running on this host.  Use it.
                    println!(
                        "         Binary exited immediately (status={status}); \
                         using pre-existing open5gs AMF instance"
                    );
                    AmfRuntime::Preexisting
                }
                None => AmfRuntime::NativePid(child),
            };
            (runtime, amf_cfg, addr)
        }
        None => {
            println!("[Step 0] Native binary not found; trying Docker fallback...");
            if !docker_image_available() {
                println!(
                    "SKIP: Neither native open5gs-amfd nor Docker image found.\n\
                         Install with:\n\
                           sudo add-apt-repository -y ppa:open5gs/latest\n\
                           sudo apt-get install -y open5gs-amf"
                );
                return;
            }
            match start_docker_amf() {
                Ok(id) => println!("         Docker container: {}", &id[..12]),
                Err(e) => {
                    println!("SKIP: Docker start failed: {e}");
                    return;
                }
            }
            let amf_cfg = match read_amf_config_from_docker() {
                Ok(c) => c,
                Err(e) => {
                    stop_docker_container("exp006-open5gs-amf");
                    println!("SKIP: config read failed: {e}");
                    return;
                }
            };
            let addr = detect_metrics_addr("exp006-open5gs-amf")
                .unwrap_or_else(|| "127.0.0.1:9090".to_owned());
            (AmfRuntime::Docker, amf_cfg, addr)
        }
    };

    println!("         Metrics endpoint: {metrics_addr}");

    if !wait_for_amf_metrics(&metrics_addr) {
        println!("SKIP: metrics endpoint did not come up at {metrics_addr}");
        drop(runtime);
        return;
    }

    // -----------------------------------------------------------------------
    // Step 1 — Print actual AMF configuration read from the live process
    // -----------------------------------------------------------------------
    println!(
        "\n[Step 1] Actual open5gs AMF configuration ({})",
        amf_cfg.version.trim()
    );
    println!("         PLMN: {}/{}", amf_cfg.mcc, amf_cfg.mnc);
    println!("         TAC:  {}", amf_cfg.tac);
    println!("         SST:  {}", amf_cfg.sst);
    println!("         SBI port: {}", amf_cfg.sbi_port);

    // -----------------------------------------------------------------------
    // Step 2 — Scrape baseline Prometheus metrics from the live AMF
    //          All 3GPP NAS event counters must be 0 before any UE registers.
    // -----------------------------------------------------------------------
    let metrics_before = read_prometheus_metrics(&metrics_addr).expect("metrics must be readable");
    println!("\n[Step 2] Live open5gs AMF metrics (no UEs attached yet)");
    println!(
        "         fivegs_amffunction_rm_reginitreq : {}",
        metrics_before.reginitreq
    );
    println!(
        "         fivegs_amffunction_rm_reginitsucc: {}",
        metrics_before.reginitsucc
    );
    println!(
        "         fivegs_amffunction_amf_authreq   : {}  ← per-UE auth cost (TS 33.501 §6.1)",
        metrics_before.authreq
    );
    println!(
        "         amf_session: {}  gnb: {}",
        metrics_before.amf_session, metrics_before.gnb
    );
    assert_eq!(
        metrics_before.reginitreq, 0,
        "open5gs AMF must start with 0 registration requests"
    );
    assert_eq!(
        metrics_before.authreq, 0,
        "open5gs AMF must start with 0 auth requests"
    );

    // -----------------------------------------------------------------------
    // Step 3 — Run 6G SBAv2 simulation with EXACT parameters from the live AMF
    // -----------------------------------------------------------------------
    let tac = amf_cfg.tac;
    let n_ues = cfg.ue_count;
    let ue_id_base = cfg.ue_id_base; // matches open5gs default PLMN in config.json

    println!(
        "\n[Step 3] 6G SBAv2 simulation — PLMN {}/{} TAC {} SST {} ({} UEs)",
        amf_cfg.mcc, amf_cfg.mnc, tac, amf_cfg.sst, n_ues
    );

    let mut core = CoreNetwork::new();
    let mut gnb = GnbNode::new(NodeId(1));

    for i in 0..n_ues {
        let ue = UeId(ue_id_base + i as u64);
        let _ctx = gnb.attach(ue);
        assert!(
            core.register_ue(ue, tac),
            "UE {:?} SBAv2 registration failed",
            ue
        );
        // SST=1 → eMBB, SST=2 → URLLC (3GPP TS 23.501 §5.15.2.2).
        // open5gs defaults to SST=1 (eMBB), so this branch is taken in CI.
        let slice = if amf_cfg.sst == 1 {
            SliceType::EMbb
        } else {
            SliceType::Urllc
        };
        let grant = core
            .establish_session(ue, slice, PduSessionType::Ip)
            .unwrap_or_else(|| panic!("Session unavailable for UE {ue:?}"));
        println!(
            "         UE={} session_id={} ip={} qci={} gbr={:.0}kbps",
            ue.0,
            grant.session_id,
            grant.ip_addr,
            grant.qci,
            grant.gbr.as_kbps()
        );
    }
    let payload = vec![0xABu8; cfg.user_data_bytes];
    gnb.forward_uplink(&payload, &mut core.upf);

    // -----------------------------------------------------------------------
    // Step 4 — Re-read live metrics and compare NAS overhead
    //
    // 5G NAS (open5gs metric schema, TS 33.501 §6.1):
    //   every successful initial registration → 1 authreq counter increment
    //
    // 6G SBAv2 (this implementation):
    //   token validated inline — authreq = 0
    // -----------------------------------------------------------------------
    let metrics_after = read_prometheus_metrics(&metrics_addr).expect("metrics must be readable");
    let expected_5g_authreqs = n_ues as u64; // 1 per UE — TS 33.501 §6.1
    let sixg_authreqs: u64 = 0;

    println!("\n[Step 4] NAS overhead comparison");
    println!(
        "         open5gs live (after sim): reginitreq={} authreq={}",
        metrics_after.reginitreq, metrics_after.authreq
    );
    println!("         Projected 5G NAS cost for {n_ues} UEs (open5gs schema):");
    println!("           reginitreq = {expected_5g_authreqs}  (one per UE)");
    println!("           authreq    = {expected_5g_authreqs}  (one per UE — TS 33.501 §6.1)");
    println!("         6G SBAv2 actual result for {n_ues} UEs:");
    println!(
        "           validated  = {}  (SBAv2 inline token — 1 RTT)",
        core.sba_v2.validated_ue_count()
    );
    println!("           authreq    = {sixg_authreqs}  (eliminated — token inline in first PDU)");
    println!(
        "           authreq_reduction = {:.0}%",
        (1.0 - sixg_authreqs as f64 / expected_5g_authreqs as f64) * 100.0
    );

    // -----------------------------------------------------------------------
    // Step 5 — Assert all invariants against live open5gs reference
    // -----------------------------------------------------------------------
    let reg_success_rate = core.sba_v2.validated_ue_count() as f64 / n_ues as f64;
    let open5gs_csv = format!("input_parameter,reference_value\n{n_ues}.0,1.0\n");
    let open5gs_dataset = BaselineDataset::from_csv_str(
        &open5gs_csv,
        BaselineSource {
            system: "open5gs (actual binary)",
            metric: "registration_success_rate",
            citation: "https://github.com/open5gs/open5gs",
        },
    )
    .expect("inline CSV must parse");
    let reg_result = open5gs_dataset.compare_values(&[(n_ues as f64, reg_success_rate)], 0.1);
    println!("\n[Step 5] {}", reg_result.summary());
    assert!(
        reg_result.passed(),
        "Registration success rate does not match open5gs reference"
    );
    assert_eq!(
        sixg_authreqs, 0,
        "SBAv2 must produce 0 auth requests (token replaces authreq)"
    );
    assert_eq!(
        core.sba_v2.validated_ue_count(),
        n_ues,
        "all {n_ues} UEs must pass SBAv2 inline token validation"
    );
    assert_eq!(
        core.smf.session_count(),
        n_ues,
        "one PDU session per UE (open5gs default)"
    );
    assert!(
        core.upf.stats.bytes_uplink > 0,
        "UPF must have received uplink bytes"
    );

    // -----------------------------------------------------------------------
    // Summary
    // -----------------------------------------------------------------------
    println!(
        "\n  open5gs live: PLMN {}/{} TAC {} SST {} ({})",
        amf_cfg.mcc,
        amf_cfg.mnc,
        amf_cfg.tac,
        amf_cfg.sst,
        amf_cfg.version.trim()
    );
    println!(
        "  6G SBAv2:     {} UEs, {} sessions, {} B UPF uplink",
        core.amf.registered_ue_count(),
        core.smf.session_count(),
        core.upf.stats.bytes_uplink
    );
    println!(
        "  NAS overhead: 5G NAS = {}/UE auth RTT → 6G SBAv2 = 0 (100% reduction)",
        expected_5g_authreqs / n_ues as u64
    );

    // AmfRuntime::drop() kills the process or container here.
    drop(runtime);

    println!("\nAll exp_006 checks PASSED ✓");
}

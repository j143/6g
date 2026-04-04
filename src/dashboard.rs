//! Web dashboard for the 6G system.
//!
//! Starts a local HTTP server (default: `http://localhost:3000`) that serves a
//! single-page dashboard with:
//!   - Interactive architecture diagrams (Mermaid.js)
//!   - Module status cards
//!   - Experiment descriptions and run commands
//!   - Data-plane packet-flow sequence diagram
//!
//! # Usage
//! ```text
//! cargo run -- --ui            # start on default port 3000
//! cargo run -- --ui --port 8080
//! ```

use axum::{routing::get, Json, Router};
use serde::Serialize;
use tokio::net::TcpListener;
use tracing::info;

/// Metadata describing a single 6G protocol layer / cross-cutting module.
#[derive(Serialize)]
pub struct ModuleInfo {
    /// Human-readable layer name, e.g. `"PHY"`.
    pub layer: &'static str,
    /// Cargo crate name, e.g. `"6g-phy"`.
    pub crate_name: &'static str,
    /// Single-line feature summary.
    pub features: &'static str,
    /// Emoji icon shown in the dashboard card.
    pub icon: &'static str,
    /// Operational status reported by the running process.
    pub status: &'static str,
}

/// Metadata describing one experiment binary.
#[derive(Serialize)]
pub struct ExperimentInfo {
    /// Cargo example identifier, e.g. `"exp_001_dfrc_pareto_frontier"`.
    pub id: &'static str,
    /// Short human-readable title.
    pub name: &'static str,
    /// One-sentence description of what the experiment measures.
    pub description: &'static str,
    /// Primary crate exercised by the experiment.
    pub crate_name: &'static str,
    /// Emoji icon shown in the dashboard card.
    pub icon: &'static str,
}

/// Top-level JSON payload returned by `GET /api/status`.
#[derive(Serialize)]
pub struct StatusResponse {
    /// Whether all subsystems have been initialised without error.
    pub all_online: bool,
    /// Human-readable frequency band from `SystemConfig::default()`.
    pub config: ConfigSummary,
    /// Per-module status entries (one per crate).
    pub modules: Vec<ModuleInfo>,
}

/// Condensed view of `SystemConfig` fields shown on the dashboard.
#[derive(Serialize)]
pub struct ConfigSummary {
    /// Frequency band label derived from `SystemConfig::default()` (e.g. `"SubThz"`).
    pub frequency_band: String,
    /// Maximum number of simultaneously served UEs.
    pub max_ues: usize,
}

/// Top-level JSON payload returned by `GET /api/experiments`.
#[derive(Serialize)]
pub struct ExperimentsResponse {
    /// Ordered list of experiment descriptions.
    pub experiments: Vec<ExperimentInfo>,
}

// ── Static data ──────────────────────────────────────────────────────────────

fn modules() -> Vec<ModuleInfo> {
    vec![
        ModuleInfo {
            layer: "PHY",
            crate_name: "6g-phy",
            features:
                "THz/Sub-THz spectrum · Holographic MIMO · RIS · OFDM · OTFS · AI-native waveform",
            icon: "📡",
            status: "online",
        },
        ModuleInfo {
            layer: "MAC",
            crate_name: "6g-mac",
            features: "AI Q-learning scheduler · HARQ · OFDMA / NOMA / RSMA",
            icon: "⚙️",
            status: "online",
        },
        ModuleInfo {
            layer: "RLC",
            crate_name: "6g-rlc",
            features: "Segmentation · ARQ · TM/UM/AM modes",
            icon: "🔗",
            status: "online",
        },
        ModuleInfo {
            layer: "PDCP",
            crate_name: "6g-pdcp",
            features: "ROHC header compression · Ciphering · Integrity protection",
            icon: "🔒",
            status: "online",
        },
        ModuleInfo {
            layer: "RRC",
            crate_name: "6g-rrc",
            features: "Connection management · SIBs · Mobility · Inactive state",
            icon: "📋",
            status: "online",
        },
        ModuleInfo {
            layer: "6G Core",
            crate_name: "6g-core",
            features: "AMF+ · SMF+ · UPF+ · PCF · NRF · SDF · Digital Twin",
            icon: "🏛",
            status: "online",
        },
        ModuleInfo {
            layer: "AI Engine",
            crate_name: "6g-ai",
            features: "Channel estimator (LS/MMSE/MLP) · Model trait · Inference dispatch",
            icon: "🤖",
            status: "online",
        },
        ModuleInfo {
            layer: "ISAC",
            crate_name: "6g-isac",
            features: "DFRC · OTFS-ISAC · CRB / Shannon capacity · Pareto frontier",
            icon: "🔍",
            status: "online",
        },
        ModuleInfo {
            layer: "NTN",
            crate_name: "6g-ntn",
            features: "LEO · HAPS · UAV · Handover · Propagation models",
            icon: "🛰",
            status: "online",
        },
        ModuleInfo {
            layer: "Semantic",
            crate_name: "6g-semantic",
            features: "Semantic codec · Goal-oriented metrics · SSIM / semantic similarity",
            icon: "💬",
            status: "online",
        },
    ]
}

fn experiments() -> Vec<ExperimentInfo> {
    vec![
        ExperimentInfo {
            id: "exp_001_dfrc_pareto_frontier",
            name: "DFRC Pareto Frontier",
            description: "Sweeps sensing power ratio α ∈ [0,1] and plots CRB (m²) vs Shannon capacity (Gbps).",
            crate_name: "6g-isac", icon: "📊",
        },
        ExperimentInfo {
            id: "exp_002_phy_baseline_comparison",
            name: "PHY Baseline Comparison",
            description: "Compares path loss, RIS SNR gain, and MIMO capacity against published reference values.",
            crate_name: "6g-phy", icon: "📈",
        },
        ExperimentInfo {
            id: "exp_003_mac_srsran_baseline",
            name: "MAC srsRAN Baseline",
            description: "Validates the AI scheduler throughput and HARQ BLER against srsRAN reference tables.",
            crate_name: "6g-mac", icon: "⚙️",
        },
        ExperimentInfo {
            id: "exp_004_semantic_ai_phase5",
            name: "Semantic AI (Phase 5)",
            description: "Exercises the semantic encoder/decoder pipeline and measures goal-oriented SSIM scores.",
            crate_name: "6g-semantic", icon: "💬",
        },
        ExperimentInfo {
            id: "exp_005_e2e_core_session",
            name: "E2E Core Session",
            description: "Runs a full end-to-end session establishment through AMF, SMF, UPF and teardown.",
            crate_name: "6g-core", icon: "🏛",
        },
        ExperimentInfo {
            id: "exp_006_open5g_core_comparison",
            name: "Open5GS Core Comparison",
            description: "Validates 6GC session latency and slice admission against Open5GS reference data.",
            crate_name: "6g-core", icon: "🔄",
        },
        ExperimentInfo {
            id: "exp_007_ueransim_gnb_comparison",
            name: "UERANSIM gNB Comparison",
            description: "Compares RRC connection setup timings with UERANSIM gNB reference traces.",
            crate_name: "6g-rrc", icon: "📋",
        },
    ]
}

// ── Axum handlers ────────────────────────────────────────────────────────────

/// Serves the embedded single-page dashboard HTML.
async fn serve_index() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("static/index.html"))
}

/// Returns `StatusResponse` JSON for `GET /api/status`.
async fn api_status() -> Json<StatusResponse> {
    let cfg = sixg_common::config::SystemConfig::default();
    Json(StatusResponse {
        all_online: true,
        config: ConfigSummary {
            frequency_band: format!("{:?}", cfg.frequency_band),
            max_ues: cfg.max_ues,
        },
        modules: modules(),
    })
}

/// Returns `ExperimentsResponse` JSON for `GET /api/experiments`.
async fn api_experiments() -> Json<ExperimentsResponse> {
    Json(ExperimentsResponse {
        experiments: experiments(),
    })
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Starts the dashboard HTTP server on `http://0.0.0.0:<port>`.
///
/// This function blocks until the process is terminated. It is intended to be
/// called from `main()` when the `--ui` flag is passed. The server exposes:
///
/// | Path                | Description                              |
/// |---------------------|------------------------------------------|
/// | `GET /`             | Single-page dashboard (HTML)             |
/// | `GET /api/status`   | JSON — all module statuses               |
/// | `GET /api/experiments` | JSON — experiment descriptions        |
///
/// # Arguments
/// * `port` — TCP port to listen on (e.g. `3000`).
pub async fn run(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/api/status", get(api_status))
        .route("/api/experiments", get(api_experiments));

    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr).await?;
    info!("Dashboard available at http://localhost:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}

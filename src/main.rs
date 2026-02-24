//! 6G System – top-level entry point.
//!
//! This binary wires together the major subsystems of the 6G stack:
//! PHY → MAC → RLC → PDCP → RRC → Core, alongside the AI engine,
//! ISAC, NTN and Semantic communications layers.

use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    info!("Initialising 6G system stack…");

    // --- Common configuration ------------------------------------------------
    let cfg = sixg_common::config::SystemConfig::default();
    info!(
        freq_band = ?cfg.frequency_band,
        max_ues   = cfg.max_ues,
        "System configuration loaded"
    );

    // --- AI Engine -----------------------------------------------------------
    let ai = sixg_ai::AiEngine::new();
    info!("AI engine ready: {:?}", ai.backend());

    // --- Physical layer -------------------------------------------------------
    let phy = sixg_phy::PhyLayer::new(&cfg);
    info!("PHY layer initialised: {:?}", phy.waveform());

    // --- MAC layer ------------------------------------------------------------
    let mac = sixg_mac::MacLayer::new();
    info!("MAC layer initialised");

    // --- RLC layer ------------------------------------------------------------
    let rlc = sixg_rlc::RlcLayer::new();
    info!("RLC layer initialised");

    // --- PDCP layer -----------------------------------------------------------
    let pdcp = sixg_pdcp::PdcpLayer::new();
    info!("PDCP layer initialised");

    // --- RRC ------------------------------------------------------------------
    let rrc = sixg_rrc::RrcLayer::new();
    info!("RRC layer initialised");

    // --- ISAC -----------------------------------------------------------------
    let isac = sixg_isac::IsacLayer::new();
    info!("ISAC layer initialised");

    // --- Non-Terrestrial Networks --------------------------------------------
    let ntn = sixg_ntn::NtnLayer::new();
    info!("NTN layer initialised");

    // --- Semantic Communications ---------------------------------------------
    let sem = sixg_semantic::SemanticLayer::new();
    info!("Semantic communications layer initialised");

    // --- Core Network (6GC) --------------------------------------------------
    let core = sixg_core::CoreNetwork::new();
    info!("6G Core Network (6GC) initialised");

    info!("All subsystems online. 6G stack ready.");

    // Prevent unused-variable warnings while the layers are stubs.
    let _ = (phy, mac, rlc, pdcp, rrc, isac, ai, ntn, sem, core);
}

//! Physical Layer (PHY) for 6G.
//!
//! The PHY is responsible for:
//! * Waveform generation and reception (OFDM variants, DFT-s-OFDM, …)
//! * Spectrum management across Sub-6 GHz → THz bands
//! * Massive / Holographic MIMO beam management
//! * Reconfigurable Intelligent Surfaces (RIS) control
//! * AI-assisted channel estimation and link adaptation

pub mod mimo;
pub mod ris;
pub mod spectrum;
pub mod validation;
pub mod waveform;

use sixg_common::config::SystemConfig;

pub use mimo::MimoConfig;
pub use ris::{RisChannel, RisConfig};
pub use spectrum::{path_loss_db, SpectrumManager};
pub use validation::PhyValidation;
pub use waveform::{bpsk_ber_awgn, ofdm_ber_high_doppler, Waveform};

/// Entry point for the 6G physical layer.
pub struct PhyLayer {
    waveform: Waveform,
    spectrum: SpectrumManager,
    mimo: MimoConfig,
    ris: Option<RisConfig>,
}

impl PhyLayer {
    /// Initialise the PHY layer from a [`SystemConfig`].
    pub fn new(cfg: &SystemConfig) -> Self {
        let waveform = Waveform::default_for_band(cfg.frequency_band);
        let spectrum = SpectrumManager::new(cfg.frequency_band);
        let mimo = MimoConfig::new(cfg.antenna_elements);
        let ris = if cfg.ai_native_enabled {
            Some(RisConfig::default())
        } else {
            None
        };

        Self {
            waveform,
            spectrum,
            mimo,
            ris,
        }
    }

    pub fn waveform(&self) -> &Waveform {
        &self.waveform
    }

    pub fn spectrum(&self) -> &SpectrumManager {
        &self.spectrum
    }

    pub fn mimo(&self) -> &MimoConfig {
        &self.mimo
    }

    pub fn ris(&self) -> Option<&RisConfig> {
        self.ris.as_ref()
    }
}

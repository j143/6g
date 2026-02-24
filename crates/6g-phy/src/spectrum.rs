//! Spectrum management for the 6G PHY.
//!
//! 6G is designed to exploit a wide range of spectrum bands from below 6 GHz
//! up to the THz range. This module tracks band assignments, channel
//! bandwidths, and carrier aggregation configurations.

use serde::{Deserialize, Serialize};
use sixg_common::types::FrequencyBand;

/// Channel bandwidth options (MHz).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelBandwidth {
    Bw100Mhz,
    Bw400Mhz,
    Bw1Ghz,
    Bw2Ghz,
    Bw10Ghz,
}

impl ChannelBandwidth {
    /// Return the bandwidth in MHz.
    pub fn mhz(self) -> u32 {
        match self {
            Self::Bw100Mhz => 100,
            Self::Bw400Mhz => 400,
            Self::Bw1Ghz => 1_000,
            Self::Bw2Ghz => 2_000,
            Self::Bw10Ghz => 10_000,
        }
    }
}

/// Manages spectrum resources for the PHY layer.
#[derive(Debug, Clone)]
pub struct SpectrumManager {
    pub band: FrequencyBand,
    pub channel_bandwidth: ChannelBandwidth,
    /// Number of component carriers in a carrier-aggregation configuration.
    pub component_carriers: u8,
}

impl SpectrumManager {
    /// Create a spectrum manager with sensible defaults for the given band.
    pub fn new(band: FrequencyBand) -> Self {
        let (bw, cc) = match band {
            FrequencyBand::SubSixGhz => (ChannelBandwidth::Bw100Mhz, 1),
            FrequencyBand::MidBand => (ChannelBandwidth::Bw400Mhz, 2),
            FrequencyBand::MmWave => (ChannelBandwidth::Bw1Ghz, 4),
            FrequencyBand::SubThz => (ChannelBandwidth::Bw2Ghz, 8),
            FrequencyBand::Thz => (ChannelBandwidth::Bw10Ghz, 16),
        };
        Self {
            band,
            channel_bandwidth: bw,
            component_carriers: cc,
        }
    }

    /// Total aggregated bandwidth in MHz.
    pub fn total_bandwidth_mhz(&self) -> u32 {
        self.channel_bandwidth.mhz() * self.component_carriers as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thz_spectrum_manager_reports_large_bandwidth() {
        let sm = SpectrumManager::new(FrequencyBand::Thz);
        assert_eq!(sm.total_bandwidth_mhz(), 160_000);
    }
}

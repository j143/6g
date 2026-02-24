//! Multiple access schemes.

use serde::{Deserialize, Serialize};

/// Multiple access scheme used by the MAC layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessScheme {
    /// Orthogonal Frequency Division Multiple Access (baseline).
    Ofdma,
    /// Non-Orthogonal Multiple Access – higher spectral efficiency.
    Noma,
    /// Grant-Free access – ultra-low-latency IoT/URLLC.
    GrantFree,
    /// Rate-Splitting Multiple Access – flexible interference management.
    Rsma,
}

impl Default for AccessScheme {
    fn default() -> Self {
        Self::Ofdma
    }
}

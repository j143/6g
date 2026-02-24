//! Error types for the 6G stack.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("PHY error: {0}")]
    Phy(String),

    #[error("MAC error: {0}")]
    Mac(String),

    #[error("RLC error: {0}")]
    Rlc(String),

    #[error("PDCP error: {0}")]
    Pdcp(String),

    #[error("RRC error: {0}")]
    Rrc(String),

    #[error("ISAC error: {0}")]
    Isac(String),

    #[error("AI engine error: {0}")]
    Ai(String),

    #[error("NTN error: {0}")]
    Ntn(String),

    #[error("Semantic layer error: {0}")]
    Semantic(String),

    #[error("Core network error: {0}")]
    Core(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, Error>;

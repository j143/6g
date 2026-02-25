//! Common types, errors, and configuration shared across the 6G stack.

pub mod config;
pub mod error;
pub mod types;
pub mod validation;

pub use error::Error;
pub use types::*;

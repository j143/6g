//! AI-Native Engine for the 6G stack.
//!
//! 6G is designed to be AI-native: machine learning models are embedded
//! directly into the air interface, scheduler, beam management, and sensing
//! pipeline rather than being added as an afterthought.
//!
//! This crate exposes:
//! * [`AiEngine`] – central AI runtime
//! * [`AiModel`] – trait implemented by every ML model in the stack
//! * [`InferenceRequest`] / [`InferenceResult`] – common exchange types

pub mod channel_estimator;
pub mod inference;
pub mod model;

pub use inference::{InferenceRequest, InferenceResult};
pub use model::{AiBackend, AiModel};

use sixg_common::error::Result;

/// Central AI engine that manages model lifecycle and dispatches inference.
pub struct AiEngine {
    backend: AiBackend,
}

impl AiEngine {
    /// Create a new AI engine using the default (CPU) backend.
    pub fn new() -> Self {
        Self {
            backend: AiBackend::Cpu,
        }
    }

    /// Create an AI engine with a specified backend.
    pub fn with_backend(backend: AiBackend) -> Self {
        Self { backend }
    }

    /// Return the active compute backend.
    pub fn backend(&self) -> &AiBackend {
        &self.backend
    }

    /// Run inference on the given request.
    pub fn infer(&self, request: InferenceRequest) -> Result<InferenceResult> {
        // TODO: dispatch to the appropriate model registry entry.
        Ok(InferenceResult {
            model_id: request.model_id,
            outputs: vec![0.0; request.inputs.len()],
        })
    }
}

impl Default for AiEngine {
    fn default() -> Self {
        Self::new()
    }
}

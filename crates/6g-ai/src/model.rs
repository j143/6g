//! AI model trait and supporting types.

use crate::inference::{InferenceRequest, InferenceResult};
use sixg_common::error::Result;

/// Compute backend used by the AI engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiBackend {
    /// CPU-only inference (always available).
    Cpu,
    /// GPU-accelerated inference via CUDA.
    Cuda,
    /// NPU / hardware accelerator.
    Npu,
}

/// Every AI/ML model in the 6G stack implements this trait.
pub trait AiModel: Send + Sync {
    /// Human-readable model identifier (e.g. `"beam_predictor_v1"`).
    fn id(&self) -> &str;

    /// Run a forward pass and return the result.
    fn predict(&self, request: &InferenceRequest) -> Result<InferenceResult>;

    /// Return the expected number of input features.
    fn input_size(&self) -> usize;

    /// Return the number of output values.
    fn output_size(&self) -> usize;
}

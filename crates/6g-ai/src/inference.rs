//! Inference request/result types.

use serde::{Deserialize, Serialize};

/// An inference request dispatched to a registered AI model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    /// Identifier of the target model.
    pub model_id: String,
    /// Flat input feature vector.
    pub inputs: Vec<f32>,
}

/// The result of a model inference pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    /// Mirror of the model identifier from the request.
    pub model_id: String,
    /// Flat output vector produced by the model.
    pub outputs: Vec<f32>,
}

//! Simulated ONNX sentence-transformer model for semantic text encoding.
//!
//! This module provides [`OnnxModel`], a deterministic simulation of a
//! 2-layer MLP sentence encoder (analogous to `all-MiniLM-L6-v2`) that maps
//! a 128-dimensional word-hash feature vector to a 32-dimensional L2-normalised
//! semantic embedding.
//!
//! ## Why simulate rather than use the ONNX runtime?
//!
//! The `ort` crate requires a native `libonnxruntime` shared library and a
//! pre-trained model file — both are infeasible to bundle in a pure-Rust
//! workspace CI.  This module implements the same **interface** as a real ONNX
//! model so that:
//!
//! 1. The API is stable and ready to swap in `ort::Session` when the runtime
//!    and model file are available (see `docs/6g-ai.md`).
//! 2. The closed-form approximation produces semantically-informed embeddings
//!    (similar texts → similar embeddings via shared word-hash features) without
//!    a runtime dependency.
//!
//! ## Mathematical model
//!
//! ```text
//! x  ← 128-dim L2-normalised word-hash feature vector (f32)
//! h  ← tanh(W·x + b)    W ∈ ℝ^{32×128},  b ∈ ℝ^{32}
//! y  ← h / ‖h‖₂          (L2 normalise → unit sphere)
//! ```
//!
//! W and b are derived **deterministically** from the model identifier string
//! using an FNV-1a hash followed by an LCG, so the same model ID always
//! produces the same weights and therefore the same embedding for the same input.
//!
//! ## References
//!
//! - Reimers & Gurevych, *Sentence-BERT: Sentence Embeddings using Siamese
//!   BERT-Networks*, EMNLP 2019
//! - ONNX Runtime docs: <https://onnxruntime.ai/docs/>

use crate::{
    inference::{InferenceRequest, InferenceResult},
    model::AiModel,
};
use sixg_common::{
    error::Result,
    validation::{Validate, ValidationCheck, ValidationResult},
};

/// Embedding dimension — number of output floats (and output bytes after quantisation).
pub const EMBEDDING_DIM: usize = 32;

/// Input feature dimension — number of word-hash buckets fed into the model.
pub const FEATURE_DIM: usize = 128;

/// Simulated ONNX sentence-transformer inference model.
///
/// Implements [`AiModel`] with:
/// * `input_size()` → [`FEATURE_DIM`] (128 word-hash frequency features)
/// * `output_size()` → [`EMBEDDING_DIM`] (32-dim dense semantic embedding)
/// * `predict()` → L2-normalised 32-dim embedding via `tanh(W·x + b)`
///
/// The projection matrix W and bias b are derived deterministically from the
/// model identifier using an FNV-1a hash, ensuring reproducible embeddings
/// without loading an external `.onnx` file.
pub struct OnnxModel {
    model_id: String,
    /// Flattened weight matrix W ∈ ℝ^{EMBEDDING_DIM × FEATURE_DIM} (row-major).
    weights: Vec<f32>,
    /// Bias vector b ∈ ℝ^{EMBEDDING_DIM}.
    bias: Vec<f32>,
}

impl OnnxModel {
    /// Create a new simulated ONNX model with the given identifier.
    ///
    /// Projection weights and biases are initialised deterministically from
    /// `model_id` via an FNV-1a hash so the model is reproducible across runs.
    ///
    /// # Arguments
    /// * `model_id` – human-readable model name (e.g. `"sentence_transformer_v1"`)
    pub fn new(model_id: impl Into<String>) -> Self {
        let model_id = model_id.into();
        let (weights, bias) = Self::init_weights(&model_id);
        Self {
            model_id,
            weights,
            bias,
        }
    }

    /// Deterministic weight initialisation from the model-id hash (FNV-1a + LCG).
    ///
    /// Uses Xavier uniform initialisation: weights ∈ [−r, r] where
    /// `r = sqrt(6 / (fan_in + fan_out))`.
    fn init_weights(id: &str) -> (Vec<f32>, Vec<f32>) {
        let mut weights = Vec::with_capacity(EMBEDDING_DIM * FEATURE_DIM);
        let mut bias = Vec::with_capacity(EMBEDDING_DIM);

        let mut seed = fnv1a_hash(id.as_bytes());
        let range = ((6.0_f64 / (FEATURE_DIM + EMBEDDING_DIM) as f64).sqrt()) as f32;

        for _ in 0..(EMBEDDING_DIM * FEATURE_DIM) {
            seed = lcg_next(seed);
            let w = (seed as f32 / u64::MAX as f32 * 2.0 - 1.0) * range;
            weights.push(w);
        }
        for _ in 0..EMBEDDING_DIM {
            seed = lcg_next(seed);
            // Small bias initialisation
            bias.push((seed as f32 / u64::MAX as f32 * 2.0 - 1.0) * 0.01);
        }
        (weights, bias)
    }

    /// Run a forward pass: `y = L2_norm(tanh(W·x + b))`.
    ///
    /// # Arguments
    /// * `input` – word-hash frequency features (up to [`FEATURE_DIM`] f32 values)
    ///
    /// # Returns
    /// [`EMBEDDING_DIM`]-dimensional L2-normalised semantic embedding as `Vec<f32>`
    pub fn forward(&self, input: &[f32]) -> Vec<f32> {
        let n = input.len().min(FEATURE_DIM);

        // Matrix-vector multiply: h[i] = sum_j(W[i][j] * x[j]) + b[i], then tanh
        let hidden: Vec<f32> = (0..EMBEDDING_DIM)
            .map(|i| {
                let row_start = i * FEATURE_DIM;
                let dot: f32 = self.bias[i]
                    + input[..n]
                        .iter()
                        .enumerate()
                        .map(|(j, &x)| self.weights[row_start + j] * x)
                        .sum::<f32>();
                dot.tanh()
            })
            .collect();

        l2_normalize(&hidden)
    }
}

impl AiModel for OnnxModel {
    fn id(&self) -> &str {
        &self.model_id
    }

    fn input_size(&self) -> usize {
        FEATURE_DIM
    }

    fn output_size(&self) -> usize {
        EMBEDDING_DIM
    }

    /// Run sentence-transformer inference on `request.inputs`.
    ///
    /// # Arguments
    /// * `request` – [`InferenceRequest`] whose `inputs` is a [`FEATURE_DIM`]-dim
    ///   L2-normalised word-hash feature vector (f32)
    ///
    /// # Returns
    /// [`InferenceResult`] with `outputs` as a [`EMBEDDING_DIM`]-dim L2-normalised
    /// semantic embedding (f32)
    fn predict(&self, request: &InferenceRequest) -> Result<InferenceResult> {
        let embedding = self.forward(&request.inputs);
        Ok(InferenceResult {
            model_id: self.model_id.clone(),
            outputs: embedding,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// L2-normalise a vector, returning a new `Vec`.
///
/// If the norm is near zero (degenerate input), the vector is returned unchanged.
fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm < f32::EPSILON {
        v.to_vec()
    } else {
        v.iter().map(|x| x / norm).collect()
    }
}

/// FNV-1a 64-bit hash of a byte slice.
///
/// Reference: Fowler–Noll–Vo hash function, variant 1a (64-bit).
fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut h: u64 = 14_695_981_039_346_656_037;
    for &byte in data {
        h ^= byte as u64;
        h = h.wrapping_mul(1_099_511_628_211);
    }
    h
}

/// Linear Congruential Generator step (Knuth MMIX parameters).
fn lcg_next(x: u64) -> u64 {
    x.wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
}

/// Cosine similarity between two embedding vectors (dimensionless, ∈ [−1, 1]).
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| (*x * *y) as f64).sum();
    let norm_a: f64 = a.iter().map(|x| (*x * *x) as f64).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| (*x * *x) as f64).sum::<f64>().sqrt();
    if norm_a < f64::EPSILON || norm_b < f64::EPSILON {
        0.0
    } else {
        (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Validate
// ─────────────────────────────────────────────────────────────────────────────

/// Validation for the [`OnnxModel`] simulated sentence transformer.
///
/// Checks:
/// 1. Output dimension equals [`EMBEDDING_DIM`] (32).
/// 2. Output is L2-normalised (‖y‖₂ ≈ 1.0).
/// 3. Inference is deterministic (same input → same output).
/// 4. Self cosine-similarity of an embedding equals 1.0 (unit sphere property).
pub struct OnnxModelValidation;

impl Validate for OnnxModelValidation {
    fn validate() -> ValidationResult {
        let model = OnnxModel::new("sentence_transformer_v1");

        // Standard test input: first 16 word-hash buckets activated
        let input: Vec<f32> = (0..FEATURE_DIM)
            .map(|i| if i < 16 { 1.0 } else { 0.0 })
            .collect();

        let emb = model.forward(&input);

        // 1. Output dimension
        let out_dim = emb.len() as f64;

        // 2. L2 norm — must be ≈ 1.0 after normalisation
        let norm: f64 = emb.iter().map(|x| (*x * *x) as f64).sum::<f64>().sqrt();

        // 3. Determinism — re-run forward pass and compare element-wise
        let emb2 = model.forward(&input);
        let is_deterministic = emb.iter().zip(emb2.iter()).all(|(a, b)| a == b);
        let determinism_flag = if is_deterministic { 1.0 } else { 0.0 };

        // 4. Self cosine-similarity must equal 1.0 (vector vs itself)
        let cos_self = cosine_similarity(&emb, &emb);

        ValidationResult {
            module: "6g-ai::onnx_model",
            checks: vec![
                ValidationCheck::new("output_dimension", out_dim, EMBEDDING_DIM as f64, 0.0),
                ValidationCheck::new("l2_norm_is_unit", norm, 1.0, 0.01),
                ValidationCheck::new("deterministic_inference", determinism_flag, 1.0, 0.0),
                ValidationCheck::new("self_cosine_similarity_is_one", cos_self, 1.0, 0.01),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Output must have exactly EMBEDDING_DIM elements.
    #[test]
    fn output_dimension_is_embedding_dim() {
        let model = OnnxModel::new("test_model");
        let input = vec![1.0f32; FEATURE_DIM];
        let req = InferenceRequest {
            model_id: model.id().to_string(),
            inputs: input,
        };
        let result = model.predict(&req).unwrap();
        assert_eq!(
            result.outputs.len(),
            EMBEDDING_DIM,
            "output must be {EMBEDDING_DIM}-dimensional"
        );
    }

    /// After L2 normalisation the output norm must be ≈ 1.0.
    #[test]
    fn output_is_l2_normalised() {
        let model = OnnxModel::new("test_model");
        let input = vec![0.5f32; FEATURE_DIM];
        let embedding = model.forward(&input);
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-5,
            "embedding must be L2-normalised, got norm={norm}"
        );
    }

    /// Repeated calls with the same input must return identical results.
    #[test]
    fn inference_is_deterministic() {
        let model = OnnxModel::new("determinism_test");
        let input: Vec<f32> = (0..FEATURE_DIM)
            .map(|i| i as f32 / FEATURE_DIM as f32)
            .collect();
        let req = InferenceRequest {
            model_id: model.id().to_string(),
            inputs: input,
        };
        let r1 = model.predict(&req).unwrap();
        let r2 = model.predict(&req).unwrap();
        for (a, b) in r1.outputs.iter().zip(r2.outputs.iter()) {
            assert_eq!(a, b, "inference must be deterministic");
        }
    }

    /// AiModel trait accessors must return the correct values.
    #[test]
    fn model_id_and_sizes() {
        let model = OnnxModel::new("sentence_transformer_v1");
        assert_eq!(model.id(), "sentence_transformer_v1");
        assert_eq!(model.input_size(), FEATURE_DIM);
        assert_eq!(model.output_size(), EMBEDDING_DIM);
    }

    /// Self cosine-similarity of an L2-normalised vector must equal 1.0.
    #[test]
    fn self_cosine_similarity_is_one() {
        let model = OnnxModel::new("cosine_test");
        let input = vec![1.0f32; FEATURE_DIM];
        let emb = model.forward(&input);
        let cos = cosine_similarity(&emb, &emb);
        assert!(
            (cos - 1.0).abs() < 1e-6,
            "self cosine-similarity must be 1.0, got {cos}"
        );
    }

    /// OnnxModelValidation must pass all numerical checks.
    #[test]
    fn onnx_model_validation_passes() {
        let result = OnnxModelValidation::validate();
        assert!(result.passed(), "{}", result.summary());
    }
}

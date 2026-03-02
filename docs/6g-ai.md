# `6g-ai` — AI-Native Engine

## Purpose

`6g-ai` provides the inference dispatch infrastructure used by every other crate that embeds AI/ML decisions. Its role is to:

1. Define the `AiModel` trait (a common interface for any learned model).
2. Dispatch inference requests to the appropriate backend (`AiBackend`: CPU, CUDA, NPU).
3. Act as a model registry in future phases (load/store ONNX models via `ort`).

## Invariants

<!-- Things that must ALWAYS be true, regardless of changes -->
- `6g-ai` only depends on `6g-common` — it must never depend on domain crates (`6g-phy`, `6g-mac`, etc.).
- `AiModel::infer()` is always deterministic for a given input (no hidden state between calls).
- `AiBackend::Cpu` must always be available as a fallback regardless of hardware.

## Architecture

```
  Caller (e.g., 6g-mac scheduler)
        │  InferenceRequest { model_id, input: Vec<f32> }
        ▼
  AiEngine::infer()
        │
        ├── AiBackend::Cpu  → run model on CPU (ndarray / candle)
        ├── AiBackend::Cuda → run model on GPU (CUDA via cuDNN)
        └── AiBackend::Npu  → run model on NPU accelerator
        │
        ▼
  InferenceResult { output: Vec<f32>, latency_us: u64 }
```

## `AiModel` Trait

```rust
pub trait AiModel: Send + Sync {
    fn model_id(&self) -> &str;
    fn infer(&self, input: &[f32]) -> Vec<f32>;
}
```

Implementations: channel estimator (MLP), scheduler policy (DQN), semantic encoder/decoder (autoencoder).

## What This Crate Does NOT Do

- Does not implement domain-specific logic (no channel models, no scheduling heuristics).
- Does not train models (inference only).
- Does not depend on `6g-phy`, `6g-mac`, or any other domain crate.

## 6G Rationale

AI-native air interface is a core design principle in the 6G vision (Qualcomm AI-Native 6G, O-RAN WG2). Rather than hand-crafted algorithms for channel estimation, beam management, or scheduling, trained models adapt to the deployment environment. The `AiEngine` enables this without coupling each crate to a specific ML framework.

## Phase 5 Target

- Replace the LS channel estimator in `6g-phy` with a trained MLP. Compare NMSE vs SNR.
- Replace the Round Robin MAC scheduler with a DQN policy. Compare Jain index.


## Phase 5 Implemented Types

- `Nmse` — dimensionless normalized mean-square-error wrapper used by channel-estimation APIs.
- `LsEstimator` — 5G baseline least-squares estimator (`NMSE = 1/SNR_linear`).
- `MmseEstimator` — 5G baseline MMSE estimator (`NMSE = 1/(1+SNR_linear)`).
- `MlpEstimator` — 6G AI-native estimator (learned residual correction on top of MMSE).
- `ChannelEstimatorValidation` — `Validate` implementation for known numerical checks.

## Phase 6 Implemented Types (ONNX Sentence Transformer)

- `OnnxModel` — simulated ONNX sentence-transformer model implementing `AiModel`.
  Inputs: 128-dim word-hash feature vector (f32). Outputs: 32-dim L2-normalised
  semantic embedding (f32). Weights initialised deterministically from model-id
  via FNV-1a hash. Ready to swap in `ort::Session` for a real `.onnx` file.
- `OnnxModelValidation` — `Validate` implementation: checks output dimension,
  L2-normalisation, determinism, and self cosine-similarity.
- `EMBEDDING_DIM` — public constant (32): number of output floats.
- `FEATURE_DIM` — public constant (128): number of input word-hash buckets.
- `cosine_similarity` — cosine similarity between two f32 embedding vectors (dimensionless, ∈ [−1, 1]).

## How to Swap in a Real ONNX Model

When `libonnxruntime` and a pre-trained `.onnx` file are available:

```rust
// 1. Add `ort = "2"` to [dependencies] in crates/6g-ai/Cargo.toml
// 2. Replace OnnxModel::forward() with:
use ort::{Environment, Session, SessionBuilder, Value};

let env = Environment::builder().build()?;
let session = SessionBuilder::new(&env)?
    .with_model_from_file("models/all-MiniLM-L6-v2.onnx")?;
let input = Value::from_array(ndarray::Array2::from_shape_vec((1, 128), features)?)?;
let outputs = session.run(vec![input])?;
let embedding: Vec<f32> = outputs[0].try_extract::<f32>()?.view().iter().cloned().collect();
```

Recommended model: `sentence-transformers/all-MiniLM-L6-v2` (22 MB, 384-dim output,
reducible to 32-dim via PCA). Suitable for ultra-low latency 6G semantic sessions.

## Reasoning Depth (Phase 5 Channel Estimation)

1. **5G baseline:** LS/MMSE channel estimation with analytical NMSE curves.
2. **6G change + why:** AI-native estimator adapts to deployment/channel structure for lower NMSE.
3. **MVP:** `MlpEstimator` residual model that improves MMSE while degrading gracefully at low SNR.
4. **Quantitative success:** `NMSE_MLP < NMSE_MMSE` for SNR ≥ 0 dB, validated by tests and `ChannelEstimatorValidation`.
5. **Known risks:** model mismatch and overfitting to a specific channel prior; mitigation is to keep analytical LS/MMSE baselines in validation.

## References

- Qualcomm, *AI-Native 6G Air Interface* (6G Foundry Series)
- O-RAN Alliance WG2, *AI/ML Workflow Description and Requirements*
- Simeone, *A Very Brief Introduction to Machine Learning for Communications*, IEEE TCCN 2018

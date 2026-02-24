# `6g-ai` — AI-Native Engine

## Purpose

`6g-ai` provides the inference dispatch infrastructure used by every other crate that embeds AI/ML decisions. Its role is to:

1. Define the `AiModel` trait (a common interface for any learned model).
2. Dispatch inference requests to the appropriate backend (CPU, CUDA NPU).
3. Act as a model registry in future phases (load/store ONNX models via `ort`).

## Architecture

```
  Caller (e.g., 6g-mac scheduler)
        │  InferenceRequest { model_id, input: Vec<f32> }
        ▼
  AiEngine::infer()
        │
        ├── Backend::Cpu  → run model on CPU (ndarray / candle)
        ├── Backend::Cuda → run model on GPU (CUDA via cuDNN)
        └── Backend::Npu  → run model on NPU accelerator
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

## 6G Rationale

AI-native air interface is a core design principle in the 6G vision (Qualcomm AI-Native 6G, O-RAN WG2). Rather than hand-crafted algorithms for channel estimation, beam management, or scheduling, trained models adapt to the deployment environment. The `AiEngine` enables this without coupling each crate to a specific ML framework.

## Phase 5 Target

- Replace the LS channel estimator in `6g-phy` with a trained MLP. Compare NMSE vs SNR.
- Replace the Round Robin MAC scheduler with a DQN policy. Compare Jain index.

## References

- Qualcomm, *AI-Native 6G Air Interface* (6G Foundry Series)
- O-RAN Alliance WG2, *AI/ML Workflow Description and Requirements*
- Simeone, *A Very Brief Introduction to Machine Learning for Communications*, IEEE TCCN 2018

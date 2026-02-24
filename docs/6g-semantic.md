# `6g-semantic` — Semantic / Goal-Oriented Communications

## Purpose

Semantic communications shift the transmission objective from **bit accuracy** to **meaning preservation** or **task success**. Rather than minimising BER, the system minimises semantic distortion (e.g., classification accuracy, control error) while maximising compression.

## Architecture

```
Source data (image / text / sensor)
      │
  ┌───▼──────────────────┐
  │  Semantic Encoder     │  ← DNN-based; trained jointly with channel model
  │  (Knowledge extraction│
  │   + compression)      │
  └───┬──────────────────┘
      │ Compact semantic payload (<<< raw bits)
  ┌───▼──────────────────┐
  │  Channel (PHY/MAC)    │  ← Standard 6G stack
  └───┬──────────────────┘
      │
  ┌───▼──────────────────┐
  │  Semantic Decoder     │  ← Reconstructs meaning, not exact bits
  └───┬──────────────────┘
      │
  Task output (classification result / action / text)
```

## `SemanticCodec` Trait

Each codec is task-specific (image classification, speech understanding, control action, text). The interface:

```rust
pub trait SemanticCodec: Send + Sync {
    fn task(&self) -> SemanticTask;
    fn encode(&self, source: &[u8]) -> Payload;
    fn decode(&self, semantic: &[u8]) -> Payload;
}
```

Phase 5 will implement a real encoder using a pre-trained sentence transformer loaded via ONNX Runtime (`ort` crate).

## Validation Target (Phase 5)

Transmission of an image for classification: compare task success rate (correct classification %) vs bandwidth at 3 operating points:

1. Raw bit transmission (no compression).
2. JPEG compression (traditional).
3. Semantic encoding (this crate).

Success criterion: semantic encoding achieves the same classification accuracy as raw transmission at < 10% of the bandwidth.

## References

- Qin et al., *Semantic Communications: Principles and Challenges*, IEEE JSAC 2022
- Xie et al., *Deep Learning Enabled Semantic Communication Systems*, IEEE Trans. Signal Process. 2021

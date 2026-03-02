# `6g-semantic` — Semantic / Goal-Oriented Communications

## Purpose

Semantic communications shift the transmission objective from **bit accuracy** to **meaning preservation** or **task success**. Rather than minimising BER, the system minimises semantic distortion (e.g., classification accuracy, control error) while maximising compression. Key types: `SemanticPacket`, `SemanticLayer`.

## Invariants

<!-- Things that must ALWAYS be true, regardless of changes -->
- `SemanticLayer` encodes to a smaller payload than the input — compression ratio must be < 1.0.
- `SemanticPacket` always carries a task identifier so the decoder knows what to reconstruct.
- The encode/decode pair must be a semantic round-trip: `decode(encode(x))` preserves the task-relevant information from `x`.
- `6g-semantic` depends only on `6g-ai` and `6g-common` — never on PHY or MAC crates.

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

`TextSemanticCodec` is the current implementation. It uses a deterministic term-frequency signature (always 64 bytes, ~15× compression for typical text). A production-quality ONNX-based sentence transformer encoder is future work (see ROADMAP.md Phase 6+).

## `GoalSpec` — 6G Semantic PDU Session Contract

`GoalSpec` expresses a session's QoS as a task-success contract rather than bandwidth/latency (used by `6g-core/smf.rs`):

```rust
pub struct GoalSpec {
    pub task: SemanticTask,
    pub min_success_rate: TaskSuccessRate,         // e.g. TaskSuccessRate(0.90)
    pub max_bandwidth_reduction: BandwidthReduction, // e.g. BandwidthReduction(10.0)
}
```

`PduSessionType::Semantic(GoalSpec)` in the SMF routes the session through `UPF::forward_semantic_uplink` → `TextSemanticCodec` rather than GTP-U, making this crate load-bearing in the 6G data path.

## What This Crate Does NOT Do

- Does not implement the PHY or MAC transmission — hands the compressed packet to the standard 6G stack.
- Does not implement model training.
- Does not depend on `6g-phy`, `6g-mac`, `6g-rlc`, `6g-pdcp`, or `6g-rrc`.

## Validation Target (Phase 5)

Transmission of an image for classification: compare task success rate (correct classification %) vs bandwidth at 3 operating points:

1. Raw bit transmission (no compression).
2. JPEG compression (traditional).
3. Semantic encoding (this crate).

Success criterion: semantic encoding achieves the same classification accuracy as raw transmission at < 10% of the bandwidth.


## Phase 5 Implemented Types

- `TextSemanticCodec` — deterministic semantic text codec (term-frequency signature).
- `BandwidthReduction` — dimensionless compression factor (`original_bytes / transmitted_bytes`).
- `TaskSuccessRate` — dimensionless task-level success metric in `[0, 1]`.
- `GoalOrientedPoint` — one operating point on task-success vs bandwidth curves.
- `GoalOrientedMetrics` — raw/JPEG/semantic task-success models and sweep helper.
- `SemanticValidation` — `Validate` implementation for Phase 5 semantic checks.

## Phase 6 Implemented Types (ONNX Semantic Codec)

- `OnnxSemanticCodec` — ONNX-based semantic codec using the simulated sentence
  transformer from `6g-ai`. Encodes UTF-8 text to a 32-byte quantised embedding
  (31.25× compression for 1 kB input). Ready for real ONNX runtime swap-in.
- `OnnxSemanticValidation` — `Validate` implementation: checks encoded size,
  compression ratio, determinism, size vs `TextSemanticCodec`, semantic similarity
  preservation, and delegates to `OnnxModelValidation`.

## Reasoning Depth (Phase 5 Semantic Layer)

1. **5G baseline:** bit-accurate transport plus traditional codecs (e.g., JPEG) evaluated via BER/packet metrics.
2. **6G change + why:** optimize for task success and meaning preservation at much lower bandwidth.
3. **MVP:** `TextSemanticCodec` + `GoalOrientedMetrics` comparing raw/JPEG/semantic operating points.
4. **Quantitative success:** semantic mode sustains > 90% task success at 10× compression, with validation in `SemanticValidation`.
5. **Known risks:** semantic drift and task-mismatch (good bit metrics but poor downstream task quality); mitigation is explicit task-success validation.

## References

- Qin et al., *Semantic Communications: Principles and Challenges*, IEEE JSAC 2022
- Xie et al., *Deep Learning Enabled Semantic Communication Systems*, IEEE Trans. Signal Process. 2021

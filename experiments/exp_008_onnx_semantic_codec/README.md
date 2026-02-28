# Experiment 008 — ONNX Semantic Codec vs Term-Frequency Codec

## Hypothesis

A simulated ONNX sentence-transformer codec (`OnnxSemanticCodec`) produces
more compact semantic embeddings than the term-frequency codec
(`TextSemanticCodec`) while preserving semantic similarity between related
texts — making it better suited for ultra-low latency 6G transmission.

## Method

Three comparisons are made:

1. **Encoded size and compression ratio** — both codecs encode messages of
   64 B – 4 096 B; ONNX codec always outputs 32 bytes (31.25× compression
   for 1 kB), TF codec always outputs 64 bytes (15.6× for 1 kB).

2. **Semantic similarity** — cosine similarity between ONNX embeddings of
   related sentence pairs (e.g. paraphrases) vs unrelated pairs.  Related
   pairs should have higher mean cosine similarity.

3. **Encode/decode round-trip** — verify 32-byte encoding and 128-byte
   dequantised decoding (32 f32 values × 4 bytes).

**Models used:**
- `sentence_transformer_v1` — simulated sentence transformer (128-dim input,
  32-dim L2-normalised embedding, Xavier-initialised weights from model-id hash)
- Analogous to `all-MiniLM-L6-v2` in interface; swap `OnnxModel` for
  `ort::Session` to use a real ONNX model file.

## Result

- `OnnxSemanticCodec` outputs 32 bytes vs 64 bytes for `TextSemanticCodec`
- Compression ratio: 31.25× (ONNX) vs 15.6× (TF) for 1 kB input
- Related sentence pairs have higher cosine similarity than unrelated pairs
- All `OnnxSemanticValidation` checks pass

## Which AI Models Suit 6G Semantic Communications?

| Model type | Suitability | Why |
|---|---|---|
| **Sentence transformer** (e.g. all-MiniLM-L6-v2) | ★★★★★ | Compact 384-dim embeddings, semantic similarity preserved, ONNX export supported |
| **Autoencoder** (task-specific) | ★★★★☆ | Jointly trained with channel model for best task success; requires training |
| **LLM (GPT-style)** | ★★☆☆☆ | Too large for real-time encoding; quantised distilled versions may work |
| **CNN image encoder** | ★★★★★ for images | Compact latent codes for image classification tasks |
| **GNN knowledge graph** | ★★★☆☆ | Rich semantic structure but high complexity |

**Recommended path:** Start with `sentence_transformer_v1` (this experiment),
then replace the simulated model with `all-MiniLM-L6-v2.onnx` via `ort::Session`
when the ONNX runtime is available.

//! Experiment 008 — ONNX Semantic Codec vs Term-Frequency Codec
//!
//! Compares two semantic text codecs on three axes:
//!
//! 1. **Encoded size** — bytes transmitted per message.
//! 2. **Compression ratio** — original_bytes / encoded_bytes.
//! 3. **Semantic similarity** — cosine similarity between embeddings of related
//!    and unrelated sentence pairs, showing that the ONNX codec preserves
//!    meaning in its compact representation.
//!
//! ## Codecs compared
//!
//! | Codec | Output size | Algorithm |
//! |-------|-------------|-----------|
//! | `TextSemanticCodec` | 64 bytes | Term-frequency signature (FNV-1a hash buckets) |
//! | `OnnxSemanticCodec` | 32 bytes | Simulated ONNX sentence transformer (MLP + L2-norm) |
//!
//! Run with:
//!   cargo run --example exp_008_onnx_semantic_codec

use sixg_ai::inference::InferenceRequest;
use sixg_ai::model::AiModel;
use sixg_ai::onnx_model::{cosine_similarity, OnnxModel, FEATURE_DIM};
use sixg_common::validation::Validate;
use sixg_semantic::codec::{OnnxSemanticCodec, OnnxSemanticValidation, TextSemanticCodec};
use sixg_semantic::SemanticCodec;

fn main() {
    // ─────────────────────────────────────────────────────────────────────────
    // Part 1: Encoded size and compression ratio
    // ─────────────────────────────────────────────────────────────────────────
    println!("=== Part 1: Encoded size and compression ratio ===");
    println!(
        "{:>8}  {:>10}  {:>10}  {:>12}  {:>12}",
        "Msg_len", "TF_bytes", "ONNX_bytes", "TF_ratio", "ONNX_ratio"
    );
    println!("{}", "-".repeat(60));

    let tf_codec = TextSemanticCodec;
    let onnx_codec = OnnxSemanticCodec::new();

    let message_sizes = [64usize, 128, 256, 512, 1_000, 4_096];
    for &msg_len in &message_sizes {
        let base = b"the quick brown fox jumps over the lazy dog ";
        let text: Vec<u8> = base.iter().copied().cycle().take(msg_len).collect();

        let tf_enc = tf_codec.encode(&text);
        let onnx_enc = onnx_codec.encode(&text);

        let tf_ratio = msg_len as f64 / tf_enc.len() as f64;
        let onnx_ratio = msg_len as f64 / onnx_enc.len() as f64;

        println!(
            "{:>8}  {:>10}  {:>10}  {:>12.2}  {:>12.2}",
            msg_len,
            tf_enc.len(),
            onnx_enc.len(),
            tf_ratio,
            onnx_ratio
        );
    }

    let sample = b"hello semantic world";
    let tf_size = tf_codec.encode(sample).len();
    let onnx_size = onnx_codec.encode(sample).len();
    assert!(
        onnx_size < tf_size,
        "ONNX codec ({onnx_size} B) must be smaller than TF codec ({tf_size} B)"
    );
    println!("\nONNX codec size < TF codec size: PASSED \u{2713}");

    // ─────────────────────────────────────────────────────────────────────────
    // Part 2: Semantic similarity preservation
    // ─────────────────────────────────────────────────────────────────────────
    println!("\n=== Part 2: Cosine similarity between sentence pairs ===");
    println!(
        "{:>42}  {:>42}  {:>8}",
        "Sentence A", "Sentence B", "cos_sim"
    );
    println!("{}", "-".repeat(98));

    let sentence_pairs: &[(&[u8], &[u8], &str)] = &[
        (b"the cat sat on the mat", b"a cat sat on a mat", "related"),
        (
            b"deep learning improves channel estimation",
            b"neural networks for channel estimation",
            "related",
        ),
        (
            b"6G enables ultra low latency communications",
            b"ultra low latency is key for 6G networks",
            "related",
        ),
        (
            b"the cat sat on the mat",
            b"stock market closes higher today",
            "unrelated",
        ),
        (
            b"deep learning improves channel estimation",
            b"the weather is sunny and warm outside",
            "unrelated",
        ),
    ];

    let model = OnnxModel::new("sentence_transformer_v1");
    let mut related_sims = Vec::new();
    let mut unrelated_sims = Vec::new();

    for &(sent_a, sent_b, label) in sentence_pairs {
        let emb_a = embed_text(&model, sent_a);
        let emb_b = embed_text(&model, sent_b);
        let sim = cosine_similarity(&emb_a, &emb_b);

        if label == "related" {
            related_sims.push(sim);
        } else {
            unrelated_sims.push(sim);
        }

        let a_str = String::from_utf8_lossy(sent_a);
        let b_str = String::from_utf8_lossy(sent_b);
        println!(
            "{:>42}  {:>42}  {:>8.4}  [{}]",
            truncate(&a_str, 42),
            truncate(&b_str, 42),
            sim,
            label
        );
    }

    let avg_related: f64 = related_sims.iter().sum::<f64>() / related_sims.len() as f64;
    let avg_unrelated: f64 = unrelated_sims.iter().sum::<f64>() / unrelated_sims.len() as f64;

    println!("\nAverage cosine similarity:");
    println!("  Related pairs:   {avg_related:.4}");
    println!("  Unrelated pairs: {avg_unrelated:.4}");

    assert!(
        avg_related > avg_unrelated,
        "Related pairs must have higher cosine similarity than unrelated: \
         related={avg_related:.4}, unrelated={avg_unrelated:.4}"
    );
    println!("\nRelated > Unrelated cosine similarity: PASSED \u{2713}");

    // ─────────────────────────────────────────────────────────────────────────
    // Part 3: Encode/decode round-trip size check
    // ─────────────────────────────────────────────────────────────────────────
    println!("\n=== Part 3: Encode/decode round-trip ===");
    let msg = b"6G semantic communications: transmit meaning not bits";
    let encoded = onnx_codec.encode(msg);
    let decoded = onnx_codec.decode(&encoded);

    println!("Original  : {} bytes", msg.len());
    println!(
        "Encoded   : {} bytes  ({:.1}x compression)",
        encoded.len(),
        msg.len() as f64 / encoded.len() as f64
    );
    println!(
        "Decoded   : {} bytes (dequantised f32 embedding)",
        decoded.len()
    );

    assert_eq!(
        decoded.len(),
        32 * 4,
        "decoded payload must be 128 bytes (32 f32 values x 4 bytes)"
    );
    println!("\nDecode size check: PASSED \u{2713}");

    // ─────────────────────────────────────────────────────────────────────────
    // Part 4: Validation suites
    // ─────────────────────────────────────────────────────────────────────────
    println!("\n=== Part 4: Validation suites ===");
    let onnx_val = OnnxSemanticValidation::validate();
    println!("{}", onnx_val.summary());
    assert!(onnx_val.passed(), "OnnxSemanticValidation FAILED");

    println!("\nAll exp_008 checks PASSED \u{2713}");
}

/// Embed a UTF-8 byte slice using the given [`OnnxModel`].
///
/// Converts text to a [`FEATURE_DIM`]-dimensional word-hash feature vector
/// then runs the ONNX forward pass to produce an L2-normalised embedding.
fn embed_text(model: &OnnxModel, text: &[u8]) -> Vec<f32> {
    let mut features = vec![0.0f32; FEATURE_DIM];
    let s = String::from_utf8_lossy(text);
    for word in s.split_whitespace() {
        let bucket = word_hash(word) % FEATURE_DIM;
        features[bucket] += 1.0;
    }
    let norm: f32 = features.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for f in &mut features {
            *f /= norm;
        }
    }
    let req = InferenceRequest {
        model_id: model.id().to_string(),
        inputs: features,
    };
    model.predict(&req).unwrap().outputs
}

/// FNV-1a hash for word-to-bucket mapping (32-bit variant).
fn word_hash(word: &str) -> usize {
    let mut h: u32 = 2_166_136_261;
    for &byte in word.to_lowercase().as_bytes() {
        h ^= byte as u32;
        h = h.wrapping_mul(16_777_619);
    }
    h as usize
}

/// Truncate a string to at most `max_len` characters for display.
fn truncate<'a>(s: &'a str, max_len: usize) -> &'a str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}

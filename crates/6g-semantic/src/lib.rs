//! Semantic / Goal-Oriented Communications.
//!
//! Semantic communications shift the focus from bit-accurate transmission
//! to conveying *meaning* or achieving *task goals*. Key ideas:
//!
//! * The transmitter extracts a compact semantic representation of the
//!   source data using a learned encoder (knowledge-graph or DNN-based).
//! * The receiver reconstructs the intended meaning, not the raw bits.
//! * This reduces the transmitted data volume by orders of magnitude for
//!   tasks such as image transmission, speech, sensor fusion, etc.

use sixg_common::types::Payload;

/// Semantic task type – defines the goal of the communication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticTask {
    /// Transmit an image such that the receiver can classify it correctly.
    ImageClassification,
    /// Transmit speech so that the semantic meaning is recovered (not the exact audio).
    SpeechUnderstanding,
    /// Transmit sensor data sufficient for a downstream control decision.
    ControlAction,
    /// Generic text / NLP task.
    TextUnderstanding,
}

/// A semantically-encoded packet.
#[derive(Debug, Clone)]
pub struct SemanticPacket {
    pub task: SemanticTask,
    /// Compressed semantic representation (latent vector, knowledge-graph, …).
    pub semantic_payload: Payload,
    /// Original size of the uncompressed source data in bytes.
    pub original_size_bytes: usize,
}

impl SemanticPacket {
    /// Compression ratio achieved by semantic encoding.
    pub fn compression_ratio(&self) -> f64 {
        if self.original_size_bytes == 0 {
            return 1.0;
        }
        self.original_size_bytes as f64 / self.semantic_payload.len().max(1) as f64
    }
}

/// Semantic encoder/decoder pair (autoencoder interface).
///
/// Implementations map raw source data to compact semantic representations
/// and back, optimised for a specific [`SemanticTask`].
pub trait SemanticCodec: Send + Sync {
    fn task(&self) -> SemanticTask;

    /// Encode raw source data into a semantic payload.
    fn encode(&self, source: &[u8]) -> Payload;

    /// Decode a received semantic payload back into the intended output.
    fn decode(&self, semantic: &[u8]) -> Payload;
}

/// Semantic layer entry point.
pub struct SemanticLayer {
    codecs: Vec<Box<dyn SemanticCodec>>,
}

impl SemanticLayer {
    pub fn new() -> Self {
        Self { codecs: Vec::new() }
    }

    pub fn register_codec(&mut self, codec: Box<dyn SemanticCodec>) {
        self.codecs.push(codec);
    }

    pub fn codec_count(&self) -> usize {
        self.codecs.len()
    }
}

impl Default for SemanticLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compression_ratio_calculation() {
        let pkt = SemanticPacket {
            task: SemanticTask::ImageClassification,
            semantic_payload: vec![0u8; 100],
            original_size_bytes: 1000,
        };
        assert!((pkt.compression_ratio() - 10.0).abs() < f64::EPSILON);
    }
}

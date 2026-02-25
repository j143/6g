//! HARQ (Hybrid Automatic Repeat reQuest) process management.
//!
//! Classic Chase Combining is implemented: the receiver accumulates soft log-
//! likelihood-ratio (LLR) buffers across transmissions and attempts decoding
//! after combining. Combining gain follows `SNR_combined = k · SNR_initial`
//! for `k` transmissions of the same codeword (ideal MRC).
//!
//! A **proactive HARQ oracle** stub is also provided: in 6G the network can
//! predict retransmission need *before* receiving a NACK (e.g. via channel
//! prediction), pre-staging retransmission data. The `ProactiveHarq` struct
//! models this as a confidence-threshold filter over predicted post-combining
//! SNR.
//!
//! Reference: 3GPP TS 38.212 §5.4 (HARQ), Makki et al., IEEE Commun. Lett. 2014.

/// Number of HARQ processes per UE (6G supports up to 32 vs 5G's 16).
pub const MAX_HARQ_PROCESSES: usize = 32;

/// Maximum retransmission attempts before a HARQ process is flushed.
pub const MAX_RETX: u8 = 4;

/// SNR threshold (linear) below which decoding is assumed to fail.
const DECODE_SNR_THRESHOLD: f64 = 2.0; // ≈ 3 dB

/// State of a single HARQ process.
#[derive(Debug, Clone, PartialEq)]
pub enum HarqState {
    Idle,
    WaitingAck,
    Retransmitting { attempt: u8 },
}

/// Chase Combining soft-buffer for one HARQ process.
///
/// Accumulates received LLRs across retransmissions using maximum-ratio
/// combining.  Combined SNR = `Σ snr_k` (ideal MRC in AWGN).
#[derive(Debug, Clone, Default)]
pub struct ChaseCombineBuffer {
    /// Accumulated combined SNR (linear) so far.
    pub combined_snr: f64,
    /// Number of transmissions combined so far.
    pub tx_count: u8,
}

impl ChaseCombineBuffer {
    /// Accumulate one transmission with the given received SNR (linear).
    ///
    /// Returns the new combined SNR (linear) after MRC.
    pub fn combine(&mut self, rx_snr_linear: f64) -> f64 {
        self.combined_snr += rx_snr_linear;
        self.tx_count += 1;
        self.combined_snr
    }

    /// Return `true` if the combined SNR exceeds the decoding threshold.
    ///
    /// Threshold: `SNR_combined ≥ DECODE_SNR_THRESHOLD` (≈ 3 dB).
    pub fn can_decode(&self) -> bool {
        self.combined_snr >= DECODE_SNR_THRESHOLD
    }

    /// Reset the buffer (after successful decode or flush).
    pub fn reset(&mut self) {
        self.combined_snr = 0.0;
        self.tx_count = 0;
    }
}

/// Manages the pool of HARQ processes for one UE.
pub struct HarqManager {
    processes: [HarqState; MAX_HARQ_PROCESSES],
    /// Chase Combining soft buffers, one per process.
    buffers: [ChaseCombineBuffer; MAX_HARQ_PROCESSES],
}

impl HarqManager {
    pub fn new() -> Self {
        Self {
            processes: std::array::from_fn(|_| HarqState::Idle),
            buffers: std::array::from_fn(|_| ChaseCombineBuffer::default()),
        }
    }

    /// Return the state of HARQ process `id`.
    pub fn state(&self, id: usize) -> Option<HarqState> {
        self.processes.get(id).cloned()
    }

    /// Mark process `id` as waiting for an ACK/NACK (first transmission).
    pub fn start(&mut self, id: usize) {
        if let Some(p) = self.processes.get_mut(id) {
            *p = HarqState::WaitingAck;
            self.buffers[id].reset();
        }
    }

    /// ACK: the transmission was decoded successfully; free the process.
    pub fn acknowledge(&mut self, id: usize) {
        if let Some(p) = self.processes.get_mut(id) {
            *p = HarqState::Idle;
            self.buffers[id].reset();
        }
    }

    /// NACK: decoding failed.  Transition to `Retransmitting`.
    ///
    /// If `MAX_RETX` is reached the process is flushed (back to `Idle`).
    pub fn nack(&mut self, id: usize) {
        if id >= MAX_HARQ_PROCESSES {
            return;
        }
        self.processes[id] = match self.processes[id] {
            HarqState::WaitingAck => HarqState::Retransmitting { attempt: 1 },
            HarqState::Retransmitting { attempt } if attempt < MAX_RETX - 1 => {
                HarqState::Retransmitting {
                    attempt: attempt + 1,
                }
            }
            _ => HarqState::Idle, // flush: MAX_RETX NACKs exhausted
        };
        if self.processes[id] == HarqState::Idle {
            self.buffers[id].reset();
        }
    }

    /// Feed a received SNR into the Chase Combining buffer for process `id`.
    ///
    /// `rx_snr_linear` — received SNR (linear) for this transmission.
    ///
    /// Returns `true` if the combined SNR now exceeds the decoding threshold
    /// (the caller should issue ACK in this case).
    pub fn chase_combine(&mut self, id: usize, rx_snr_linear: f64) -> bool {
        if id >= MAX_HARQ_PROCESSES {
            return false;
        }
        self.buffers[id].combine(rx_snr_linear);
        self.buffers[id].can_decode()
    }

    /// Return the current combined SNR (linear) for process `id`.
    pub fn combined_snr(&self, id: usize) -> f64 {
        self.buffers.get(id).map(|b| b.combined_snr).unwrap_or(0.0)
    }
}

impl Default for HarqManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Proactive HARQ oracle (6G extension)
// ---------------------------------------------------------------------------

/// Proactive HARQ oracle: predicts whether a retransmission will be needed
/// *before* the NACK arrives, enabling pre-staging of retransmission data.
///
/// Model: if the predicted post-combining SNR is below `threshold_linear`
/// with `confidence ≥ min_confidence`, the oracle recommends pre-staging.
///
/// Reference: Makki et al., "On the Performance of HARQ-based Self-Interference
/// Cancellation", IEEE Commun. Lett. 2014.
pub struct ProactiveHarq {
    /// SNR threshold (linear) below which retransmission is likely needed.
    pub threshold_linear: f64,
    /// Minimum confidence in the prediction to trigger pre-staging.
    pub min_confidence: f64,
}

impl ProactiveHarq {
    /// Create a proactive HARQ oracle with default parameters.
    ///
    /// `threshold_linear` ≈ 3 dB (2.0), `min_confidence` = 0.8.
    pub fn new() -> Self {
        Self {
            threshold_linear: DECODE_SNR_THRESHOLD,
            min_confidence: 0.8,
        }
    }

    /// Predict whether a retransmission will be needed.
    ///
    /// `predicted_snr_linear` — channel-prediction model output.
    /// `prediction_confidence` — model confidence in [0, 1].
    ///
    /// Returns `true` if pre-staging is recommended.
    pub fn should_prestage(&self, predicted_snr_linear: f64, prediction_confidence: f64) -> bool {
        prediction_confidence >= self.min_confidence && predicted_snr_linear < self.threshold_linear
    }
}

impl Default for ProactiveHarq {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harq_process_lifecycle() {
        let mut mgr = HarqManager::new();
        assert_eq!(mgr.state(0), Some(HarqState::Idle));
        mgr.start(0);
        assert_eq!(mgr.state(0), Some(HarqState::WaitingAck));
        mgr.acknowledge(0);
        assert_eq!(mgr.state(0), Some(HarqState::Idle));
    }

    #[test]
    fn nack_transitions_to_retransmitting() {
        let mut mgr = HarqManager::new();
        mgr.start(0);
        mgr.nack(0);
        assert_eq!(mgr.state(0), Some(HarqState::Retransmitting { attempt: 1 }));
    }

    #[test]
    fn nack_flush_after_max_retx() {
        let mut mgr = HarqManager::new();
        mgr.start(0);
        for _ in 0..MAX_RETX {
            mgr.nack(0);
        }
        assert_eq!(
            mgr.state(0),
            Some(HarqState::Idle),
            "should flush after MAX_RETX NACKs"
        );
    }

    #[test]
    fn chase_combine_accumulates_snr() {
        let mut mgr = HarqManager::new();
        mgr.start(0);
        // Two transmissions each at SNR=1.0 → combined SNR = 2.0 ≥ threshold.
        let decoded = mgr.chase_combine(0, 1.0) || mgr.chase_combine(0, 1.0);
        assert!(
            decoded,
            "combined SNR should exceed threshold after 2 transmissions"
        );
        assert!((mgr.combined_snr(0) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn chase_combine_single_weak_does_not_decode() {
        let mut mgr = HarqManager::new();
        mgr.start(0);
        let decoded = mgr.chase_combine(0, 0.5);
        assert!(!decoded, "single weak transmission should not decode");
    }

    #[test]
    fn proactive_harq_prestages_on_low_snr() {
        let oracle = ProactiveHarq::new();
        assert!(
            oracle.should_prestage(1.0, 0.9),
            "low SNR + high confidence → prestage"
        );
        assert!(!oracle.should_prestage(5.0, 0.9), "good SNR → no prestage");
        assert!(
            !oracle.should_prestage(1.0, 0.5),
            "low confidence → no prestage"
        );
    }
}

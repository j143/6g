//! Sensing tasks and results.

use sixg_common::types::Position3D;

/// Type of sensing task.
#[derive(Debug, Clone)]
pub enum SensingTask {
    /// Estimate the position of an object.
    Localisation,
    /// Measure the radial velocity of a target.
    VelocityEstimation,
    /// Reconstruct a 2-D / 3-D map of the environment.
    EnvironmentMapping,
    /// Detect and classify a gesture.
    GestureRecognition,
}

/// Result of a sensing measurement.
#[derive(Debug, Clone)]
pub struct SensingResult {
    pub task: SensingTask,
    /// Estimated position (if applicable).
    pub position: Option<Position3D>,
    /// Estimated radial velocity m/s (if applicable).
    pub velocity_m_s: Option<f64>,
    /// Confidence score in [0, 1].
    pub confidence: f64,
}

impl SensingResult {
    /// Produce a stub result with zero confidence.
    pub fn stub(task: SensingTask) -> Self {
        Self {
            task,
            position: None,
            velocity_m_s: None,
            confidence: 0.0,
        }
    }
}

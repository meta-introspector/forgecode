use std::time::Duration;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ModelId, ProviderId};

/// The result of testing a single model.
///
/// Contains timing information, success/failure status, and a composite score
/// that can be used to rank models.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ModelTestResult {
    /// The provider that hosts this model.
    pub provider_id: ProviderId,

    /// The model that was tested.
    pub model_id: ModelId,

    /// Whether the model responded successfully.
    pub success: bool,

    /// Time to first token (TTFT) — how long until the first response chunk
    /// arrived.
    pub time_to_first_token: Option<Duration>,

    /// Total elapsed time for the test request.
    pub total_duration: Option<Duration>,

    /// Composite score (0.0–1.0) combining speed, success, and capability
    /// signals. Higher is better.
    pub score: f64,

    /// Error message if the test failed.
    pub error: Option<String>,
}

impl ModelTestResult {
    /// Creates a successful test result.
    pub fn success(
        provider_id: ProviderId,
        model_id: ModelId,
        time_to_first_token: Duration,
        total_duration: Duration,
    ) -> Self {
        // Score based primarily on TTFT (faster = better) and success.
        // TTFT under 1s → 1.0, under 3s → 0.8, under 5s → 0.6, under 10s → 0.4, else 0.2
        let ttft_secs = time_to_first_token.as_secs_f64();
        let speed_score = if ttft_secs <= 1.0 {
            1.0
        } else if ttft_secs <= 3.0 {
            0.8
        } else if ttft_secs <= 5.0 {
            0.6
        } else if ttft_secs <= 10.0 {
            0.4
        } else {
            0.2
        };

        Self {
            provider_id,
            model_id,
            success: true,
            time_to_first_token: Some(time_to_first_token),
            total_duration: Some(total_duration),
            score: speed_score,
            error: None,
        }
    }

    /// Creates a failed test result.
    pub fn failure(
        provider_id: ProviderId,
        model_id: ModelId,
        error: String,
    ) -> Self {
        Self {
            provider_id,
            model_id,
            success: false,
            time_to_first_token: None,
            total_duration: None,
            score: 0.0,
            error: Some(error),
        }
    }
}

/// The full report from running model tests across all configured providers.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ModelTestReport {
    /// Individual test results, sorted by score descending (best first).
    pub results: Vec<ModelTestResult>,
}

impl ModelTestReport {
    /// Creates a new report from the given results, sorted by score descending.
    pub fn new(mut results: Vec<ModelTestResult>) -> Self {
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Self { results }
    }
}

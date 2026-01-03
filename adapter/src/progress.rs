//! Progress event types for real-time benchmark updates

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Progress event emitted during benchmark execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    /// Benchmark session ID
    pub session_id: String,
    /// Current phase
    pub phase: BenchmarkPhase,
    /// Time elapsed since benchmark start
    pub elapsed: Duration,
    /// Expected total duration (if known)
    pub total_duration: Option<Duration>,
    /// Statistics for this interval (if in running phase)
    pub stats: Option<IntervalStats>,
    /// Optional message (e.g., error message for failed phase)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Benchmark execution phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkPhase {
    /// Preparing to run (connecting, compiling script)
    Preparing,
    /// Running schema() function
    Schema,
    /// Running prepare() function
    Prepare,
    /// Loading data via load() function
    Loading,
    /// Warming up
    Warming,
    /// Running the main benchmark
    Running,
    /// Benchmark completed successfully
    Completed,
    /// Benchmark failed
    Failed,
    /// Benchmark was cancelled
    Cancelled,
}

/// Statistics for a sampling interval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntervalStats {
    /// Operations completed in this interval
    pub ops_count: u64,
    /// Operations per second
    pub ops_per_second: f64,
    /// Errors in this interval
    pub error_count: u64,
    /// Error rate (0.0 - 1.0)
    pub error_rate: f64,
    /// Latency statistics
    pub latency: LatencyStats,
}

/// Latency statistics in microseconds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    pub mean_us: u64,
    pub min_us: u64,
    pub max_us: u64,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
    pub p999_us: u64,
}

impl ProgressEvent {
    pub fn preparing(session_id: &str) -> Self {
        Self {
            session_id: session_id.into(),
            phase: BenchmarkPhase::Preparing,
            elapsed: Duration::ZERO,
            total_duration: None,
            stats: None,
            message: None,
        }
    }

    pub fn schema(session_id: &str, elapsed: Duration) -> Self {
        Self {
            session_id: session_id.into(),
            phase: BenchmarkPhase::Schema,
            elapsed,
            total_duration: None,
            stats: None,
            message: None,
        }
    }

    pub fn prepare(session_id: &str, elapsed: Duration) -> Self {
        Self {
            session_id: session_id.into(),
            phase: BenchmarkPhase::Prepare,
            elapsed,
            total_duration: None,
            stats: None,
            message: None,
        }
    }

    pub fn loading(session_id: &str, elapsed: Duration) -> Self {
        Self {
            session_id: session_id.into(),
            phase: BenchmarkPhase::Loading,
            elapsed,
            total_duration: None,
            stats: None,
            message: None,
        }
    }

    pub fn warming(session_id: &str, elapsed: Duration) -> Self {
        Self {
            session_id: session_id.into(),
            phase: BenchmarkPhase::Warming,
            elapsed,
            total_duration: None,
            stats: None,
            message: None,
        }
    }

    pub fn running(session_id: &str, elapsed: Duration, total: Option<Duration>, stats: IntervalStats) -> Self {
        Self {
            session_id: session_id.into(),
            phase: BenchmarkPhase::Running,
            elapsed,
            total_duration: total,
            stats: Some(stats),
            message: None,
        }
    }

    pub fn completed(session_id: &str, elapsed: Duration) -> Self {
        Self {
            session_id: session_id.into(),
            phase: BenchmarkPhase::Completed,
            elapsed,
            total_duration: None,
            stats: None,
            message: None,
        }
    }

    pub fn failed(session_id: &str, elapsed: Duration, error_message: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            phase: BenchmarkPhase::Failed,
            elapsed,
            total_duration: None,
            stats: None,
            message: Some(error_message.into()),
        }
    }

    pub fn cancelled(session_id: &str, elapsed: Duration) -> Self {
        Self {
            session_id: session_id.into(),
            phase: BenchmarkPhase::Cancelled,
            elapsed,
            total_duration: None,
            stats: None,
            message: None,
        }
    }
}

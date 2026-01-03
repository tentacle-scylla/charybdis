//! Benchmark result types

use serde::{Deserialize, Serialize};

use crate::progress::LatencyStats;

/// Complete benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResults {
    /// Session ID
    pub session_id: String,
    /// Final status
    pub status: BenchmarkStatus,
    /// Total operations completed
    pub total_ops: u64,
    /// Average operations per second
    pub ops_per_second: f64,
    /// Total errors
    pub error_count: u64,
    /// Error rate (0.0 - 1.0)
    pub error_rate: f64,
    /// Total duration in milliseconds
    pub duration_ms: u64,
    /// Final latency statistics
    pub latency: LatencyStats,
    /// Time series data points
    pub timeseries: Vec<TimeseriesPoint>,
    /// Error message (if failed)
    pub error_message: Option<String>,
}

/// Benchmark completion status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkStatus {
    Completed,
    Failed,
    Cancelled,
}

/// A point in the benchmark timeseries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeseriesPoint {
    /// Timestamp in milliseconds from start
    pub time_ms: u64,
    /// Operations per second at this point
    pub ops_per_second: f64,
    /// P99 latency in microseconds
    pub latency_p99_us: u64,
    /// Error count in this interval
    pub errors: u64,
}

impl BenchmarkResults {
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.into(),
            status: BenchmarkStatus::Completed,
            total_ops: 0,
            ops_per_second: 0.0,
            error_count: 0,
            error_rate: 0.0,
            duration_ms: 0,
            latency: LatencyStats {
                mean_us: 0,
                min_us: 0,
                max_us: 0,
                p50_us: 0,
                p95_us: 0,
                p99_us: 0,
                p999_us: 0,
            },
            timeseries: Vec::new(),
            error_message: None,
        }
    }

    pub fn failed(session_id: &str, error: String) -> Self {
        Self {
            session_id: session_id.into(),
            status: BenchmarkStatus::Failed,
            error_message: Some(error),
            ..Self::new(session_id)
        }
    }

    pub fn cancelled(session_id: &str) -> Self {
        Self {
            session_id: session_id.into(),
            status: BenchmarkStatus::Cancelled,
            ..Self::new(session_id)
        }
    }
}

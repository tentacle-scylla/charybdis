//! Configuration types for Charybdis benchmarks

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Benchmark configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// ScyllaDB hosts
    pub hosts: Vec<String>,
    /// CQL port (default: 9042)
    #[serde(default = "default_port")]
    pub port: u16,
    /// Username for authentication
    pub username: Option<String>,
    /// Password for authentication
    pub password: Option<String>,
    /// Number of OS threads
    #[serde(default = "default_threads")]
    pub threads: u32,
    /// Concurrent operations per thread
    #[serde(default = "default_concurrency")]
    pub concurrency: u64,
    /// Rate limit (ops/sec), 0 = unlimited
    #[serde(default)]
    pub rate_limit: f64,
    /// Benchmark duration
    #[serde(with = "humantime_serde", default = "default_duration")]
    pub duration: Duration,
    /// Warmup duration
    #[serde(with = "humantime_serde", default = "default_warmup")]
    pub warmup: Duration,
    /// Sampling interval for progress updates
    #[serde(with = "humantime_serde", default = "default_sampling")]
    pub sampling_interval: Duration,
    /// Consistency level
    #[serde(default)]
    pub consistency: Consistency,
}

fn default_port() -> u16 { 9042 }
fn default_threads() -> u32 { 1 }
fn default_concurrency() -> u64 { 1 }
fn default_duration() -> Duration { Duration::from_secs(60) }
fn default_warmup() -> Duration { Duration::from_secs(1) }
fn default_sampling() -> Duration { Duration::from_secs(1) }

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            hosts: vec!["127.0.0.1".into()],
            port: default_port(),
            username: None,
            password: None,
            threads: default_threads(),
            concurrency: default_concurrency(),
            rate_limit: 0.0,
            duration: default_duration(),
            warmup: default_warmup(),
            sampling_interval: default_sampling(),
            consistency: Consistency::default(),
        }
    }
}

/// Workload configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadConfig {
    /// Rune script content
    pub script: String,
    /// Script parameters
    #[serde(default)]
    pub params: HashMap<String, String>,
}

impl Default for WorkloadConfig {
    fn default() -> Self {
        Self {
            script: String::new(),
            params: HashMap::new(),
        }
    }
}

/// CQL consistency level
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Consistency {
    Any,
    One,
    Two,
    Three,
    #[default]
    Quorum,
    All,
    LocalQuorum,
    EachQuorum,
    LocalOne,
}

mod humantime_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

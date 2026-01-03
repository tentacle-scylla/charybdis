//! Benchmark session management

use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio_util::sync::CancellationToken;

use latte_core::config::Interval;
use latte_core::exec::workload::{FnRef, Program, Workload};
use latte_core::exec::{par_execute_with_callback, ExecutionOptions};
use latte_core::stats::percentiles::Percentile;
use latte_core::Sample;
use rune::termcolor::Buffer;
use rune::{Diagnostics, Source, Sources};

use crate::config::{BenchmarkConfig, WorkloadConfig};
use crate::connect::connect;
use crate::error::{CharybdisError, Result};
use crate::progress::{IntervalStats, LatencyStats, ProgressEvent};
use crate::results::{BenchmarkResults, BenchmarkStatus};

/// A script diagnostic with location information
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScriptDiagnostic {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub severity: String, // "error" or "warning"
}

/// Parse line:column from diagnostic text output
/// Format: "┌─ <memory>:6:5" -> (6, 5)
fn parse_location_from_diagnostic(text: &str) -> Option<(usize, usize)> {
    // Look for pattern like "<memory>:LINE:COL" or "file.rn:LINE:COL"
    for line in text.lines() {
        if let Some(pos) = line.find(":") {
            // Try to parse "path:line:col" pattern
            let after_path = &line[pos + 1..];
            let parts: Vec<&str> = after_path.split(':').collect();
            if parts.len() >= 2 {
                if let (Ok(line_num), Ok(col_num)) = (parts[0].trim().parse::<usize>(), parts[1].trim().parse::<usize>()) {
                    return Some((line_num, col_num));
                }
            }
        }
    }
    None
}

/// Extract error message from diagnostic text
fn extract_error_message(text: &str) -> String {
    // First line usually contains the error message
    // Format: "error: Wrong number of arguments..."
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("error:") {
            return trimmed.trim_start_matches("error:").trim().to_string();
        }
        if trimmed.starts_with("warning:") {
            return trimmed.trim_start_matches("warning:").trim().to_string();
        }
    }
    // Fallback: return first non-empty line
    text.lines().find(|l| !l.trim().is_empty()).unwrap_or(text).trim().to_string()
}

/// Validate a Rune script and return detailed diagnostics if it fails.
fn validate_script_with_diagnostics(script: &str) -> std::result::Result<(), String> {
    let mut context = rune::Context::with_default_modules()
        .map_err(|e| format!("Failed to create Rune context: {}", e))?;

    // Install latte's scripting modules
    latte_core::scripting::install(&mut context, std::collections::HashMap::new());

    let source = Source::memory(script)
        .map_err(|e| format!("Failed to create source: {}", e))?;

    let mut sources = Sources::new();
    sources
        .insert(source)
        .map_err(|e| format!("Failed to insert source: {}", e))?;

    let mut diagnostics = Diagnostics::new();

    let _result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        // Capture diagnostics to plain text first
        let mut buffer = Buffer::no_color();
        if diagnostics.emit(&mut buffer, &sources).is_ok() {
            let diag_text = String::from_utf8_lossy(buffer.as_slice()).to_string();

            // Parse location and message from the text
            let (line, column) = parse_location_from_diagnostic(&diag_text).unwrap_or((1, 1));
            let message = extract_error_message(&diag_text);
            let severity = if diagnostics.has_error() { "error" } else { "warning" };

            let diag = ScriptDiagnostic {
                message,
                line,
                column,
                severity: severity.to_string(),
            };

            // Return as JSON for structured parsing on frontend
            if let Ok(json) = serde_json::to_string(&vec![diag]) {
                return Err(format!("DIAG_JSON:{}", json));
            }

            // Fallback to plain text
            return Err(diag_text);
        } else {
            return Err("Script compilation failed".to_string());
        }
    }

    Ok(())
}

/// A benchmark session that can be run with progress callbacks.
pub struct BenchmarkSession {
    id: String,
    config: BenchmarkConfig,
    workload: WorkloadConfig,
    cancellation: CancellationToken,
    started: AtomicBool,
}

impl BenchmarkSession {
    /// Create a new benchmark session.
    pub fn new(config: BenchmarkConfig, workload: WorkloadConfig) -> Result<Self> {
        let id = uuid::Uuid::new_v4().to_string();
        Ok(Self {
            id,
            config,
            workload,
            cancellation: CancellationToken::new(),
            started: AtomicBool::new(false),
        })
    }

    /// Get the session ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Request cancellation of the running benchmark.
    pub fn cancel(&self) {
        self.cancellation.cancel();
    }

    /// Check if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    /// Run the benchmark with a progress callback.
    ///
    /// The callback will be invoked periodically with progress events.
    /// Returns the complete benchmark results when finished.
    pub async fn run<F>(&self, callback: F) -> Result<BenchmarkResults>
    where
        F: Fn(ProgressEvent) + Send + Sync + 'static,
    {
        // Ensure session is only run once
        if self.started.swap(true, Ordering::SeqCst) {
            return Err(CharybdisError::AlreadyStarted);
        }

        let callback = Arc::new(callback);
        let start = Instant::now();

        // Emit preparing event
        callback(ProgressEvent::preparing(&self.id));

        // Run the real benchmark
        self.run_benchmark(&callback, start).await
    }

    /// Run the real benchmark using latte-core
    async fn run_benchmark<F>(&self, callback: &Arc<F>, start: Instant) -> Result<BenchmarkResults>
    where
        F: Fn(ProgressEvent) + Send + Sync + 'static,
    {
        // Check for cancellation
        if self.is_cancelled() {
            return Ok(BenchmarkResults::cancelled(&self.id));
        }

        // Connect to ScyllaDB
        let mut context = match connect(&self.config).await {
            Ok(ctx) => ctx,
            Err(e) => {
                callback(ProgressEvent::failed(&self.id, start.elapsed(), e.to_string()));
                return Err(e);
            }
        };

        // Compile the Rune script
        let source = Source::memory(&self.workload.script)
            .map_err(|e| CharybdisError::Config(format!("Failed to create script source: {}", e)))?;

        let params = self.workload.params.clone();

        // First, validate the script to get detailed diagnostics
        if let Err(diag_err) = validate_script_with_diagnostics(&self.workload.script) {
            callback(ProgressEvent::failed(&self.id, start.elapsed(), &diag_err));
            return Err(CharybdisError::Script(diag_err));
        }

        let mut program = match Program::new(source, params) {
            Ok(p) => p,
            Err(e) => {
                let err_msg = format!("{}", e);
                callback(ProgressEvent::failed(&self.id, start.elapsed(), &err_msg));
                return Err(e.into());
            }
        };

        // Run schema function if it exists
        if program.has_schema() {
            callback(ProgressEvent::schema(&self.id, start.elapsed()));
            if let Err(e) = program.schema(&mut context).await {
                let err_msg = format!("{}", e);
                callback(ProgressEvent::failed(&self.id, start.elapsed(), &err_msg));
                return Err(e.into());
            }
        }

        // Check for cancellation after schema
        if self.is_cancelled() {
            return Ok(BenchmarkResults::cancelled(&self.id));
        }

        // Run prepare function if it exists
        if program.has_prepare() {
            callback(ProgressEvent::prepare(&self.id, start.elapsed()));
            if let Err(e) = program.prepare(&mut context).await {
                let err_msg = format!("{}", e);
                callback(ProgressEvent::failed(&self.id, start.elapsed(), &err_msg));
                return Err(e.into());
            }
        }

        // Check for cancellation after prepare
        if self.is_cancelled() {
            return Ok(BenchmarkResults::cancelled(&self.id));
        }

        // TODO: Run load function if it exists (requires separate par_execute call like main.rs)
        // For now, load() will run as part of the main benchmark if duration is used for loading

        // Check for cancellation before warmup
        if self.is_cancelled() {
            return Ok(BenchmarkResults::cancelled(&self.id));
        }

        // Warmup phase
        callback(ProgressEvent::warming(&self.id, start.elapsed()));

        // Determine which function to run (default to "run")
        let run_fn = FnRef::new("run");
        let functions = vec![(run_fn.clone(), 1.0)];

        // Create the workload
        let workload = Workload::new(context, program, &functions);

        // Configure execution
        let exec_options = ExecutionOptions {
            duration: Interval::Time(self.config.duration),
            cycle_range: (0, i64::MAX),
            rate: if self.config.rate_limit > 0.0 { Some(self.config.rate_limit) } else { None },
            rate_sine_amplitude: None,
            rate_sine_period: Duration::from_secs(60),
            threads: NonZeroUsize::new(self.config.threads.max(1) as usize).unwrap(),
            concurrency: NonZeroUsize::new(self.config.concurrency.max(1) as usize).unwrap(),
        };

        let sampling = Interval::Time(self.config.sampling_interval);

        // Create results structure
        let mut results = BenchmarkResults::new(&self.id);
        let session_id = self.id.clone();
        let callback_clone = callback.clone();

        // Histogram writer (None for now)
        let mut hdrh_writer: Option<Box<dyn latte_core::stats::histogram::HistogramWriter>> = None;

        // Create a progress callback that converts Sample to our ProgressEvent
        let session_id_for_progress = self.id.clone();
        let total_duration = Some(self.config.duration);
        let progress_callback_clone = callback.clone();
        let progress_callback = move |sample: &Sample| {
            let interval_stats = sample_to_interval_stats(sample);
            let elapsed = Duration::from_secs_f32(sample.time_s);
            progress_callback_clone(ProgressEvent::running(
                &session_id_for_progress,
                elapsed,
                total_duration,
                interval_stats,
            ));
        };

        // Run the benchmark with cancellation support
        let benchmark_result = tokio::select! {
            result = par_execute_with_callback(
                "benchmark",
                &exec_options,
                sampling,
                workload,
                false, // show_progress (we handle our own)
                false, // keep_log
                &mut hdrh_writer,
                progress_callback,
            ) => {
                result
            }
            _ = self.cancellation.cancelled() => {
                results.status = BenchmarkStatus::Cancelled;
                callback_clone(ProgressEvent::cancelled(&session_id, start.elapsed()));
                return Ok(results);
            }
        };

        match benchmark_result {
            Ok(stats) => {
                let final_elapsed = start.elapsed();

                // Convert latte stats to our results format
                results.total_ops = stats.cycle_count;
                results.ops_per_second = stats.cycle_throughput.value;
                results.error_count = stats.error_count;
                results.error_rate = if stats.cycle_count > 0 {
                    stats.error_count as f64 / stats.cycle_count as f64
                } else {
                    0.0
                };
                results.duration_ms = final_elapsed.as_millis() as u64;

                // Extract latency stats from cycle_latency
                let latency = &stats.cycle_latency;
                results.latency = LatencyStats {
                    mean_us: (latency.mean.value * 1_000_000.0) as u64,  // convert seconds to microseconds
                    min_us: (latency.percentiles.get(Percentile::Min).value * 1_000_000.0) as u64,
                    max_us: (latency.percentiles.get(Percentile::Max).value * 1_000_000.0) as u64,
                    p50_us: (latency.percentiles.get(Percentile::P50).value * 1_000_000.0) as u64,
                    p95_us: (latency.percentiles.get(Percentile::P95).value * 1_000_000.0) as u64,
                    p99_us: (latency.percentiles.get(Percentile::P99).value * 1_000_000.0) as u64,
                    p999_us: (latency.percentiles.get(Percentile::P99_9).value * 1_000_000.0) as u64,
                };

                callback(ProgressEvent::completed(&self.id, final_elapsed));
                Ok(results)
            }
            Err(e) => {
                let err_msg = format!("{}", e);
                callback(ProgressEvent::failed(&self.id, start.elapsed(), &err_msg));
                Err(e.into())
            }
        }
    }
}

/// Convert a latte-core Sample to our IntervalStats
fn sample_to_interval_stats(sample: &Sample) -> IntervalStats {
    let latency = &sample.cycle_latency;
    IntervalStats {
        ops_count: sample.cycle_count,
        ops_per_second: sample.cycle_throughput as f64,
        error_count: sample.cycle_error_count,
        error_rate: if sample.cycle_count > 0 {
            sample.cycle_error_count as f64 / sample.cycle_count as f64
        } else {
            0.0
        },
        latency: LatencyStats {
            mean_us: (latency.mean.value * 1_000_000.0) as u64,
            min_us: (latency.percentiles.get(Percentile::Min).value * 1_000_000.0) as u64,
            max_us: (latency.percentiles.get(Percentile::Max).value * 1_000_000.0) as u64,
            p50_us: (latency.percentiles.get(Percentile::P50).value * 1_000_000.0) as u64,
            p95_us: (latency.percentiles.get(Percentile::P95).value * 1_000_000.0) as u64,
            p99_us: (latency.percentiles.get(Percentile::P99).value * 1_000_000.0) as u64,
            p999_us: (latency.percentiles.get(Percentile::P99_9).value * 1_000_000.0) as u64,
        },
    }
}

//! latte-core - ScyllaDB benchmarking library
//!
//! Core functionality from latte, exposed as a library.

pub mod config;
pub mod error;
pub mod exec;
pub mod report;
pub mod scripting;
pub mod stats;
pub mod version;

// Re-exports that upstream modules expect at crate root
pub use config::Interval;
pub use error::{LatteError, Result};
pub use exec::cycle::BoundedCycleCounter;
pub use exec::progress::Progress;
pub use exec::workload::{Workload, WorkloadStats};
pub use stats::{BenchmarkStats, Recorder, Sample};

use std::path::Path;
use std::process::exit;

/// Load a report from disk, exit on error (used by report/plot.rs)
pub fn load_report_or_abort(path: &Path) -> report::Report {
    match report::Report::load(path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "error: Failed to read report from {}: {}",
                path.display(),
                e
            );
            exit(1)
        }
    }
}

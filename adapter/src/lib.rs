//! Charybdis - Clean API wrapper for latte benchmarking
//!
//! Provides a simplified, callback-based API for running ScyllaDB benchmarks.

use rune::termcolor::Buffer;
use rune::{Diagnostics, Source, Sources};

pub mod config;
pub mod connect;
pub mod error;
pub mod progress;
pub mod results;
pub mod session;

pub use config::{BenchmarkConfig, WorkloadConfig};
pub use error::{CharybdisError, Result};
pub use progress::ProgressEvent;
pub use results::BenchmarkResults;
pub use session::{BenchmarkSession, ScriptDiagnostic};

/// Validate a Rune workload script without running it.
/// Returns structured diagnostics on failure.
pub fn validate_script(script: &str) -> Result<()> {
    if script.trim().is_empty() {
        return Err(CharybdisError::Validation("Script is empty".into()));
    }

    // Check for required function signatures
    let has_valid_fn = script.contains("fn run")
        || script.contains("fn load")
        || script.contains("fn schema")
        || script.contains("fn prepare")
        || script.contains("fn read")
        || script.contains("fn write")
        || script.contains("fn erase");

    if !has_valid_fn {
        return Err(CharybdisError::Validation(
            "Script must contain at least one public function (run, load, schema, prepare, read, write, or erase)".into()
        ));
    }

    // Now do actual Rune compilation to catch syntax errors
    let mut context = rune::Context::with_default_modules()
        .map_err(|e| CharybdisError::Validation(format!("Failed to create Rune context: {}", e)))?;

    // Install latte's scripting modules
    latte_core::scripting::install(&mut context, std::collections::HashMap::new());

    let source = Source::memory(script)
        .map_err(|e| CharybdisError::Validation(format!("Failed to create source: {}", e)))?;

    let mut sources = Sources::new();
    sources
        .insert(source)
        .map_err(|e| CharybdisError::Validation(format!("Failed to insert source: {}", e)))?;

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

            // Return as JSON for structured parsing
            if let Ok(json) = serde_json::to_string(&vec![diag]) {
                return Err(CharybdisError::Script(format!("DIAG_JSON:{}", json)));
            }

            return Err(CharybdisError::Script(diag_text));
        } else {
            return Err(CharybdisError::Validation("Script compilation failed".into()));
        }
    }

    Ok(())
}

/// Parse line:column from diagnostic text output
fn parse_location_from_diagnostic(text: &str) -> Option<(usize, usize)> {
    for line in text.lines() {
        if let Some(pos) = line.find(':') {
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
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("error:") {
            return trimmed.trim_start_matches("error:").trim().to_string();
        }
        if trimmed.starts_with("warning:") {
            return trimmed.trim_start_matches("warning:").trim().to_string();
        }
    }
    text.lines().find(|l| !l.trim().is_empty()).unwrap_or(text).trim().to_string()
}

/// Get list of built-in workload templates.
pub fn builtin_workloads() -> Vec<WorkloadTemplate> {
    vec![
        WorkloadTemplate {
            name: "basic_read".into(),
            description: "Simple read benchmark against system.local".into(),
            script: r#"
pub async fn run(ctx) {
    ctx.execute("SELECT * FROM system.local").await?;
}
"#.into(),
        },
        WorkloadTemplate {
            name: "basic_write".into(),
            description: "Simple write benchmark with inserts".into(),
            script: r#"
pub async fn schema(ctx) {
    ctx.execute("CREATE KEYSPACE IF NOT EXISTS charybdis WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 1}").await?;
    ctx.execute("CREATE TABLE IF NOT EXISTS charybdis.basic (id bigint PRIMARY KEY, value text)").await?;
}

pub async fn run(ctx, i) {
    ctx.execute("INSERT INTO charybdis.basic (id, value) VALUES (?, ?)", (i, latte::text(16))).await?;
}
"#.into(),
        },
    ]
}

/// A built-in workload template
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkloadTemplate {
    pub name: String,
    pub description: String,
    pub script: String,
}
// Test cache

//! End-to-end CLI tests. Per CLAUDE.md, these are the only kind of
//! test the project ships: they drive the binary as a black box against
//! fixture input and assert on rendered outputs and exit codes.

#[path = "cli/support.rs"]
mod support;

#[path = "cli/cache_and_debug.rs"]
mod cache_and_debug;
#[path = "cli/config.rs"]
mod config;
#[path = "cli/detection.rs"]
mod detection;
#[path = "cli/embedding_ollama.rs"]
mod embedding_ollama;
#[path = "cli/embedding_stub.rs"]
mod embedding_stub;
#[path = "cli/from_report.rs"]
mod from_report;
#[path = "cli/invocation.rs"]
mod invocation;
#[path = "cli/logging.rs"]
mod logging;
#[path = "cli/metrics.rs"]
mod metrics;
#[path = "cli/thresholds.rs"]
mod thresholds;

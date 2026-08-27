//! Command-line wrapper for the reproducible shared-subtree benchmark.

use std::{
    fs,
    io::{self, Write},
};

use anyhow::{Context, Result};
use deslop_core::overlap::benchmark::{compare, measure, BenchmarkReport};

/// Default run label.
const DEFAULT_LABEL: &str = "run";

/// Default repetitions in each timing sample.
const DEFAULT_REPETITIONS: &str = "100";

/// Selects every workload.
const DEFAULT_SHAPE: &str = "all";

/// Runs the benchmark and writes one JSON artifact to standard output.
fn main() -> Result<()> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let label = argument(&arguments, 0).unwrap_or(DEFAULT_LABEL);
    let repetitions = repetitions(&arguments)?;
    let baseline = argument(&arguments, 2);
    let shape = argument(&arguments, 3).unwrap_or(DEFAULT_SHAPE);
    let mut report = measure(label, repetitions, shape).map_err(anyhow::Error::msg)?;
    if let Some(path) = baseline {
        compare(&mut report, &read_report(path)?);
    }
    emit(&report)
}

/// Optional positional argument.
fn argument(arguments: &[String], index: usize) -> Option<&str> {
    arguments
        .get(index)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
}

/// Validated repetitions argument.
fn repetitions(arguments: &[String]) -> Result<usize> {
    let value = argument(arguments, 1)
        .unwrap_or(DEFAULT_REPETITIONS)
        .parse::<usize>()
        .context("repetitions must be a positive integer")?;
    anyhow::ensure!(value > 0, "repetitions must be positive");
    Ok(value)
}

/// Reads one previous benchmark artifact.
fn read_report(path: &str) -> Result<BenchmarkReport> {
    let bytes = fs::read(path).with_context(|| format!("failed to read baseline {path}"))?;
    serde_json::from_slice(&bytes).with_context(|| format!("failed to parse baseline {path}"))
}

/// Writes the artifact to standard output.
fn emit(report: &BenchmarkReport) -> Result<()> {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer_pretty(&mut output, report).context("failed to serialize benchmark")?;
    output.write_all(b"\n").context("failed to write benchmark")
}

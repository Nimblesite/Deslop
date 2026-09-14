//! Reproducible workloads for profiling exact shared-subtree alignment.

use std::{hint::black_box, time::Instant};

use serde::{Deserialize, Serialize};

use super::alignment::{Aligner, PostNode};

/// Independent timing samples per workload.
const SAMPLE_COUNT: usize = 7;

/// Warm-up repetitions excluded from measurements.
const WARMUP_REPETITIONS: usize = 5;

/// Exact-alignment production cap.
const MAX_NODES: usize = super::ALIGNMENT_MAX_NODES;

/// Levels in the perfect binary-tree workload.
const BALANCED_LEVELS: usize = 9;

/// Left-side kind; every right-side node differs to force real edit work.
const LEFT_KIND: &str = "alpha";

/// Right-side kind; every node must be relabelled.
const RIGHT_KIND: &str = "beta";

/// Selects every benchmark workload.
const ALL_SHAPES: &str = "all";

/// Fixed-point scale for before/after speedup (`1000` means unchanged).
const SPEEDUP_SCALE: u128 = 1_000;

/// Complete benchmark artifact.
#[derive(Debug, Deserialize, Serialize)]
pub struct BenchmarkReport {
    /// User-selected run label.
    label: String,
    /// Package version.
    deslop_version: String,
    /// Compilation target operating system.
    os: String,
    /// Compilation target architecture.
    arch: String,
    /// Repetitions per timing sample.
    repetitions: usize,
    /// Timing samples per workload.
    sample_count: usize,
    /// Measured workload reports.
    workloads: Vec<WorkloadReport>,
    /// Optional comparison with an earlier artifact.
    comparisons: Vec<Comparison>,
}

/// One shape's raw and aggregate measurement.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct WorkloadReport {
    /// Stable workload name.
    name: String,
    /// Nodes on the left endpoint.
    left_nodes: usize,
    /// Nodes on the right endpoint.
    right_nodes: usize,
    /// Raw elapsed nanoseconds, one entry per sample.
    elapsed_ns: Vec<u128>,
    /// Median elapsed nanoseconds.
    median_ns: u128,
    /// Sum of returned distances, preventing dead-code elimination.
    checksum: usize,
}

/// Before/after result calculated from two artifacts.
#[derive(Debug, Deserialize, Serialize)]
struct Comparison {
    /// Stable workload name.
    name: String,
    /// Baseline median nanoseconds.
    baseline_median_ns: u128,
    /// Current median nanoseconds.
    current_median_ns: u128,
    /// Baseline/current in thousandths; greater than 1000 is faster.
    speedup_thousandths: u128,
}

/// One in-memory endpoint pair.
struct Workload {
    /// Stable workload name.
    name: &'static str,
    /// Left post-order sequence.
    left: Vec<PostNode>,
    /// Right post-order sequence.
    right: Vec<PostNode>,
}

/// Runs every selected deterministic workload.
///
/// # Errors
///
/// Returns an error for zero repetitions or an unknown shape.
pub fn measure(
    label: &str,
    repetitions: usize,
    selected: &str,
) -> Result<BenchmarkReport, &'static str> {
    if repetitions == 0 {
        return Err("repetitions must be positive");
    }
    let selected_workloads = workloads(selected)?;
    Ok(benchmark_report(
        label,
        repetitions,
        selected_workloads
            .iter()
            .map(|workload| measure_workload(workload, repetitions))
            .collect(),
    ))
}

/// Wraps measured workloads in the shared reproducibility metadata.
pub(crate) fn benchmark_report(
    label: &str,
    repetitions: usize,
    workloads: Vec<WorkloadReport>,
) -> BenchmarkReport {
    BenchmarkReport {
        label: label.to_owned(),
        deslop_version: env!("CARGO_PKG_VERSION").to_owned(),
        os: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
        repetitions,
        sample_count: SAMPLE_COUNT,
        workloads,
        comparisons: Vec::new(),
    }
}

/// Adds Rust-calculated before/after comparisons to `current`.
pub fn compare(current: &mut BenchmarkReport, baseline: &BenchmarkReport) {
    current.comparisons = current
        .workloads
        .iter()
        .filter_map(|report| comparison(baseline, report))
        .collect();
}

/// Builds every requested deterministic shape.
fn workloads(selected: &str) -> Result<Vec<Workload>, &'static str> {
    let all = [flat_workload(), chain_workload(), balanced_workload()];
    let selected = all
        .into_iter()
        .filter(|workload| selected == ALL_SHAPES || selected == workload.name)
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err("shape must be all, flat, chain, or balanced");
    }
    Ok(selected)
}

/// Wide, shallow tree with many leaf keyroots.
fn flat_workload() -> Workload {
    Workload {
        name: "flat",
        left: flat_sequence(LEFT_KIND),
        right: flat_sequence(RIGHT_KIND),
    }
}

/// Deep tree with one keyroot and one large forest grid.
fn chain_workload() -> Workload {
    Workload {
        name: "chain",
        left: chain_sequence(LEFT_KIND),
        right: chain_sequence(RIGHT_KIND),
    }
}

/// Branching tree representative of nested syntax.
fn balanced_workload() -> Workload {
    Workload {
        name: "balanced",
        left: balanced_sequence(LEFT_KIND),
        right: balanced_sequence(RIGHT_KIND),
    }
}

/// Measures one workload with warm-up and raw repeated samples.
fn measure_workload(workload: &Workload, repetitions: usize) -> WorkloadReport {
    measure_repeated(
        workload.name,
        workload.left.len(),
        workload.right.len(),
        repetitions,
        |count| run(workload, count),
    )
}

/// Measures one deterministic operation with the shared warm-up and
/// raw-sample policy.
pub(crate) fn measure_repeated(
    name: &str,
    left_nodes: usize,
    right_nodes: usize,
    repetitions: usize,
    mut operation: impl FnMut(usize) -> usize,
) -> WorkloadReport {
    let _warmup = operation(WARMUP_REPETITIONS);
    let samples = (0..SAMPLE_COUNT)
        .map(|_sample| timed_repeated(repetitions, &mut operation))
        .collect::<Vec<_>>();
    workload_report(name, left_nodes, right_nodes, samples)
}

/// Reduces raw samples into one serializable workload report.
fn workload_report(
    name: &str,
    left_nodes: usize,
    right_nodes: usize,
    mut samples: Vec<(u128, usize)>,
) -> WorkloadReport {
    let checksum = samples
        .iter()
        .fold(0_usize, |sum, sample| sum.saturating_add(sample.1));
    let elapsed_ns = samples.iter().map(|sample| sample.0).collect::<Vec<_>>();
    samples.sort_unstable_by_key(|sample| sample.0);
    WorkloadReport {
        name: name.to_owned(),
        left_nodes,
        right_nodes,
        median_ns: median(&samples),
        elapsed_ns,
        checksum,
    }
}

/// Middle elapsed value after samples have been sorted.
fn median(samples: &[(u128, usize)]) -> u128 {
    let middle = SAMPLE_COUNT.checked_div(2).unwrap_or(0);
    samples.get(middle).map_or(0, |sample| sample.0)
}

/// Times fixed work and returns `(nanoseconds, checksum)`.
fn timed_repeated(repetitions: usize, operation: &mut impl FnMut(usize) -> usize) -> (u128, usize) {
    let started = Instant::now();
    let checksum = operation(repetitions);
    (started.elapsed().as_nanos(), checksum)
}

/// Executes exact alignment repeatedly with one reusable aligner.
fn run(workload: &Workload, repetitions: usize) -> usize {
    let mut aligner = Aligner::default();
    (0..repetitions).fold(0_usize, |sum, _iteration| {
        sum.saturating_add(black_box(
            aligner.distance(black_box(&workload.left), black_box(&workload.right)),
        ))
    })
}

/// Flat post-order sequence: leaves followed by their common root.
fn flat_sequence(kind: &'static str) -> Vec<PostNode> {
    let mut nodes = (1..MAX_NODES)
        .map(|position| PostNode {
            kind,
            leftmost: position,
        })
        .collect::<Vec<_>>();
    nodes.push(PostNode { kind, leftmost: 1 });
    nodes
}

/// Chain post-order sequence: every ancestor shares the first leaf.
fn chain_sequence(kind: &'static str) -> Vec<PostNode> {
    (0..MAX_NODES)
        .map(|_position| PostNode { kind, leftmost: 1 })
        .collect()
}

/// Perfect binary-tree post-order sequence.
fn balanced_sequence(kind: &'static str) -> Vec<PostNode> {
    let mut nodes = Vec::new();
    let _leftmost = append_balanced(BALANCED_LEVELS, kind, &mut nodes);
    nodes
}

/// Appends one perfect binary tree and returns its leftmost leaf.
fn append_balanced(levels: usize, kind: &'static str, nodes: &mut Vec<PostNode>) -> usize {
    if levels <= 1 {
        let leftmost = nodes.len().saturating_add(1);
        nodes.push(PostNode { kind, leftmost });
        return leftmost;
    }
    let leftmost = append_balanced(levels.saturating_sub(1), kind, nodes);
    let _right = append_balanced(levels.saturating_sub(1), kind, nodes);
    nodes.push(PostNode { kind, leftmost });
    leftmost
}

/// Compares one current workload with its matching baseline.
fn comparison(baseline: &BenchmarkReport, current: &WorkloadReport) -> Option<Comparison> {
    let previous = baseline
        .workloads
        .iter()
        .find(|report| report.name == current.name)?;
    Some(Comparison {
        name: current.name.clone(),
        baseline_median_ns: previous.median_ns,
        current_median_ns: current.median_ns,
        speedup_thousandths: speedup(previous.median_ns, current.median_ns),
    })
}

/// Fixed-point `baseline/current` speedup.
fn speedup(baseline: u128, current: u128) -> u128 {
    baseline
        .saturating_mul(SPEEDUP_SCALE)
        .checked_div(current.max(1))
        .unwrap_or(0)
}

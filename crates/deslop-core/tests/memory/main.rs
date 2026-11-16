//! [CLONE-NOISE-LITERAL-VARIATION-CALLS-BOUNDED-MEMO] [TEST-MEMORY-BINARY] Heap held by one scan must grow with the code it scans, never with the square of it.
//!
//! The literal-variation filter memoises one call shape per cluster member. When every member sits inside one immediately-invoked wrapper — a bundled module, `(function () { … })()` — the call around each member is that wrapper, and its callee is the whole module. A memo cell that copied the callee's syntax held a copy of the entire file per member: on the React repository that was 2.7 GB of a 5.5 GB peak.
//!
//! This binary installs a counting global allocator (`peak_alloc`), so it measures live heap bytes exactly and deterministically — no RSS sampling, no sleeps. It is its own test target because a global allocator is process-wide: inside the shared suite every concurrently running test would count.

use std::{fmt::Write as _, fs, path::Path};

use anyhow::{Context, Result};
use deslop_core::{
    pipeline::{run, EmbeddingSettings, PipelineConfig},
    report::Report,
    EmbeddingMode,
};
use peak_alloc::PeakAlloc;

/// Counts every live heap byte and the high-water mark since the last reset.
#[global_allocator]
static ALLOCATOR: PeakAlloc = PeakAlloc;

/// Copies of the registration function in the module.
const COPIES: usize = 160;
/// How much more heap the wrapped module may cost than the same functions written bare, in percent. The wrapper adds one call and a few nodes; copying the module into every member's memo cell multiplied the scan's heap.
const MAX_WRAPPER_OVERHEAD_PERCENT: usize = 125;
/// Scale for percentages.
const PERCENT: usize = 100;
/// Small enough that every registration function is a cluster member.
const MIN_NODES: u32 = 15;
/// The module's file name.
const MODULE_FILE: &str = "bundle.js";

/// One registration function, varying only its string payload, so the family reaches the literal-variation filter.
fn registration(index: usize) -> String {
    format!(
        "  function registerEntry{index}(registry, items) {{\n    registry.add(\"entry-{index}\", items.map(function (item) {{ return item.value * 2 + item.offset; }}));\n    registry.flush(\"entry-{index}\");\n    return registry.size() + items.length;\n  }}\n"
    )
}

/// The module's functions, inside one immediately-invoked wrapper when `wrapped`.
fn module(wrapped: bool) -> String {
    let mut module = String::from(if wrapped {
        "(function () {\n  'use strict';\n"
    } else {
        "'use strict';\n"
    });
    (0..COPIES).for_each(|index| module.push_str(&registration(index)));
    if wrapped {
        let _written = writeln!(module, "}})();");
    }
    module
}

/// Scans a workspace holding only the module; returns the report and the heap the scan added at its peak.
fn scan_peak(wrapped: bool) -> Result<(Report, usize)> {
    let root = tempfile::tempdir().context("tempdir")?;
    fs::write(root.path().join(MODULE_FILE), module(wrapped)).context("write module")?;
    let baseline = ALLOCATOR.current_usage();
    ALLOCATOR.reset_peak_usage();
    let report = scan(root.path())?;
    Ok((report, ALLOCATOR.peak_usage().saturating_sub(baseline)))
}
/// Analyses `root` with embeddings off, so the run is deterministic.
fn scan(root: &Path) -> Result<Report> {
    run(&PipelineConfig {
        root: root.to_path_buf(),
        min_nodes: MIN_NODES,
        config_path: None,
        embedding: EmbeddingSettings {
            mode: EmbeddingMode::Off,
            provider: None,
            batch_yield: None,
            progress: None,
        },
        incremental: false,
    })
    .context("pipeline run")
}

/// Cluster members across the report, visible and hidden.
fn examined_members(report: &Report) -> usize {
    report
        .clusters
        .iter()
        .map(|cluster| cluster.occurrences.len())
        .sum::<usize>()
        .saturating_add(report.clusters_hidden)
}

#[test]
fn wrapping_a_module_in_one_call_does_not_multiply_its_heap() -> Result<()> {
    let (bare_report, bare) = scan_peak(false)?;
    let (wrapped_report, wrapped) = scan_peak(true)?;
    assert!(
        examined_members(&bare_report) >= COPIES && examined_members(&wrapped_report) >= COPIES,
        "every registration function must be a cluster member, or this test measures nothing"
    );
    assert!(bare > 0, "the bare scan must allocate");
    assert!(
        wrapped.saturating_mul(PERCENT) <= bare.saturating_mul(MAX_WRAPPER_OVERHEAD_PERCENT),
        "the wrapped module peaked at {wrapped} heap bytes against {bare} bare: {}%, past the {MAX_WRAPPER_OVERHEAD_PERCENT}% bound",
        wrapped.saturating_mul(PERCENT) / bare,
    );
    Ok(())
}

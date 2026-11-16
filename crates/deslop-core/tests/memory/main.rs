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

/// Copies of the wrapped function in the small module.
const SMALL_COPIES: usize = 40;
/// Copies in the large module: four times the small one.
const LARGE_COPIES: usize = 160;
/// How much more heap the large scan may add than the small one. The module is four times larger, so linear growth is four; the slack covers fixed per-scan costs. Copying the module into every member's memo cell grows sixteen-fold.
const MAX_GROWTH: usize = 6;
/// Small enough that every wrapped function is a cluster member.
const MIN_NODES: u32 = 15;
/// The wrapped module's file name.
const MODULE_FILE: &str = "bundle.js";

/// One registration function, varying only its string payload, so the family reaches the literal-variation filter.
fn registration(index: usize) -> String {
    format!(
        "  function registerEntry{index}(registry, items) {{\n    registry.add(\"entry-{index}\", items.map(function (item) {{ return item.value * 2 + item.offset; }}));\n    registry.flush(\"entry-{index}\");\n    return registry.size() + items.length;\n  }}\n"
    )
}

/// A module of `copies` functions inside one immediately-invoked wrapper.
fn wrapped_module(copies: usize) -> String {
    let mut module = String::from("(function () {\n  'use strict';\n");
    (0..copies).for_each(|index| module.push_str(&registration(index)));
    let _written = writeln!(module, "}})();");
    module
}

/// Scans a workspace holding only the wrapped module; returns the report and the heap the scan added at its peak.
fn scan_peak(copies: usize) -> Result<(Report, usize)> {
    let root = tempfile::tempdir().context("tempdir")?;
    fs::write(root.path().join(MODULE_FILE), wrapped_module(copies)).context("write module")?;
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
fn a_wrapped_module_costs_heap_in_proportion_to_its_size() -> Result<()> {
    let (small_report, small) = scan_peak(SMALL_COPIES)?;
    let (large_report, large) = scan_peak(LARGE_COPIES)?;
    assert!(
        examined_members(&small_report) > 0 && examined_members(&large_report) > 0,
        "the wrapped functions must form a family, or this test measures nothing"
    );
    assert!(small > 0, "the small scan must allocate");
    assert!(
        large <= small.saturating_mul(MAX_GROWTH),
        "a module {}x larger added {large} heap bytes against {small}: {:.1}x, past the {MAX_GROWTH}x linear bound",
        LARGE_COPIES / SMALL_COPIES,
        large as f64 / small as f64,
    );
    Ok(())
}

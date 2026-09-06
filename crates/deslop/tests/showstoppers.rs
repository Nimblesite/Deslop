//! End-to-end regression coverage for the three showstopper issues
//! fixed together:
//!
//! - #140 [EXCLUSION-CONFIG]: `report_hide` occurrences must not
//!   dominate Top Offenders ranking in mixed clusters.
//! - #141 [MCP-SAFETY]: MCP / CLI must not surface duplicates from
//!   wrong-root locations such as `~/.cargo/git/checkouts/...`.
//! - #142 [EXCLUSION-CONFIG]: generated Cargo dependency boilerplate
//!   under `.cargo/` must never enter discovery as actionable code.

use std::fs;

use anyhow::Result;

mod common;
use crate::common::*;

fn visible_count(cluster: &serde_json::Value) -> usize {
    cluster
        .get("occurrences")
        .and_then(serde_json::Value::as_array)
        .map_or(0, |values| {
            values
                .iter()
                .filter(|occ| {
                    !occ.get("hidden")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false)
                })
                .count()
        })
}

// A Summer-style C# class: one method, if-guard, for-loop accumulator.
// Structural shape is stable across copies (only identifier names
// change) so all copies collapse into one cluster.
fn csharp_summer_body(namespace: &str, class: &str, method: &str, accumulator: &str) -> String {
    format!(
        "namespace {namespace}\n\
         {{\n\
         public class {class}\n\
         {{\n\
         public int {method}(int limit)\n\
         {{\n\
         if (limit < 0) {{ return 0; }}\n\
         int {accumulator} = 0;\n\
         for (int idx = 0; idx < limit; idx = idx + 1) {{ {accumulator} = {accumulator} + idx; }}\n\
         return {accumulator};\n\
         }}\n\
         }}\n\
         }}\n",
    )
}

// A Python class with the same structural shape across copies but a
// different parser language than the C# Summer pattern. With
// `allow_cross_language_comparison = false` (the default) these
// cannot fuse with C# clusters, so the two cluster sizes stay
// independent in the test.
fn python_trio_body(class: &str) -> String {
    format!(
        "class {class}:\n\
         \x20\x20\x20\x20def alpha(self):\n\
         \x20\x20\x20\x20\x20\x20\x20\x20if self.value < 0:\n\
         \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20return 0\n\
         \x20\x20\x20\x20\x20\x20\x20\x20total = 0\n\
         \x20\x20\x20\x20\x20\x20\x20\x20for idx in range(self.value):\n\
         \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20total = total + idx\n\
         \x20\x20\x20\x20\x20\x20\x20\x20return total\n",
    )
}

// ---------------------------------------------------------------------------
// #140 — report_hide occurrences must not dominate mixed-cluster ranking.
// ---------------------------------------------------------------------------

#[test]
fn issue_140_visible_cluster_outranks_hidden_dominated_mixed_cluster() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    let visible_dir = scan_root.join("visible");
    // Built-in `BUILTIN_REPORT_HIDE_COMPONENTS` already includes
    // "generated" so any file under this dir is marked hidden at
    // render time.
    let generated_dir = scan_root.join("generated");
    fs::create_dir_all(&visible_dir)?;
    fs::create_dir_all(&generated_dir)?;

    // Cluster A: three visible Python copies (3 visible, 0 hidden).
    // Python files do not cluster with the C# Summer pattern at
    // default config because cross-language comparison is off.
    for index in 0..3 {
        fs::write(
            visible_dir.join(format!("trio_{index}.py")),
            python_trio_body(&format!("TrioCls{index}")),
        )?;
    }

    // Cluster B: one visible Summer copy plus five hidden copies in
    // `generated/`. Before the fix, B's `cluster_size` (6) dwarfs A's
    // (3) and B ends up ranked #1 even though only one of its six
    // occurrences is actionable.
    fs::write(
        visible_dir.join("SummerVisible.cs"),
        csharp_summer_body("VisibleSummer", "Tally", "Run", "acc"),
    )?;
    for index in 0..5 {
        fs::write(
            generated_dir.join(format!("SummerGen{index}.cs")),
            csharp_summer_body(
                &format!("GenSummer{index}"),
                &format!("Tally{index}"),
                "Run",
                "acc",
            ),
        )?;
    }

    let report = run_report(&scan_root, 8)?;
    let clusters = clusters(&report);
    assert!(
        clusters.len() >= 2,
        "expected at least two clusters in report, got {}",
        clusters.len(),
    );

    let top = clusters
        .first()
        .ok_or_else(|| anyhow::anyhow!("no top cluster"))?;
    let top_paths = occurrence_paths(top);
    let top_visible = visible_count(top);

    // The Python trio cluster (3 visible / 0 hidden) must outrank
    // the mixed Summer cluster (1 visible / 5 hidden). Assert by
    // occurrence path content so the assertion is independent of
    // cluster ids (which shift when weight changes).
    let top_is_trio = top_paths.iter().filter(|p| p.contains("trio_")).count() >= 3;
    assert!(
        top_is_trio,
        "top offender must be the 3-visible Trio cluster; got paths {top_paths:?}",
    );
    assert_eq!(
        top_visible, 3,
        "top offender must have three visible occurrences: {top_paths:?}",
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// #141 + #142 — `.cargo/` paths (registry + git checkouts) must not enter
// discovery, even when nested under the scan root.
// ---------------------------------------------------------------------------

#[test]
fn issue_142_cargo_cache_paths_are_built_in_excluded() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("workspace");
    let visible_dir = scan_root.join("src");
    let cargo_checkout = scan_root
        .join(".cargo")
        .join("git")
        .join("checkouts")
        .join("ruff-deadbeef")
        .join("c6516e9")
        .join("crates")
        .join("ruff_python_ast")
        .join("src");
    let cargo_registry = scan_root
        .join(".cargo")
        .join("registry")
        .join("src")
        .join("index.crates.io-deadbeef");
    fs::create_dir_all(&visible_dir)?;
    fs::create_dir_all(&cargo_checkout)?;
    fs::create_dir_all(&cargo_registry)?;

    // One real workspace clone-pair so the run still produces a report.
    fs::write(
        visible_dir.join("Alpha.cs"),
        csharp_summer_body("AlphaNs", "AlphaCls", "Run", "acc"),
    )?;
    fs::write(
        visible_dir.join("Beta.cs"),
        csharp_summer_body("BetaNs", "BetaCls", "Run", "acc"),
    )?;

    // Boilerplate clones in the vendored cargo cache. Before the fix
    // these enter the discovery walk and dominate the report.
    let generated_pattern = "namespace Generated\n\
         {\n\
         public class GenWidget\n\
         {\n\
         public int Run(int limit)\n\
         {\n\
         if (limit < 0) { return 0; }\n\
         int acc = 0;\n\
         for (int idx = 0; idx < limit; idx = idx + 1) { acc = acc + idx; }\n\
         return acc;\n\
         }\n\
         }\n\
         }\n";
    for index in 0..6 {
        fs::write(
            cargo_checkout.join(format!("Generated{index}.cs")),
            generated_pattern,
        )?;
        fs::write(
            cargo_registry.join(format!("Vendored{index}.cs")),
            generated_pattern,
        )?;
    }

    let report = run_report(&scan_root, 8)?;

    // Exactly the two real workspace files must be analysed.
    let files_analysed = report
        .get("files_analysed")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    assert_eq!(
        files_analysed, 2,
        "only the two workspace files should be analysed; report={report}",
    );

    // No rendered occurrence may point inside the cargo cache.
    for cluster in clusters(&report) {
        for path in occurrence_paths(cluster) {
            assert!(
                !path.contains(".cargo/"),
                "cargo-cache path leaked into a rendered cluster: {path}",
            );
        }
    }
    Ok(())
}

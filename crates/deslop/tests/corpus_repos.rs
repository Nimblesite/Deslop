//! [CORPUS] Accuracy and resource gate against real public repositories,
//! each pinned to a commit by `corpus/<name>.json`.
//!
//! One `#[test]` per repository, so a language that regresses is named
//! directly in the failure output rather than hidden behind a sibling.
//!
//! Excluded from `make test` / `make ci` via `--skip corpus_`: these need a
//! clone on disk and they measure wall time and peak memory, which are
//! runner-dependent. `make test-corpus` runs them, single-threaded, because
//! a scan can hold gigabytes and parallel scans would evict each other.
//!
//! A repository scan costs minutes, so each test performs **one** scan and
//! accumulates every failure before asserting. A single run reports every way
//! the engine is currently wrong instead of stopping at the first defect.
//!
//! An empty `must_find` list asserts nothing about recall. It is not evidence
//! that recall is good — it means duplicates for that repository have not been
//! hand-verified yet.

use std::{path::Path, time::Duration};

use anyhow::{anyhow, Result};
use deslop_test_support::corpus::{
    baseline_mode, classify, clone_dir, cluster_paths, first_occurrence_text, manifest,
    reports_clone_spanning, scan, string_field, u64_field, Baseline, CorpusRun, Failure,
};
use serde_json::Value;

#[test]
fn corpus_flutter_dart() -> Result<()> {
    gate("flutter")
}

#[test]
fn corpus_jellyfin_csharp() -> Result<()> {
    gate("jellyfin")
}

#[test]
fn corpus_tokio_rust() -> Result<()> {
    gate("tokio")
}

#[test]
fn corpus_django_python() -> Result<()> {
    gate("django")
}

#[test]
fn corpus_react_javascript() -> Result<()> {
    gate("react")
}

#[test]
fn corpus_nest_typescript() -> Result<()> {
    gate("nest")
}

#[test]
fn corpus_laravel_php() -> Result<()> {
    gate("laravel")
}

#[test]
fn corpus_hugo_go() -> Result<()> {
    gate("hugo")
}

#[test]
fn corpus_fsharp() -> Result<()> {
    gate("fsharp")
}

#[test]
fn corpus_determinism_nest_typescript() -> Result<()> {
    determinism_gate("nest")
}

#[test]
fn corpus_determinism_jellyfin_csharp() -> Result<()> {
    determinism_gate("jellyfin")
}

/// Scans the same pinned commit twice with identical flags and asserts the
/// two reports agree. Everything else in this suite — and every `--fail-over`
/// CI gate — is meaningless if the engine does not clear this bar, so it runs
/// against the two cheapest corpora rather than not at all.
fn determinism_gate(name: &str) -> Result<()> {
    let manifest = manifest(name)?;
    let root = clone_dir(&manifest)?;
    let tmp = tempfile::tempdir()?;

    let first = scan(&root, &tmp.path().join("first"))?;
    let second = scan(&root, &tmp.path().join("second"))?;

    let (first_ids, second_ids) = (cluster_ids(&first.report), cluster_ids(&second.report));
    println!(
        "{name}: run1 clusters={} dup={:.4}%  run2 clusters={} dup={:.4}%",
        first_ids.len(),
        duplication_percent(&first.report),
        second_ids.len(),
        duplication_percent(&second.report),
    );

    let mut failures = Vec::new();
    if first_ids != second_ids {
        let (left, right) = (duplication_percent(&first.report), duplication_percent(&second.report));
        failures.push(Failure::new(
            "determinism",
            format!(
                "two identical scans disagreed — clusters {} vs {}, duplication_percent \
                 {left:.4}% vs {right:.4}%. No report and no --fail-over gate is reproducible.",
                first_ids.len(),
                second_ids.len()
            ),
        ));
    }
    fail_on(name, &["determinism"], &failures)
}

/// The ordered cluster ids of a report.
fn cluster_ids(report: &Value) -> Vec<String> {
    report
        .get("clusters")
        .and_then(Value::as_array)
        .map(|clusters| {
            clusters
                .iter()
                .filter_map(|cluster| cluster.get("id").and_then(Value::as_str))
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// The report's repo-level duplication percentage.
fn duplication_percent(report: &Value) -> f64 {
    report
        .pointer("/metrics/duplication_percent")
        .and_then(Value::as_f64)
        .unwrap_or_default()
}

/// Checks the main gate evaluates. Used to scope baseline reconciliation so
/// it never reports the determinism gate's entries as fixed.
const GATE_CHECKS: &[&str] = &[
    "recall",
    "boilerplate_rank",
    "data_table_rank",
    "wall",
    "memory",
];

/// Scans one pinned repository and asserts every curated property of the
/// resulting report.
fn gate(name: &str) -> Result<()> {
    let manifest = manifest(name)?;
    let root = clone_dir(&manifest)?;
    let tmp = tempfile::tempdir()?;

    let run = scan(&root, &tmp.path().join(name))?;
    report_measurements(name, &manifest, &run);
    warn_when_accuracy_unasserted(name, &manifest);

    let mut failures = Vec::new();
    check_recall(&manifest, &run, &mut failures);
    check_boilerplate_not_ranked_first(&manifest, &root, &run, &mut failures)?;
    check_data_tables_not_ranked_as_logic(&root, &run, &mut failures)?;
    check_ceilings(&manifest, &run, &mut failures)?;

    fail_on(name, GATE_CHECKS, &failures)
}

/// Classifies observed failures against `corpus/known-failures.json` and fails
/// the test on whatever survives. Strict mode fails on everything; baseline
/// mode fails only on checks that are not already tracked, so CI reports the
/// known defect list without blocking on it.
fn fail_on(name: &str, evaluated: &[&str], failures: &[Failure]) -> Result<()> {
    let baseline = Baseline::load()?;
    let fatal = classify(name, evaluated, failures, &baseline);
    assert!(
        fatal.is_empty(),
        "{name} corpus gate failed {} {}check(s):\n  - {}",
        fatal.len(),
        if baseline_mode() { "NEW " } else { "" },
        fatal
            .iter()
            .map(|failure| format!("{}: {}", failure.check, failure.detail))
            .collect::<Vec<_>>()
            .join("\n  - ")
    );
    Ok(())
}

/// Prints the measured cost so a passing run still records the numbers.
fn report_measurements(name: &str, manifest: &Value, run: &CorpusRun) {
    println!(
        "{name} [{}]: files={} loc={} clusters={} dup={:.1}% wall={:.1}s peak_rss={}MB",
        string_field(manifest, "language").unwrap_or("?"),
        field_u64(&run.report, "files_analysed"),
        pointer_u64(&run.report, "/metrics/analysed_loc"),
        cluster_paths(&run.report).len(),
        run.report
            .pointer("/metrics/duplication_percent")
            .and_then(Value::as_f64)
            .unwrap_or_default(),
        run.wall.as_secs_f64(),
        run.peak_rss_mb,
    );
}

/// Shouts when a repository has no curated accuracy assertions at all, so a
/// green result is never mistaken for evidence that Deslop is accurate on it.
/// Such a run has proven only that the scan fit inside its resource budget.
fn warn_when_accuracy_unasserted(name: &str, manifest: &Value) {
    let no_recall = array(manifest, "must_find").is_empty();
    let no_precision = manifest.get("must_not_rank_first").is_none();
    if no_recall && no_precision {
        println!(
            "  !! {name}: ACCURACY UNASSERTED — no curated duplicates and no ranking rule. \
             This run checked resource ceilings ONLY. A pass here is NOT evidence that \
             detection on {name} is correct."
        );
    }
}

/// Recall: every hand-verified duplicate in the manifest must be reported.
/// A miss is a false negative on code a human confirmed is byte-identical.
fn check_recall(manifest: &Value, run: &CorpusRun, failures: &mut Vec<Failure>) {
    for entry in array(manifest, "must_find") {
        let files: Vec<String> = array(entry, "files")
            .iter()
            .filter_map(|file| file.as_str().map(ToOwned::to_owned))
            .collect();
        if !reports_clone_spanning(&run.report, &files) {
            failures.push(Failure::new(
                "recall",
                format!(
                    "no cluster spans {files:?}. Verified duplicate: {}",
                    entry.get("why").and_then(Value::as_str).unwrap_or("")
                ),
            ));
        }
    }
}

/// Precision: language- or framework-mandated scaffolding must never outrank
/// genuine copy-paste. Such a cluster is unactionable by construction, so it
/// must not sit at the head of a "worst offenders first" report.
fn check_boilerplate_not_ranked_first(
    manifest: &Value,
    root: &Path,
    run: &CorpusRun,
    failures: &mut Vec<Failure>,
) -> Result<()> {
    let Some(rule) = manifest.get("must_not_rank_first") else {
        return Ok(());
    };
    let top_n = usize::try_from(u64_field(rule, "top_n")?).unwrap_or(0);
    let forbidden: Vec<&str> = array(rule, "forbidden_top_shapes")
        .iter()
        .filter_map(Value::as_str)
        .collect();

    let empty = Vec::new();
    let clusters = run
        .report
        .get("clusters")
        .and_then(Value::as_array)
        .unwrap_or(&empty);

    for (rank, cluster) in clusters.iter().take(top_n).enumerate() {
        let text = first_occurrence_text(root, cluster)?;
        for shape in &forbidden {
            if text.contains(shape) {
                failures.push(Failure::new(
                    "boilerplate_rank",
                    format!(
                        "rank {rank}: cluster of {} occurrences is `{shape}` boilerplate, which \
                         cannot be deduplicated. First occurrence: {}",
                        field_u64(cluster, "size"),
                        text.lines().next().unwrap_or("").trim()
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// Number of top-ranked clusters subjected to the language-agnostic
/// precision checks. Ranking is the product, so the head of the report is
/// where a false positive does the most damage.
const RANKED_HEAD: usize = 10;

/// Fraction of non-whitespace characters that must be digits or data
/// punctuation before a snippet counts as a data table rather than logic.
/// Real logic carries identifiers and keywords, so it lands far below this.
const DATA_TABLE_RATIO: f64 = 0.6;

/// Precision, language-agnostic: a top-ranked cluster that is essentially a
/// numeric table must not be classified `logic`. `CloneCategory::data`
/// exists so such clusters can be demoted; when the classifier does not cover
/// a language they arrive at full logic weight and outrank real clones.
fn check_data_tables_not_ranked_as_logic(
    root: &Path,
    run: &CorpusRun,
    failures: &mut Vec<Failure>,
) -> Result<()> {
    let empty = Vec::new();
    let clusters = run
        .report
        .get("clusters")
        .and_then(Value::as_array)
        .unwrap_or(&empty);

    for (rank, cluster) in clusters.iter().take(RANKED_HEAD).enumerate() {
        let text = first_occurrence_text(root, cluster)?;
        let ratio = data_character_ratio(&text);
        let category = cluster.get("category").and_then(Value::as_str).unwrap_or("absent");
        if ratio >= DATA_TABLE_RATIO && category != "data" {
            failures.push(Failure::new(
                "data_table_rank",
                format!(
                    "rank {rank}: cluster of {} occurrences is {:.0}% numeric/separator \
                     characters — a data table — but is categorised `{category}`, so it ranks \
                     at full logic weight. Snippet: {}",
                    field_u64(cluster, "size"),
                    ratio * 100.0,
                    text.chars().take(70).collect::<String>().replace('\n', " "),
                ),
            ));
        }
    }
    Ok(())
}

/// Fraction of non-whitespace characters that are digits or the punctuation
/// that separates literals in a table.
fn data_character_ratio(text: &str) -> f64 {
    let significant = || text.chars().filter(|character| !character.is_whitespace());
    let total = significant().count();
    if total == 0 {
        return 0.0;
    }
    let data = significant().filter(|character| is_data_character(*character)).count();
    as_f64(data) / as_f64(total)
}

/// True for digits and the punctuation that separates literals in a table.
fn is_data_character(character: char) -> bool {
    character.is_ascii_digit() || matches!(character, ';' | ',' | '|' | '[' | ']' | '.' | '-')
}

/// Widens a count to `f64`, saturating rather than wrapping.
fn as_f64(count: usize) -> f64 {
    u32::try_from(count).map_or(f64::from(u32::MAX), f64::from)
}

/// Resource: the scan must finish inside the manifest's wall-clock and memory
/// budget. The memory ceiling is a standard CI runner's RAM — a scan that
/// exceeds it cannot run in the GitHub Action this project ships.
fn check_ceilings(manifest: &Value, run: &CorpusRun, failures: &mut Vec<Failure>) -> Result<()> {
    let ceilings = manifest
        .get("ceilings")
        .ok_or_else(|| anyhow!("manifest has no `ceilings`"))?;

    let max_wall = Duration::from_secs(u64_field(ceilings, "max_wall_seconds")?);
    if run.wall > max_wall {
        failures.push(Failure::new(
            "wall",
            format!(
                "scan took {:.1}s, ceiling is {}s",
                run.wall.as_secs_f64(),
                max_wall.as_secs()
            ),
        ));
    }

    let max_rss = u64_field(ceilings, "max_peak_rss_mb")?;
    if run.peak_rss_mb > max_rss {
        failures.push(Failure::new(
            "memory",
            format!("peak RSS {}MB exceeds the {max_rss}MB ceiling", run.peak_rss_mb),
        ));
    }
    Ok(())
}

/// Array-valued field, or an empty slice when absent.
fn array<'a>(value: &'a Value, name: &str) -> &'a [Value] {
    value
        .get(name)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// Unsigned scalar field, or `0` when absent.
fn field_u64(value: &Value, name: &str) -> u64 {
    value.get(name).and_then(Value::as_u64).unwrap_or_default()
}

/// Unsigned scalar at a JSON pointer, or `0` when absent.
fn pointer_u64(value: &Value, pointer: &str) -> u64 {
    value.pointer(pointer).and_then(Value::as_u64).unwrap_or_default()
}

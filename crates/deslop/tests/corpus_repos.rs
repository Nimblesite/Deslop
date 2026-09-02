//! [CORPUS-PIN] [CORPUS-RECALL] [CORPUS-PRECISION] [CORPUS-CEILINGS]
//! Accuracy and resource gate against real public repositories, each pinned
//! to a commit by `corpus/<name>.json`. Spec: `docs/specs/corpus.md`.
//!
//! One `#[test]` per repository, so a language that regresses is named
//! directly in the failure output rather than hidden behind a sibling.
//!
//! [TEST-SELECTION-SKIP] Every test here is `#[ignore]`d as
//! [SKIP-TOO-LARGE-FOR-CI], citing gh #422: they need a clone on disk and they
//! measure wall time and peak memory, which are runner-dependent. The reason
//! is stated at each test rather than filtered away in the Makefile, so it is
//! printed on every run and `skip_policy_contract` holds it to the policy.
//! `make test-corpus` runs them via `-- --ignored`, single-threaded, because a
//! scan can hold gigabytes and parallel scans would evict each other.
//!
//! `#[ignore]` keeps this target inside `--all-targets`, so `make test` and
//! `make lint` still compile and lint it. The `required-features` gate that
//! preceded it did not, and commit `77bcbaed5` left this file uncompilable
//! with nothing to notice until someone ran `make test-corpus`.
//!
//! A repository scan costs minutes, so each test performs **one** scan and
//! accumulates every failure before asserting. A single run reports every way
//! the engine is currently wrong instead of stopping at the first defect.
//!
//! An empty `must_find` list asserts nothing about recall. It is not evidence
//! that recall is good — it means duplicates for that repository have not been
//! hand-verified yet.
//!
//! # The determinism gate (#301, `[PIPELINE-DETERMINISM]`)
//!
//! `corpus_determinism_*` re-scans one repository twice and asserts the two
//! reports are identical. It pins two defects that both made the corpus order
//! a function of something other than the corpus.
//!
//! The first was hash-map iteration: the snapshot flattened `per_file` in
//! `HashMap` order, whose `RandomState` seed changes per process. Two runs over
//! byte-identical sources emitted different fingerprint sequences, which moved
//! the LSH star centre and so changed cluster ids, occurrence ranges, and
//! `duplication_percent` between runs of the same binary on the same repository.
//!
//! The second survived the first fix, because sorting by `FileId` looks
//! deterministic and is not: ids are issued in registration order and the
//! registry never unregisters, so removing and re-adding a byte-identical file
//! hands it a fresh, higher id. Determinism must hold over corpus *state*, not
//! edit history — identical paths and bytes produce an identical report
//! whatever sequence of edits got there. Measured in the LSP before the fix:
//! restoring byte-identical source and config moved duplicated LOC from 96
//! (100%) to 56 (58.33%). Every ordering is now keyed by workspace-relative
//! path with the id as a tie-breaker only. This gate catches the rerun half;
//! `deslop-lsp/tests/history_determinism.rs` catches the edit-history half.

use std::{path::Path, time::Duration};

use anyhow::{anyhow, Result};
use deslop_test_support::{
    corpus::{
        array, baseline_mode, classify, clone_dir, cluster_paths, cluster_shows_span, compare_pair,
        field_u64, first_occurrence_text, manifest, occurrence_endpoint, scan, string_field,
        u64_field, visible_clusters, Baseline, CorpusRun, Failure,
    },
    corpus_confidence::{
        check_cluster_mass_contract, check_curated_recall, check_type2_curated_recall,
    },
    corpus_data_table::occurrence_is_a_literal_table,
    corpus_determinism::check_reports_agree,
    corpus_precision::{check_boilerplate_not_ranked_first, check_curated_precision},
    corpus_scope::check_scan_scope,
    enclosure::Span,
};
use serde_json::{json, Value};

#[test]
#[ignore = "[SKIP-TOO-LARGE-FOR-CI] GH #422 [CORPUS-PIN] [CORPUS-PRECISION] \
            docs/plans/corpus-assertion.md — clones flutter/flutter at its pinned commit and \
            scans the whole Dart tree: the largest in the corpus, measured at 295 s wall / \
            7947 MB peak RSS (gh #166 fixed; ceilings live in corpus/flutter.json). The \
            release gate compiles this target and never runs it. `make test-corpus` runs it, \
            single-threaded, via `-- --ignored`."]
fn corpus_flutter_dart() -> Result<()> {
    gate("flutter")
}

#[test]
#[ignore = "[SKIP-TOO-LARGE-FOR-CI] GH #422 [CORPUS-PIN] [CORPUS-RECALL] \
            docs/plans/corpus-assertion.md — clones jellyfin/jellyfin at its pinned commit \
            and scans the whole C# tree: several thousand files, minutes per scan. The \
            release gate compiles this target and never runs it. `make test-corpus` runs it, \
            single-threaded, via `-- --ignored`."]
fn corpus_jellyfin_csharp() -> Result<()> {
    gate("jellyfin")
}

#[test]
#[ignore = "[SKIP-TOO-LARGE-FOR-CI] GH #422 [CORPUS-PIN] [CORPUS-RECALL] \
            docs/plans/corpus-assertion.md — clones tokio-rs/tokio at its pinned commit and \
            scans the whole Rust tree: the cheapest in the corpus, and still a clone the \
            release gate must not make. The release gate compiles this target and never runs \
            it. `make test-corpus` runs it, single-threaded, via `-- --ignored`."]
fn corpus_tokio_rust() -> Result<()> {
    gate("tokio")
}

#[test]
#[ignore = "[SKIP-TOO-LARGE-FOR-CI] GH #422 [CORPUS-PIN] [CORPUS-SCOPE] \
            docs/plans/corpus-assertion.md — clones django/django at its pinned commit and \
            scans the whole Python tree: a clone plus a whole-repository scan. The release \
            gate compiles this target and never runs it. `make test-corpus` runs it, \
            single-threaded, via `-- --ignored`."]
fn corpus_django_python() -> Result<()> {
    gate("django")
}

#[test]
#[ignore = "[SKIP-TOO-LARGE-FOR-CI] GH #422 [CORPUS-PIN] [CORPUS-SCOPE] \
            docs/plans/corpus-assertion.md — clones facebook/react at its pinned commit and \
            scans the whole JavaScript tree: a clone plus a whole-repository scan. The \
            release gate compiles this target and never runs it. `make test-corpus` runs it, \
            single-threaded, via `-- --ignored`."]
fn corpus_react_javascript() -> Result<()> {
    gate("react")
}

#[test]
#[ignore = "[SKIP-TOO-LARGE-FOR-CI] GH #422 [CORPUS-PIN] [CORPUS-RECALL] \
            docs/plans/corpus-assertion.md — clones nestjs/nest at its pinned commit and \
            scans the whole TypeScript tree: a clone plus a whole-repository scan. The \
            release gate compiles this target and never runs it. `make test-corpus` runs it, \
            single-threaded, via `-- --ignored`."]
fn corpus_nest_typescript() -> Result<()> {
    gate("nest")
}

#[test]
#[ignore = "[SKIP-TOO-LARGE-FOR-CI] GH #422 [CORPUS-PIN] [CORPUS-SCOPE] \
            docs/plans/corpus-assertion.md — clones laravel/framework at its pinned commit \
            and scans the whole PHP tree: a clone plus a whole-repository scan. The release \
            gate compiles this target and never runs it. `make test-corpus` runs it, \
            single-threaded, via `-- --ignored`."]
fn corpus_laravel_php() -> Result<()> {
    gate("laravel")
}

#[test]
#[ignore = "[SKIP-TOO-LARGE-FOR-CI] GH #422 [CORPUS-PIN] [CORPUS-SCOPE] \
            docs/plans/corpus-assertion.md — clones gohugoio/hugo at its pinned commit and \
            scans the whole Go tree: a clone plus a whole-repository scan. The release gate \
            compiles this target and never runs it. `make test-corpus` runs it, \
            single-threaded, via `-- --ignored`."]
fn corpus_hugo_go() -> Result<()> {
    gate("hugo")
}

#[test]
#[ignore = "[SKIP-TOO-LARGE-FOR-CI] GH #422 [CORPUS-PIN] [CORPUS-PRECISION] \
            docs/plans/corpus-assertion.md — clones dotnet/fsharp at its pinned commit and \
            scans the whole F# tree: peaks above 13 GB, past every hosted-runner tier. The \
            release gate compiles this target and never runs it. `make test-corpus` runs it, \
            single-threaded, via `-- --ignored`."]
fn corpus_fsharp() -> Result<()> {
    gate("fsharp")
}

#[test]
#[ignore = "[SKIP-TOO-LARGE-FOR-CI] GH #422 [CORPUS-PIN] [PIPELINE-DETERMINISM] \
            docs/plans/corpus-assertion.md — scans nestjs/nest twice over, so it costs a \
            clone plus two whole-repository TypeScript scans. The release gate compiles this \
            target and never runs it. `make test-corpus` runs it, single-threaded, via `-- \
            --ignored`."]
fn corpus_determinism_nest_typescript() -> Result<()> {
    determinism_gate("nest")
}

#[test]
#[ignore = "[SKIP-TOO-LARGE-FOR-CI] GH #422 [CORPUS-PIN] [PIPELINE-DETERMINISM] \
            docs/plans/corpus-assertion.md — scans jellyfin/jellyfin twice over, so it costs \
            a clone plus two whole-repository C# scans. The release gate compiles this \
            target and never runs it. `make test-corpus` runs it, single-threaded, via `-- \
            --ignored`."]
fn corpus_determinism_jellyfin_csharp() -> Result<()> {
    determinism_gate("jellyfin")
}

/// [PIPELINE-DETERMINISM] Scans the same pinned commit twice with identical flags and asserts the
/// two reports agree. Everything else in this suite — and every `--fail-over`
/// CI gate — is meaningless if the engine does not clear this bar, so it runs
/// against the two cheapest corpora rather than not at all.
fn determinism_gate(name: &str) -> Result<()> {
    let manifest = manifest(name)?;
    let root = clone_dir(&manifest)?;
    let tmp = tempfile::tempdir()?;

    let first = scan(&root, &tmp.path().join("first"))?;
    let second = scan(&root, &tmp.path().join("second"))?;

    println!(
        "{name}: run1 clusters={} dup={:.4}%  run2 clusters={} dup={:.4}%",
        rendered_cluster_count(&first.report),
        duplication_percent(&first.report),
        rendered_cluster_count(&second.report),
        duplication_percent(&second.report),
    );

    // [PIPELINE-DETERMINISM] The whole rendered payload, not the ordered
    // cluster ids: ids come from the smallest member's hash and survive
    // moved ranges, changed buckets, changed signals, reordered ranks and
    // a moved `duplication_percent` alike. `corpus_determinism` states
    // each of those as its own unit case.
    let mut failures = Vec::new();
    check_reports_agree(&first.report, &second.report, &mut failures);
    fail_on(name, &["determinism"], &failures)
}

/// How many clusters a report rendered.
fn rendered_cluster_count(report: &Value) -> usize {
    report
        .get("clusters")
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
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
    "files_analysed",
    "cluster_count_band",
    "recall",
    "recall_quality",
    "precision",
    "boilerplate_rank",
    "data_table_rank",
    "fused_bounded_max",
    "cluster_contract",
    "cluster_mass",
    "cluster_rank",
    "type2_recall",
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
    // [CORPUS-SCOPE] First, because every check below iterates a set an
    // empty report leaves empty: a scan that reached nothing satisfies all
    // of them at once (gh #342).
    check_scan_scope(&manifest, &run.report, &mut failures);
    let verdicts = curated_pair_verdicts(&manifest, &run.report, &root)?;
    check_curated_recall(&manifest, &run.report, &verdicts, &mut failures);
    check_curated_precision(&manifest, &run.report, &mut failures);
    check_boilerplate_not_ranked_first(&manifest, &root, &run, &mut failures)?;
    check_data_tables_not_ranked_as_logic(&manifest, &root, &run, &mut failures)?;
    // [CORPUS-BASELINE] The confidence checks. The first reads no
    // manifest — it judges the *shape* of the rendered report, so it runs
    // on every repository including the ones whose recall is not yet
    // curated. The third is the curated Type-2 recall assertion
    // ([CORPUS-RECALL]): it reads `must_find_type2` and asserts nothing
    // where the manifest curates nothing.
    check_cluster_mass_contract(&run.report, &mut failures);
    check_type2_curated_recall(&manifest, &run.report, &verdicts, &mut failures);
    check_ceilings(&manifest, &run, &mut failures)?;

    fail_on(name, GATE_CHECKS, &failures)
}

/// [CORPUS-BASELINE] Classifies observed failures against `corpus/known-failures.json` and fails
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

/// [CORPUS-RECALL] Shouts when a repository has no curated accuracy assertions at all, so a
/// green result is never mistaken for evidence that Deslop is accurate on it.
/// Such a run has proven only that the scan fit inside its resource budget.
fn warn_when_accuracy_unasserted(name: &str, manifest: &Value) {
    let no_recall =
        array(manifest, "must_find").is_empty() && array(manifest, "must_find_type2").is_empty();
    let no_precision = manifest.get("must_not_rank_first").is_none();
    if no_recall && no_precision {
        println!(
            "  !! {name}: ACCURACY UNASSERTED — no curated duplicates and no ranking rule. \
             This run checked resource ceilings ONLY. A pass here is NOT evidence that \
             detection on {name} is correct."
        );
    }
}

/// Number of top-ranked clusters subjected to the language-agnostic
/// precision checks. Ranking is the product, so the head of the report is
/// where a false positive does the most damage.
const RANKED_HEAD: usize = 10;

/// [CORPUS-RECALL] Admission evidence for every curated Type-2 pair.
///
/// A cluster says two files share a shape; whether the engine *admitted
/// this pair* as a rename lives in the pair record, which the mass-only
/// wire keeps off clusters entirely. The gate therefore asks the measured
/// binary directly, using the endpoints of the curated occurrences in the
/// cluster it already found ([PAIR-COMPARE-CLI], gh #488).
///
/// An entry whose pair cannot be located yields no verdict, and the
/// recall check fails it rather than passing on a missing measurement.
fn curated_pair_verdicts(manifest: &Value, report: &Value, root: &Path) -> Result<Vec<Value>> {
    let mut verdicts = Vec::new();
    for entry in array(manifest, "must_find_type2") {
        let files: Vec<String> = array(entry, "files")
            .iter()
            .filter_map(|file| file.as_str().map(ToOwned::to_owned))
            .collect();
        let Some(endpoints) = curated_endpoints(report, &files) else {
            continue;
        };
        let verdict = compare_pair(root, &endpoints.0, &endpoints.1)?;
        verdicts.push(json!({ "files": files, "evidence": verdict.get("evidence") }));
    }
    Ok(verdicts)
}

/// The two curated occurrences' endpoints, taken from the widest visible
/// cluster that shows both files.
fn curated_endpoints(report: &Value, files: &[String]) -> Option<(String, String)> {
    let cluster = visible_clusters(report)
        .into_iter()
        .filter(|cluster| cluster_shows_span(cluster, files))
        .max_by_key(|cluster| field_u64(cluster, "canonical_node_count"))?;
    let endpoint_for = |wanted: &String| {
        array(cluster, "occurrences")
            .iter()
            .find(|occurrence| occurrence.get("path").and_then(Value::as_str) == Some(wanted))
            .and_then(|occurrence| occurrence_endpoint(occurrence).ok())
    };
    let [left, right] = [files.first()?, files.get(1)?];
    Some((endpoint_for(left)?, endpoint_for(right)?))
}

/// [CORPUS-PRECISION] A table of literals must not reach the ranked head.
///
/// A repeated data structure is not extractable logic — no shared control
/// flow, nothing to hoist — so the engine's noise filters drop it
/// ([CLONE-NOISE-CONSTANT-TABLE], [CLONE-NOISE-DART-DATA-TABLE-LITERAL]).
/// One still visible in the head means a filter missed it, and it is
/// outranking real clones.
///
/// The `category != "data"` clause this carried is gone. The mass-only
/// wire forbids `category` on a cluster, so the field was always absent,
/// the comparison was always true, and the demotion half of the assertion
/// had quietly stopped asserting anything (gh #452).
fn check_data_tables_not_ranked_as_logic(
    manifest: &Value,
    root: &Path,
    run: &CorpusRun,
    failures: &mut Vec<Failure>,
) -> Result<()> {
    let language = string_field(manifest, "language")?;
    for (position, cluster) in visible_clusters(&run.report)
        .into_iter()
        .take(RANKED_HEAD)
        .enumerate()
    {
        let text = first_occurrence_text(root, cluster)?;
        let span = Span::new("", 0, u64::try_from(text.len()).unwrap_or(0));
        if occurrence_is_a_literal_table(language, &text, &span)? {
            failures.push(Failure::new(
                "data_table_rank",
                format!(
                    "rank {}: cluster of {} occurrences is a table of literals, not \
                     extractable logic, yet it is visible in the ranked head — a noise \
                     filter missed it and it outranks real clones. Snippet: {}",
                    position.saturating_add(1),
                    field_u64(cluster, "size"),
                    text.chars().take(70).collect::<String>().replace('\n', " "),
                ),
            ));
        }
    }
    Ok(())
}

/// [CORPUS-CEILINGS] The scan must finish inside the manifest's wall-clock and memory
/// ceilings. The manifest is the single source of truth for both figures —
/// per-repo values tolerated for now, sized above the repository's own
/// measured scan so the gate catches regressions.
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
            format!(
                "peak RSS {}MB exceeds the {max_rss}MB ceiling",
                run.peak_rss_mb
            ),
        ));
    }
    Ok(())
}

/// Unsigned scalar at a JSON pointer, or `0` when absent.
fn pointer_u64(value: &Value, pointer: &str) -> u64 {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .unwrap_or_default()
}

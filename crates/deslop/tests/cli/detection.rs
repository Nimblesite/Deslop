use super::support::*;
use crate::common::{
    go_scope::*,
    signals::{assert_no_pair_surface_on_cluster, has_verbatim_pair},
};

#[path = "detection/contract.rs"]
mod contract;
use contract::*;

#[path = "detection/prologue.rs"]
mod prologue;
use prologue::{assert_no_cross_file_prologue_cluster, occurrence_source};

#[test]
fn detects_type2_clone_in_csharp_fixture() -> Result<()> {
    let json = run_min_nodes("csharp-small", "8")?;
    assert_type2_report(&json, "Alpha.cs", "Beta.cs")?;
    Ok(())
}

// Implements [PIPELINE-LANG-TRAIT] for Rust: Type-2 clone detection.
#[test]
fn detects_type2_clone_in_rust_fixture() -> Result<()> {
    let json = run_min_nodes("rust-small", "10")?;
    assert_type2_report(&json, "alpha.rs", "beta.rs")?;
    Ok(())
}

// Implements [PIPELINE-LANG-TRAIT] for Python: Type-2 clone detection.
#[test]
fn detects_type2_clone_in_python_fixture() -> Result<()> {
    let json = run_min_nodes("python-small", "10")?;
    assert_type2_report(&json, "alpha.py", "beta.py")?;
    Ok(())
}

// [PIPELINE-LANG-TRAIT] Dart Type-2 fixture: the report cluster is mass-only; any pair measurements require explicit endpoints.
#[test]
fn detects_type2_clone_in_dart_fixture() -> Result<()> {
    let json = run_min_nodes("dart-small", "10")?;
    assert_type2_report(&json, "alpha.dart", "beta.dart")?;
    Ok(())
}

// [PIPELINE-LANG-TRAIT] PHP Type-2 fixture: the report cluster is mass-only; any pair measurements require explicit endpoints.
#[test]
fn detects_type2_clone_in_php_fixture() -> Result<()> {
    let json = run_min_nodes("php-small", "10")?;
    assert_type2_report(&json, "alpha.php", "beta.php")?;
    Ok(())
}

// [PIPELINE-LANG-TRAIT] F# Type-2 fixture: the report cluster is mass-only; any pair measurements require explicit endpoints.
#[test]
fn detects_type2_clone_in_fsharp_fixture() -> Result<()> {
    let json = run_min_nodes("fsharp-small", "10")?;
    assert_type2_report(&json, "alpha.fs", "beta.fs")?;
    Ok(())
}

// [PIPELINE-LANG-TRAIT] Go Type-2 fixture: the report cluster is mass-only; any pair measurements require explicit endpoints.
#[test]
fn detects_type2_clone_in_go_fixture() -> Result<()> {
    let (scan_root, report) =
        run_with_args(GO_SMALL_FIXTURE, &[MIN_NODES_FLAG, GO_SMALL_MIN_NODES])?;
    assert_type2_report(
        &serde_json::to_string(&report)?,
        GO_SMALL_FIRST,
        GO_SMALL_SECOND,
    )?;

    // [PIPELINE-CLUSTER-EXACT-SCOPE] A Type-2 pair is the same authored
    // declaration in both files. Neither half may reach back to row 1 for
    // the package clause and import block, and because the two sides are
    // the same shape they must cover the same number of rows.
    assert_go_authored_scope(&scan_root, &report, GO_SMALL_FIXTURE)?;
    assert_every_occurrence_opens_a_declaration(&scan_root, &report, GO_SMALL_FIXTURE)?;
    assert_symmetric_rows_everywhere(&report, GO_SMALL_FIXTURE);
    Ok(())
}

// Implements [FUSED-SIGNALS-THREE-LAYER] for F#: a genuine Type-3
// near-miss. `delta.fs`'s loop body runs two accumulator updates per
// iteration; `epsilon.fs`'s runs one. The shared control-flow subtrees
// (`_ < 0 then 0`, `_ <- _ + _`, `_ in 0 .. _`) surface as a cross-file
// cluster, while the signature-only sibling match
// (`f (_: int) : int`, whose bodies differ) is correctly suppressed
// ([CLONE-NOISE-SIGNATURE-ONLY], #154) — proving both the structural
// near-miss path and the signature filter are wired for F#.
#[test]
fn detects_type3_clone_in_fsharp_fixture() -> Result<()> {
    let json = run_min_nodes("fsharp-type3", "8")?;
    let scan_root = fixture("fsharp-type3");
    let clusters = report_clusters(&json)?;
    let cluster = require_cluster_spanning(&clusters, "delta.fs", "epsilon.fs")?;
    assert_type3_signals(&scan_root, cluster, "F#")?;
    Ok(())
}

/// The source text an occurrence claims is duplicated, read back from
/// disk. Errors rather than returning `None` so a cluster pointing at an
/// unreadable range fails the test instead of silently skipping it.
fn require_occurrence_text(scan_root: &Path, occurrence: &serde_json::Value) -> Result<String> {
    let bytes = occurrence_source(scan_root, occurrence).ok_or_else(|| {
        anyhow::anyhow!("occurrence range is not readable from disk: {occurrence:#?}")
    })?;
    Ok(String::from_utf8(bytes)?)
}

/// Asserts a cross-file cluster is a genuine Type-3 near-miss: the
/// reported view spans the whole enclosing declaration in both files,
/// and the statement that exists in only one of them appears in exactly
/// one occurrence — which is precisely what makes the pair a near-miss
/// rather than a Type-1/2 whole-unit copy ([CLONE-TYPE-TAXONOMY]).
///
/// **This contract was inverted, deliberately, by gh #408.** It used to
/// require the opposite: a strict *sub*-range of each function, with
/// the divergent statement excluded from every occurrence. That is the
/// fragment view — the shared statements either side of the insertion,
/// reported as separate findings — and #408 is the issue filed because
/// it leaves the actual duplicated method invisible in every language,
/// "reported as unactionable line noise". The old rationale ("a
/// statement in only one file can never be part of a cross-file clone")
/// holds for an exact clone and fails for a near-miss, where divergence
/// inside the reported range is the defining property of the bucket.
///
/// Asserting exactly one occurrence carries the divergence is stronger
/// than asserting none does: it fails both if the pair regresses to a
/// fragment view (nobody carries it) and if the fixture ever stops
/// being a near-miss at all (both carry it, i.e. an exact copy).
fn assert_enclosing_near_miss(
    scan_root: &Path,
    cluster: &serde_json::Value,
    divergent_statement: &str,
) -> Result<()> {
    let occurrences = require_array(cluster, "/occurrences", "cluster")?;
    let mut carrying = 0_usize;
    for occurrence in occurrences {
        let text = require_occurrence_text(scan_root, occurrence)?;
        assert_contains(
            &text,
            "func ",
            "the near-miss view must span the whole enclosing declaration, not a \
             fragment of it (gh #408); got",
        );
        if text.contains(divergent_statement) {
            carrying = carrying.saturating_add(1);
        }
    }
    assert_eq!(
        carrying, 1,
        "`{divergent_statement}` must appear in exactly one occurrence: zero means the \
         report regressed to the fragment view #408 removed, and two would mean the \
         fixture is an exact copy rather than a near-miss",
    );
    Ok(())
}

// Implements [FUSED-SIGNALS-THREE-LAYER] for Go ([LANG-CAND-GO]): a
// genuine Type-3 near-miss. `delta.go` and `epsilon.go` run the same
// guarded accumulator algorithm, but `delta.go`'s loop body performs two
// updates per iteration and `epsilon.go`'s performs one. The shared
// control-flow scaffolding surfaces as a cross-file cluster at
// structural = 1.0, and — the part that makes this a near-miss rather
// than a copy — the diverging loop bodies do not:
//
//   * no reported range covers a whole `func` declaration, so the two
//     functions are never claimed to be whole-unit clones; and
//   * no reported range contains `running += 2`, the statement that
//     exists only in `delta.go`.
//
// Both bounds fail loudly if Go normalisation ever over-collapses (for
// example by treating a statement list as shape-free), which the
// original "some cluster spans both files" assertion could not.
#[test]
fn detects_type3_clone_in_go_fixture() -> Result<()> {
    // [FUSED-SHARED-SUBTREE] [FUSED-PRE-RESCUE-SCORE] Stronger measured overlap must not disable an admitted declaration.
    let json = run_min_nodes(GO_TYPE3_FIXTURE, GO_TYPE3_MIN_NODES)?;
    let scan_root = fixture(GO_TYPE3_FIXTURE);
    let report: serde_json::Value = serde_json::from_str(&json)?;
    let clusters = report_clusters(&json)?;

    // [PIPELINE-CLUSTER-EXACT-SCOPE] A near-miss is still one authored
    // declaration on each side. No occurrence may open at row 1, carry the
    // package clause or import block, or sit at a different depth from its
    // counterpart.
    assert_go_authored_scope(&scan_root, &report, GO_TYPE3_FIXTURE)?;
    assert_eq!(
        files_analysed(&json)?,
        2,
        "the go-type3 fixture is a two-file pair; anything else means discovery missed a file",
    );
    let cluster = require_cluster_spanning(&clusters, "delta.go", "epsilon.go")?;
    assert_type3_signals(&scan_root, cluster, "Go")?;
    for cluster in &clusters {
        let files = cluster_file_basenames(cluster);
        if files.len() > 1 {
            assert_enclosing_near_miss(&scan_root, cluster, "running += 2")?;
        }
    }
    Ok(())
}

// Implements [CLONE-NOISE-SIGNATURE-ONLY] (#154) for Go closures
// ([LANG-CAND-GO]). `alpha.go` and `beta.go` each return a closure whose
// parameter list and result types are identical — `func(name string,
// count int, active bool) (int, error)` — while the closure bodies are
// deliberately different shapes (a guard and a return vs. a counted
// accumulator loop). Two functions that merely agree on a signature are
// not duplicated code, and reporting them is the false positive #154
// exists to kill.
//
// Reaching that verdict requires `func_literal` to be a recognised Go
// function kind: the filter resolves the *innermost* enclosing function
// for a matched range, and only if that resolves to the closure does the
// signature sit in front of a body it can compare. Drop `func_literal`
// from `function_kinds` and the enclosing node becomes the outer
// declaration, the closure signature looks like it lives inside a body,
// the filter declines — and the signature match is published as a
// cross-file `identical` cluster. This test is that mutation's detector.
#[test]
fn go_closure_signature_only_match_is_suppressed() -> Result<()> {
    let json = run_min_nodes(GO_CLOSURE_FIXTURE, GO_CLOSURE_MIN_NODES)?;
    let scan_root = fixture(GO_CLOSURE_FIXTURE);
    let report: serde_json::Value = serde_json::from_str(&json)?;

    // [PIPELINE-CLUSTER-EXACT-SCOPE] Suppressing the cross-file signature
    // match does not license the survivors to take their whole file.
    assert_go_authored_scope(&scan_root, &report, GO_CLOSURE_FIXTURE)?;
    assert_eq!(
        files_analysed(&json)?,
        2,
        "both closure files must be analysed or the suppression below proves nothing",
    );
    let clusters = report_clusters(&json)?;
    for cluster in &clusters {
        let files = cluster_file_basenames(cluster);
        assert_eq!(
            files.len(),
            1,
            "alpha.go and beta.go share only a closure signature; a cross-file cluster \
             spanning {files:?} is the #154 false positive",
        );
    }
    let hidden = require_u64(&serde_json::from_str(&json)?, "/clusters_hidden", "report")?;
    assert!(
        hidden >= 1,
        "the signature-only match must be found and then suppressed, not merely never \
         formed; clusters_hidden was {hidden}",
    );
    Ok(())
}

// Audience: HUMAN. Issue #34, Go arm ([LANG-CAND-GO]). Six Go files in
// one package open with the same `package service` clause and the same
// grouped `import ( … )` block, then diverge completely — an index
// builder, a schema type, a retry loop, a CSV parser, a repository, and
// a policy factory. That prologue is the Go analogue of C# `using`
// directives: file scaffolding, never duplicated logic.
//
// `package_clause` and `import_declaration` are the boilerplate carriers
// that keep it out of fingerprints ([PIPELINE-BOILERPLATE-FILTER]).
// Without them the twenty-two-node prologue subtree is identical across
// all six files and lands as a six-occurrence `identical` cluster at
// line 1 — the single worst offender in the report, and pure noise.
#[test]
fn go_package_and_import_prologue_never_becomes_a_cross_file_cluster() -> Result<()> {
    let (scan_root, report) = run_with_args(
        GO_PROLOGUE_FIXTURE,
        &[MIN_NODES_FLAG, GO_PROLOGUE_MIN_NODES],
    )?;

    // [PIPELINE-CLUSTER-EXACT-SCOPE] Whatever this report publishes, no
    // occurrence may contain the `package` clause or `import` block, and
    // none may open at row 1.
    assert_go_authored_scope(&scan_root, &report, GO_PROLOGUE_FIXTURE)?;
    assert_eq!(
        require_u64(&report, "/files_analysed", "report")?,
        6,
        "all six package files must be analysed; report={report:#?}",
    );
    // Liveness proof on the mass-only wire: the fixture's six files
    // genuinely diverge below the shared prologue, so the honest report
    // carries no clusters at all — the old "some cluster must appear"
    // bound was satisfied by the very over-clustering this regression
    // exists to kill. What proves the scan was live is the metrics: the
    // parser consumed the whole corpus (analysed_loc > 0) and the
    // boilerplate carriers were counted, so a detector that stopped
    // looking would fail `files_analysed`, not pass it.
    let analysed_loc = require_u64(&report, "/metrics/analysed_loc", "report")?;
    assert!(
        analysed_loc >= 180,
        "the six divergent Go files must all be parsed (analysed_loc >= 180, got \
         {analysed_loc}) — the prologue guard must never double as a silence guard",
    );
    assert_no_cross_file_prologue_cluster(&report, &scan_root, "go prologue");
    Ok(())
}

// Audience: HUMAN. Zero-false-positive guard for F#. `tally()` folds a
// word-count `Map` inside a `for` loop; `describe()` is an `if`/`elif`
// cascade of early string returns. The two share no real shape, so a
// human reading the report must never see them paired as duplicates.
// Positive bound: every cluster's occurrences come from a single file.
#[test]
fn dissimilar_fsharp_functions_across_files_stay_in_separate_clusters() -> Result<()> {
    let json = run_min_nodes("fsharp-dissimilar-functions", "8")?;
    assert_every_cluster_single_file(&json, "F#")
}

// Audience: HUMAN. Zero-false-positive guard for Go ([LANG-CAND-GO]).
// `tally()` counts words into a map inside a range loop; `describe()` is
// a chain of `if code == … { return … }` early exits. The two share no
// real shape, so a human reading the report must never see them paired
// as duplicates. Positive bound: every cluster's occurrences come from a
// single file.
#[test]
fn dissimilar_go_functions_across_files_stay_in_separate_clusters() -> Result<()> {
    let (scan_root, report) = run_with_args(
        GO_DISSIMILAR_FIXTURE,
        &[MIN_NODES_FLAG, GO_DISSIMILAR_MIN_NODES],
    )?;
    let json = serde_json::to_string(&report)?;
    assert_every_cluster_single_file(&json, "Go")?;

    // [PIPELINE-CLUSTER-EXACT-SCOPE] A single-file cluster is still bound
    // by the authored window: it may not open at row 1 or swallow the
    // package clause and import block above the function it reports.
    assert_go_authored_scope(&scan_root, &report, GO_DISSIMILAR_FIXTURE)?;
    assert_every_occurrence_opens_a_declaration(&scan_root, &report, GO_DISSIMILAR_FIXTURE)
}

// Audience: HUMAN. Zero-false-positive guard for Dart ([LANG-CAND-DART]).
// `tally()` builds a map inside a for-each loop; `describe()` is a chain
// of `if (code …) return …` early exits. The two share no real shape, so
// a human reading the report must never see them paired as duplicates.
// Positive bound: every cluster's occurrences come from a single file.
#[test]
fn dissimilar_dart_functions_across_files_stay_in_separate_clusters() -> Result<()> {
    let json = run_min_nodes("dart-dissimilar-functions", "8")?;
    assert_every_cluster_single_file(&json, "Dart")
}

// Audience: HUMAN. Issue #34. When a human opens two Python test
// files whose functions are structurally unrelated — one synchronous
// test calling `registry.has(...)` in a for-loop assertion block,
// one async helper doing `db.add(UsageEvent(...)); await db.flush()`
// — Deslop must not report them as members of the same clone cluster.
// A human reading the cluster panel should never see two
// dissimilar-shape functions sitting side by side claiming to be
// duplicates; that makes the whole tool untrustworthy.
//
// Positive bound: every cluster in the report has occurrences from a
// single file. Intra-file similarity (e.g. three sibling tests that
// all do `x = registry.get("..."); result = x(...); assert ...`) is
// legitimate and allowed.
#[test]
fn dissimilar_python_functions_across_files_stay_in_separate_clusters() -> Result<()> {
    let json = run_min_nodes("python-dissimilar-functions", "10")?;
    assert_every_cluster_single_file(&json, "Python")
}

// Implements multi-language dispatch — three files routed by extension
// in one run.
#[test]
fn handles_mixed_language_fixture() -> Result<()> {
    let json = run_min_nodes("mixed-small", "10")?;
    assert!(json.contains("\"files_analysed\": 3"));
    assert!(json.contains("Lib.cs"));
    assert!(json.contains("lib.rs"));
    assert!(json.contains("lib.py"));
    Ok(())
}

// Implements [DECISION-TYPE3-TWO-PASS] + [FUSED-STRATEGY-BOUNDED-MAX]:
// Type-3 near-miss cross-file cluster with `structural=0.0`.
#[test]
fn detects_type3_clone_in_csharp_fixture() -> Result<()> {
    let json = run_min_nodes("csharp-type3", "15")?;
    let scan_root = fixture("csharp-type3");
    assert!(json.contains("Delta.cs"));
    assert!(json.contains("Epsilon.cs"));
    // This asserted the raw literal `"structural": 0.0`, which gh #408
    // is the issue filed against: the two methods share ~90% of their
    // AST, and the zero was the candidate layer writing a literal for
    // every cross-bucket pair rather than a measurement
    // ([FUSED-SHARED-SUBTREE]). Asserting the zero asserted the defect.
    // The honest contract is the two-sided one — real shape evidence,
    // short of the Merkle equality a near-miss cannot have.
    let clusters = report_clusters(&json)?;
    let cluster = require_cluster_spanning(&clusters, "Delta.cs", "Epsilon.cs")?;
    assert_type3_signals(&scan_root, cluster, "C#")?;
    Ok(())
}

// Implements [EXCLUSION-CONFIG] `exclude` tier: a file matched by the

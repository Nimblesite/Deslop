use crate::support::*;

/// Runs the CLI against `fixture(fixture_name)` with `--min-nodes
/// <min_nodes>`, asserts the process succeeded, and returns the raw
/// JSON report text. Shared by every detection test that drives a
/// fixture with an explicit `--min-nodes` and then asserts on the
/// rendered report.
fn run_min_nodes(fixture_name: &str, min_nodes: &str) -> Result<String> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = fixture_command(fixture_name, &tmp.path().join("report"))?;
    let _assertion = cmd.args(["--min-nodes", min_nodes]).assert().success();
    Ok(fs::read_to_string(&out.json)?)
}

/// Runs the CLI against `fixture(fixture_name)` with `extra_args`,
/// asserts success, and returns the scan root plus the parsed JSON
/// report. Shared by the issue-#34 prologue regressions, which need
/// the scan root to read back the byte slices the report claims are
/// clones.
fn run_with_args(fixture_name: &str, extra_args: &[&str]) -> Result<(PathBuf, serde_json::Value)> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let scan_root = fixture(fixture_name);
    let mut cmd = fixture_command(fixture_name, &tmp.path().join("report"))?;
    let _assertion = cmd.args(extra_args).assert().success();
    let json = fs::read_to_string(&out.json)?;
    let report: serde_json::Value = serde_json::from_str(&json)?;
    Ok((scan_root, report))
}

/// [`run_with_args`] with the CLI's default flags.
fn run_default(fixture_name: &str) -> Result<(PathBuf, serde_json::Value)> {
    run_with_args(fixture_name, &[])
}

/// Asserts the canonical Type-2 report shape shared by every
/// per-language `*-small` fixture: both files analysed, both file
/// names present, and a structural = 1.0 cluster signal.
fn assert_type2_report(json: &str, first_file: &str, second_file: &str) {
    assert!(json.contains("\"files_analysed\": 2"));
    assert!(json.contains(first_file));
    assert!(json.contains(second_file));
    assert!(json.contains("\"structural\": 1.0"));
}

/// Parses a JSON report and returns its cluster array. A report with no
/// `clusters` key is a malformed report, not an empty one — returning
/// `Vec::new()` there would let every "no cluster does X" guard below
/// pass vacuously on a run that produced nothing at all.
fn report_clusters(json: &str) -> Result<Vec<serde_json::Value>> {
    let report: serde_json::Value = serde_json::from_str(json)?;
    let clusters = report
        .pointer("/clusters")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            anyhow::anyhow!("report carries no `clusters` array — malformed report: {report:#?}")
        })?;
    Ok(clusters.clone())
}

/// Number of files the run parsed, or an error when the report omits the
/// field. Every "no cluster does X" guard pairs with this so a run that
/// silently discovered nothing cannot masquerade as a clean result.
fn files_analysed(json: &str) -> Result<u64> {
    let report: serde_json::Value = serde_json::from_str(json)?;
    report
        .pointer("/files_analysed")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("report carries no `files_analysed` count: {report:#?}"))
}

/// Returns the set of occurrence file basenames carried by `cluster`.
fn cluster_file_basenames(cluster: &serde_json::Value) -> std::collections::BTreeSet<String> {
    cluster
        .get("occurrences")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|occurrence| occurrence.get("path").and_then(serde_json::Value::as_str))
        .map(|path| {
            Path::new(path).file_name().map_or_else(
                || path.to_owned(),
                |name| name.to_string_lossy().into_owned(),
            )
        })
        .collect()
}

/// Finds the cluster spanning both `first_file` and `second_file`, or
/// fails with the full cluster dump so the report shape is visible.
fn require_cluster_spanning<'a>(
    clusters: &'a [serde_json::Value],
    first_file: &str,
    second_file: &str,
) -> Result<&'a serde_json::Value> {
    clusters
        .iter()
        .find(|cluster| {
            let files = cluster_file_basenames(cluster);
            files.contains(first_file) && files.contains(second_file)
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "expected a cross-file cluster spanning {first_file} and {second_file} \
                 (genuine Type-3 body near-miss); the signature-only match must not be \
                 the only cluster; got clusters: {clusters:#?}"
            )
        })
}

/// Asserts the Type-3 near-miss contract on a cross-file cluster:
/// structural = 1.0 on the shared body subtree, at least two
/// occurrences, and a `token_jaccard` signal present.
fn assert_type3_signals(cluster: &serde_json::Value) {
    let structural = cluster
        .pointer("/signals/structural")
        .and_then(serde_json::Value::as_f64);
    assert_eq!(
        structural,
        Some(1.0),
        "the near-miss cluster must reach structural = 1.0 on the shared body subtree, \
         got {structural:?}",
    );
    let occurrences = cluster
        .pointer("/occurrences")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    assert!(
        occurrences >= 2,
        "a clone cluster must have at least two occurrences, got {occurrences}",
    );
    assert!(
        cluster.pointer("/signals/token_jaccard").is_some(),
        "the cross-file cluster must carry a token_jaccard signal",
    );
}

/// Zero-false-positive guard shared by every `*-dissimilar-functions`
/// fixture: every cluster's occurrences stay within a single file, so
/// two structurally unrelated functions are never paired as duplicates.
fn assert_every_cluster_single_file(json: &str, language_label: &str) -> Result<()> {
    assert_eq!(
        files_analysed(json)?,
        2,
        "the {language_label} dissimilar-functions fixture has two source files; a run \
         that analysed a different number never exercised the guard below",
    );
    for (index, cluster) in report_clusters(json)?.iter().enumerate() {
        let cluster_id = cluster
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("<unknown>");
        let files = cluster_file_basenames(cluster);
        assert_eq!(
            files.len(),
            1,
            "cluster #{index} ({cluster_id}) spans multiple files {files:?}; the two \
             {language_label} functions are structurally unrelated and must not be \
             reported as duplicates",
        );
    }
    Ok(())
}

#[test]
fn detects_type2_clone_in_csharp_fixture() -> Result<()> {
    let json = run_min_nodes("csharp-small", "8")?;
    assert_type2_report(&json, "Alpha.cs", "Beta.cs");
    Ok(())
}

// Implements [PIPELINE-LANG-TRAIT] for Rust: Type-2 clone detection.
#[test]
fn detects_type2_clone_in_rust_fixture() -> Result<()> {
    let json = run_min_nodes("rust-small", "10")?;
    assert_type2_report(&json, "alpha.rs", "beta.rs");
    Ok(())
}

// Implements [PIPELINE-LANG-TRAIT] for Python: Type-2 clone detection.
#[test]
fn detects_type2_clone_in_python_fixture() -> Result<()> {
    let json = run_min_nodes("python-small", "10")?;
    assert_type2_report(&json, "alpha.py", "beta.py");
    Ok(())
}

// Implements [PIPELINE-LANG-TRAIT] for Dart ([LANG-CAND-DART]): Type-2
// renamed-clone detection. `alpha.dart` and `beta.dart` are the same
// accumulate loop with every identifier renamed; Dart normalisation
// collapses identifiers/literals so the two functions fingerprint
// identically and cluster at structural = 1.0.
#[test]
fn detects_type2_clone_in_dart_fixture() -> Result<()> {
    let json = run_min_nodes("dart-small", "10")?;
    assert_type2_report(&json, "alpha.dart", "beta.dart");
    Ok(())
}

// Implements [PIPELINE-LANG-TRAIT] for PHP ([PARSE-PHP-NORMALIZE]):
// Type-2 renamed-clone detection. `alpha.php` and `beta.php` implement
// the same accumulate loop with every identifier renamed; PHP
// normalisation collapses identifiers/literals so the two functions
// fingerprint identically and cluster at structural = 1.0.
#[test]
fn detects_type2_clone_in_php_fixture() -> Result<()> {
    let json = run_min_nodes("php-small", "10")?;
    assert_type2_report(&json, "alpha.php", "beta.php");
    Ok(())
}

// Implements [PIPELINE-LANG-TRAIT] for F# ([PARSE-FSHARP-NORMALIZE]):
// Type-2 renamed-clone detection. `alpha.fs` and `beta.fs` implement the
// same accumulate loop with every identifier renamed and the integer
// literals changed; F# normalisation collapses identifiers/literals so the
// two functions fingerprint identically and cluster at structural = 1.0.
#[test]
fn detects_type2_clone_in_fsharp_fixture() -> Result<()> {
    let json = run_min_nodes("fsharp-small", "10")?;
    assert_type2_report(&json, "alpha.fs", "beta.fs");
    Ok(())
}

// Implements [PIPELINE-LANG-TRAIT] for Go ([LANG-CAND-GO]): Type-2
// renamed-clone detection. `alpha.go` and `beta.go` implement the same
// accumulate loop with every identifier renamed and the integer literals
// changed; Go normalisation collapses identifiers/literals so the two
// functions fingerprint identically and cluster at structural = 1.0.
#[test]
fn detects_type2_clone_in_go_fixture() -> Result<()> {
    let json = run_min_nodes("go-small", "10")?;
    assert_type2_report(&json, "alpha.go", "beta.go");
    Ok(())
}

// Implements [FUSION-SIGNALS-THREE-LAYER] for F#: a genuine Type-3
// near-miss. `delta.fs`'s loop body runs two accumulator updates per
// iteration; `epsilon.fs`'s runs one. The shared control-flow subtrees
// (`_ < 0 then 0`, `_ <- _ + _`, `_ in 0 .. _`) surface as a cross-file
// cluster at structural = 1.0, while the signature-only sibling match
// (`f (_: int) : int`, whose bodies differ) is correctly suppressed
// ([CLONE-NOISE-SIGNATURE-ONLY], #154) — proving both the structural
// near-miss path and the signature filter are wired for F#.
#[test]
fn detects_type3_clone_in_fsharp_fixture() -> Result<()> {
    let json = run_min_nodes("fsharp-type3", "8")?;
    let clusters = report_clusters(&json)?;
    let cluster = require_cluster_spanning(&clusters, "delta.fs", "epsilon.fs")?;
    assert_type3_signals(cluster);
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

/// Asserts a cross-file cluster is a *partial* match: no occurrence
/// covers a whole `func` declaration, and no occurrence carries a
/// statement that exists in only one of the two files.
///
/// This is what separates a Type-3 near-miss from a Type-1/2 whole-unit
/// copy ([CLONE-TYPE-TAXONOMY]). Asserting only "a cross-file cluster
/// exists at structural = 1.0" is satisfied by either, so it cannot fail
/// when the near-miss path regresses.
fn assert_partial_near_miss(
    scan_root: &Path,
    cluster: &serde_json::Value,
    divergent_statement: &str,
) -> Result<()> {
    let occurrences = cluster
        .pointer("/occurrences")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("cluster carries no occurrences: {cluster:#?}"))?;
    for occurrence in occurrences {
        let text = require_occurrence_text(scan_root, occurrence)?;
        assert!(
            !text.trim_start().starts_with("func "),
            "a near-miss cluster must span a strict sub-range of each function, not the \
             whole declaration; got:\n{text}",
        );
        assert!(
            !text.contains(divergent_statement),
            "`{divergent_statement}` exists in only one of the two files, so it can never \
             be part of a cross-file clone; the reported range is:\n{text}",
        );
    }
    Ok(())
}

// Implements [FUSION-SIGNALS-THREE-LAYER] for Go ([LANG-CAND-GO]): a
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
    let json = run_min_nodes("go-type3", "8")?;
    let scan_root = fixture("go-type3");
    let clusters = report_clusters(&json)?;
    assert_eq!(
        files_analysed(&json)?,
        2,
        "the go-type3 fixture is a two-file pair; anything else means discovery missed a file",
    );
    let cluster = require_cluster_spanning(&clusters, "delta.go", "epsilon.go")?;
    assert_type3_signals(cluster);
    for cluster in &clusters {
        let files = cluster_file_basenames(cluster);
        if files.len() > 1 {
            assert_partial_near_miss(&scan_root, cluster, "running += 2")?;
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
    let json = run_min_nodes("go-closure-signature-only", "8")?;
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
    let hidden = serde_json::from_str::<serde_json::Value>(&json)?
        .pointer("/clusters_hidden")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("report carries no `clusters_hidden` count"))?;
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
    let (scan_root, report) = run_with_args("go-prologue-false-positive", &["--min-nodes", "15"])?;
    assert_eq!(
        report
            .pointer("/files_analysed")
            .and_then(serde_json::Value::as_u64),
        Some(6),
        "all six package files must be analysed; report={report:#?}",
    );
    let clusters = report
        .pointer("/clusters")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("report carries no `clusters` array: {report:#?}"))?;
    assert!(
        !clusters.is_empty(),
        "the fixture must still produce clone candidates below the prologue, otherwise \
         the guard proves only that nothing was fingerprinted at all",
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
    let json = run_min_nodes("go-dissimilar-functions", "8")?;
    assert_every_cluster_single_file(&json, "Go")
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

// Returns the byte slice of `path` spanned by `occurrence`'s reported
// `[start_byte, end_byte)`. Used by the prologue-cluster regressions
// to read the source text the report claims is a clone.
fn occurrence_source(scan_root: &Path, occurrence: &serde_json::Value) -> Option<Vec<u8>> {
    let path = occurrence.get("path").and_then(serde_json::Value::as_str)?;
    let start = usize::try_from(
        occurrence
            .get("start_byte")
            .and_then(serde_json::Value::as_u64)?,
    )
    .ok()?;
    let end = usize::try_from(
        occurrence
            .get("end_byte")
            .and_then(serde_json::Value::as_u64)?,
    )
    .ok()?;
    let bytes = fs::read(scan_root.join(path)).ok()?;
    bytes.get(start..end).map(<[u8]>::to_vec)
}

// Returns true when `text` opens with a top-level import/prologue
// construct in any of the languages Deslop currently parses: a Python
// triple-quoted module docstring, `import` / `from` / `if TYPE_CHECKING:`,
// a C# `using` directive or `namespace` declaration, or a Rust
// `use` / `extern crate` statement. The check looks only at the first
// non-whitespace line so a window that starts with prologue and
// extends into real code still counts as prologue-anchored.
fn opens_with_prologue_keyword(text: &str) -> bool {
    let trimmed = text.trim_start();
    if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
        return true;
    }
    let first_line = trimmed.lines().next().unwrap_or("").trim_start();
    if first_line.starts_with("if TYPE_CHECKING") {
        return true;
    }
    let token = first_line.split_whitespace().next().unwrap_or("");
    matches!(
        token,
        "use" | "using" | "namespace" | "import" | "from" | "extern"
    )
}

// Asserts no cluster in `report` is a cross-file prologue false
// positive: a multi-file cluster whose every occurrence starts on an
// import/use/namespace/docstring line. Drives all three issue-#34
// regression tests (Python, C#, Rust).
fn assert_no_cross_file_prologue_cluster(
    report: &serde_json::Value,
    scan_root: &Path,
    label: &str,
) {
    let clusters = report
        .pointer("/clusters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    for cluster in &clusters {
        let occurrences = cluster
            .get("occurrences")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let cluster_id = cluster
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("<unknown>");
        let mut files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut all_prologue = !occurrences.is_empty();
        for occurrence in &occurrences {
            if let Some(path) = occurrence.get("path").and_then(serde_json::Value::as_str) {
                let _inserted = files.insert(path.to_owned());
            }
            let bytes = occurrence_source(scan_root, occurrence).unwrap_or_default();
            let text = std::str::from_utf8(&bytes).unwrap_or("");
            if !opens_with_prologue_keyword(text) {
                all_prologue = false;
            }
        }
        assert!(
            !(all_prologue && files.len() > 1),
            "{label}: cluster {cluster_id} is a cross-file prologue cluster spanning \
             {files:?}; import / use / namespace / docstring scaffolding must never \
             anchor a cross-file clone",
        );
    }
}

// Audience: HUMAN. Issue #34. Python test suites conventionally open
// with a module docstring, `from __future__ import annotations`,
// `import pytest`, `from typing import TYPE_CHECKING`, and an
// `if TYPE_CHECKING:` import block. That prologue is pure
// import/prologue boilerplate: it carries no semantic content a human
// would recognise as "copy-pasted code". Before the fix for #34 the
// prologue subtree survived the boilerplate filter (no
// `future_import_statement` carrier, no module-docstring carrier, and
// the `if_statement` wrapper around imports was not treated as an
// imports-only subtree), so deslop reported the prologue as a
// cross-file clone spanning every Python file in the repo. For a
// 40-file repo that produced a 109-member cluster; even a 6-file
// fixture reproduces the symptom.
#[test]
fn python_module_prologue_never_becomes_a_cross_file_cluster() -> Result<()> {
    let (scan_root, report) = run_default("python-prologue-false-positive")?;
    assert_no_cross_file_prologue_cluster(&report, &scan_root, "python prologue");
    Ok(())
}

// Audience: HUMAN. Issue #34, C# arm. The same prologue
// false-positive that hits Python `from __future__ import` /
// `if TYPE_CHECKING:` blocks also hits C# files: when many `.cs`
// files share the same `using ...;` block + `namespace X;` prologue
// but have entirely different class bodies, the sibling-window pass
// emits windows that span from the prologue into the class
// declaration. The `using_directive`/`file_scoped_namespace_declaration`
// k-grams dominate the window's token signature, so token Jaccard
// approaches 1.00 and LSH-only matching links every file into one
// cross-file cluster — even though the structural Merkle hashes
// disagree. The reported cluster from the user's repo had 109
// occurrences pinned at line 1, column 1 across the codebase; six
// distinct files reproduce the same shape here.
#[test]
fn csharp_using_namespace_prologue_never_becomes_a_cross_file_cluster() -> Result<()> {
    let (scan_root, report) = run_default("csharp-prologue-false-positive")?;
    assert_no_cross_file_prologue_cluster(&report, &scan_root, "csharp prologue");
    Ok(())
}

// Audience: HUMAN. Issue #34, Rust arm. Six Rust files share the same
// eight-line `use ...;` block but contain completely different items
// (a function, a struct, an async fetcher, a trait, a CSV parser, a
// retry policy). Sibling windows that begin on a `use_declaration`
// and extend into the next `function_item` / `struct_item` /
// `trait_item` carry token signatures dominated by
// `use_declaration __ident__` k-grams, pushing token Jaccard to 1.00.
// `use_declaration` is already a boilerplate carrier for subtree
// fingerprints, but the sibling-window emitter still produces windows
// that *start* inside the use block — and those windows do anchor
// cross-file LSH-only clusters. The fix must keep import scaffolding
// from anchoring cross-file matches in any language we parse.
#[test]
fn rust_use_prologue_never_becomes_a_cross_file_cluster() -> Result<()> {
    let (scan_root, report) = run_default("rust-prologue-false-positive")?;
    assert_no_cross_file_prologue_cluster(&report, &scan_root, "rust prologue");
    Ok(())
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

// Implements [DECISION-TYPE3-TWO-PASS] + [FUSION-STRATEGY-BOUNDED-MAX]:
// Type-3 near-miss cross-file cluster with `structural=0.0`.
#[test]
fn detects_type3_clone_in_csharp_fixture() -> Result<()> {
    let json = run_min_nodes("csharp-type3", "15")?;
    assert!(json.contains("Delta.cs"));
    assert!(json.contains("Epsilon.cs"));
    assert!(json.contains("\"structural\": 0.0"));
    assert!(json.contains("\"token_jaccard\""));
    Ok(())
}

// Implements [EXCLUSION-CONFIG] `exclude` tier: a file matched by the

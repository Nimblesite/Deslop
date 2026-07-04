use crate::support::*;

/// Runs the CLI against `fixture(fixture_name)` with `--min-nodes
/// <min_nodes>`, asserts the process succeeded, and returns the raw
/// JSON report text. Shared by every detection test that drives a
/// fixture with an explicit `--min-nodes` and then asserts on the
/// rendered report.
fn run_min_nodes(fixture_name: &str, min_nodes: &str) -> Result<String> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&fixture(fixture_name), &tmp.path().join("report"))?;
    let _assertion = cmd.args(["--min-nodes", min_nodes]).assert().success();
    Ok(fs::read_to_string(&out.json)?)
}

/// Runs the CLI against `fixture(fixture_name)` with default flags,
/// asserts success, and returns the scan root plus the parsed JSON
/// report. Shared by the issue-#34 prologue regressions, which need
/// the scan root to read back the byte slices the report claims are
/// clones.
fn run_default(fixture_name: &str) -> Result<(PathBuf, serde_json::Value)> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let scan_root = fixture(fixture_name);
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd.assert().success();
    let json = fs::read_to_string(&out.json)?;
    let report: serde_json::Value = serde_json::from_str(&json)?;
    Ok((scan_root, report))
}

#[test]
fn detects_type2_clone_in_csharp_fixture() -> Result<()> {
    let json = run_min_nodes("csharp-small", "8")?;
    assert!(json.contains("\"files_analysed\": 2"));
    assert!(json.contains("Alpha.cs"));
    assert!(json.contains("Beta.cs"));
    assert!(json.contains("\"structural\": 1.0"));
    Ok(())
}

// Implements [PIPELINE-LANG-TRAIT] for Rust: Type-2 clone detection.
#[test]
fn detects_type2_clone_in_rust_fixture() -> Result<()> {
    let json = run_min_nodes("rust-small", "10")?;
    assert!(json.contains("\"files_analysed\": 2"));
    assert!(json.contains("alpha.rs"));
    assert!(json.contains("beta.rs"));
    assert!(json.contains("\"structural\": 1.0"));
    Ok(())
}

// Implements [PIPELINE-LANG-TRAIT] for Python: Type-2 clone detection.
#[test]
fn detects_type2_clone_in_python_fixture() -> Result<()> {
    let json = run_min_nodes("python-small", "10")?;
    assert!(json.contains("\"files_analysed\": 2"));
    assert!(json.contains("alpha.py"));
    assert!(json.contains("beta.py"));
    assert!(json.contains("\"structural\": 1.0"));
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
    assert!(json.contains("\"files_analysed\": 2"));
    assert!(json.contains("alpha.dart"));
    assert!(json.contains("beta.dart"));
    assert!(json.contains("\"structural\": 1.0"));
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
    assert!(json.contains("\"files_analysed\": 2"));
    assert!(json.contains("alpha.php"));
    assert!(json.contains("beta.php"));
    assert!(json.contains("\"structural\": 1.0"));
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
    assert!(json.contains("\"files_analysed\": 2"));
    assert!(json.contains("alpha.fs"));
    assert!(json.contains("beta.fs"));
    assert!(json.contains("\"structural\": 1.0"));
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
    let report: serde_json::Value = serde_json::from_str(&json)?;
    let clusters = report
        .pointer("/clusters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let cross_file = clusters.iter().find(|cluster| {
        let files: std::collections::BTreeSet<String> = cluster
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
            .collect();
        files.contains("delta.fs") && files.contains("epsilon.fs")
    });
    let Some(cluster) = cross_file else {
        anyhow::bail!(
            "fsharp-type3 must produce a cross-file cluster spanning delta.fs and epsilon.fs \
             (genuine Type-3 body near-miss); the signature-only match must not be the only \
             cluster; got clusters: {clusters:#?}"
        );
    };
    let structural = cluster.pointer("/signals/structural").and_then(serde_json::Value::as_f64);
    assert_eq!(
        structural,
        Some(1.0),
        "the F# near-miss cluster must reach structural = 1.0 on the shared body subtree, \
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
        "the cross-file F# cluster must carry a token_jaccard signal",
    );
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
    let report: serde_json::Value = serde_json::from_str(&json)?;
    let clusters = report
        .pointer("/clusters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    for (index, cluster) in clusters.iter().enumerate() {
        let occurrences = cluster
            .get("occurrences")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for occurrence in &occurrences {
            let Some(path) = occurrence.get("path").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let basename = Path::new(path).file_name().map_or_else(
                || path.to_owned(),
                |name| name.to_string_lossy().into_owned(),
            );
            let _inserted = files.insert(basename);
        }
        assert_eq!(
            files.len(),
            1,
            "cluster #{index} spans multiple files {files:?}; the two F# functions are \
             structurally unrelated and must not be reported as duplicates",
        );
    }
    Ok(())
}

// Audience: HUMAN. Zero-false-positive guard for Dart ([LANG-CAND-DART]).
// `tally()` builds a map inside a for-each loop; `describe()` is a chain
// of `if (code …) return …` early exits. The two share no real shape, so
// a human reading the report must never see them paired as duplicates.
// Positive bound: every cluster's occurrences come from a single file.
#[test]
fn dissimilar_dart_functions_across_files_stay_in_separate_clusters() -> Result<()> {
    let json = run_min_nodes("dart-dissimilar-functions", "8")?;
    let report: serde_json::Value = serde_json::from_str(&json)?;
    let clusters = report
        .pointer("/clusters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    for (index, cluster) in clusters.iter().enumerate() {
        let occurrences = cluster
            .get("occurrences")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for occurrence in &occurrences {
            let Some(path) = occurrence.get("path").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let basename = Path::new(path).file_name().map_or_else(
                || path.to_owned(),
                |name| name.to_string_lossy().into_owned(),
            );
            let _inserted = files.insert(basename);
        }
        assert_eq!(
            files.len(),
            1,
            "cluster #{index} spans multiple files {files:?}; the two Dart functions \
             are structurally unrelated and must not be reported as duplicates",
        );
    }
    Ok(())
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
    let report: serde_json::Value = serde_json::from_str(&json)?;
    let clusters = report
        .pointer("/clusters")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();

    for (index, cluster) in clusters.iter().enumerate() {
        let cluster_id = cluster
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("<unknown>");
        let occurrences = cluster
            .get("occurrences")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for occurrence in &occurrences {
            let Some(path) = occurrence.get("path").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let basename = Path::new(path).file_name().map_or_else(
                || path.to_owned(),
                |name| name.to_string_lossy().into_owned(),
            );
            let _inserted = files.insert(basename);
        }
        assert_eq!(
            files.len(),
            1,
            "cluster #{index} ({cluster_id}) spans multiple files {files:?}; \
             the human reader would be confused because the bodies are not similar",
        );
    }
    Ok(())
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

// Implements [DECISION-TYPE3-TWO-PASS] + [FUSION-STRATEGY-MAX-SUM]:
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

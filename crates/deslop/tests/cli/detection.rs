use super::support::*;
use crate::common::signals::{assert_no_pair_surface_on_cluster, has_verbatim_pair};

const TYPE2_EXPECTED_FILES_ANALYSED: u64 = 2;
const TYPE2_EXPECTED_OCCURRENCES: usize = 2;
const MINIMUM_DUPLICATED_MASS: u64 = 1;
const VALID_MASS_RANK_BANDS: [&str; 4] = ["worst", "top10", "mid", "faint"];

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

/// Asserts the canonical Type-2 report shape shared by every
/// per-language `*-small` fixture: both files analysed, one component spans
/// exactly both source files, and its only cluster-level measures are
/// mass-derived ([RANK-MASS-SUM], [FUSED-PAIR-SIGNALS]). The old
/// `structural: 1.0` cluster signal is retired from the wire; no pair-only
/// evidence may silently return.
fn assert_type2_report(json: &str, first_file: &str, second_file: &str) -> Result<()> {
    let report: serde_json::Value = serde_json::from_str(json)?;
    assert_eq!(
        require_u64(&report, "/files_analysed", "report")?,
        TYPE2_EXPECTED_FILES_ANALYSED,
        "the Type-2 fixture must analyse both authored source files"
    );
    let clusters = require_array(&report, "/clusters", "report")?;
    assert!(
        !clusters.is_empty(),
        "a Type-2 fixture must surface a cluster: {json}"
    );
    let clone = require_cluster_spanning(clusters, first_file, second_file)?;
    assert_eq!(
        cluster_file_basenames(clone),
        std::collections::BTreeSet::from([first_file.to_owned(), second_file.to_owned()]),
        "the Type-2 component must contain exactly its two fixture files"
    );
    assert_eq!(
        require_array(clone, "/occurrences", "Type-2 cluster")?.len(),
        TYPE2_EXPECTED_OCCURRENCES,
        "the Type-2 component must preserve its two exact occurrences"
    );
    assert_no_pair_surface_on_cluster(clone, "Type-2 cluster");
    for cluster in clusters {
        let band = cluster
            .get("rank_band")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("cluster carries no rank band: {cluster}"))?;
        assert!(
            VALID_MASS_RANK_BANDS.contains(&band),
            "cluster {} carries no rank band: {cluster}",
            cluster_id(cluster)
        );
        let mass = require_u64(cluster, "/mass", "cluster")?;
        assert!(
            mass >= MINIMUM_DUPLICATED_MASS,
            "cluster mass must be positive: {cluster}"
        );
    }
    Ok(())
}

/// The array `owner` carries at `pointer`, or an error dumping the whole
/// value. A missing array is a *malformed* report, not an empty one —
/// defaulting to `Vec::new()` there would let every "no cluster does X"
/// guard below pass vacuously on a run that produced nothing at all.
fn require_array<'a>(
    value: &'a serde_json::Value,
    pointer: &str,
    owner: &str,
) -> Result<&'a Vec<serde_json::Value>> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            anyhow::anyhow!("{owner} carries no `{pointer}` array — malformed: {value:#?}")
        })
}

/// The count `owner` carries at `pointer`, or an error when it is absent.
/// A count a guard depends on must be present, never defaulted.
fn require_u64(value: &serde_json::Value, pointer: &str, owner: &str) -> Result<u64> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("{owner} carries no `{pointer}` count: {value:#?}"))
}

/// The array at `pointer`, or an empty vector. Only for callers that have
/// already proved the surrounding report is well formed.
fn array_or_empty(value: &serde_json::Value, pointer: &str) -> Vec<serde_json::Value> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// Parses a JSON report and returns its cluster array.
fn report_clusters(json: &str) -> Result<Vec<serde_json::Value>> {
    let report: serde_json::Value = serde_json::from_str(json)?;
    Ok(require_array(&report, "/clusters", "report")?.clone())
}

/// Number of files the run parsed. Every "no cluster does X" guard pairs
/// with this so a run that silently discovered nothing cannot masquerade
/// as a clean result.
fn files_analysed(json: &str) -> Result<u64> {
    require_u64(&serde_json::from_str(json)?, "/files_analysed", "report")
}

/// The cluster's reported id, or `<unknown>` when the report omits it.
/// Every cross-file guard names it in its failure message.
fn cluster_id(cluster: &serde_json::Value) -> &str {
    cluster
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("<unknown>")
}

/// Returns the set of occurrence paths carried by `cluster`, as reported.
fn cluster_file_paths(cluster: &serde_json::Value) -> std::collections::BTreeSet<String> {
    array_or_empty(cluster, "/occurrences")
        .iter()
        .filter_map(|occurrence| occurrence.get("path").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
        .collect()
}

/// Returns the set of occurrence file basenames carried by `cluster`.
fn cluster_file_basenames(cluster: &serde_json::Value) -> std::collections::BTreeSet<String> {
    cluster_file_paths(cluster)
        .iter()
        .map(|path| {
            Path::new(path)
                .file_name()
                .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned())
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
/// at least two occurrences and the mass-only wire fields. The
/// admission-floor and not-Merkle-exact bounds this helper used to
/// pin via `signals.structural` moved to the pair surface when the
/// cluster wire went mass-only: a cluster names components, pair
/// evidence names edges ([FUSED-PAIR-SIGNALS]). The not-verbatim half
/// is proven by the byte truth — a one-statement Type-3 near-miss can
/// never be Merkle-exact by construction (gh #408) — and the clean
/// surface keeps pair-only fields off the cluster.
fn assert_type3_signals(
    scan_root: &Path,
    cluster: &serde_json::Value,
    language: &str,
) -> Result<()> {
    let occurrences = cluster
        .pointer("/occurrences")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    assert!(
        occurrences >= 2,
        "a clone cluster must have at least two occurrences, got {occurrences}",
    );
    assert!(
        !has_verbatim_pair(scan_root, cluster)?,
        "the reported view must be the near-miss itself, not a byte-identical pair: \
         a one-statement Type-3 near-miss cannot be Merkle-exact by construction (gh #408); \
         got {cluster:#}",
    );
    assert_no_pair_surface_on_cluster(cluster, &format!("{language} type-3 near-miss"));
    Ok(())
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
        let files = cluster_file_basenames(cluster);
        assert_eq!(
            files.len(),
            1,
            "cluster #{index} ({}) spans multiple files {files:?}; the two \
             {language_label} functions are structurally unrelated and must not be \
             reported as duplicates",
            cluster_id(cluster),
        );
    }
    Ok(())
}

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
    let json = run_min_nodes("go-small", "10")?;
    assert_type2_report(&json, "alpha.go", "beta.go")?;
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
        assert!(
            text.contains("func "),
            "the near-miss view must span the whole enclosing declaration, not a \
             fragment of it (gh #408); got:\n{text}",
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
    let json = run_min_nodes("go-type3", "8")?;
    let scan_root = fixture("go-type3");
    let clusters = report_clusters(&json)?;
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
    let (scan_root, report) = run_with_args("go-prologue-false-positive", &["--min-nodes", "15"])?;
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
    let start = occurrence_byte(occurrence, "start_byte")?;
    let end = occurrence_byte(occurrence, "end_byte")?;
    let bytes = fs::read(scan_root.join(path)).ok()?;
    bytes.get(start..end).map(<[u8]>::to_vec)
}

// The `key` byte offset an occurrence reports, narrowed to a `usize`.
fn occurrence_byte(occurrence: &serde_json::Value, key: &str) -> Option<usize> {
    usize::try_from(occurrence.get(key).and_then(serde_json::Value::as_u64)?).ok()
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
    let clusters = array_or_empty(report, "/clusters");
    for cluster in &clusters {
        let occurrences = array_or_empty(cluster, "/occurrences");
        let all_prologue = !occurrences.is_empty()
            && occurrences.iter().all(|occurrence| {
                let bytes = occurrence_source(scan_root, occurrence).unwrap_or_default();
                opens_with_prologue_keyword(std::str::from_utf8(&bytes).unwrap_or(""))
            });
        let files = cluster_file_paths(cluster);
        assert!(
            !(all_prologue && files.len() > 1),
            "{label}: cluster {} is a cross-file prologue cluster spanning \
             {files:?}; import / use / namespace / docstring scaffolding must never \
             anchor a cross-file clone",
            cluster_id(cluster),
        );
    }
}

// Drives `fixture_name` with the CLI's default flags and asserts its
// report carries no cross-file prologue cluster. Shared by the three
// issue-#34 prologue regressions (Python, C#, Rust).
fn assert_no_prologue_false_positive(fixture_name: &str, label: &str) -> Result<()> {
    let (scan_root, report) = run_with_args(fixture_name, &[])?;
    assert_no_cross_file_prologue_cluster(&report, &scan_root, label);
    Ok(())
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
    assert_no_prologue_false_positive("python-prologue-false-positive", "python prologue")
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
    assert_no_prologue_false_positive("csharp-prologue-false-positive", "csharp prologue")
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
    assert_no_prologue_false_positive("rust-prologue-false-positive", "rust prologue")
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

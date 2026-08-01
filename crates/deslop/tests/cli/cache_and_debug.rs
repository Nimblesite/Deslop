use crate::support::*;
use std::fmt::Write as _;

/// Runs an `--incremental` pass over `scan_root`, writing `<prefix>.json`
/// (and siblings), asserts the process succeeded, and returns the JSON
/// report body as a string. Centralises the seed-already-present
/// run-and-read shape shared by the cache tests.
fn run_incremental_pass(scan_root: &Path, output_prefix: &Path) -> Result<String> {
    let mut cmd = deslop_command(scan_root, output_prefix)?;
    let _assertion = cmd
        .args(["--min-nodes", "8", "--incremental"])
        .assert()
        .success();
    Ok(fs::read_to_string(with_ext(output_prefix, "json"))?)
}

#[test]
fn output_path_with_missing_parent_is_created() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let base = tmp.path().join("a").join("b").join("c").join("report");
    let mut cmd = deslop_command(&fixture("csharp-small"), &base)?;
    let _assertion = cmd.args(["--min-nodes", "8"]).assert().success();
    assert!(
        base.with_extension("json").exists(),
        "nested-parent json missing"
    );
    assert!(
        base.with_extension("txt").exists(),
        "nested-parent txt missing"
    );
    Ok(())
}

// Implements [EXCLUSION-CONFIG] `report_hide` drops a cluster whose
// members are all hidden and increments `clusters_hidden`.
#[test]
fn report_hide_drops_cluster_when_all_members_hidden() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let out = outputs_under(tmp.path());
    let config = tmp.path().join("deslop.toml");
    fs::write(&config, "[defaults]\nreport_hide = [\"**/*.cs\"]\n")?;
    let mut cmd = deslop_command(&fixture("csharp-small"), &tmp.path().join("report"))?;
    let _assertion = cmd
        .args(["--min-nodes", "8", "--config"])
        .arg(&config)
        .assert()
        .success();
    let json = fs::read_to_string(&out.json)?;
    assert!(json.contains("\"files_analysed\": 2"));
    assert!(
        !json.contains("\"hidden\": false"),
        "every member should be hidden: {json}"
    );
    assert!(
        !json.contains("\"structural\": 1.0"),
        "hidden-only cluster must be dropped from visible list"
    );
    assert!(
        json.contains("\"clusters_hidden\": 1"),
        "clusters_hidden must count the suppressed cluster"
    );
    Ok(())
}

// ===========================================================================
// P6 — incremental, perf, fixture-per-bug
// ===========================================================================

// Implements [PIPELINE-INCREMENTAL]: `--incremental` populates the
// fingerprint cache on the first pass and reports every file as a
// miss. A second run over the same unchanged tree must report every
// file as a hit and still surface the duplicated cluster.
#[test]
fn incremental_cache_hits_on_second_run() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
    let first_json = run_incremental_pass(&scan_root, &tmp.path().join("first"))?;
    assert!(
        first_json.contains("\"hits\": 0"),
        "first run must be a clean miss: {first_json}"
    );
    assert!(
        first_json.contains("\"misses\": 2"),
        "first run must register two misses: {first_json}"
    );
    let cache_dir = scan_root.join(".deslop/cache").join("fingerprints");
    assert!(
        cache_dir.is_dir(),
        "fingerprint cache directory missing: {}",
        cache_dir.display()
    );
    let second_json = run_incremental_pass(&scan_root, &tmp.path().join("second"))?;
    assert!(
        second_json.contains("\"hits\": 2"),
        "second run must hit the cache for both files: {second_json}"
    );
    assert!(
        second_json.contains("\"misses\": 0"),
        "second run must have zero misses: {second_json}"
    );
    // Deduplication must still fire even when the fingerprints came
    // from the cache — the rehydration is only useful if downstream
    // clustering sees identical results.
    assert!(
        second_json.contains("\"structural\": 1.0"),
        "cached run must still detect the Type-2 cluster: {second_json}"
    );
    let second_txt = fs::read_to_string(tmp.path().join("second.txt"))?;
    assert!(
        second_txt.contains("cache: 2 hit / 0 miss"),
        "text renderer must surface cache stats: {second_txt}"
    );
    Ok(())
}

// Implements [PIPELINE-INCREMENTAL] default-off: without
// `--incremental` the cache is neither read nor written. Stats read
// as a clean no-cache run (both counters zero) and no blobs land on
// disk — analysing a read-only checkout must never mutate it.
#[test]
fn default_run_skips_the_cache() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd.args(["--min-nodes", "8"]).assert().success();
    let json = fs::read_to_string(tmp.path().join("report.json"))?;
    assert!(
        json.contains("\"hits\": 0"),
        "default run must record zero hits: {json}"
    );
    assert!(
        json.contains("\"misses\": 0"),
        "default run must not increment misses either: {json}"
    );
    assert!(
        !scan_root
            .join(".deslop/cache")
            .join("fingerprints")
            .exists(),
        "default run must not populate the fingerprint cache",
    );
    Ok(())
}

// Implements [PIPELINE-INCREMENTAL] stale-blob recovery: a corrupt
// cache entry must be treated as a miss and overwritten, not surfaced
// as a hard error. The pipeline still produces a correct report.
#[test]
fn corrupt_cache_entry_degrades_to_miss() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
    let _first_json = run_incremental_pass(&scan_root, &tmp.path().join("first"))?;
    let fingerprints_root = scan_root.join(".deslop/cache").join("fingerprints");
    for language_dir in fs::read_dir(&fingerprints_root)? {
        let language_path = language_dir?.path();
        for version_dir in fs::read_dir(&language_path)? {
            let version_path = version_dir?.path();
            for min_nodes_dir in fs::read_dir(&version_path)? {
                let min_nodes_path = min_nodes_dir?.path();
                for blob in fs::read_dir(&min_nodes_path)? {
                    let blob = blob?.path();
                    fs::write(&blob, b"not a valid cache blob")?;
                }
            }
        }
    }
    let second_json = run_incremental_pass(&scan_root, &tmp.path().join("second"))?;
    assert!(
        second_json.contains("\"misses\": 2"),
        "corrupt entries must be treated as misses: {second_json}"
    );
    assert!(
        second_json.contains("\"structural\": 1.0"),
        "analysis still produces the cluster after recovery: {second_json}"
    );
    Ok(())
}

// Implements [PIPELINE-INCREMENTAL] help-text exposure: the
// `--incremental` opt-in must be documented so users can discover
// the cache without reading the source.
#[test]
fn help_text_documents_incremental_flag() -> Result<()> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("--incremental"));
    Ok(())
}

// Implements [PIPELINE-INCREMENTAL] cache-write degradation: when
// the cache directory is read-only, the pipeline must log a warning
// and still produce a complete report. Exercises the
// `tracing::warn!(error, "fingerprint cache write failed")` branch
// that is otherwise only reachable on a genuine disk failure.
#[cfg(unix)]
#[test]
fn cache_write_failure_is_degraded_not_fatal() -> Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    fs::create_dir_all(&scan_root)?;
    for entry in fs::read_dir(fixture("csharp-small"))? {
        let entry = entry?;
        let _bytes = fs::copy(entry.path(), scan_root.join(entry.file_name()))?;
    }
    let locked_dir = scan_root
        .join(".deslop/cache")
        .join("fingerprints")
        .join("csharp")
        .join(env!("CARGO_PKG_VERSION"))
        .join("8");
    fs::create_dir_all(&locked_dir)?;
    let mut perms = fs::metadata(&locked_dir)?.permissions();
    perms.set_mode(0o555);
    fs::set_permissions(&locked_dir, perms)?;
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd
        .args(["--min-nodes", "8", "--incremental"])
        .assert()
        .success();
    let mut restore = fs::metadata(&locked_dir)?.permissions();
    restore.set_mode(0o755);
    fs::set_permissions(&locked_dir, restore)?;
    let json = fs::read_to_string(tmp.path().join("report.json"))?;
    assert!(
        json.contains("\"files_analysed\": 2"),
        "pipeline must still report both files: {json}"
    );
    Ok(())
}

// Implements the P6 "perf pass" target in PLAN.md. The user-facing
// budget is <30 s on 100K-LOC C# with no embeddings. We can't assert
// on wallclock directly from `cargo test` (coverage instrumentation
// triples debug runtime), so this test exercises the pipeline on a
// modest synthetic C# corpus and asserts every file parsed + at
// least one cluster was ranked. The wallclock bound is deliberately
// lax — a pure regression guard against infinite loops or
// catastrophic quadratic explosions. The true SLA lives in
// [PERF-BUDGET-TYPE12] and is validated manually against a release
// binary on a real corpus.
#[test]
fn synthetic_corpus_scale_smoke_test() -> Result<()> {
    use std::time::Instant;

    let tmp = tempfile::tempdir()?;
    let corpus = tmp.path().join("corpus");
    fs::create_dir_all(&corpus)?;
    let files: u32 = 10;
    let methods_per_file: u32 = 10;
    for file_index in 0..files {
        let mut source = String::with_capacity(2048);
        source.push_str("namespace Generated;\npublic class Class");
        source.push_str(&file_index.to_string());
        source.push_str(" {\n");
        for method_index in 0..methods_per_file {
            let _ = writeln!(
                &mut source,
                "    public int Method{method_index}(int a, int b) {{ int x = a + b; int y = x * 2; if (y > 0) {{ return y; }} return x - a; }}",
            );
        }
        source.push_str("}\n");
        fs::write(corpus.join(format!("Class{file_index}.cs")), source)?;
    }
    let started = Instant::now();
    let mut cmd = deslop_command(&corpus, &tmp.path().join("report"))?;
    let _assertion = cmd.args(["--min-nodes", "30"]).assert().success();
    let elapsed = started.elapsed();
    // 180 s is a regression guard, not a performance SLA. The real
    // budget is validated manually. A release-mode run against this
    // corpus takes well under a second.
    assert!(
        elapsed.as_secs() < 180,
        "synthetic corpus ran for {elapsed:?} — something is catastrophically wrong",
    );
    let json = fs::read_to_string(tmp.path().join("report.json"))?;
    assert!(
        json.contains("\"files_analysed\": 10"),
        "synthetic corpus must analyse every generated file: {json}"
    );
    // Every file shares the identical method template, so the
    // ranked output must contain at least one cluster — catches
    // pipelines that silently drop everything.
    assert!(
        json.contains("\"weight\":"),
        "synthetic corpus produced no clusters: {json}"
    );
    Ok(())
}

// Implements the [BUG-FIXTURE] workflow from CLAUDE.md: every bug
// reproduced into `tests/fixtures/bug-*/` becomes a permanent e2e
// test. This is the seed example — an empty C# class body used to
// be silently dropped before the sibling-window fingerprint pass
// existed; the cluster test below pins that behaviour so the bug
// cannot regress.
#[test]
fn bug_fixture_walks_trivial_class_body_without_panicking() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let mut cmd = deslop_command(&fixture("bug-empty-class"), &tmp.path().join("report"))?;
    let _assertion = cmd.args(["--min-nodes", "4"]).assert().success();
    let json = fs::read_to_string(tmp.path().join("report.json"))?;
    assert!(
        json.contains("\"files_analysed\": 1"),
        "empty-class fixture must still analyse its one file: {json}"
    );
    Ok(())
}

// Implements [PIPELINE-NORMALIZE-AST] golden guard: `--debug-ast`
// on a hand-picked per-language fixture must match the committed
// expected dump byte-for-byte. Any drift in the grammar version,
// the `normalise_kind` match arms, or the child-ordering policy
// will trip this test — which is exactly what we want, because
// any of those changes silently alters the fingerprint and
// invalidates every user's cache.
//
// Each fixture exercises identifier collapse, literal collapse,
// comment drop, and the language-specific structural forms most
// likely to shift between grammar patch releases.
fn assert_ast_golden(fixture_dir: &str, sample_name: &str) -> Result<()> {
    let source = fixture(fixture_dir).join(sample_name);
    let expected_path = fixture(fixture_dir).join("Sample.expected.ast");
    let expected = fs::read_to_string(&expected_path)?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let output = cmd
        .arg("--debug-ast")
        .arg(&source)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let actual = String::from_utf8(output)?;
    assert_eq!(
        actual,
        expected,
        "AST dump drifted from {}. If this is intentional, regenerate with \
         `cargo run -q -- --debug-ast {}` and commit the updated .expected.ast.",
        expected_path.display(),
        source.display(),
    );
    Ok(())
}

#[test]
fn debug_ast_dump_matches_committed_golden() -> Result<()> {
    assert_ast_golden("ast-golden-csharp", "Sample.cs")
}

#[test]
fn debug_ast_dump_matches_committed_golden_rust() -> Result<()> {
    assert_ast_golden("ast-golden-rust", "Sample.rs")
}

#[test]
fn debug_ast_dump_matches_committed_golden_python() -> Result<()> {
    assert_ast_golden("ast-golden-python", "Sample.py")
}

// [LANG-CAND-DART] golden: Sample.dart exercises identifier collapse,
// every Dart literal form (string interpolation, int, double, bool,
// `null`), comment + doc-comment drop, and the Dart-3 structural forms
// most likely to shift between grammar patch releases (records, switch
// expression with record/variable/wildcard patterns + guard). Any
// grammar bump or `normalise_kind` edit trips this byte-for-byte check.
#[test]
fn debug_ast_dump_matches_committed_golden_dart() -> Result<()> {
    assert_ast_golden("ast-golden-dart", "Sample.dart")
}

// [LANG-CAND-JAVASCRIPT] golden: Sample.js exercises the shared
// ECMAScript-family identifier/literal/comment normalisation path with
// template strings, object shorthand, ternaries, and regex literals.
#[test]
fn debug_ast_dump_matches_committed_golden_javascript() -> Result<()> {
    assert_ast_golden("ast-golden-javascript", "Sample.js")
}

// [LANG-CAND-TYPESCRIPT] golden: Sample.ts exercises the shared
// ECMAScript-family normaliser plus TypeScript-only type alias,
// optional property, type annotation, and generic wrapper nodes.
#[test]
fn debug_ast_dump_matches_committed_golden_typescript() -> Result<()> {
    assert_ast_golden("ast-golden-typescript", "Sample.ts")
}

// [LANG-CAND-TYPESCRIPT] golden: Sample.tsx proves TSX reaches the
// same normaliser through the separate TSX grammar entry point and
// preserves JSX structure while collapsing JSX text/literals.
#[test]
fn debug_ast_dump_matches_committed_golden_tsx() -> Result<()> {
    assert_ast_golden("ast-golden-tsx", "Sample.tsx")
}

// [PARSE-PHP-NORMALIZE] golden: Sample.php exercises identifier collapse
// (`name`), literal collapse (string, integer, float, boolean, null),
// comment and doc-comment drop, and the PHP structural forms most likely
// to shift between grammar patch releases (class, method, for-loop,
// cast). Any grammar bump or `normalise_kind` edit trips this check.
#[test]
fn debug_ast_dump_matches_committed_golden_php() -> Result<()> {
    assert_ast_golden("ast-golden-php", "Sample.php")
}

// [PARSE-FSHARP-NORMALIZE] golden: Sample.fs exercises identifier collapse
// (`identifier`/`op_identifier` under the `long_identifier` wrappers),
// literal collapse (string, float, bool, char, hex `int`, and the unit
// value `()`), line / xml-doc / block comment drop, and the F# structural
// forms most likely to shift between grammar patch releases (module,
// nested `let ... in` desugaring, typed binding, if/else). Any grammar
// bump or `normalise_kind` edit trips this byte-for-byte check.
#[test]
fn debug_ast_dump_matches_committed_golden_fsharp() -> Result<()> {
    assert_ast_golden("ast-golden-fsharp", "Sample.fs")
}

// [LANG-CAND-GO] golden: Sample.go exercises identifier collapse
// (`identifier`, `field_identifier`, `type_identifier`,
// `package_identifier` in a qualified type, `blank_identifier`, and
// `label_name`), literal collapse (int, float, imaginary, rune,
// interpreted + raw strings with escape sequences, `true`, `false`,
// `nil`, `iota`), line / block comment drop, and the Go structural
// forms most likely to shift between grammar patch releases (struct,
// method with receiver, labeled loop, expression switch, composite
// literal). It also pins the shapes that are Go's alone and appear in no
// other golden: `interface_type` with method specs, struct field tags,
// `type_parameter_list` on both a generic type and a generic function,
// `variadic_parameter_declaration`, directional `channel_type`s,
// `go_statement`, `defer_statement`, `send_statement`,
// `select_statement` with communication and default arms,
// `type_switch_statement` with multi-type / interface / nil cases,
// `range_clause` over a channel, and `generic_type` instantiation. Any
// grammar bump or `normalise_kind` edit trips this byte-for-byte check.
#[test]
fn debug_ast_dump_matches_committed_golden_go() -> Result<()> {
    assert_ast_golden("ast-golden-go", "Sample.go")
}

// [LANG-CAND-JAVASCRIPT] golden: Sample.jsx proves the plain JavaScript
// grammar's JSX path normalises identically — and pins that JSX text AND
// html_character_reference entities (`&amp;`, `&copy;`) both collapse to
// the literal placeholder rather than leaking entity structure into the
// fingerprint.
#[test]
fn debug_ast_dump_matches_committed_golden_jsx() -> Result<()> {
    assert_ast_golden("ast-golden-jsx", "Sample.jsx")
}

// Implements [PIPELINE-NORMALIZE-AST] unsupported-extension: running
// `--debug-ast` on a file whose extension no parser claims must exit
// non-zero with a clear error, not panic or emit an empty dump.
#[test]
fn debug_ast_rejects_unknown_extension() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let path = tmp.path().join("sample.unknown");
    fs::write(&path, "// not a supported language\n")?;
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg("--debug-ast")
        .arg(&path)
        .assert()
        .failure()
        .stderr(contains("no language parser matches extension"));
    Ok(())
}

// Implements [PIPELINE-NORMALIZE-AST] help-text exposure: the
// `--debug-ast` developer flag must be documented.
#[test]
fn help_text_documents_debug_ast_flag() -> Result<()> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("--debug-ast"));
    Ok(())
}

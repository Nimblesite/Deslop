use super::support::*;
use deslop_core::lang::shared::OPERATOR_KIND_PREFIX;
use std::fmt::Write as _;

/// Runs a default (cache-on, [PIPELINE-INCREMENTAL]) pass over
/// `scan_root`, writing `<prefix>.json` (and siblings), asserts the
/// process succeeded, and returns the JSON report body as a string.
/// Centralises the run-and-read shape shared by the cache tests.
fn run_incremental_pass(scan_root: &Path, output_prefix: &Path) -> Result<String> {
    let mut cmd = deslop_command(scan_root, output_prefix)?;
    let _assertion = cmd.args(["--min-nodes", "8"]).assert().success();
    Ok(fs::read_to_string(with_ext(output_prefix, "json"))?)
}

#[test]
fn output_path_with_missing_parent_is_created() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let base = tmp.path().join("a").join("b").join("c").join("report");
    let mut cmd = fixture_command("csharp-small", &base)?;
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
    let mut cmd = fixture_command("csharp-small", &tmp.path().join("report"))?;
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
    // clustering sees identical results. The mass-only wire proves the
    // Type-2 cluster by its presence with the mass fields and both
    // occurrences ([RANK-MASS-SUM]); the retired `structural: 1.0`
    // cluster signal no longer exists.
    assert!(
        second_json.contains("\"mass\":"),
        "cached run must still detect the Type-2 cluster: {second_json}"
    );
    assert!(
        second_json.contains("\"Alpha.cs\"") && second_json.contains("\"Beta.cs\""),
        "cached run must report both copies of the Type-2 cluster: {second_json}"
    );
    let second_txt = fs::read_to_string(tmp.path().join("second.txt"))?;
    assert!(
        second_txt.contains("cache: 2 hit / 0 miss"),
        "text renderer must surface cache stats: {second_txt}"
    );
    Ok(())
}

// Implements [PIPELINE-INCREMENTAL] default-on: incremental analysis is
// the first-class path, so a bare run populates the cache. Stats show
// the cache was consulted (every file a miss on a cold tree) and blobs
// land under `.deslop/cache/fingerprints/`.
#[test]
fn default_run_uses_the_cache() -> Result<()> {
    let (tmp, scan_root, _out) =
        seeded_scan("src", |root| seed_scan_root(&fixture("csharp-small"), root))?;
    run_scan(
        &scan_root,
        &tmp.path().join("report"),
        &[MIN_NODES_FLAG, MIN_NODES_VALUE],
    )?;
    let json = report_json_text(&tmp)?;
    assert!(
        json.contains("\"misses\": 2"),
        "a bare run must consult the cache and miss on a cold tree: {json}"
    );
    assert!(
        scan_root
            .join(".deslop/cache")
            .join("fingerprints")
            .is_dir(),
        "a bare run must populate the fingerprint cache",
    );
    Ok(())
}

// Implements [PIPELINE-INCREMENTAL] opt-out: `--no-incremental` leaves
// the cache neither read nor written. Stats read as a clean no-cache
// run (both counters zero) and no blobs land on disk, so a caller who
// must not mutate the tree has an explicit way to say so.
#[test]
fn no_incremental_flag_skips_the_cache() -> Result<()> {
    let (tmp, scan_root, _out) =
        seeded_scan("src", |root| seed_scan_root(&fixture("csharp-small"), root))?;
    run_scan(
        &scan_root,
        &tmp.path().join("report"),
        &[MIN_NODES_FLAG, MIN_NODES_VALUE, "--no-incremental"],
    )?;
    let json = report_json_text(&tmp)?;
    assert!(
        json.contains("\"hits\": 0"),
        "--no-incremental must record zero hits: {json}"
    );
    assert!(
        json.contains("\"misses\": 0"),
        "--no-incremental must not increment misses either: {json}"
    );
    assert!(
        !scan_root
            .join(".deslop/cache")
            .join("fingerprints")
            .exists(),
        "--no-incremental must not populate the fingerprint cache",
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
        second_json.contains("\"mass\":") && second_json.contains("\"Alpha.cs\""),
        "analysis still produces the Type-2 cluster after recovery: {second_json}"
    );
    Ok(())
}

// Implements [PIPELINE-INCREMENTAL] help-text exposure: the
// `--no-incremental` opt-out must be documented so users can discover
// how to turn the cache off without reading the source.
#[test]
fn help_text_documents_incremental_flag() -> Result<()> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("--no-incremental"));
    Ok(())
}

// Implements [PIPELINE-INCREMENTAL] content-addressed invalidation —
// the property that makes a cache-on-by-default CLI safe. The cache key
// *is* the file's content hash, so a file edited while nothing was
// watching is unaddressable in the old entry and must be re-parsed,
// while its untouched neighbour still hits. Corpus membership always
// comes from a fresh discovery walk, so the run can never serve a
// snapshot of a tree that no longer exists on disk.
#[test]
fn offline_edit_invalidates_only_the_changed_file() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    seed_scan_root(&fixture("csharp-small"), &scan_root)?;
    let _cold = run_incremental_pass(&scan_root, &tmp.path().join("cold"))?;
    let warm = run_incremental_pass(&scan_root, &tmp.path().join("warm"))?;
    assert!(
        warm.contains("\"hits\": 2") && warm.contains("\"misses\": 0"),
        "an unchanged tree must hit for every file: {warm}"
    );
    // Edit one file with no watcher running, exactly as an agent or a
    // `git checkout` would while the LSP is stopped.
    let edited = scan_root.join("Alpha.cs");
    let source = fs::read_to_string(&edited)?;
    fs::write(
        &edited,
        format!("{source}\n// edited with nothing watching\n"),
    )?;
    let after = run_incremental_pass(&scan_root, &tmp.path().join("after"))?;
    assert!(
        after.contains("\"misses\": 1"),
        "the edited file must miss and be re-parsed from disk: {after}"
    );
    assert!(
        after.contains("\"hits\": 1"),
        "the untouched file must still hit: {after}"
    );
    // A run against a wiped cache must agree with the warm run — the
    // cache is an accelerator, never a source of truth.
    fs::remove_dir_all(scan_root.join(".deslop").join("cache"))?;
    let rebuilt = run_incremental_pass(&scan_root, &tmp.path().join("rebuilt"))?;
    assert!(
        rebuilt.contains("\"misses\": 2") && rebuilt.contains("\"hits\": 0"),
        "a wiped cache must re-parse everything: {rebuilt}"
    );
    assert_eq!(
        cluster_count(&rebuilt)?,
        cluster_count(&after)?,
        "cold and warm runs must produce the same clusters",
    );
    Ok(())
}

/// Number of clusters in a rendered JSON report body.
fn cluster_count(json: &str) -> Result<usize> {
    Ok(serde_json::from_str::<Value>(json)?
        .get("clusters")
        .and_then(Value::as_array)
        .map_or(0, Vec::len))
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
    let _assertion = cmd.args(["--min-nodes", "8"]).assert().success();
    let mut restore = fs::metadata(&locked_dir)?.permissions();
    restore.set_mode(0o755);
    fs::set_permissions(&locked_dir, restore)?;
    let json = report_json_text(&tmp)?;
    assert!(
        json.contains("\"files_analysed\": 2"),
        "pipeline must still report both files: {json}"
    );
    Ok(())
}

// Perf regression guard [PERF-BUDGET-TYPE12]. The user-facing
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
    let json = report_json_text(&tmp)?;
    assert!(
        json.contains("\"files_analysed\": 10"),
        "synthetic corpus must analyse every generated file: {json}"
    );
    // Every file shares the identical method template, so the
    // ranked output must contain at least one cluster — catches
    // pipelines that silently drop everything. The mass-only wire
    // carries the mass fields instead of the retired `weight`.
    assert!(
        json.contains("\"mass\":") && json.contains("\"rank\":"),
        "synthetic corpus produced no clusters: {json}"
    );
    Ok(())
}

// Implements the fixture-per-bug workflow from
// `.claude/skills/fix-bug/SKILL.md`: every bug reproduced into
// `tests/fixtures/bug-*/` becomes a permanent e2e test. This is the
// seed example — an empty C# class body used to be silently dropped
// before the sibling-window fingerprint pass existed; the assertion
// below pins that behaviour so the bug cannot regress.
#[test]
fn bug_fixture_walks_trivial_class_body_without_panicking() -> Result<()> {
    let (tmp, mut cmd) = fixture_run_command("bug-empty-class")?;
    let _assertion = cmd.args(["--min-nodes", "4"]).assert().success();
    let json = report_json_text(&tmp)?;
    assert!(
        json.contains("\"files_analysed\": 1"),
        "empty-class fixture must still analyse its one file: {json}"
    );
    Ok(())
}

// Implements [PIPELINE-NORMALIZE-AST] golden guard, in two halves.
//
// **Unchanged**: `--debug-ast` on a hand-picked per-language fixture
// must match the committed expected dump byte-for-byte. Any drift in
// the grammar version, the `normalise_kind` match arms, or the
// child-ordering policy trips this — which is exactly what we want,
// because any of those changes silently alters the fingerprint and
// invalidates every user's cache.
//
// **Correct**: the committed dump must also satisfy the normalisation
// contract on its own terms (`assert_dump_is_correct`). Equality alone
// only proves the tool still agrees with a file the tool wrote, so a
// wrong expectation is self-certifying: every one of these goldens
// recorded `__file__` spanning trivia the normaliser had already
// dropped — 759 bytes of comments in Go, 52 in F#, the trailing
// newline in all eleven — and the byte-for-byte check called it
// expected for as long as the fixtures existed. Regenerating a golden
// is therefore never the remedy on its own; the new dump has to be
// shown correct, and these invariants are what show it.
//
// Each fixture exercises identifier collapse, literal collapse,
// comment drop, and the language-specific structural forms most
// likely to shift between grammar patch releases.
//
// See `crates/deslop/tests/fixtures/AST-GOLDEN-README.md` before
// regenerating any of these files.
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
        "AST dump drifted from {}. Regenerating is NOT the default remedy — \
         prove the new dump satisfies the contract first; the committed file \
         is only a golden while it is correct.\n\
         If the new dump IS correct, the normalised tree has changed meaning \
         while the parse store's blob layout has not, so **bump \
         `fpcache::blob::SEMANTIC_EPOCH`** in the same change \
         ([PIPELINE-INCREMENTAL-INTEGRITY]). Blobs are addressed by \
         `(language, tool_version, min_nodes, source_hash)` and the workspace \
         version is the permanently-reused `0.0.0-dev`, so without that bump \
         every already-stored tree stays addressable and a warm run serves \
         the pre-change normalisation — the one way a warm report can differ \
         from the cold report of the same tree.",
        expected_path.display(),
    );
    assert_dump_is_correct(&expected, &fs::read(&source)?, fixture_dir);
    Ok(())
}

/// One line of a `--debug-ast` dump: `<indent><kind> [start..end]`.
struct DumpNode {
    depth: usize,
    kind: String,
    start: u64,
    end: u64,
}

/// [PIPELINE-NORMALIZE-AST] Asserts the committed dump is *correct*, not
/// merely unchanged.
///
/// Byte-for-byte equality alone cannot say a golden is right: regenerating
/// the file promotes whatever the current build emits to "expected". That
/// is precisely how `__file__` came to claim 759 bytes of dropped Go
/// comments and 52 of F# — wrong in the tree for as long as the fixtures
/// existed, and invisible because the only check compared the tool against
/// a file the tool wrote. These invariants come from the contract instead,
/// so a regenerated dump that re-admits trivia fails here even though it
/// matches the committed bytes exactly.
fn assert_dump_is_correct(dump: &str, source: &[u8], label: &str) {
    let source_len = u64::try_from(source.len()).unwrap_or(u64::MAX);
    let nodes: Vec<DumpNode> = dump.lines().filter_map(parse_dump_line).collect();
    assert!(!nodes.is_empty(), "{label}: dump has no nodes");
    for node in &nodes {
        assert!(
            node.start < node.end && node.end <= source_len,
            "{label}: {} [{}..{}] is not a valid range over {source_len} bytes",
            node.kind,
            node.start,
            node.end,
        );
        assert!(
            !node.kind.contains("comment"),
            "{label}: comment node {} survived normalisation",
            node.kind,
        );
    }
    assert_root_spans_retained_children(&nodes, label);
    assert_ranges_nest(&nodes, label);
    assert_operators_carry_their_token(&nodes, source, label);
}

/// [PIPELINE-NORMALIZE-AST-OPERATOR] Every operator leaf must be named
/// by the token it stands for, and its name must be the bytes it spans.
///
/// This is what stops the golden from being self-certifying on the one
/// axis that matters most here. A dump full of a shared `__op__`
/// placeholder is byte-for-byte stable and completely wrong: it records
/// a tree in which `alpha + beta` and `alpha - beta` are the same
/// subtree, and regenerating the file would promote that to "expected"
/// exactly as it once promoted the dropped Go comments. Reading the
/// name back out of the source proves the leaf discriminates, and it
/// proves it against the fixture rather than against the tool.
fn assert_operators_carry_their_token(nodes: &[DumpNode], source: &[u8], label: &str) {
    let operators = nodes
        .iter()
        .filter(|node| node.kind.starts_with(OPERATOR_KIND_PREFIX));
    for node in operators {
        let range = usize::try_from(node.start)
            .ok()
            .zip(usize::try_from(node.end).ok());
        let spanned = range
            .and_then(|(start, end)| source.get(start..end))
            .map(String::from_utf8_lossy)
            .unwrap_or_default();
        assert_eq!(
            node.kind,
            format!("{OPERATOR_KIND_PREFIX}{spanned}"),
            "{label}: operator leaf `{}` at [{}..{}] spans `{spanned}`. An \
             operator leaf named anything but its own token cannot tell `+` \
             from `-`, and every signal taken from the digest inherits that",
            node.kind,
            node.start,
            node.end,
        );
    }
}

/// Splits `<indent><kind> [start..end]`; indent is two spaces per level.
/// Returns `None` for a blank or malformed line so the caller's other
/// invariants still run over the lines that did parse.
fn parse_dump_line(line: &str) -> Option<DumpNode> {
    let body = line.trim_start_matches(' ');
    let open = body.rfind(" [")?;
    let span = body.get(open.saturating_add(2)..)?;
    let (start, end) = span.trim_end_matches(']').split_once("..")?;
    Some(DumpNode {
        depth: line.len().saturating_sub(body.len()) / 2,
        kind: body[..open].to_owned(),
        start: start.parse().ok()?,
        end: end.parse().ok()?,
    })
}

/// The synthetic root must span exactly what normalisation kept. Tree-sitter's
/// parse root also covers leading and trailing trivia the normaliser dropped,
/// so inheriting it reports bytes contributing zero nodes to any match.
fn assert_root_spans_retained_children(nodes: &[DumpNode], label: &str) {
    let Some(root) = nodes.first() else { return };
    assert_eq!(root.kind, "__file__", "{label}: root must be __file__");
    let child_depth = root.depth.saturating_add(1);
    let children = nodes.iter().filter(|node| node.depth == child_depth);
    let spans: Vec<(u64, u64)> = children.map(|node| (node.start, node.end)).collect();
    let (Some(start), Some(end)) = (
        spans.iter().map(|span| span.0).min(),
        spans.iter().map(|span| span.1).max(),
    ) else {
        return;
    };
    assert_eq!(
        (root.start, root.end),
        (start, end),
        "{label}: __file__ [{}..{}] must span exactly the retained children \
         [{start}..{end}] — the difference is dropped trivia being reported \
         as duplicated code",
        root.start,
        root.end,
    );
}

/// Every node sits inside its nearest shallower ancestor.
fn assert_ranges_nest(nodes: &[DumpNode], label: &str) {
    let mut ancestors: Vec<&DumpNode> = Vec::new();
    for node in nodes {
        while ancestors.last().is_some_and(|top| top.depth >= node.depth) {
            let _popped = ancestors.pop();
        }
        if let Some(parent) = ancestors.last() {
            assert!(
                node.start >= parent.start && node.end <= parent.end,
                "{label}: {} [{}..{}] escapes parent {} [{}..{}]",
                node.kind,
                node.start,
                node.end,
                parent.kind,
                parent.start,
                parent.end,
            );
        }
        ancestors.push(node);
    }
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

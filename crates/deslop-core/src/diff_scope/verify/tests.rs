//! Verification tests ([CLI-ARG-DIFF], [PIPELINE-DIFF-INGEST]): the
//! byte-check against the corpus, the fate of corpus misses, and the
//! git-copy projection.

use anyhow::{Context as _, Result};

use super::{super::parse_unified_diff, *};

/// Builds an in-memory corpus from `(relative path, bytes)` pairs.
fn corpus(entries: &[(&str, &'static [u8])]) -> BTreeMap<PathBuf, &'static [u8]> {
    entries
        .iter()
        .map(|(path, bytes)| (PathBuf::from(path), *bytes))
        .collect()
}

/// Parses `text` and builds the scope over `corpus` with `/repo` as
/// both working directory and scan root.
fn scope_at_repo(
    text: &str,
    corpus: &BTreeMap<PathBuf, &[u8]>,
) -> Result<DiffScope, CoreError> {
    let parsed = parse_unified_diff(text)?;
    build_diff_scope(&parsed, Path::new("/repo"), Path::new("/repo"), corpus)
}

/// Unwraps a refusal into its `DiffStale` path and line.
fn stale(result: Result<DiffScope, CoreError>, why: &str) -> Result<(PathBuf, u64)> {
    let error = result.err().with_context(|| format!("must be refused: {why}"))?;
    let CoreError::DiffStale { path, line } = error else {
        anyhow::bail!("{why}: expected DiffStale, got {error:?}");
    };
    Ok((path, line))
}

// [CLI-ARG-DIFF] verification: matching context + added lines
// project the added line numbers into the scope.
#[test]
fn matching_diff_projects_added_lines() -> Result<()> {
    let text = "--- a/src/x.rs\n\
                +++ b/src/x.rs\n\
                @@ -1,1 +1,3 @@\n \
                fn keep() {}\n\
                +fn one() {}\n\
                +fn two() {}\n";
    let corpus = corpus(&[("src/x.rs", b"fn keep() {}\nfn one() {}\nfn two() {}\n")]);
    let scope = scope_at_repo(text, &corpus).context("clean diff verifies")?;
    assert_eq!(scope.added_line_total(), 2);
    assert!(scope.contains(Path::new("src/x.rs"), 2));
    assert!(scope.contains(Path::new("src/x.rs"), 3));
    assert!(!scope.contains(Path::new("src/x.rs"), 1));
    Ok(())
}

// [CLI-ARG-DIFF] verification: a context line that disagrees with
// the corpus is refused with the file and new-side line.
#[test]
fn stale_context_line_names_file_and_line() -> Result<()> {
    let text = "--- a/src/x.rs\n\
                +++ b/src/x.rs\n\
                @@ -1,1 +1,2 @@\n \
                fn old_shape() {}\n\
                +fn added() {}\n";
    let corpus = corpus(&[("src/x.rs", b"fn keep() {}\nfn added() {}\n")]);
    let (path, line) = stale(scope_at_repo(text, &corpus), "stale context")?;
    assert_eq!(path, PathBuf::from("src/x.rs"));
    assert_eq!(line, 1);
    Ok(())
}

// [CLI-ARG-DIFF] verification: files outside the scan root are
// skipped, never verified, never counted.
#[test]
fn out_of_corpus_files_are_ignored() -> Result<()> {
    let text = "--- /dev/null\n+++ b/docs/notes.md\n@@ -0,0 +1 @@\n+# Notes\n";
    let parsed = parse_unified_diff(text).context("diff parses")?;
    let corpus = corpus(&[("src/x.rs", b"fn keep() {}\n")]);
    let scope = build_diff_scope(&parsed, Path::new("/repo"), Path::new("/repo/src"), &corpus)
        .context("out-of-root file is skipped, not an error")?;
    assert_eq!(scope.added_line_total(), 0);
    assert_eq!(scope.files_with_added_lines(), 0);
    Ok(())
}

// [CLI-ARG-DIFF] verification: CRLF sources verify byte-exactly
// when the diff payload carries the same `\r`.
#[test]
fn crlf_source_verifies_byte_exactly() -> Result<()> {
    let text = "--- /dev/null\n+++ b/win.cs\n@@ -0,0 +1 @@\n+var x = 1;\r\n";
    let corpus = corpus(&[("win.cs", b"var x = 1;\r\n")]);
    let scope = scope_at_repo(text, &corpus).context("CRLF content matches CRLF source")?;
    assert_eq!(scope.added_line_total(), 1);
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-2: a diff target inside the scan root,
// with a language extension the registry supports, that exists neither
// in the corpus nor on disk is a stale diff — the tree has moved past
// it. Ignoring it silently zeroes the scope of a merge gate, so the
// refusal must name the path and the first claimed new-side line.
#[test]
fn missing_supported_target_in_root_is_refused_as_stale() -> Result<()> {
    let text = "diff --git a/src/missing.rs b/src/missing.rs\n\
                new file mode 100644\n\
                --- /dev/null\n\
                +++ b/src/missing.rs\n\
                @@ -0,0 +1 @@\n\
                +pub fn ghost() {}\n";
    let corpus = corpus(&[("src/x.rs", b"fn keep() {}\n")]);
    let (path, line) = stale(scope_at_repo(text, &corpus), "missing supported target")?;
    assert_eq!(path, PathBuf::from("src/missing.rs"));
    assert_eq!(line, 1, "the first claimed new-side line");
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-2 contrast: a missing target whose
// extension no registered parser claims could never be analysed, so
// its absence proves nothing about staleness — it stays ignorable.
#[test]
fn missing_unsupported_target_in_root_stays_ignored() -> Result<()> {
    let text = "--- /dev/null\n+++ b/notes.md\n@@ -0,0 +1 @@\n+# Notes\n";
    let corpus = corpus(&[("src/x.rs", b"fn keep() {}\n")]);
    let scope = scope_at_repo(text, &corpus).context("unsupported extension is skipped")?;
    assert_eq!(scope.added_line_total(), 0);
    assert_eq!(scope.files_with_added_lines(), 0);
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-2 contrast: a supported file present on
// disk but absent from the corpus was deliberately excluded
// (gitignore / config exclusion) — the diff is not stale, the file is
// just out of the analysed population. It stays ignorable.
#[test]
fn corpus_miss_present_on_disk_is_ignored_as_deliberately_excluded() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let root = std::fs::canonicalize(tmp.path())?;
    std::fs::create_dir_all(root.join("src"))?;
    std::fs::write(root.join("src/excluded.rs"), "pub fn excluded() {}\n")?;
    let text = "--- /dev/null\n+++ b/src/excluded.rs\n@@ -0,0 +1 @@\n+pub fn excluded() {}\n";
    let parsed = parse_unified_diff(text).context("diff parses")?;
    let corpus = corpus(&[("src/other.rs", b"fn keep() {}\n")]);
    let scope = build_diff_scope(&parsed, &root, &root, &corpus)
        .context("excluded-but-present file is skipped, not an error")?;
    assert_eq!(scope.added_line_total(), 0);
    assert_eq!(scope.files_with_added_lines(), 0);
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-2 contrast: a section that claims no
// new-side lines (a removal-only `-U0` hunk) asserts nothing the
// verifier can check against a missing file, and adds nothing to the
// scope — it stays ignorable rather than becoming a refusal.
#[test]
fn removal_only_hunk_for_a_missing_file_stays_ignored() -> Result<()> {
    let text = "--- a/src/missing.rs\n+++ b/src/missing.rs\n@@ -1,2 +1,0 @@\n-a\n-b\n";
    let corpus = corpus(&[("src/x.rs", b"fn keep() {}\n")]);
    let scope = scope_at_repo(text, &corpus).context("removal-only section is skipped")?;
    assert_eq!(scope.added_line_total(), 0);
    Ok(())
}

/// The metadata-only copy diff over `src/a.rs` → `src/b.rs`.
const METADATA_ONLY_COPY: &str = "diff --git a/src/a.rs b/src/b.rs\n\
    similarity index 100%\n\
    copy from src/a.rs\n\
    copy to src/b.rs\n";

// [PIPELINE-DIFF-INGEST] P0-3: a metadata-only 100%-similarity copy is
// a wholesale file duplication. Every line of the verified target is
// added by the change ([METRICS-DIFF-SCOPE] `added_loc`); the source
// file stays untouched and out of the scope.
#[test]
fn metadata_only_copy_marks_every_target_line_added() -> Result<()> {
    let corpus = corpus(&[
        ("src/a.rs", b"alpha\nbeta\ngamma\n"),
        ("src/b.rs", b"alpha\nbeta\ngamma\n"),
    ]);
    let scope = scope_at_repo(METADATA_ONLY_COPY, &corpus).context("clean copy verifies")?;
    assert_eq!(scope.added_line_total(), 3, "every target line is added");
    assert_eq!(scope.files_with_added_lines(), 1, "only the copy target");
    for line in 1..=3 {
        assert!(scope.contains(Path::new("src/b.rs"), line), "line {line}");
    }
    assert!(!scope.contains(Path::new("src/a.rs"), 1), "source stays existing");
    assert!(!scope.contains(Path::new("src/b.rs"), 4), "no phantom lines");
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-3: `similarity index 100%` with no hunks
// asserts the target byte-equals the source. A divergence means the
// tree moved past the diff — refused naming the target and its first
// divergent line, never trusted.
#[test]
fn metadata_only_copy_divergence_is_refused_at_first_divergent_line() -> Result<()> {
    let corpus = corpus(&[
        ("src/a.rs", b"alpha\nbeta\ngamma\n"),
        ("src/b.rs", b"alpha\nCHANGED\ngamma\n"),
    ]);
    let (path, line) = stale(scope_at_repo(METADATA_ONLY_COPY, &corpus), "divergent copy")?;
    assert_eq!(path, PathBuf::from("src/b.rs"));
    assert_eq!(line, 2, "the first divergent line");
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-3: a copy whose target is nowhere in the
// tree is a stale diff — the wholesale duplication the diff describes
// cannot be verified, and ignoring it would hide exactly the event
// this tool exists to catch.
#[test]
fn copy_target_missing_from_tree_is_refused() -> Result<()> {
    let text = "diff --git a/src/a.rs b/src/missing.rs\n\
                similarity index 100%\n\
                copy from src/a.rs\n\
                copy to src/missing.rs\n";
    let corpus = corpus(&[("src/a.rs", b"alpha\n")]);
    let (path, line) = stale(scope_at_repo(text, &corpus), "missing copy target")?;
    assert_eq!(path, PathBuf::from("src/missing.rs"));
    assert_eq!(line, 1);
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-3 contrast: a copy target outside the scan
// root or with an unsupported extension is invisible to the analysis
// either way — both stay ignorable, like every other such target.
#[test]
fn copy_target_out_of_root_or_unsupported_stays_ignored() -> Result<()> {
    let out_of_root = "diff --git a/docs/a.md b/docs/b.md\n\
                       similarity index 100%\n\
                       copy from docs/a.md\n\
                       copy to docs/b.md\n";
    let parsed = parse_unified_diff(out_of_root).context("copy diff parses")?;
    let corpus = corpus(&[("x.rs", b"fn keep() {}\n")]);
    let scope = build_diff_scope(&parsed, Path::new("/repo"), Path::new("/repo/src"), &corpus)
        .context("out-of-root copy target is skipped")?;
    assert_eq!(scope.added_line_total(), 0);
    let unsupported = "diff --git a/a.md b/b.md\n\
                       similarity index 100%\n\
                       copy from a.md\n\
                       copy to b.md\n";
    let scope = scope_at_repo(unsupported, &corpus)
        .context("unsupported-extension copy target is skipped")?;
    assert_eq!(scope.added_line_total(), 0);
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-3: the copy's source must also resolve
// against the tree — a source that exists nowhere means the diff
// describes a different revision.
#[test]
fn copy_source_missing_everywhere_is_refused() -> Result<()> {
    let text = "diff --git a/src/ghost.rs b/src/b.rs\n\
                similarity index 100%\n\
                copy from src/ghost.rs\n\
                copy to src/b.rs\n";
    let corpus = corpus(&[("src/b.rs", b"alpha\n")]);
    let (path, _line) = stale(scope_at_repo(text, &corpus), "missing copy source")?;
    assert_eq!(path, PathBuf::from("src/ghost.rs"));
    Ok(())
}

/// The sub-100% copy diff: `src/b.rs` is `src/a.rs` with line 2
/// changed, and the hunk describes that delta.
const COPY_WITH_HUNKS: &str = "diff --git a/src/a.rs b/src/b.rs\n\
    similarity index 75%\n\
    copy from src/a.rs\n\
    copy to src/b.rs\n\
    index 1111111..2222222 100644\n\
    --- a/src/a.rs\n\
    +++ b/src/b.rs\n\
    @@ -2 +2 @@\n\
    -old\n\
    +new\n";

// [PIPELINE-DIFF-INGEST] P0-3: below 100% similarity git emits hunks
// describing the delta against the *source* — but the target file did
// not exist before the change, so every one of its lines is still new
// content. The full-range projection must subsume the hunk's added
// lines: the total is the target's line count, once, never the sum of
// both.
#[test]
fn copy_with_hunks_counts_every_target_line_once() -> Result<()> {
    let corpus = corpus(&[
        ("src/a.rs", b"one\nold\nthree\nfour\n"),
        ("src/b.rs", b"one\nnew\nthree\nfour\n"),
    ]);
    let scope = scope_at_repo(COPY_WITH_HUNKS, &corpus).context("copy with hunks verifies")?;
    assert_eq!(scope.added_line_total(), 4, "whole target, counted once");
    assert!(scope.contains(Path::new("src/b.rs"), 1), "line before the hunk");
    assert!(scope.contains(Path::new("src/b.rs"), 4), "line after the hunk");
    assert!(!scope.contains(Path::new("src/a.rs"), 2), "source stays existing");
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-3: a copy hunk that disagrees with the
// analysed target bytes is a stale diff like any other hunk.
#[test]
fn copy_with_stale_hunk_is_refused() -> Result<()> {
    let corpus = corpus(&[
        ("src/a.rs", b"one\nold\nthree\nfour\n"),
        ("src/b.rs", b"one\nDIFFERENT\nthree\nfour\n"),
    ]);
    let (path, line) = stale(scope_at_repo(COPY_WITH_HUNKS, &corpus), "stale copy hunk")?;
    assert_eq!(path, PathBuf::from("src/b.rs"));
    assert_eq!(line, 2);
    Ok(())
}

// [PIPELINE-DIFF-INGEST] deliberate contrast pinned by the plan: a
// pure rename with no content change moves a file — it introduces no
// new lines, so it adds nothing to the scope and tags nothing, while
// the metadata-only *copy* above adds every target line.
#[test]
fn pure_rename_metadata_projects_nothing() -> Result<()> {
    let text = "diff --git a/src/a.rs b/src/renamed.rs\n\
                similarity index 100%\n\
                rename from src/a.rs\n\
                rename to src/renamed.rs\n";
    let corpus = corpus(&[
        ("src/a.rs", b"alpha\nbeta\n"),
        ("src/renamed.rs", b"alpha\nbeta\n"),
    ]);
    let scope = scope_at_repo(text, &corpus).context("pure rename is skipped")?;
    assert_eq!(scope.added_line_total(), 0, "a rename adds no lines");
    assert_eq!(scope.files_with_added_lines(), 0);
    assert!(!scope.contains(Path::new("src/renamed.rs"), 1));
    Ok(())
}

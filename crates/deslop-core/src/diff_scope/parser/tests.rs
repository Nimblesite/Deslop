//! Grammar tests for the unified-diff parser ([CLI-ARG-DIFF],
//! [PIPELINE-DIFF-INGEST]). Copy-metadata and target-line grammar is
//! pinned separately in `copy_tests`.

use anyhow::{Context as _, Result};

use super::*;

/// The single file section every grammar test above parses, or an
/// error saying the parse produced none.
fn first_file(parsed: &ParsedDiff) -> Result<&FilePatch> {
    parsed.files.first().context("the parsed file section")
}

/// The added-line contents of every hunk in `patch`, in order.
fn added_lines(patch: &FilePatch) -> Vec<&str> {
    patch
        .hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .filter(|line| line.kind == HunkLineKind::Added)
        .map(|line| line.content.as_str())
        .collect()
}

// [CLI-ARG-DIFF] grammar: the empty diff is valid and empty.
#[test]
fn empty_input_parses_to_no_files() -> Result<()> {
    let parsed = parse_unified_diff("").context("empty diff must parse")?;
    assert!(parsed.files.is_empty(), "no file sections expected");
    Ok(())
}

// [CLI-ARG-DIFF] grammar: a git-style modification with context,
// removal, and addition round-trips paths, counts, and content.
#[test]
fn git_modification_parses_paths_counts_and_content() -> Result<()> {
    let text = "diff --git a/src/lib.rs b/src/lib.rs\n\
                index 1111111..2222222 100644\n\
                --- a/src/lib.rs\n\
                +++ b/src/lib.rs\n\
                @@ -1,3 +1,3 @@\n \
                fn keep() {}\n\
                -fn old() {}\n\
                +fn new() {}\n \
                fn tail() {}\n";
    let parsed = parse_unified_diff(text).context("valid diff must parse")?;
    assert_eq!(parsed.files.len(), 1);
    let file = first_file(&parsed)?;
    assert_eq!(file.new_path.as_deref(), Some("src/lib.rs"));
    assert_eq!(file.hunks.len(), 1);
    let hunk = file.hunks.first().context("the only hunk")?;
    assert_eq!(hunk.new_start, 1);
    assert_eq!(hunk.lines.len(), 4);
    assert_eq!(added_lines(file), vec!["fn new() {}"]);
    Ok(())
}

// [PIPELINE-DIFF-INGEST] grammar: a plain (prefix-less) diff carries
// no `diff --git` line, so each `+++` is what starts the next file's
// section — `--- ` is metadata and cannot. Attaching the first
// file's hunks to the second file's path verifies them against the
// wrong file: where both files share the hunk's content (the
// copy-paste this tool exists to find) verification passes and the
// first file silently receives no added spans at all, so under
// `--only-changed` its new duplication is dropped from the report
// and from the `added_loc` denominator.
#[test]
fn plain_multi_file_diff_keeps_each_file_section_separate() -> Result<()> {
    let text = "--- x.rs\n\
                +++ x.rs\n\
                @@ -1,1 +1,2 @@\n \
                fn keep() {}\n\
                +fn from_x() {}\n\
                --- y.rs\n\
                +++ y.rs\n\
                @@ -1,1 +1,2 @@\n \
                fn keep() {}\n\
                +fn from_y() {}\n";
    let parsed = parse_unified_diff(text).context("plain multi-file diff must parse")?;
    assert_eq!(
        parsed.files.len(),
        2,
        "two `+++` targets are two file sections, not one"
    );
    let first = first_file(&parsed)?;
    assert_eq!(first.new_path.as_deref(), Some("x.rs"));
    assert_eq!(first.hunks.len(), 1, "x.rs keeps only its own hunk");
    assert_eq!(added_lines(first), vec!["fn from_x() {}"]);
    let second = parsed.files.get(1).context("the second file section")?;
    assert_eq!(second.new_path.as_deref(), Some("y.rs"));
    assert_eq!(second.hunks.len(), 1, "y.rs keeps only its own hunk");
    assert_eq!(added_lines(second), vec!["fn from_y() {}"]);
    Ok(())
}

// [PIPELINE-DIFF-INGEST] grammar: the spec lists C-quoted paths as
// recognised. `git` quotes any path carrying non-ASCII bytes, a
// quote, or a backslash, escaping the bytes in octal — so a repo
// with a single accented filename produces them on every diff. Left
// quoted, the path matches nothing in the corpus and the file is
// counted as merely *ignored*: no error, and every clone added in it
// is untagged, dropped by `--only-changed`, and absent from
// `added_loc`.
#[test]
fn c_quoted_new_side_path_is_unquoted() -> Result<()> {
    let text = "--- \"a/caf\\303\\251.rs\"\n\
                +++ \"b/caf\\303\\251.rs\"\n\
                @@ -0,0 +1 @@\n\
                +fn added() {}\n";
    let parsed = parse_unified_diff(text).context("quoted-path diff must parse")?;
    assert_eq!(
        first_file(&parsed)?.new_path.as_deref(),
        Some("café.rs"),
        "the octal-escaped UTF-8 bytes are the real filename"
    );
    Ok(())
}

// [PIPELINE-DIFF-INGEST] grammar: the non-octal escapes decode to
// the bytes they denote — a tab, a quote, or a backslash in a
// filename is payload, and `git` escapes it precisely so the path
// survives the tab-splitting and quote-stripping around it.
#[test]
fn c_quoted_simple_escapes_decode_to_their_bytes() -> Result<()> {
    let tabbed = "+++ \"b/tab\\there.rs\"\n@@ -0,0 +1 @@\n+fn added() {}\n";
    let parsed = parse_unified_diff(tabbed).context("escaped-tab path must parse")?;
    assert_eq!(
        first_file(&parsed)?.new_path.as_deref(),
        Some("tab\there.rs"),
        "the escaped tab is a payload byte, not a timestamp separator"
    );
    let quoted = "+++ \"b/say \\\"hi\\\"\\\\now.rs\"\n@@ -0,0 +1 @@\n+fn added() {}\n";
    let parsed = parse_unified_diff(quoted).context("escaped-quote path must parse")?;
    assert_eq!(
        first_file(&parsed)?.new_path.as_deref(),
        Some("say \"hi\"\\now.rs"),
        "escaped quotes and backslashes are payload, not structure"
    );
    Ok(())
}

// [PIPELINE-DIFF-INGEST] grammar: a malformed C-quoted path is
// refused with its line number, never guessed into a filename — a
// guessed path matches nothing in the corpus and its file is
// silently dropped from the scope.
#[test]
fn malformed_c_quoted_paths_are_refused() -> Result<()> {
    let cases = [
        ("+++ \"b/x.rs\n@@ -0,0 +1 @@\n+x\n", "missing closing quote"),
        ("+++ \"b/x\\q.rs\"\n@@ -0,0 +1 @@\n+x\n", "unknown escape"),
        (
            "+++ \"b/x\\777.rs\"\n@@ -0,0 +1 @@\n+x\n",
            "octal past one byte",
        ),
        ("+++ \"b/x\\377.rs\"\n@@ -0,0 +1 @@\n+x\n", "invalid UTF-8"),
        (
            "+++ \"b/x.rs\\\"\n@@ -0,0 +1 @@\n+x\n",
            "escape eats the closing quote",
        ),
    ];
    for (text, why) in cases {
        let error = parse_unified_diff(text)
            .err()
            .with_context(|| format!("must be refused: {why}"))?;
        assert!(
            matches!(error, CoreError::DiffParse { line: 1, .. }),
            "{why}: the refusal names the +++ line, got {error:?}"
        );
    }
    Ok(())
}

// [CLI-ARG-DIFF] grammar: renames carry metadata lines and the
// `+++` path wins as the new-side identity.
#[test]
fn rename_uses_the_new_side_path() -> Result<()> {
    let text = "diff --git a/old_name.rs b/new_name.rs\n\
                similarity index 95%\n\
                rename from old_name.rs\n\
                rename to new_name.rs\n\
                --- a/old_name.rs\n\
                +++ b/new_name.rs\n\
                @@ -1 +1 @@\n\
                -fn a() {}\n\
                +fn b() {}\n";
    let parsed = parse_unified_diff(text).context("rename diff must parse")?;
    assert_eq!(parsed.files.len(), 1);
    let file = first_file(&parsed)?;
    assert_eq!(file.new_path.as_deref(), Some("new_name.rs"));
    assert_eq!(added_lines(file), vec!["fn b() {}"]);
    Ok(())
}

// [CLI-ARG-DIFF] grammar: CRLF payloads keep their `\r` so the
// verifier can byte-match CRLF sources.
#[test]
fn crlf_payload_retains_carriage_returns() -> Result<()> {
    let text = "--- a/win.cs\r\n+++ b/win.cs\r\n@@ -0,0 +1 @@\r\n+var x = 1;\r\n";
    let parsed = parse_unified_diff(text).context("CRLF diff must parse")?;
    assert_eq!(added_lines(first_file(&parsed)?), vec!["var x = 1;\r"]);
    Ok(())
}

// [CLI-ARG-DIFF] grammar: the no-trailing-newline annotation is
// consumed without counting toward either side.
#[test]
fn no_newline_marker_does_not_count_as_a_body_line() -> Result<()> {
    let text = "--- a/x.rs\n\
                +++ b/x.rs\n\
                @@ -1 +1 @@\n\
                -old\n\
                +new\n\
                \\ No newline at end of file\n";
    let parsed = parse_unified_diff(text).context("marker diff must parse")?;
    assert_eq!(added_lines(first_file(&parsed)?), vec!["new"]);
    Ok(())
}

// [CLI-ARG-DIFF] grammar: the marker trails the last body line of a
// hunk, so it arrives after the declared counts are satisfied and
// the hunk has closed. The file section it belongs to must survive
// it, and so must the next one.
#[test]
fn no_newline_marker_after_a_closed_hunk_does_not_end_the_diff() -> Result<()> {
    let text = "diff --git a/a.rs b/a.rs\n\
                --- a/a.rs\n\
                +++ b/a.rs\n\
                @@ -1 +1 @@\n\
                -old\n\
                +new\n\
                \\ No newline at end of file\n\
                diff --git a/b.rs b/b.rs\n\
                --- a/b.rs\n\
                +++ b/b.rs\n\
                @@ -0,0 +1 @@\n\
                +second\n";
    let parsed = parse_unified_diff(text).context("marker between sections must parse")?;
    assert_eq!(parsed.files.len(), 2, "the marker ends neither section");
    assert_eq!(added_lines(first_file(&parsed)?), vec!["new"]);
    let second = parsed.files.get(1).context("the second file section")?;
    assert_eq!(second.new_path.as_deref(), Some("b.rs"));
    assert_eq!(added_lines(second), vec!["second"]);
    Ok(())
}

// [CLI-ARG-DIFF] grammar: the marker describes the terminator of the
// line above it, so it must never consume a hunk count — counting it
// would shift every new-side line number after it and mis-tag the
// occurrences those numbers address ([OUTPUT-SCHEMA-DIFF-TAGS]).
#[test]
fn no_newline_marker_does_not_shift_new_side_line_numbers() -> Result<()> {
    let text = "--- a/x.rs\n\
                +++ b/x.rs\n\
                @@ -1,2 +1,3 @@\n \
                keep\n\
                -old\n\
                \\ No newline at end of file\n\
                +one\n\
                +two\n";
    let parsed = parse_unified_diff(text).context("mid-hunk marker must parse")?;
    let file = first_file(&parsed)?;
    let hunk = file.hunks.first().context("the only hunk")?;
    assert_eq!(
        hunk.lines.len(),
        4,
        "one context, one removal, two additions — the marker is not a body line"
    );
    assert_eq!(added_lines(file), vec!["one", "two"]);
    Ok(())
}

// [CLI-ARG-DIFF] grammar: strictness is preserved — a marker with no
// file section above it is junk like any other unrecognised line.
#[test]
fn stray_no_newline_marker_is_refused() -> Result<()> {
    let error = parse_unified_diff("\\ No newline at end of file\n")
        .err()
        .context("a marker before any file header must be refused")?;
    assert!(matches!(error, CoreError::DiffParse { line: 1, .. }));
    Ok(())
}

// [CLI-ARG-DIFF] grammar: binary entries contribute no hunks.
#[test]
fn binary_entry_parses_with_no_hunks() -> Result<()> {
    let text = "diff --git a/logo.png b/logo.png\n\
                index 1111111..2222222 100644\n\
                Binary files a/logo.png and b/logo.png differ\n";
    let parsed = parse_unified_diff(text).context("binary diff must parse")?;
    assert_eq!(parsed.files.len(), 1);
    assert!(first_file(&parsed)?.hunks.is_empty());
    Ok(())
}

// [CLI-ARG-DIFF] grammar: deletions resolve to `new_path == None`.
#[test]
fn deletion_has_no_new_side_path() -> Result<()> {
    let text = "diff --git a/gone.rs b/gone.rs\n\
                deleted file mode 100644\n\
                --- a/gone.rs\n\
                +++ /dev/null\n\
                @@ -1 +0,0 @@\n\
                -fn gone() {}\n";
    let parsed = parse_unified_diff(text).context("deletion diff must parse")?;
    assert_eq!(first_file(&parsed)?.new_path, None);
    Ok(())
}

// [CLI-ARG-DIFF] grammar: an unmarked hunk-body line is refused
// with the offending diff line number.
#[test]
fn unmarked_hunk_body_line_is_refused_with_its_line_number() -> Result<()> {
    let text = "--- a/x.rs\n+++ b/x.rs\n@@ -1,2 +1,2 @@\n context\nxoops\n";
    let error = parse_unified_diff(text)
        .err()
        .context("junk body line must be refused")?;
    let CoreError::DiffParse { line, .. } = error else {
        anyhow::bail!("expected DiffParse, got {error:?}");
    };
    assert_eq!(line, 5, "the junk sits on diff line 5");
    Ok(())
}

// [CLI-ARG-DIFF] grammar: a body longer than its header counts is
// refused rather than silently absorbed.
#[test]
fn hunk_body_exceeding_declared_counts_is_refused() -> Result<()> {
    let text = "--- a/x.rs\n+++ b/x.rs\n@@ -1,1 +1,1 @@\n keep\n+extra\n";
    let error = parse_unified_diff(text)
        .err()
        .context("over-long hunk must be refused")?;
    assert!(matches!(error, CoreError::DiffParse { .. }));
    Ok(())
}

// [CLI-ARG-DIFF] grammar: a truncated trailing hunk is refused.
#[test]
fn truncated_trailing_hunk_is_refused() -> Result<()> {
    let text = "--- a/x.rs\n+++ b/x.rs\n@@ -1,2 +1,2 @@\n only-one\n";
    let error = parse_unified_diff(text)
        .err()
        .context("truncated hunk must be refused")?;
    assert!(matches!(error, CoreError::DiffParse { .. }));
    Ok(())
}

// [PIPELINE-DIFF-INGEST] grammar: new-side line numbers are
// 1-indexed, so `+0` with a non-zero count describes lines that
// cannot exist. Absorbing it shifts the whole added span one line up
// — `verify_line` saturates `new_line - 1` at zero, so lines 0 and 1
// both read the first line — and the real trailing added line then
// falls outside the scope, tagging its occurrences `in_diff: false`
// and letting `--only-changed` pass a change it should fail.
#[test]
fn zero_new_side_start_with_added_lines_is_refused() -> Result<()> {
    let text = "--- a/x.rs\n+++ b/x.rs\n@@ -0,0 +0,1 @@\n+fn added() {}\n";
    let error = parse_unified_diff(text)
        .err()
        .context("a +0 new-side start with added lines must be refused")?;
    assert!(
        matches!(error, CoreError::DiffParse { line: 3, .. }),
        "the refusal names the hunk header's line, got {error:?}"
    );
    Ok(())
}

// [CLI-ARG-DIFF] grammar: arbitrary prose is not a diff.
#[test]
fn arbitrary_text_is_refused_not_silently_emptied() -> Result<()> {
    let error = parse_unified_diff("hello world\n")
        .err()
        .context("prose must be refused as a diff")?;
    assert!(matches!(error, CoreError::DiffParse { line: 1, .. }));
    Ok(())
}

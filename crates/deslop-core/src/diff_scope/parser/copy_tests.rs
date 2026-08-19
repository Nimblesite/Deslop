//! Target-line and copy-metadata grammar ([PIPELINE-DIFF-INGEST]):
//! a hunk may only follow a seen `+++` target, and git `copy from` /
//! `copy to` metadata is parsed into [`FileCopy`] rather than being
//! swallowed — a swallowed copy is a wholesale file duplication that
//! never enters the diff scope.

use anyhow::{Context as _, Result};

use super::*;

/// Parses `text` and returns the single file section it must hold, or
/// an error saying the parse failed or produced no section.
fn parse_only_file(text: &str, why: &str) -> Result<FilePatch> {
    let parsed = parse_unified_diff(text).with_context(|| format!("{why} must parse"))?;
    assert_eq!(parsed.files.len(), 1, "exactly one file section: {why}");
    let only = parsed.files.into_iter().next();
    only.context("the parsed file section")
}

/// Asserts the section's copy payload names the `from` → `to` pair.
fn assert_copy(file: &FilePatch, from: &str, to: &str, why: &str) {
    assert_eq!(
        file.copy,
        Some(FileCopy {
            from: from.to_owned(),
            to: to.to_owned(),
        }),
        "{why}"
    );
}

/// Unwraps a refusal into its `DiffParse` line and message.
fn refusal(text: &str, why: &str) -> Result<(usize, String)> {
    let error = parse_unified_diff(text)
        .err()
        .with_context(|| format!("must be refused: {why}"))?;
    let CoreError::DiffParse { line, message } = error else {
        anyhow::bail!("{why}: expected DiffParse, got {error:?}");
    };
    Ok((line, message))
}

/// One diff the parser must refuse, and where it must say so.
#[derive(Debug)]
struct RefusalCase {
    /// The diff text handed to the parser.
    text: &'static str,
    /// What the case is, carried into the failure context.
    why: &'static str,
    /// The diff line number the refusal must name.
    line: usize,
    /// A fragment the refusal message must contain; `None` when the
    /// case pins only the line number.
    fragment: Option<&'static str>,
}

/// Asserts every case is refused at the diff line it names, with a
/// message carrying its fragment when the case pins one.
fn assert_all_refused(cases: &[RefusalCase]) -> Result<()> {
    for case in cases {
        let why = case.why;
        let (line, message) = refusal(case.text, why)?;
        assert_eq!(line, case.line, "{why}: the refusal names the diff line");
        if let Some(fragment) = case.fragment {
            let named = message.contains(fragment);
            assert!(named, "{why}: the refusal names '{fragment}': {message}");
        }
    }
    Ok(())
}

/// A hunk after junk, and a hunk in a section that skipped both paths.
const NO_TARGET_LINE_CASES: &[RefusalCase] = &[
    RefusalCase {
        text: "diff nonsense\n@@ -0,0 +1 @@\n+x\n",
        why: "hunk after a junk diff line",
        line: 2,
        fragment: Some("+++"),
    },
    RefusalCase {
        text: "diff --git a/x.rs b/x.rs\nindex 1111111..2222222 100644\n@@ -1 +1 @@\n-a\n+b\n",
        why: "hunk in a git section that skipped its '---'/'+++' lines",
        line: 3,
        fragment: None,
    },
];

// [PIPELINE-DIFF-INGEST] P0-1: any `diff ` line opens a section, so
// junk like `diff nonsense` followed by a hunk used to assemble a
// pathless section the verifier ignores — the added lines silently
// vanished from the scope and `added_loc`, and `--fail-over 0` passed
// a run it should have gated. A hunk header in a section that has not
// seen a `+++` target line must be refused at its own line number.
#[test]
fn hunk_in_a_section_without_a_target_line_is_refused() -> Result<()> {
    assert_all_refused(NO_TARGET_LINE_CASES)
}

// [PIPELINE-DIFF-INGEST] P0-1 contrast: `+++ /dev/null` is a SEEN
// target even though it resolves to `new_path == None` — a deletion's
// hunk must stay accepted, or every deletion-bearing diff would be
// refused.
#[test]
fn dev_null_target_counts_as_seen_for_the_following_hunk() -> Result<()> {
    let text = "diff --git a/gone.rs b/gone.rs\n\
                deleted file mode 100644\n\
                --- a/gone.rs\n\
                +++ /dev/null\n\
                @@ -1 +0,0 @@\n\
                -fn gone() {}\n";
    let file = parse_only_file(text, "deletion diff")?;
    assert_eq!(file.new_path, None, "a deletion has no new-side path");
    assert_eq!(file.hunks.len(), 1, "the deletion hunk is kept");
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-1 contrast: legitimate hunkless sections —
// binary entries, GIT binary patch headers, and pure rename metadata —
// carry no hunks, so the target requirement never fires on them.
#[test]
fn hunkless_sections_without_targets_still_parse() -> Result<()> {
    let text = "diff --git a/logo.png b/logo.png\n\
                index 1111111..2222222 100644\n\
                Binary files a/logo.png and b/logo.png differ\n\
                diff --git a/blob.bin b/blob.bin\n\
                GIT binary patch\n\
                diff --git a/old.rs b/new.rs\n\
                similarity index 100%\n\
                rename from old.rs\n\
                rename to new.rs\n";
    let parsed = parse_unified_diff(text).context("hunkless sections must parse")?;
    assert_eq!(parsed.files.len(), 3, "three hunkless sections");
    for file in &parsed.files {
        assert!(file.hunks.is_empty(), "no hunks expected: {file:?}");
        assert_eq!(file.copy, None, "renames and binaries are not copies");
    }
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-3: a metadata-only 100%-similarity copy is
// git's statement that an entire file was duplicated. The parser must
// surface the `copy from` / `copy to` pair instead of swallowing it as
// inert metadata.
#[test]
fn metadata_only_copy_parses_into_the_file_patch() -> Result<()> {
    let text = "diff --git a/src/legacy_a.rs b/src/legacy_b.rs\n\
                similarity index 100%\n\
                copy from src/legacy_a.rs\n\
                copy to src/legacy_b.rs\n";
    let file = parse_only_file(text, "metadata-only copy diff")?;
    assert_copy(
        &file,
        "src/legacy_a.rs",
        "src/legacy_b.rs",
        "the copy pair is the section's payload",
    );
    assert!(file.hunks.is_empty(), "a 100% copy carries no hunks");
    assert_eq!(file.new_path, None, "no '+++' line in a metadata-only copy");
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-3: below 100% similarity git emits the
// copy metadata AND hunks describing the delta against the source; the
// section must keep both.
#[test]
fn copy_with_hunks_keeps_metadata_and_hunks() -> Result<()> {
    let text = "diff --git a/src/a.rs b/src/b.rs\n\
                similarity index 90%\n\
                copy from src/a.rs\n\
                copy to src/b.rs\n\
                index 1111111..2222222 100644\n\
                --- a/src/a.rs\n\
                +++ b/src/b.rs\n\
                @@ -1 +1 @@\n\
                -fn a() {}\n\
                +fn b() {}\n";
    let file = parse_only_file(text, "copy-with-hunks diff")?;
    assert_copy(
        &file,
        "src/a.rs",
        "src/b.rs",
        "both halves survive the hunks",
    );
    assert_eq!(file.hunks.len(), 1, "the delta hunk is kept");
    assert_eq!(file.new_path.as_deref(), Some("src/b.rs"));
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-3: git C-quotes copy paths exactly as it
// does `+++` targets; a quoted copy path left undecoded would match
// nothing downstream and the copy would silently vanish.
#[test]
fn c_quoted_copy_paths_are_unquoted() -> Result<()> {
    let text = "diff --git \"a/caf\\303\\251.rs\" \"b/caf\\303\\251_copy.rs\"\n\
                similarity index 100%\n\
                copy from \"caf\\303\\251.rs\"\n\
                copy to \"caf\\303\\251_copy.rs\"\n";
    let file = parse_only_file(text, "quoted copy diff")?;
    assert_copy(
        &file,
        "café.rs",
        "café_copy.rs",
        "the octal-escaped UTF-8 bytes are the real filenames",
    );
    Ok(())
}

/// Both dangling halves, closed by EOF and by the next `diff ` line.
const DANGLING_COPY_CASES: &[RefusalCase] = &[
    RefusalCase {
        text: "diff --git a/a.rs b/b.rs\nsimilarity index 100%\ncopy from a.rs\n",
        why: "copy from without copy to, closed by EOF",
        line: 3,
        fragment: Some("copy to"),
    },
    RefusalCase {
        text: "diff --git a/a.rs b/b.rs\ncopy to b.rs\n",
        why: "copy to without copy from, closed by EOF",
        line: 2,
        fragment: Some("copy from"),
    },
    RefusalCase {
        text: "diff --git a/a.rs b/b.rs\ncopy from a.rs\ndiff --git a/c.rs b/c.rs\nindex 1..2 100644\n",
        why: "copy from without copy to, closed by the next section",
        line: 2,
        fragment: None,
    },
];

// [PIPELINE-DIFF-INGEST] P0-3: half a copy pair names either no source
// or no destination — ingesting it would either drop the copy target
// (invisible duplication) or fabricate one. Both dangling forms are
// refused at the dangling line, whether the section is closed by the
// next `diff ` line or by end of input.
#[test]
fn dangling_copy_metadata_is_refused() -> Result<()> {
    assert_all_refused(DANGLING_COPY_CASES)
}

/// Two `copy from` lines in one section.
const DUPLICATE_COPY_CASES: &[RefusalCase] = &[RefusalCase {
    text: "diff --git a/a.rs b/b.rs\ncopy from a.rs\ncopy from other.rs\ncopy to b.rs\n",
    why: "two copy from lines in one section",
    line: 3,
    fragment: Some("duplicate"),
}];

// [PIPELINE-DIFF-INGEST] P0-3: duplicated copy metadata in one section
// is not git grammar; guessing which pair wins could ingest the wrong
// file wholesale.
#[test]
fn duplicate_copy_metadata_is_refused() -> Result<()> {
    assert_all_refused(DUPLICATE_COPY_CASES)
}

/// Copy metadata before any file header, and naming an empty path.
const STRAY_OR_EMPTY_COPY_CASES: &[RefusalCase] = &[
    RefusalCase {
        text: "copy from a.rs\n",
        why: "copy metadata before any file header",
        line: 1,
        fragment: None,
    },
    RefusalCase {
        text: "diff --git a/a.rs b/b.rs\ncopy from \"\"\ncopy to b.rs\n",
        why: "copy metadata naming no path",
        line: 2,
        fragment: Some("no path"),
    },
];

// [PIPELINE-DIFF-INGEST] P0-3 strictness: copy metadata outside any
// file section, or naming no path, is junk like any other stray line.
#[test]
fn stray_or_empty_copy_metadata_is_refused() -> Result<()> {
    assert_all_refused(STRAY_OR_EMPTY_COPY_CASES)
}

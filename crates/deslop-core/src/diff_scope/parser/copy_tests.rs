//! Target-line and copy-metadata grammar ([PIPELINE-DIFF-INGEST]):
//! a hunk may only follow a seen `+++` target, and git `copy from` /
//! `copy to` metadata is parsed into [`FileCopy`] rather than being
//! swallowed — a swallowed copy is a wholesale file duplication that
//! never enters the diff scope.

use anyhow::{Context as _, Result};

use super::*;

/// The single file section a test parses, or an error saying the
/// parse produced none.
fn only_file(parsed: &ParsedDiff) -> Result<&FilePatch> {
    assert_eq!(parsed.files.len(), 1, "exactly one file section expected");
    parsed.files.first().context("the parsed file section")
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

// [PIPELINE-DIFF-INGEST] P0-1: any `diff ` line opens a section, so
// junk like `diff nonsense` followed by a hunk used to assemble a
// pathless section the verifier ignores — the added lines silently
// vanished from the scope and `added_loc`, and `--fail-over 0` passed
// a run it should have gated. A hunk header in a section that has not
// seen a `+++` target line must be refused at its own line number.
#[test]
fn hunk_in_a_section_without_a_target_line_is_refused() -> Result<()> {
    let (line, message) = refusal(
        "diff nonsense\n@@ -0,0 +1 @@\n+x\n",
        "hunk after a junk diff line",
    )?;
    assert_eq!(line, 2, "the refusal names the hunk header's diff line");
    assert!(
        message.contains("+++"),
        "the refusal names the missing '+++' target: {message}"
    );
    let (line, _message) = refusal(
        "diff --git a/x.rs b/x.rs\nindex 1111111..2222222 100644\n@@ -1 +1 @@\n-a\n+b\n",
        "hunk in a git section that skipped its '---'/'+++' lines",
    )?;
    assert_eq!(line, 3, "the refusal names the hunk header's diff line");
    Ok(())
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
    let parsed = parse_unified_diff(text).context("deletion diff must parse")?;
    let file = only_file(&parsed)?;
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
    let parsed = parse_unified_diff(text).context("copy diff must parse")?;
    let file = only_file(&parsed)?;
    assert_copy(
        file,
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
    let parsed = parse_unified_diff(text).context("copy-with-hunks diff must parse")?;
    let file = only_file(&parsed)?;
    assert_copy(
        file,
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
    let parsed = parse_unified_diff(text).context("quoted copy diff must parse")?;
    let file = only_file(&parsed)?;
    assert_copy(
        file,
        "café.rs",
        "café_copy.rs",
        "the octal-escaped UTF-8 bytes are the real filenames",
    );
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-3: half a copy pair names either no source
// or no destination — ingesting it would either drop the copy target
// (invisible duplication) or fabricate one. Both dangling forms are
// refused at the dangling line, whether the section is closed by the
// next `diff ` line or by end of input.
#[test]
fn dangling_copy_metadata_is_refused() -> Result<()> {
    let (line, message) = refusal(
        "diff --git a/a.rs b/b.rs\nsimilarity index 100%\ncopy from a.rs\n",
        "copy from without copy to, closed by EOF",
    )?;
    assert_eq!(line, 3, "the refusal names the dangling 'copy from' line");
    assert!(
        message.contains("copy to"),
        "names the missing half: {message}"
    );
    let (line, message) = refusal(
        "diff --git a/a.rs b/b.rs\ncopy to b.rs\n",
        "copy to without copy from, closed by EOF",
    )?;
    assert_eq!(line, 2, "the refusal names the dangling 'copy to' line");
    assert!(
        message.contains("copy from"),
        "names the missing half: {message}"
    );
    let (line, _message) = refusal(
        "diff --git a/a.rs b/b.rs\ncopy from a.rs\ndiff --git a/c.rs b/c.rs\nindex 1..2 100644\n",
        "copy from without copy to, closed by the next section",
    )?;
    assert_eq!(
        line, 2,
        "the refusal names the dangling line, not the closer"
    );
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-3: duplicated copy metadata in one section
// is not git grammar; guessing which pair wins could ingest the wrong
// file wholesale.
#[test]
fn duplicate_copy_metadata_is_refused() -> Result<()> {
    let (line, message) = refusal(
        "diff --git a/a.rs b/b.rs\ncopy from a.rs\ncopy from other.rs\ncopy to b.rs\n",
        "two copy from lines in one section",
    )?;
    assert_eq!(line, 3, "the refusal names the second 'copy from'");
    assert!(
        message.contains("duplicate"),
        "says what is wrong: {message}"
    );
    Ok(())
}

// [PIPELINE-DIFF-INGEST] P0-3 strictness: copy metadata outside any
// file section, or naming no path, is junk like any other stray line.
#[test]
fn stray_or_empty_copy_metadata_is_refused() -> Result<()> {
    let (line, _message) = refusal("copy from a.rs\n", "copy metadata before any file header")?;
    assert_eq!(line, 1);
    let (line, message) = refusal(
        "diff --git a/a.rs b/b.rs\ncopy from \"\"\ncopy to b.rs\n",
        "copy metadata naming no path",
    )?;
    assert_eq!(line, 2);
    assert!(message.contains("no path"), "says what is wrong: {message}");
    Ok(())
}

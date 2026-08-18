//! Strict line-oriented unified-diff parser ([CLI-ARG-DIFF]).
//!
//! Hand-written state machine over physical lines — no regex, per the
//! repo-wide prohibition on pattern-matching structured data. The
//! grammar accepted is `git diff` output: per-file headers, hunk
//! headers with explicit counts, and hunk bodies whose every line
//! carries a ` `/`+`/`-` marker (plus the `\ No newline at end of
//! file` annotation). Anything else is a [`CoreError::DiffParse`]
//! naming the offending diff line — a silently mis-scoped diff would
//! mislabel every downstream population.

use crate::error::CoreError;

/// A parsed unified diff: one entry per file section.
#[derive(Debug, Clone)]
pub struct ParsedDiff {
    /// File sections in input order.
    pub files: Vec<FilePatch>,
}

/// One file's worth of diff: the new-side path and its hunks.
#[derive(Debug, Clone)]
pub struct FilePatch {
    /// New-side path as written in the diff with any `b/` prefix
    /// stripped. `None` when the file was deleted (`+++ /dev/null`).
    pub new_path: Option<String>,
    /// Hunks in input order. Empty for binary or metadata-only entries.
    pub hunks: Vec<Hunk>,
}

/// One `@@` hunk: the new-side starting line and the body lines.
#[derive(Debug, Clone)]
pub struct Hunk {
    /// 1-indexed first new-side line the hunk covers.
    pub new_start: u64,
    /// Body lines in order, markers stripped.
    pub lines: Vec<HunkLine>,
}

/// One hunk-body line with its marker classified.
#[derive(Debug, Clone)]
pub struct HunkLine {
    /// Which side(s) of the diff the line belongs to.
    pub kind: HunkLineKind,
    /// Line content with the leading marker removed. Retains any `\r`
    /// so CRLF sources verify byte-exactly.
    pub content: String,
}

/// Classification of a hunk-body line by its leading marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunkLineKind {
    /// ` ` — present on both sides.
    Context,
    /// `+` — present only on the new side.
    Added,
    /// `-` — present only on the old side.
    Removed,
}

/// Parses `text` as a unified diff. Empty input is a valid empty diff.
///
/// # Errors
///
/// Returns [`CoreError::DiffParse`] naming the first line that is not
/// valid unified-diff grammar: an unmarked hunk-body line, a malformed
/// `@@` header, or an unrecognised top-level line.
pub fn parse_unified_diff(text: &str) -> Result<ParsedDiff, CoreError> {
    let mut parser = Parser::default();
    // Split on `\n` only: `str::lines` would also strip `\r`, which is
    // payload when the diffed file uses CRLF endings. `\n`-terminated
    // input yields a final empty fragment that is a split artefact,
    // not a diff line — feeding it into an open hunk would fabricate
    // an empty context line — so it is dropped, never parsed.
    let mut fragments = text.split('\n').enumerate().peekable();
    while let Some((index, line)) = fragments.next() {
        if fragments.peek().is_none() && line.is_empty() {
            break;
        }
        parser.feed(index.saturating_add(1), line)?;
    }
    parser.finish()
}

/// Incremental parser state.
#[derive(Debug, Default)]
struct Parser {
    /// Completed file sections.
    files: Vec<FilePatch>,
    /// Section currently being assembled.
    current: Option<FilePatch>,
    /// Open hunk plus its outstanding old-side / new-side line budget.
    open_hunk: Option<OpenHunk>,
}

/// A hunk mid-parse with its remaining line counts.
#[derive(Debug)]
struct OpenHunk {
    /// The hunk being filled.
    hunk: Hunk,
    /// Old-side lines (context + removed) still expected.
    old_remaining: u64,
    /// New-side lines (context + added) still expected.
    new_remaining: u64,
}

impl Parser {
    /// Consumes one physical line of diff text.
    fn feed(&mut self, line_no: usize, line: &str) -> Result<(), CoreError> {
        // The no-newline marker annotates the line above it rather than
        // being a body line of its own, so it is recognised here, before
        // either branch: `git` emits it *after* the last line of a hunk,
        // by which point the declared counts are satisfied and the hunk
        // is already closed, so a check inside the body branch alone
        // never sees it and the header branch refuses the whole diff.
        if is_no_newline_marker(line) {
            return self.consume_no_newline_marker(line_no);
        }
        if self.open_hunk.is_some() {
            return self.feed_hunk_body(line_no, line);
        }
        self.feed_header(line_no, line)
    }

    /// Accepts the no-newline marker without letting it consume a hunk
    /// count — it describes the preceding line's terminator, and counting
    /// it would shift every new-side line number after it. Outside a file
    /// section it is still junk, and refused like any other stray line.
    fn consume_no_newline_marker(&mut self, line_no: usize) -> Result<(), CoreError> {
        if self.current.is_none() {
            return Err(parse_error(
                line_no,
                "no-newline marker before any file header",
            ));
        }
        Ok(())
    }

    /// Consumes a line while a hunk still expects body lines.
    fn feed_hunk_body(&mut self, line_no: usize, line: &str) -> Result<(), CoreError> {
        let Some(open) = self.open_hunk.as_mut() else {
            return Err(parse_error(line_no, "no open hunk"));
        };
        let (kind, content) = classify_body_line(line_no, line)?;
        open.consume(line_no, kind)?;
        open.hunk.lines.push(HunkLine {
            kind,
            content: content.to_owned(),
        });
        if open.old_remaining == 0 && open.new_remaining == 0 {
            self.close_hunk(line_no)?;
        }
        Ok(())
    }

    /// Moves the completed open hunk into the current file section.
    fn close_hunk(&mut self, line_no: usize) -> Result<(), CoreError> {
        let Some(open) = self.open_hunk.take() else {
            return Err(parse_error(line_no, "no open hunk to close"));
        };
        let Some(current) = self.current.as_mut() else {
            return Err(parse_error(line_no, "hunk before any file header"));
        };
        current.hunks.push(open.hunk);
        Ok(())
    }

    /// Consumes a line between hunks: file headers, hunk headers, and
    /// the metadata `git diff` emits around them. A trailing `\r` here
    /// is a CRLF-saved patch file, not payload — header lines never
    /// carry content — so it is stripped; hunk-body lines keep theirs.
    fn feed_header(&mut self, line_no: usize, raw: &str) -> Result<(), CoreError> {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.starts_with("diff ") {
            self.begin_file();
            return Ok(());
        }
        if let Some(rest) = line.strip_prefix("+++ ") {
            self.set_new_path(rest);
            return Ok(());
        }
        if let Some(header) = line.strip_prefix("@@ ") {
            return self.begin_hunk(line_no, header);
        }
        if is_metadata_line(line) {
            return Ok(());
        }
        Err(parse_error(
            line_no,
            "expected a file header, hunk header, or diff metadata",
        ))
    }

    /// Flushes the in-progress section and starts a new one.
    fn begin_file(&mut self) {
        if let Some(done) = self.current.take() {
            self.files.push(done);
        }
        self.current = Some(FilePatch {
            new_path: None,
            hunks: Vec::new(),
        });
    }

    /// Records the `+++ ` new-side path on the current section,
    /// opening one for prefix-less plain diffs that skip `diff --git`.
    fn set_new_path(&mut self, raw: &str) {
        if self.current.is_none() {
            self.begin_file();
        }
        if let Some(current) = self.current.as_mut() {
            current.new_path = new_side_path(raw);
        }
    }

    /// Opens a hunk from the text after `@@ `.
    fn begin_hunk(&mut self, line_no: usize, header: &str) -> Result<(), CoreError> {
        if self.current.is_none() {
            return Err(parse_error(line_no, "hunk header before any file header"));
        }
        let (old_remaining, new_start, new_remaining) = parse_hunk_ranges(line_no, header)?;
        let open = OpenHunk {
            hunk: Hunk {
                new_start,
                lines: Vec::new(),
            },
            old_remaining,
            new_remaining,
        };
        if old_remaining == 0 && new_remaining == 0 {
            self.open_hunk = Some(open);
            return self.close_hunk(line_no);
        }
        self.open_hunk = Some(open);
        Ok(())
    }

    /// Finalises parsing, rejecting a truncated trailing hunk.
    fn finish(mut self) -> Result<ParsedDiff, CoreError> {
        if let Some(open) = &self.open_hunk {
            return Err(parse_error(
                usize::MAX,
                &format!(
                    "diff ends inside a hunk ({old} old-side and {new} new-side line(s) missing)",
                    old = open.old_remaining,
                    new = open.new_remaining,
                ),
            ));
        }
        if let Some(done) = self.current.take() {
            self.files.push(done);
        }
        Ok(ParsedDiff { files: self.files })
    }
}

impl OpenHunk {
    /// Debits the side budgets for one classified body line.
    fn consume(&mut self, line_no: usize, kind: HunkLineKind) -> Result<(), CoreError> {
        let (old_cost, new_cost) = match kind {
            HunkLineKind::Context => (1, 1),
            HunkLineKind::Added => (0, 1),
            HunkLineKind::Removed => (1, 0),
        };
        if self.old_remaining < old_cost || self.new_remaining < new_cost {
            return Err(parse_error(
                line_no,
                "hunk body exceeds the counts declared in its header",
            ));
        }
        self.old_remaining = self.old_remaining.saturating_sub(old_cost);
        self.new_remaining = self.new_remaining.saturating_sub(new_cost);
        Ok(())
    }
}

/// Classifies a hunk-body line by its first byte.
fn classify_body_line(line_no: usize, line: &str) -> Result<(HunkLineKind, &str), CoreError> {
    match line.as_bytes().first() {
        Some(b' ') => Ok((HunkLineKind::Context, line.get(1..).unwrap_or(""))),
        Some(b'+') => Ok((HunkLineKind::Added, line.get(1..).unwrap_or(""))),
        Some(b'-') => Ok((HunkLineKind::Removed, line.get(1..).unwrap_or(""))),
        // GNU diff emits a completely empty line for an empty context
        // line; git pads with a space, but both forms occur in the wild.
        None => Ok((HunkLineKind::Context, "")),
        Some(_) => Err(parse_error(
            line_no,
            "hunk body line carries no ' ', '+', or '-' marker",
        )),
    }
}

/// Parses `-old_start[,old_count] +new_start[,new_count] @@…` and
/// returns `(old_count, new_start, new_count)`.
fn parse_hunk_ranges(line_no: usize, header: &str) -> Result<(u64, u64, u64), CoreError> {
    let rest = header
        .strip_prefix('-')
        .ok_or_else(|| parse_error(line_no, "hunk header missing '-' range"))?;
    let (old_range, rest) = rest
        .split_once(" +")
        .ok_or_else(|| parse_error(line_no, "hunk header missing '+' range"))?;
    let (new_range, _section) = rest
        .split_once(" @@")
        .ok_or_else(|| parse_error(line_no, "hunk header missing closing '@@'"))?;
    let (_old_start, old_count) = parse_range(line_no, old_range)?;
    let (new_start, new_count) = parse_range(line_no, new_range)?;
    Ok((old_count, new_start, new_count))
}

/// Parses `start[,count]`; a missing count means 1 per the grammar.
fn parse_range(line_no: usize, range: &str) -> Result<(u64, u64), CoreError> {
    let (start_text, count_text) = match range.split_once(',') {
        Some((start, count)) => (start, count),
        None => (range, "1"),
    };
    let start = parse_number(line_no, start_text)?;
    let count = parse_number(line_no, count_text)?;
    Ok((start, count))
}

/// Parses one decimal number from a hunk range.
fn parse_number(line_no: usize, text: &str) -> Result<u64, CoreError> {
    text.parse::<u64>()
        .map_err(|_| parse_error(line_no, &format!("hunk range component {text:?} is not a number")))
}

/// Strips the `b/` prefix from a `+++ ` payload; `/dev/null` means the
/// file was deleted. Trailing tab-separated timestamps (plain `diff -u`
/// output) are dropped.
fn new_side_path(raw: &str) -> Option<String> {
    let path = raw.split('\t').next().unwrap_or(raw);
    if path == "/dev/null" {
        return None;
    }
    Some(path.strip_prefix("b/").unwrap_or(path).to_owned())
}

/// True for the metadata lines `git diff` interleaves between file and
/// hunk headers. Blank lines separate sections in some producers.
fn is_metadata_line(line: &str) -> bool {
    const METADATA_PREFIXES: &[&str] = &[
        "--- ",
        "index ",
        "new file mode",
        "deleted file mode",
        "old mode",
        "new mode",
        "similarity index",
        "dissimilarity index",
        "rename from",
        "rename to",
        "copy from",
        "copy to",
        "Binary files ",
        "GIT binary patch",
    ];
    line.is_empty()
        || METADATA_PREFIXES
            .iter()
            .any(|prefix| line.starts_with(prefix))
}

/// True for `git`'s "\ No newline at end of file" annotation, with or
/// without a CRLF terminator.
fn is_no_newline_marker(line: &str) -> bool {
    line.strip_suffix('\r').unwrap_or(line) == "\\ No newline at end of file"
}

/// Builds a [`CoreError::DiffParse`] for `line_no`.
fn parse_error(line_no: usize, message: &str) -> CoreError {
    CoreError::DiffParse {
        line: line_no,
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use anyhow::{Context as _, Result};

    use super::*;

    /// The single file section every grammar test above parses, or an
    /// error saying the parse produced none.
    fn first_file(parsed: &ParsedDiff) -> Result<&FilePatch> {
        parsed.files.first().context("the parsed file section")
    }

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
        let error = parse_unified_diff(text).err()
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
        let error = parse_unified_diff(text).err()
            .context("over-long hunk must be refused")?;
        assert!(matches!(error, CoreError::DiffParse { .. }));
        Ok(())
    }

    // [CLI-ARG-DIFF] grammar: a truncated trailing hunk is refused.
    #[test]
    fn truncated_trailing_hunk_is_refused() -> Result<()> {
        let text = "--- a/x.rs\n+++ b/x.rs\n@@ -1,2 +1,2 @@\n only-one\n";
        let error = parse_unified_diff(text).err()
            .context("truncated hunk must be refused")?;
        assert!(matches!(error, CoreError::DiffParse { .. }));
        Ok(())
    }

    // [CLI-ARG-DIFF] grammar: arbitrary prose is not a diff.
    #[test]
    fn arbitrary_text_is_refused_not_silently_emptied() -> Result<()> {
        let error =
            parse_unified_diff("hello world\n").err()
            .context("prose must be refused as a diff")?;
        assert!(matches!(error, CoreError::DiffParse { line: 1, .. }));
        Ok(())
    }
}

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

mod metadata;
mod quoting;

#[cfg(test)]
mod copy_tests;
#[cfg(test)]
mod tests;

use crate::error::CoreError;

/// A parsed unified diff: one entry per file section.
#[derive(Debug, Clone)]
pub struct ParsedDiff {
    /// File sections in input order.
    pub files: Vec<FilePatch>,
}

/// One file's worth of diff: the new-side path and its hunks.
#[derive(Debug, Clone, Default)]
pub struct FilePatch {
    /// New-side path as written in the diff with any `b/` prefix
    /// stripped. `None` when the file was deleted (`+++ /dev/null`).
    pub new_path: Option<String>,
    /// The `copy from` / `copy to` pair when the section is a git copy
    /// ([PIPELINE-DIFF-INGEST]); `None` for every other section kind.
    pub copy: Option<FileCopy>,
    /// Hunks in input order. Empty for binary or metadata-only entries.
    pub hunks: Vec<Hunk>,
}

/// A git copy section's source and destination, as named by its
/// `copy from` / `copy to` metadata lines (which carry no `a/`/`b/`
/// prefixes, per git's diff format).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileCopy {
    /// Old-side path the file was copied from.
    pub from: String,
    /// New-side path the copy created.
    pub to: String,
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
    current: Option<Section>,
    /// Open hunk plus its outstanding old-side / new-side line budget.
    open_hunk: Option<OpenHunk>,
}

/// A file section mid-parse: the patch being assembled plus the state
/// the grammar checks need — whether a `+++` target line has been seen
/// (`+++ /dev/null` counts, whatever path it names), and any copy
/// metadata not yet paired, each remembering its diff line for the
/// dangling-half refusal.
#[derive(Debug, Default)]
struct Section {
    /// The section's accumulated public patch.
    patch: FilePatch,
    /// True once a `+++` line was consumed — distinct from
    /// `patch.new_path.is_some()`, which a deletion leaves `None`.
    saw_target: bool,
    /// `copy from` payload and the diff line it appeared on.
    copy_from: Option<(String, usize)>,
    /// `copy to` payload and the diff line it appeared on.
    copy_to: Option<(String, usize)>,
}

impl Section {
    /// Seals the section into its public [`FilePatch`], pairing the
    /// copy metadata. A lone `copy from` / `copy to` names only half a
    /// copy — ingesting the half would either drop the copy target
    /// (invisible wholesale duplication) or fabricate one — so the
    /// diff is refused at the dangling line. Pinned by
    /// `dangling_copy_metadata_is_refused`.
    fn into_patch(self) -> Result<FilePatch, CoreError> {
        let mut patch = self.patch;
        patch.copy = match (self.copy_from, self.copy_to) {
            (None, None) => None,
            (Some((from, _)), Some((to, _))) => Some(FileCopy { from, to }),
            (Some((_, line)), None) => {
                return Err(parse_error(
                    line,
                    "'copy from' without a matching 'copy to'",
                ));
            }
            (None, Some((_, line))) => {
                return Err(parse_error(
                    line,
                    "'copy to' without a matching 'copy from'",
                ));
            }
        };
        Ok(patch)
    }
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
        if metadata::is_no_newline_marker(line) {
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
        current.patch.hunks.push(open.hunk);
        Ok(())
    }

    /// Consumes a line between hunks: file headers, hunk headers, and
    /// the metadata `git diff` emits around them. A trailing `\r` here
    /// is a CRLF-saved patch file, not payload — header lines never
    /// carry content — so it is stripped; hunk-body lines keep theirs.
    fn feed_header(&mut self, line_no: usize, raw: &str) -> Result<(), CoreError> {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.starts_with("diff ") {
            return self.begin_file();
        }
        if let Some(rest) = line.strip_prefix("+++ ") {
            return self.set_new_path(line_no, rest);
        }
        if let Some(header) = line.strip_prefix("@@ ") {
            return self.begin_hunk(line_no, header);
        }
        if let Some((side, rest)) = metadata::copy_line(line) {
            return self.set_copy(line_no, side, rest);
        }
        if metadata::is_metadata_line(line) {
            return Ok(());
        }
        Err(parse_error(
            line_no,
            "expected a file header, hunk header, or diff metadata",
        ))
    }

    /// Flushes the in-progress section and starts a new one.
    fn begin_file(&mut self) -> Result<(), CoreError> {
        self.flush_section()?;
        self.current = Some(Section::default());
        Ok(())
    }

    /// Seals the in-progress section, if any, into the completed list.
    fn flush_section(&mut self) -> Result<(), CoreError> {
        if let Some(section) = self.current.take() {
            self.files.push(section.into_patch()?);
        }
        Ok(())
    }

    /// Records one half of a git `copy from` / `copy to` pair
    /// ([PIPELINE-DIFF-INGEST]). Outside a file section the line is
    /// stray junk; a repeated half in one section is not git grammar,
    /// and guessing which occurrence wins could resolve the wholesale
    /// copy against the wrong file — both are refused.
    fn set_copy(
        &mut self,
        line_no: usize,
        side: metadata::CopySide,
        raw: &str,
    ) -> Result<(), CoreError> {
        let Some(section) = self.current.as_mut() else {
            return Err(parse_error(line_no, "copy metadata before any file header"));
        };
        let path = metadata::copy_path(line_no, raw)?;
        let slot = match side {
            metadata::CopySide::From => &mut section.copy_from,
            metadata::CopySide::To => &mut section.copy_to,
        };
        if slot.is_some() {
            return Err(parse_error(
                line_no,
                "duplicate copy metadata in one file section",
            ));
        }
        *slot = Some((path, line_no));
        Ok(())
    }

    /// Records the `+++ ` new-side path ([PIPELINE-DIFF-INGEST]). The
    /// `+++` target also *delimits* file sections: it opens one when
    /// none is open, and it starts the *next* one when the current
    /// section already saw a target or hunks — plain (prefix-less)
    /// diffs have no `diff ` line to do that, and `--- ` is swallowed
    /// as metadata, so a second `+++` is the only line that can. Letting
    /// it overwrite the path instead would attach the first file's hunks
    /// to the second file's path and silently drop the first file's
    /// added lines from a merge gate. Pinned by
    /// `plain_multi_file_diff_keeps_each_file_section_separate`.
    fn set_new_path(&mut self, line_no: usize, raw: &str) -> Result<(), CoreError> {
        let section_already_targeted = self
            .current
            .as_ref()
            .is_some_and(|section| section.saw_target || !section.patch.hunks.is_empty());
        if self.current.is_none() || section_already_targeted {
            self.begin_file()?;
        }
        let path = metadata::new_side_path(line_no, raw)?;
        if let Some(section) = self.current.as_mut() {
            section.patch.new_path = path;
            section.saw_target = true;
        }
        Ok(())
    }

    /// Refuses a hunk header arriving before the section's `+++`
    /// target line ([PIPELINE-DIFF-INGEST]). Any `diff ` line opens a
    /// section, so junk like `diff nonsense` followed by a hunk would
    /// otherwise assemble a pathless section the verifier ignores —
    /// its added lines silently vanish from the scope and `added_loc`,
    /// letting `--fail-over 0` pass a run it should gate.
    /// `+++ /dev/null` counts as seen: a deletion *has* a target line,
    /// it just names no new-side file. Pinned by
    /// `hunk_in_a_section_without_a_target_line_is_refused`.
    fn require_target(&self, line_no: usize) -> Result<(), CoreError> {
        let Some(section) = self.current.as_ref() else {
            return Err(parse_error(line_no, "hunk header before any file header"));
        };
        if section.saw_target {
            return Ok(());
        }
        Err(parse_error(
            line_no,
            "hunk header in a file section with no '+++' target line",
        ))
    }

    /// Opens a hunk from the text after `@@ `.
    fn begin_hunk(&mut self, line_no: usize, header: &str) -> Result<(), CoreError> {
        self.require_target(line_no)?;
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
        self.flush_section()?;
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
    reject_zero_new_start(line_no, new_start, new_count)?;
    Ok((old_count, new_start, new_count))
}

/// Refuses a hunk that claims new-side lines starting at line `0`.
///
/// New-side line numbers are 1-indexed, so `+0` with a non-zero count
/// describes lines that cannot exist. `verify_line` computes its source
/// index as `new_line - 1` saturating at zero, so lines `0` and `1` both
/// read the file's first line: the whole added span silently shifts one
/// line up, the real trailing added line falls outside the scope, and
/// the occurrences covering it tag `in_diff: false`. Under
/// `--only-changed` that lets a change pass a gate it should fail. An
/// unrecognised construct must reject the diff rather than guess at
/// spans ([PIPELINE-DIFF-INGEST]). `+0,0` stays legal — that is how a
/// deletion names an empty new side. Pinned by
/// `zero_new_side_start_with_added_lines_is_refused`.
fn reject_zero_new_start(line_no: usize, new_start: u64, new_count: u64) -> Result<(), CoreError> {
    if new_start == 0 && new_count > 0 {
        return Err(parse_error(
            line_no,
            "hunk claims new-side lines starting at line 0; new-side lines are 1-indexed",
        ));
    }
    Ok(())
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
    text.parse::<u64>().map_err(|_| {
        parse_error(
            line_no,
            &format!("hunk range component {text:?} is not a number"),
        )
    })
}

/// Builds a [`CoreError::DiffParse`] for `line_no`.
fn parse_error(line_no: usize, message: &str) -> CoreError {
    CoreError::DiffParse {
        line: line_no,
        message: message.to_owned(),
    }
}

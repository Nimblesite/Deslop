//! Cached source and syntax-highlighted HTML snippets.

use std::{
    collections::HashMap,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use crate::{render::highlight::highlight_snippet, report_location::format_occurrence};

/// Reads source files lazily and caches them so a cluster with many
/// occurrences in the same file does only one disk read.
pub(super) struct SnippetLoader<'a> {
    /// Directory occurrence paths resolve against. `None` disables disk
    /// reads entirely.
    scan_root: Option<&'a Path>,
    /// Cache keyed by relative path.
    cache: HashMap<PathBuf, Option<String>>,
}

impl<'a> SnippetLoader<'a> {
    /// Creates a loader rooted at `scan_root` (or no-op if `None`).
    pub(super) fn new(scan_root: Option<&'a Path>) -> Self {
        Self {
            scan_root,
            cache: HashMap::new(),
        }
    }

    /// Returns the snippet of `source[start..end]` for the file at
    /// `relative` under the configured scan root, plus the 1-indexed
    /// starting line number. `None` when the file cannot be loaded or
    /// the byte range is outside the file.
    pub(super) fn snippet(
        &mut self,
        relative: &Path,
        start: usize,
        end: usize,
    ) -> Option<(String, usize)> {
        let source = self.source(relative)?;
        let safe_end = end.min(source.len());
        let safe_start = start.min(safe_end);
        let slice = source.get(safe_start..safe_end)?;
        let line = line_for_offset(source, safe_start);
        Some((slice.to_owned(), line))
    }

    /// Resolves and caches the full source text for `relative`. Returns
    /// `None` when the source cannot be read as UTF-8 (binary blobs are
    /// not displayed inline).
    fn source(&mut self, relative: &Path) -> Option<&str> {
        let cached = self.cache.entry(relative.to_path_buf()).or_insert_with(|| {
            let root = self.scan_root?;
            let absolute = root.join(relative);
            fs::read_to_string(&absolute).ok()
        });
        cached.as_deref()
    }

    /// Formats the occurrence location from cached source when present.
    pub(super) fn location(&mut self, relative: &Path, start: usize) -> String {
        let source = self.source(relative).map(str::as_bytes);
        format_occurrence(relative, start, source)
    }
}

/// Returns the 1-indexed line number that contains `offset` in `source`.
fn line_for_offset(source: &str, offset: usize) -> usize {
    let safe = offset.min(source.len());
    let prefix = source.get(..safe).unwrap_or("");
    prefix
        .bytes()
        .filter(|b| *b == b'\n')
        .count()
        .saturating_add(1)
}

/// Soft cap on inline snippet height. A 320-line clone is real but
/// useless to scan visually — render the first [`SNIPPET_PREVIEW_LINES`]
/// lines in the visible block and fold the rest into a `<details>` so
/// the card stays compact while the full source remains one click away.
const SNIPPET_PREVIEW_LINES: usize = 40;

/// Renders the snippet body. Up to [`SNIPPET_PREVIEW_LINES`] are shown
/// inline; if the snippet is longer, the remainder is tucked into a
/// `<details>` continuing the line numbers, with a summary chip
/// reporting how many more lines were hidden.
pub(super) fn render_snippet_body(source: &str, start_line: usize, language: &str) -> String {
    let highlighted = highlight_snippet(source, language);
    let lines: Vec<&str> = split_html_lines(&highlighted);
    let line_count = lines.len();
    let gutter_width = digits(start_line.saturating_add(line_count.saturating_sub(1)));
    let preview_end = line_count.min(SNIPPET_PREVIEW_LINES);
    let mut out = String::with_capacity(
        highlighted
            .len()
            .saturating_add(line_count.saturating_mul(20)),
    );
    let preview = lines.get(..preview_end).unwrap_or(&[]);
    write_snippet_pre(&mut out, preview, start_line, gutter_width);
    if line_count > preview_end {
        let hidden = line_count.saturating_sub(preview_end);
        let rest_start = start_line.saturating_add(preview_end);
        let rest = lines.get(preview_end..).unwrap_or(&[]);
        let _ = write!(
            &mut out,
            "<details class=\"also-toggle\"><summary>Show {hidden} more line(s)</summary>",
        );
        write_snippet_pre(&mut out, rest, rest_start, gutter_width);
        out.push_str("</details>");
    }
    out
}

/// Writes one `<pre class="snippet">` block for `lines`, numbering
/// each row from `start_line` and right-aligning the gutter to
/// `gutter_width` characters.
fn write_snippet_pre(out: &mut String, lines: &[&str], start_line: usize, gutter_width: usize) {
    out.push_str("<pre class=\"snippet\">");
    for (index, line) in lines.iter().enumerate() {
        let line_no = start_line.saturating_add(index);
        let _ = writeln!(
            out,
            "<span class=\"ln\">{line_no:>gutter_width$}</span> {line}",
        );
    }
    out.push_str("</pre>");
}

/// Splits `highlighted` HTML into one entry per source line. Splits on
/// raw `\n` bytes — the highlighter never emits `\n` inside a `<span>`
/// for the kinds we classify, so the split never breaks a tag.
fn split_html_lines(highlighted: &str) -> Vec<&str> {
    if highlighted.is_empty() {
        return vec![""];
    }
    highlighted.split('\n').collect()
}

/// Returns the decimal digit count of `value`, with a floor of 1.
fn digits(value: usize) -> usize {
    let mut n = value;
    if n == 0 {
        return 1;
    }
    let mut count: usize = 0;
    while n > 0 {
        count = count.saturating_add(1);
        n /= 10;
    }
    count
}

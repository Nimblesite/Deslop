//! [CORPUS-REPORT-VERDICT] The first thing a scorecard says: whether the run
//! passed, and every blocking failure in a red caution block. Explicitly accepted
//! judged defects follow in a separate note; they remain bugs but are tolerated.

use super::{cells::counted, Scorecard};
use crate::corpus_score::verdict::FailureReason;

/// The document's title, after the verdict.
const TITLE: &str = "Corpus accuracy scorecard";
/// The mark a regression is listed with, and a known defect.
const NEW_MARK: &str = "🔴";
/// See [`NEW_MARK`].
const KNOWN_MARK: &str = "⚠️";

/// The verdict heading, every blocking failure, and explicitly accepted defects.
pub(super) fn verdict_banner(card: &Scorecard) -> Vec<String> {
    let (known, new): (Vec<&FailureReason>, Vec<&FailureReason>) = card
        .verdict
        .failures
        .iter()
        .partition(|failure| !failure.fails_the_run());
    let mut lines = if new.is_empty() {
        vec![format!("# ✅ PASSED — {TITLE}"), String::new()]
    } else {
        failed_block(&new)
    };
    lines.extend(known_block(&known));
    lines
}

/// The failed heading and the caution block listing every blocking failure.
fn failed_block(new: &[&FailureReason]) -> Vec<String> {
    let mut lines = vec![
        format!("# {NEW_MARK} FAILED — {TITLE}"),
        String::new(),
        "> [!CAUTION]".to_owned(),
        format!(
            "> **{NEW_MARK} FAILED** — {}. Every blocking failure is listed here; \
             the tables below give the detail.",
            counted(new.len(), "failure", "failures")
        ),
        ">".to_owned(),
    ];
    lines.extend(new.iter().map(|failure| failure_line(failure, NEW_MARK)));
    lines.push(String::new());
    lines
}

/// The note listing every known defect; nothing when there is none.
fn known_block(known: &[&FailureReason]) -> Vec<String> {
    if known.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![
        "> [!NOTE]".to_owned(),
        format!(
            "> {KNOWN_MARK} {} — explicitly accepted on record. None fails the run. \
             Each is still a bug.",
            counted(known.len(), "accepted defect", "accepted defects")
        ),
        ">".to_owned(),
    ];
    lines.extend(
        known
            .iter()
            .map(|failure| failure_line(failure, KNOWN_MARK)),
    );
    lines.push(String::new());
    lines
}

/// One failure, kept on one line so its block stays whole. A known defect
/// names the record that tracks it.
fn failure_line(failure: &FailureReason, mark: &str) -> String {
    let tracked = failure
        .tracked_in
        .as_ref()
        .map_or_else(String::new, |record| format!(" (tracked in `{record}`)"));
    format!(
        "> - {mark} **{}** — `{}`{tracked}: {}",
        failure.repo,
        failure.what,
        failure.detail.replace('\n', " ")
    )
}

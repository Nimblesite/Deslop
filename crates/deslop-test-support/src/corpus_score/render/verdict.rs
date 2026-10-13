//! [CORPUS-REPORT-VERDICT] The first thing a scorecard says: whether the run
//! passed, and — when it did not — every reason, in a red caution block no
//! reader can scroll past.

use super::{cells::counted, Scorecard};
use crate::corpus_score::verdict::FailureReason;

/// The document's title, after the verdict.
const TITLE: &str = "Corpus accuracy scorecard";
/// What a tracked failure says, so it is never mistaken for a new one — nor
/// for a pass.
const TRACKED: &str = " (tracked in `corpus/known-failures.json`)";

/// The verdict heading, and for a failed run the caution block that lists
/// every failure.
pub(super) fn verdict_banner(card: &Scorecard) -> Vec<String> {
    let failures = &card.verdict.failures;
    if card.verdict.passed() {
        return vec![format!("# ✅ PASSED — {TITLE}"), String::new()];
    }
    let mut lines = vec![
        format!("# 🔴 FAILED — {TITLE}"),
        String::new(),
        "> [!CAUTION]".to_owned(),
        format!(
            "> **🔴 FAILED** — {}. This run did not pass. Every failure is listed here; the \
             tables below give the detail.",
            counted(failures.len(), "failure", "failures")
        ),
        ">".to_owned(),
    ];
    lines.extend(failures.iter().map(failure_line));
    lines.push(String::new());
    lines
}

/// One failure, kept on one line so the caution block stays whole.
fn failure_line(failure: &FailureReason) -> String {
    let tracked = if failure.tracked { TRACKED } else { "" };
    format!(
        "> - 🔴 **{}** — `{}`{tracked}: {}",
        failure.repo,
        failure.what,
        failure.detail.replace('\n', " ")
    )
}

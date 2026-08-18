//! The `--diff` / `--only-changed` CLI surface ([CLI-ARG-DIFF],
//! [CLI-ARG-ONLY-CHANGED], [METRICS-DIFF-SCOPE]).
//!
//! Owns three things the diff flags add to the run: reading and
//! parsing the diff text before the pipeline starts, resolving the
//! fail-over verdict for both the repo-wide and the diff-scoped
//! percentage, and deciding which of those two verdicts gates the exit
//! code. Every rejection here is a *usage* error (exit `2`), never an
//! analysis failure — a bad diff is a mis-invocation, and under
//! `--only-changed` a tolerated one would be a silent false negative
//! in a merge gate.

use std::{fs, io::Read as _, path::Path};

use anyhow::Result;
use deslop_core::{
    parse_unified_diff, CoreError, ParsedDiff, Report, ThresholdSource, ThresholdSummary,
};

use crate::{output::UsageError, Cli};

/// The `--diff` value that means "read the diff from stdin".
const STDIN_SENTINEL: &str = "-";

/// Reads and parses the `--diff` input, or `None` when the flag was
/// not given. Both an unreadable file and malformed diff text are
/// usage errors ([CLI-ARG-DIFF]).
pub(crate) fn load_diff(args: &Cli) -> Result<Option<ParsedDiff>> {
    let Some(source) = args.diff_scope.diff.as_deref() else {
        return Ok(None);
    };
    let text = read_diff_text(source)?;
    let parsed = parse_unified_diff(&text).map_err(|err| UsageError::new(err.to_string()))?;
    tracing::info!(
        files = parsed.files.len(),
        stdin = source == Path::new(STDIN_SENTINEL),
        "diff read",
    );
    Ok(Some(parsed))
}

/// Reads the raw diff text from `source`, or from stdin when `source`
/// is the `-` sentinel.
fn read_diff_text(source: &Path) -> Result<String> {
    if source == Path::new(STDIN_SENTINEL) {
        let mut text = String::new();
        let _bytes = std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|err| UsageError::new(format!("read --diff from stdin: {err}")))?;
        return Ok(text);
    }
    fs::read_to_string(source).map_err(|err| {
        UsageError::new(format!(
            "read --diff file {}: {err}",
            source.display()
        ))
        .into()
    })
}

/// Maps a pipeline failure onto the CLI's error taxonomy. A stale diff
/// is the user's diff disagreeing with the user's tree — a usage error
/// naming the file and line ([CLI-ARG-DIFF]) — while everything else
/// stays an analysis failure.
pub(crate) fn pipeline_error(err: CoreError) -> anyhow::Error {
    match err {
        stale @ CoreError::DiffStale { .. } => UsageError::new(stale.to_string()).into(),
        other => anyhow::Error::new(other).context("analysis pipeline failed"),
    }
}

/// Resolves the fail-over threshold from CLI flags + config file and
/// writes the verdict into `report.metrics.threshold`. Per [EXIT-CODES]:
/// `--no-fail-over` wins; `--fail-over` beats the config key; absence
/// means no gate.
///
/// Under `--only-changed` the same precedence also stamps
/// `metrics.diff.threshold` against the diff-scoped percentage
/// ([METRICS-DIFF-SCOPE]). The repo-wide verdict is still resolved and
/// emitted either way: rerouting the *gate* must never make the report
/// lie about the repository.
pub(crate) fn apply_threshold(args: &Cli, report: &mut Report) -> Result<()> {
    report.metrics.threshold = resolve_threshold(args, report.metrics.duplication_percent)?;
    if !args.diff_scope.only_changed {
        return Ok(());
    }
    let Some(measured) = report.metrics.diff.as_ref().map(|diff| diff.duplication_percent) else {
        return Ok(());
    };
    let verdict = resolve_threshold(args, measured)?;
    if let Some(diff) = report.metrics.diff.as_mut() {
        diff.threshold = verdict;
    }
    Ok(())
}

/// Builds one threshold verdict for `measured` under the flag/config
/// precedence [EXIT-CODES] defines.
fn resolve_threshold(args: &Cli, measured: f64) -> Result<ThresholdSummary> {
    if let Some(percent) = args.fail_over {
        return Ok(ThresholdSummary::resolve(
            percent,
            ThresholdSource::Cli,
            measured,
        ));
    }
    if args.no_fail_over {
        return Ok(ThresholdSummary::none());
    }
    Ok(crate::load_run_config(args)?.resolve_threshold(measured))
}

/// Which verdict gates the exit code: the diff-scoped one under
/// `--only-changed` ([METRICS-DIFF-SCOPE]), the repo-wide one
/// otherwise. A missing `metrics.diff` block cannot gate — there is no
/// measurement to gate on — so it reads as unbreached.
pub(crate) fn gate_breached(args: &Cli, report: &Report) -> bool {
    if args.diff_scope.only_changed {
        return report
            .metrics
            .diff
            .as_ref()
            .is_some_and(|diff| diff.threshold.breached);
    }
    report.metrics.breached()
}

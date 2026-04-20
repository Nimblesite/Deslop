//! Human-facing stderr output for the CLI.
//!
//! Three responsibilities: announce what the run is about to do
//! (`preamble`), print a colored summary of the top clusters
//! (`summary`), and close with a success / error footer (`finish_ok`
//! / `finish_err`). Everything here writes to stderr so stdout stays
//! available for future stream-based integrations.
//!
//! Colour is auto-disabled when stderr is not a TTY, when `NO_COLOR`
//! is set (per <https://no-color.org>), or when the user passes
//! `--no-color`.

use std::{io::IsTerminal as _, path::Path};

use codededup_core::Report;

use crate::logging::LogSink;

/// Number of clusters shown in the summary. Ten is roughly the most
/// an operator can visually triage without paging; the full list
/// lives in the rendered JSON / text / HTML.
const TOP_CLUSTERS_IN_SUMMARY: usize = 10;
/// Cluster-size ceiling for the "worst offenders" tagline in the
/// preamble — purely cosmetic.
const MAX_FILE_COUNT_SUMMARY_SEGMENTS: usize = 3;

/// Global colour policy for the current run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChoice {
    /// Emit ANSI escapes.
    Always,
    /// Never emit ANSI escapes.
    Never,
}

impl ColorChoice {
    /// Resolves the effective colour choice from the user-supplied
    /// `--no-color` flag, the `NO_COLOR` environment variable, the
    /// `CODEDEDUP_FORCE_COLOR` override (tests and CI logs that want
    /// ANSI even without a TTY), and the stderr TTY state, in that
    /// precedence order.
    #[must_use]
    pub fn resolve(force_off: bool) -> Self {
        if force_off {
            return Self::Never;
        }
        if std::env::var_os("NO_COLOR").is_some() {
            return Self::Never;
        }
        if std::env::var_os("CODEDEDUP_FORCE_COLOR").is_some() {
            return Self::Always;
        }
        if std::io::stderr().is_terminal() {
            Self::Always
        } else {
            Self::Never
        }
    }
}

/// Prints the "about to do" line — what path is being analysed,
/// what knobs are set, where output + logs will land. Always flushed
/// before the pipeline starts so a long run can't leave the operator
/// staring at a blank terminal.
pub fn preamble(
    color: ColorChoice,
    scan_path: &Path,
    output_base: &Path,
    log_sink: &LogSink,
    knobs: &PreambleKnobs<'_>,
) {
    let theme = Theme::pick(color);
    eprintln!(
        "{bold}codededup{reset} analysing {cyan}{path}{reset}",
        bold = theme.bold,
        reset = theme.reset,
        cyan = theme.cyan,
        path = scan_path.display(),
    );
    eprintln!(
        "  {dim}min-nodes={min_nodes}, embeddings={embeddings}, incremental={incremental}{reset}",
        dim = theme.dim,
        reset = theme.reset,
        min_nodes = knobs.min_nodes,
        embeddings = knobs.embedding_mode,
        incremental = knobs.incremental,
    );
    eprintln!(
        "  {dim}report → {output}.{{json,txt,html}}{reset}",
        dim = theme.dim,
        reset = theme.reset,
        output = output_base.display(),
    );
    match log_sink {
        LogSink::File(path) => eprintln!(
            "  {dim}log    → {path}{reset}",
            dim = theme.dim,
            reset = theme.reset,
            path = path.display(),
        ),
        LogSink::Console => eprintln!(
            "  {dim}log    → stderr (--log-to-console){reset}",
            dim = theme.dim,
            reset = theme.reset,
        ),
    }
}

/// CLI knobs surfaced in the preamble. Grouped so the function
/// signature stays under the 7-argument budget.
#[derive(Debug)]
pub struct PreambleKnobs<'a> {
    /// Value of `--min-nodes`.
    pub min_nodes: u32,
    /// String form of `--embeddings`.
    pub embedding_mode: &'a str,
    /// Whether the incremental-cache path is enabled.
    pub incremental: bool,
}

/// Prints the summary block (run stats + top-N clusters) to stderr.
/// Called after the pipeline finishes and the renderers write their
/// files. Uses colour when `color` is `Always`.
pub fn summary(color: ColorChoice, report: &Report) {
    let theme = Theme::pick(color);
    eprintln!();
    eprintln!(
        "{bold}Summary{reset}  {files} file(s)  {clusters} cluster(s)  {hidden} hidden",
        bold = theme.bold,
        reset = theme.reset,
        files = report.files_analysed,
        clusters = report.clusters.len(),
        hidden = report.clusters_hidden,
    );
    let cache = report.cache_stats;
    if cache.hits != 0 || cache.misses != 0 {
        eprintln!(
            "  {dim}cache {hits} hit / {misses} miss{reset}",
            dim = theme.dim,
            reset = theme.reset,
            hits = cache.hits,
            misses = cache.misses,
        );
    }
    if let Some(provenance) = report.embedding_provenance.as_ref() {
        eprintln!(
            "  {dim}embeddings {provider}/{model}@{version} ({dims}-d){reset}",
            dim = theme.dim,
            reset = theme.reset,
            provider = provenance.provider_id,
            model = provenance.model_id,
            version = provenance.model_version,
            dims = provenance.dimensions,
        );
    }
    if report.clusters.is_empty() {
        eprintln!(
            "  {green}no duplication detected{reset}",
            green = theme.green,
            reset = theme.reset,
        );
        return;
    }
    eprintln!();
    eprintln!(
        "{bold}Top {n} clusters{reset}",
        bold = theme.bold,
        reset = theme.reset,
        n = report.clusters.len().min(TOP_CLUSTERS_IN_SUMMARY),
    );
    for (index, cluster) in report
        .clusters
        .iter()
        .take(TOP_CLUSTERS_IN_SUMMARY)
        .enumerate()
    {
        render_cluster(&theme, index, cluster);
    }
    if report.clusters.len() > TOP_CLUSTERS_IN_SUMMARY {
        eprintln!(
            "  {dim}... {more} more (see report){reset}",
            dim = theme.dim,
            reset = theme.reset,
            more = report
                .clusters
                .len()
                .saturating_sub(TOP_CLUSTERS_IN_SUMMARY),
        );
    }
}

/// Prints the "wrote these files" footer on a successful run.
pub fn finish_ok(color: ColorChoice, written: &WrittenArtefacts<'_>) {
    let theme = Theme::pick(color);
    eprintln!();
    eprintln!(
        "{green}✔{reset} {bold}done{reset}",
        green = theme.green,
        bold = theme.bold,
        reset = theme.reset,
    );
    for path in written.reports {
        eprintln!(
            "    {dim}report{reset} {path}",
            dim = theme.dim,
            reset = theme.reset,
            path = path.display(),
        );
    }
    if let Some(log_path) = written.log {
        eprintln!(
            "    {dim}log   {reset} {path}",
            dim = theme.dim,
            reset = theme.reset,
            path = log_path.display(),
        );
    }
}

/// Prints a red "run failed" footer with the log location so the
/// operator knows where to look.
pub fn finish_err(color: ColorChoice, log_sink: &LogSink, error: &dyn std::fmt::Display) {
    let theme = Theme::pick(color);
    eprintln!();
    eprintln!(
        "{red}✘{reset} {bold}failed{reset}: {error}",
        red = theme.red,
        bold = theme.bold,
        reset = theme.reset,
    );
    if let LogSink::File(path) = log_sink {
        eprintln!(
            "    {dim}log{reset} {path}",
            dim = theme.dim,
            reset = theme.reset,
            path = path.display(),
        );
    }
}

/// Paths written by a successful run, grouped so [`finish_ok`] stays
/// under the 7-argument function budget.
#[derive(Debug)]
pub struct WrittenArtefacts<'a> {
    /// On-disk report paths (JSON / text / HTML, in whatever subset
    /// was enabled).
    pub reports: &'a [std::path::PathBuf],
    /// Path to the log file; `None` when logs went to stderr.
    pub log: Option<&'a Path>,
}

/// Renders one cluster row of the summary table.
fn render_cluster(theme: &Theme, index: usize, cluster: &codededup_core::report::ReportCluster) {
    let signal_color = classify(theme, cluster);
    let files = summarise_files(&cluster.occurrences);
    eprintln!(
        "  {bold}#{rank:<2}{reset} {color}●{reset} [{dim}{id}{reset}] \
         size={size} nodes={nodes} weight={weight:.1}  \
         s={s:.2} j={j:.2} e={e:.2}  {files}",
        bold = theme.bold,
        reset = theme.reset,
        color = signal_color,
        dim = theme.dim,
        rank = index.saturating_add(1),
        id = &cluster.id.get(..8).unwrap_or(&cluster.id),
        size = cluster.size,
        nodes = cluster.canonical_node_count,
        weight = cluster.weight,
        s = cluster.signals.structural,
        j = cluster.signals.token_jaccard,
        e = cluster.signals.embedding_cos,
        files = files,
    );
}

/// Picks a colour for the cluster dot based on its signal
/// combination. Type-1/2 exact clones → green (safe extract). Type-3
/// near-miss → yellow (review). Type-4 / LSH-only → red (manual
/// inspection).
fn classify(theme: &Theme, cluster: &codededup_core::report::ReportCluster) -> &'static str {
    let s = cluster.signals.structural;
    let j = cluster.signals.token_jaccard;
    let e = cluster.signals.embedding_cos;
    if s >= 0.99 && j >= 0.99 {
        theme.green
    } else if s < 0.01 && (j >= 0.90 || e >= 0.80) {
        theme.yellow
    } else {
        theme.red
    }
}

/// Collapses the occurrence list into `"file.ext + N more"`.
fn summarise_files(occurrences: &[codededup_core::report::ReportOccurrence]) -> String {
    let mut names: Vec<String> = Vec::new();
    for occ in occurrences {
        let name = occ
            .path
            .file_name()
            .map(|os| os.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !names.contains(&name) {
            names.push(name);
        }
        if names.len() >= MAX_FILE_COUNT_SUMMARY_SEGMENTS {
            break;
        }
    }
    let shown = names.join(", ");
    let unique_count = occurrences
        .iter()
        .map(|occ| occ.path.as_path())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    if unique_count > names.len() {
        format!(
            "{shown} (+{} more)",
            unique_count.saturating_sub(names.len())
        )
    } else {
        shown
    }
}

/// ANSI escape strings; all empty when [`ColorChoice::Never`].
#[derive(Debug, Clone, Copy)]
struct Theme {
    /// Bold start.
    bold: &'static str,
    /// Dim / faint start.
    dim: &'static str,
    /// Foreground green.
    green: &'static str,
    /// Foreground yellow.
    yellow: &'static str,
    /// Foreground red.
    red: &'static str,
    /// Foreground cyan (used for paths).
    cyan: &'static str,
    /// Reset — always emitted after any style change.
    reset: &'static str,
}

impl Theme {
    /// Returns the ANSI theme for `choice`. `Never` yields an empty
    /// theme so the same `eprintln!` templates work unchanged.
    const fn pick(choice: ColorChoice) -> Self {
        match choice {
            ColorChoice::Always => Self {
                bold: "\x1b[1m",
                dim: "\x1b[2m",
                green: "\x1b[32m",
                yellow: "\x1b[33m",
                red: "\x1b[31m",
                cyan: "\x1b[36m",
                reset: "\x1b[0m",
            },
            ColorChoice::Never => Self {
                bold: "",
                dim: "",
                green: "",
                yellow: "",
                red: "",
                cyan: "",
                reset: "",
            },
        }
    }
}

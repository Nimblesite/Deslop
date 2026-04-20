//! Human-facing stderr output: `preamble`, `summary`, `finish_ok` /
//! `finish_err`. Colour is auto-disabled without a TTY, with `NO_COLOR`
//! set, or with `--no-color`.

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

/// Prints the "about to do" line. Plain English by default; technical
/// knobs (min-nodes, incremental, etc.) are hidden unless `--technical`
/// is set.
pub fn preamble(
    color: ColorChoice,
    scan_path: &Path,
    output_base: &Path,
    log_sink: &LogSink,
    knobs: &PreambleKnobs<'_>,
) {
    let theme = Theme::pick(color);
    eprintln!(
        "{bold}codededup{reset} scanning {cyan}{path}{reset} for duplicated code...",
        bold = theme.bold,
        reset = theme.reset,
        cyan = theme.cyan,
        path = scan_path.display(),
    );
    if knobs.technical {
        eprintln!(
            "  {dim}min-nodes={min_nodes}, embeddings={embeddings}, incremental={incremental}{reset}",
            dim = theme.dim,
            reset = theme.reset,
            min_nodes = knobs.min_nodes,
            embeddings = knobs.embedding_mode,
            incremental = knobs.incremental,
        );
    }
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

/// CLI knobs surfaced in the preamble.
#[derive(Debug)]
pub struct PreambleKnobs<'a> {
    /// Value of `--min-nodes`.
    pub min_nodes: u32,
    /// String form of `--embeddings`.
    pub embedding_mode: &'a str,
    /// Whether the incremental-cache path is enabled.
    pub incremental: bool,
    /// When true, the preamble + summary surface the researcher-jargon
    /// view (signal letters, AST node counts, taxonomy IDs). When
    /// false (default), output is plain English.
    pub technical: bool,
}

/// Prints the summary block to stderr. Plain English by default;
/// `technical = true` switches to the researcher view (signal letters,
/// AST node counts, weight, taxonomy IDs).
pub fn summary(color: ColorChoice, report: &Report, technical: bool) {
    let theme = Theme::pick(color);
    eprintln!();
    let total_duplicated_bytes: usize = report
        .clusters
        .iter()
        .flat_map(|c| c.occurrences.iter())
        .map(|occ| occ.end_byte.saturating_sub(occ.start_byte))
        .sum();
    eprintln!(
        "{bold}Found {clusters} groups of duplicated code{reset} across {files} file(s) \
         {dim}(~{kb} KB of duplication total){reset}",
        bold = theme.bold,
        dim = theme.dim,
        reset = theme.reset,
        clusters = report.clusters.len(),
        files = report.files_analysed,
        kb = total_duplicated_bytes.div_ceil(1024).max(1),
    );
    if report.clusters_hidden > 0 {
        eprintln!(
            "  {dim}({hidden} more groups hidden by your .codededup.toml config){reset}",
            dim = theme.dim,
            reset = theme.reset,
            hidden = report.clusters_hidden,
        );
    }
    write_cache_line(&theme, report, technical);
    write_provenance_line(&theme, report, technical);
    if report.clusters.is_empty() {
        eprintln!(
            "  {green}✔ no duplication detected — your codebase is clean.{reset}",
            green = theme.green,
            reset = theme.reset,
        );
        return;
    }
    write_breakdown_line(&theme, report, technical);
    write_worst_offender_line(&theme, report);
    eprintln!();
    write_top_clusters_header(&theme, report, technical);
    for (index, cluster) in report
        .clusters
        .iter()
        .take(TOP_CLUSTERS_IN_SUMMARY)
        .enumerate()
    {
        render_cluster(&theme, index, cluster, technical);
    }
    if report.clusters.len() > TOP_CLUSTERS_IN_SUMMARY {
        eprintln!(
            "  {dim}... {more} more in the full report{reset}",
            dim = theme.dim,
            reset = theme.reset,
            more = report
                .clusters
                .len()
                .saturating_sub(TOP_CLUSTERS_IN_SUMMARY),
        );
    }
    write_next_steps(&theme);
}

/// Header above the top-N cluster list. Plain-English version is two
/// lines (heading + tiny legend); technical version adds the column
/// dictionary.
fn write_top_clusters_header(theme: &Theme, report: &Report, technical: bool) {
    eprintln!(
        "{bold}Worst {n} groups{reset}  {dim}(● green = identical · yellow = nearly identical · red = similar){reset}",
        bold = theme.bold,
        dim = theme.dim,
        reset = theme.reset,
        n = report.clusters.len().min(TOP_CLUSTERS_IN_SUMMARY),
    );
    if technical {
        eprintln!(
            "  {dim}columns: rank, signal, id, copies, AST nodes, weight, (s=structural j=token e=embedding), files{reset}",
            dim = theme.dim,
            reset = theme.reset,
        );
    }
}

/// Cache-stats line. Hidden in plain mode unless the cache actually
/// did something useful (a hit on a re-run); shown in technical mode
/// whenever the cache was active.
fn write_cache_line(theme: &Theme, report: &Report, technical: bool) {
    let cache = report.cache_stats;
    if cache.hits == 0 && cache.misses == 0 {
        return;
    }
    if technical {
        eprintln!(
            "  {dim}cache: {hits} hit / {misses} miss{reset}",
            dim = theme.dim,
            reset = theme.reset,
            hits = cache.hits,
            misses = cache.misses,
        );
    } else if cache.hits > 0 {
        eprintln!(
            "  {dim}skipped {hits} unchanged file(s) using the cache{reset}",
            dim = theme.dim,
            reset = theme.reset,
            hits = cache.hits,
        );
    }
}

/// Embedding-provenance line. Plain mode just notes that meaning-based
/// detection was on; technical mode shows the full provider/model pin.
fn write_provenance_line(theme: &Theme, report: &Report, technical: bool) {
    let Some(provenance) = report.embedding_provenance.as_ref() else {
        return;
    };
    if technical {
        eprintln!(
            "  {dim}embeddings: {provider}/{model}@{version} ({dims}-d){reset}",
            dim = theme.dim,
            reset = theme.reset,
            provider = provenance.provider_id,
            model = provenance.model_id,
            version = provenance.model_version,
            dims = provenance.dimensions,
        );
    } else {
        eprintln!(
            "  {dim}meaning-based detection enabled (catches code that does the same thing different ways){reset}",
            dim = theme.dim,
            reset = theme.reset,
        );
    }
}

/// One-line breakdown of how many groups fall into each bucket.
/// Plain English by default; researcher labels behind `--technical`.
fn write_breakdown_line(theme: &Theme, report: &Report, technical: bool) {
    let counts = ClusterBreakdown::from(report);
    if technical {
        eprintln!(
            "  {green}{exact} exact{reset} {dim}(Type-1/2){reset}  ·  \
             {yellow}{near} near-miss{reset} {dim}(Type-3){reset}  ·  \
             {red}{weak} weak{reset} {dim}(LSH-only){reset}{semantic}",
            green = theme.green,
            yellow = theme.yellow,
            red = theme.red,
            dim = theme.dim,
            reset = theme.reset,
            exact = counts.exact,
            near = counts.near_miss,
            weak = counts.weak,
            semantic = if counts.semantic == 0 {
                String::new()
            } else {
                format!(
                    "  ·  {cyan}{n} semantic{reset} {dim}(Type-4){reset}",
                    cyan = theme.cyan,
                    dim = theme.dim,
                    reset = theme.reset,
                    n = counts.semantic,
                )
            },
        );
    } else {
        eprintln!(
            "  {green}{exact} identical{reset} {dim}(safe to merge){reset}  ·  \
             {yellow}{near} nearly identical{reset} {dim}(worth reviewing){reset}  ·  \
             {red}{weak} loosely similar{reset} {dim}(check manually){reset}{semantic}",
            green = theme.green,
            yellow = theme.yellow,
            red = theme.red,
            dim = theme.dim,
            reset = theme.reset,
            exact = counts.exact,
            near = counts.near_miss,
            weak = counts.weak,
            semantic = if counts.semantic == 0 {
                String::new()
            } else {
                format!(
                    "  ·  {cyan}{n} same idea, different code{reset}",
                    cyan = theme.cyan,
                    reset = theme.reset,
                    n = counts.semantic,
                )
            },
        );
    }
}

/// Plain-English worst-offender callout. Always plain — the technical
/// view of "worst offender" is just the row in the cluster table.
fn write_worst_offender_line(theme: &Theme, report: &Report) {
    let Some(worst) = report.clusters.first() else {
        return;
    };
    let files = summarise_files(&worst.occurrences);
    let total_bytes: usize = worst
        .occurrences
        .iter()
        .map(|occ| occ.end_byte.saturating_sub(occ.start_byte))
        .sum();
    eprintln!(
        "  {bold}Worst offender:{reset} a block of code copy-pasted {size} times in {cyan}{files}{reset} \
         {dim}(~{kb} KB total){reset}",
        bold = theme.bold,
        cyan = theme.cyan,
        dim = theme.dim,
        reset = theme.reset,
        size = worst.size,
        files = files,
        kb = total_bytes.div_ceil(1024).max(1),
    );
}

/// Closing advice. Same in both modes — pointing at the HTML/JSON
/// reports is a human concern.
fn write_next_steps(theme: &Theme) {
    eprintln!();
    eprintln!(
        "{bold}Next:{reset} open the .html report in a browser to see the actual duplicated code, side by side.",
        bold = theme.bold,
        reset = theme.reset,
    );
}

/// Counts of clusters in each signal bucket. Mirrors `report::interpret`
/// at a coarser grain so the summary line stays one row.
#[derive(Debug, Default, Clone, Copy)]
struct ClusterBreakdown {
    /// Type-1 / Type-2 exact clones — safe to extract.
    exact: usize,
    /// Type-3 near-miss — needs review.
    near_miss: usize,
    /// Weak / LSH-only — manual inspection.
    weak: usize,
    /// Type-4 semantic — embedding-driven matches.
    semantic: usize,
}

impl From<&Report> for ClusterBreakdown {
    fn from(report: &Report) -> Self {
        let mut out = Self::default();
        for cluster in &report.clusters {
            let s = cluster.signals.structural;
            let j = cluster.signals.token_jaccard;
            let e = cluster.signals.embedding_cos;
            if s >= 0.99 && j >= 0.99 {
                out.exact = out.exact.saturating_add(1);
            } else if s < 0.01 && j >= 0.90 {
                out.near_miss = out.near_miss.saturating_add(1);
            } else if e >= 0.80 && s < 0.5 {
                out.semantic = out.semantic.saturating_add(1);
            } else {
                out.weak = out.weak.saturating_add(1);
            }
        }
        out
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

/// Renders one cluster row plus a one-line interpretation underneath.
fn render_cluster(
    theme: &Theme,
    index: usize,
    cluster: &codededup_core::report::ReportCluster,
    technical: bool,
) {
    let signal_color = classify(theme, cluster);
    let files = summarise_files(&cluster.occurrences);
    if technical {
        eprintln!(
            "  {bold}#{rank:<2}{reset} {color}●{reset} [{dim}{id}{reset}] \
             {size}× copies · {nodes} AST nodes · weight {weight:.1}  \
             {dim}(s={s:.2} j={j:.2} e={e:.2}){reset}  {cyan}{files}{reset}",
            bold = theme.bold,
            reset = theme.reset,
            color = signal_color,
            dim = theme.dim,
            cyan = theme.cyan,
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
    } else {
        eprintln!(
            "  {bold}#{rank:<2}{reset} {color}●{reset} {size}× copies in {cyan}{files}{reset}",
            bold = theme.bold,
            reset = theme.reset,
            color = signal_color,
            cyan = theme.cyan,
            rank = index.saturating_add(1),
            size = cluster.size,
            files = files,
        );
    }
    eprintln!(
        "       {dim}↳ {interp}{reset}",
        dim = theme.dim,
        reset = theme.reset,
        interp = plain_interpretation(cluster, technical),
    );
}

/// Returns the per-cluster interpretation string. Plain mode rewrites
/// the report's researcher-jargon `interpretation` into something a
/// non-specialist can read; technical mode passes it through.
fn plain_interpretation(
    cluster: &codededup_core::report::ReportCluster,
    technical: bool,
) -> String {
    if technical {
        return cluster.interpretation.clone();
    }
    let s = cluster.signals.structural;
    let j = cluster.signals.token_jaccard;
    let e = cluster.signals.embedding_cos;
    if s >= 0.99 && j >= 0.99 {
        "Identical code — safe to extract into a shared function.".to_owned()
    } else if s >= 0.99 {
        "Same shape, slightly different details — likely the same clone seen from different angles."
            .to_owned()
    } else if s <= 0.01 && j >= 0.90 {
        "Nearly identical — small differences may matter, so review before merging.".to_owned()
    } else if s > 0.0 && j >= 0.95 {
        "A family of variants on the same theme — usually genuine duplication.".to_owned()
    } else if e >= 0.80 {
        "Different code that does the same thing — worth a manual look.".to_owned()
    } else {
        "Loosely similar — inspect manually before acting.".to_owned()
    }
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

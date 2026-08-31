//! Summary block + per-cluster row rendering.
//!
//! CLI stderr is a **shared-text** surface per [CLONE-BUCKETS-DUAL-LABEL]:
//! a human reads it in the terminal, an AI agent scrapes it from a
//! subprocess. So every bucket name uses the hybrid form —
//! `"Identical code [Type-1/2]"`, `"Same behavior, different code
//! [Type-4, AI match]"` — taken from `BucketLabels::hybrid_title`.
//! `--technical` is a **verbosity** toggle (adds signal triples, cache
//! counters, embedding provenance), not a label-variant toggle.

use deslop_core::{
    buckets::{bucket_labels, classify, ClusterKind},
    clone_category::CloneCategory,
    report::ReportCluster,
    report::ReportOccurrence,
    Report,
};

use super::{theme::Theme, ColorChoice};

/// Number of clusters shown in the stderr summary; the full list lives
/// in the JSON / text / HTML reports.
const TOP_CLUSTERS_IN_SUMMARY: usize = 10;
/// Maximum distinct file names listed inline in a cluster row.
const MAX_FILE_COUNT_SUMMARY_SEGMENTS: usize = 3;

/// Prints the summary block to stderr.
pub fn summary(color: ColorChoice, report: &Report, technical: bool) {
    let theme = Theme::pick(color);
    eprintln!();
    write_headline(&theme, report);
    write_diff_delta_line(&theme, report);
    // `clusters_hidden` counts every suppressed cluster regardless of
    // cause — built-in noise filters and report policy alike — so the
    // line must not attribute them to the user's `.deslop.toml`, which
    // may not even exist (gh #373,
    // `hidden_group_summary_names_the_hider_not_the_users_config`).
    if report.clusters_hidden > 0 {
        eprintln!(
            "  {dim}({hidden} more groups hidden by built-in noise filters or report policy){reset}",
            dim = theme.dim,
            reset = theme.reset,
            hidden = report.clusters_hidden,
        );
    }
    write_cache_line(&theme, report, technical);
    write_provenance_line(&theme, report, technical);
    if report.clusters.is_empty() {
        write_clean_line(&theme, report);
        return;
    }
    write_breakdown_line(&theme, report, technical);
    write_category_line(&theme, report);
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

/// Top "Found N groups..." line with total duplicated bytes.
fn write_headline(theme: &Theme, report: &Report) {
    let total_bytes: usize = report
        .clusters
        .iter()
        .flat_map(|c| c.occurrences.iter())
        .map(|occ| occ.end_byte.saturating_sub(occ.start_byte))
        .sum();
    eprintln!(
        "{bold}Found {clusters} groups of duplicated code{reset} across {files} file(s) \
         {dim}(~{kb} KB total){reset}",
        bold = theme.bold,
        dim = theme.dim,
        reset = theme.reset,
        clusters = report.clusters.len(),
        files = report.files_analysed,
        kb = total_bytes.div_ceil(1024).max(1),
    );
}

/// Diff-scoped delta line ([CLI-ARG-ONLY-CHANGED],
/// [METRICS-DIFF-SCOPE]): leads the summary with what this diff newly
/// introduced, how many surviving groups clone untouched code (#364's
/// requested cross-file classification), and how many untouched groups
/// the filter omitted. Silent unless `--only-changed` filtered the
/// cluster list, so every other run's stderr stays identical.
fn write_diff_delta_line(theme: &Theme, report: &Report) {
    let Some(outside) = report.clusters_outside_diff else {
        return;
    };
    let newly = report
        .clusters
        .iter()
        .filter(|cluster| cluster.is_newly_introduced == Some(true))
        .count();
    let cross_file = report.clusters.len().saturating_sub(newly);
    eprintln!(
        "  {bold}{newly} group(s) newly introduced by this diff, {cross_file} cross-file with untouched code{reset} \
         {dim}({outside} untouched group(s) omitted by --only-changed){reset}",
        bold = theme.bold,
        dim = theme.dim,
        reset = theme.reset,
    );
}

/// The empty-body closer. A plain run with no clusters means the
/// codebase is clean; an `--only-changed` run that omitted untouched
/// groups only proves the *diff* added nothing — claiming the codebase
/// is clean right after naming the omitted legacy debt would be false
/// ([METRICS-DIFF-SCOPE]).
fn write_clean_line(theme: &Theme, report: &Report) {
    let omitted = report.clusters_outside_diff.unwrap_or(0);
    if omitted > 0 {
        eprintln!(
            "  {green}✔ no diff-affected duplication — {omitted} untouched group(s) omitted by --only-changed.{reset}",
            green = theme.green,
            reset = theme.reset,
        );
    } else {
        eprintln!(
            "  {green}✔ no duplication detected — your codebase is clean.{reset}",
            green = theme.green,
            reset = theme.reset,
        );
    }
}

/// "Worst N groups" heading with the colour legend. Legend covers all
/// four [CLONE-BUCKETS] buckets so nothing in the per-cluster rows
/// below is un-explained.
fn write_top_clusters_header(theme: &Theme, report: &Report, technical: bool) {
    eprintln!(
        "{bold}Worst {n} groups{reset}  {dim}(● green = identical · yellow = nearly identical · red = loosely similar · cyan = same behavior [AI]){reset}",
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

/// Cache-stats line. Suppressed in plain mode unless we got hits.
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

/// Embedding-pass provenance line. Plain mode summarises in English.
fn write_provenance_line(theme: &Theme, report: &Report, technical: bool) {
    let Some(provenance) = report.embedding_provenance.as_ref() else {
        return;
    };
    if technical {
        eprintln!(
            "  {dim}embeddings: {provider}/{model}@{version} ({dims}-d, indexed {indexed}/{attempted}, failures {failed}){reset}",
            dim = theme.dim,
            reset = theme.reset,
            provider = provenance.provider_id,
            model = provenance.model_id,
            version = provenance.model_version,
            dims = provenance.dimensions,
            indexed = provenance.indexed_subtrees,
            failed = provenance.failed_subtrees,
            attempted = provenance.attempted_subtrees,
        );
    } else {
        eprintln!(
            "  {dim}meaning-based detection enabled (catches code that does the same thing different ways){reset}",
            dim = theme.dim,
            reset = theme.reset,
        );
    }
}

/// Categorised breakdown line — one segment per non-zero [CLONE-BUCKETS]
/// bucket. Shared-text surface: uses `hybrid_title` (e.g.
/// `"Identical code [Type-1/2]"`) so the line reads as plain English
/// to a human and as a grep-able `[Type-N]` suffix to an AI scraper.
/// `technical` is a verbosity flag, not a label switch — same labels
/// regardless.
fn write_breakdown_line(theme: &Theme, report: &Report, _technical: bool) {
    let counts = ClusterBreakdown::from(report);
    let mut parts: Vec<String> = Vec::new();
    for kind in ClusterKind::all() {
        let count = counts.count(kind);
        if count == 0 {
            continue;
        }
        let labels = bucket_labels(kind);
        let colour = bucket_colour(theme, kind);
        parts.push(format!(
            "{colour}{count} × {title}{reset}",
            colour = colour,
            count = count,
            title = labels.hybrid_title,
            reset = theme.reset,
        ));
    }
    if parts.is_empty() {
        return;
    }
    eprintln!("  {}", parts.join("  ·  "));
}

/// Category breakdown line ([FACET-CLI]) — one dim segment per
/// non-logic category present, e.g. `2 × data table`. Logic is the
/// implicit default so it never prints; the literal families join
/// automatically through `CloneCategory::all()` when [LITERAL-CATEGORY]
/// ships. Silent when every cluster is ordinary logic.
fn write_category_line(theme: &Theme, report: &Report) {
    let parts: Vec<String> = CloneCategory::all()
        .into_iter()
        .filter_map(|category| {
            let chip = category.chip()?;
            let count = report
                .clusters
                .iter()
                .filter(|cluster| CloneCategory::from_wire_label(&cluster.category) == category)
                .count();
            (count > 0).then(|| format!("{count} × {chip}"))
        })
        .collect();
    if parts.is_empty() {
        return;
    }
    eprintln!(
        "  {dim}{line}{reset}",
        dim = theme.dim,
        line = parts.join("  ·  "),
        reset = theme.reset,
    );
}

/// Theme colour for a bucket. Mirrors the card band in the HTML
/// renderer: green=identical, yellow=nearly identical, dim=structural
/// only (demoted shape-only evidence), red=loosely similar, cyan=same
/// behavior (AI match). Terminal themes can't render purple reliably,
/// so `SameBehavior` uses cyan — aligned with
/// `docs/specs/taxonomy.md [CLONE-BUCKETS]` colour-band commentary.
fn bucket_colour(theme: &Theme, kind: ClusterKind) -> &'static str {
    match kind {
        ClusterKind::Identical => theme.green,
        ClusterKind::NearlyIdentical => theme.yellow,
        ClusterKind::StructuralOnly => theme.dim,
        ClusterKind::LooselySimilar => theme.red,
        ClusterKind::SameBehavior => theme.cyan,
    }
}

/// Plain-English worst-offender callout — one sentence naming the
/// biggest cluster, its copy count, and the files it spans. Silently
/// bails on an empty cluster list, even though callers guard that
/// case — keeps the renderer total.
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
        "  {bold}Worst offender:{reset} a block copy-pasted {size} times in {cyan}{files}{reset} \
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

/// Closing advice. Same in both modes — pointing at the HTML report
/// is a human concern.
fn write_next_steps(theme: &Theme) {
    eprintln!();
    eprintln!(
        "{bold}Next:{reset} open the .html report in a browser to see the actual duplicated code, side by side.",
        bold = theme.bold,
        reset = theme.reset,
    );
}

/// Counts of clusters in each [CLONE-BUCKETS] bucket. Routing goes
/// through `buckets::classify` so the CLI, HTML, and JSON
/// `interpret()` never disagree.
#[derive(Debug, Default, Clone, Copy)]
struct ClusterBreakdown {
    /// Cluster count classified as [`ClusterKind::Identical`].
    identical: usize,
    /// Cluster count classified as [`ClusterKind::NearlyIdentical`].
    nearly_identical: usize,
    /// Cluster count classified as [`ClusterKind::StructuralOnly`].
    structural_only: usize,
    /// Cluster count classified as [`ClusterKind::LooselySimilar`].
    loosely_similar: usize,
    /// Cluster count classified as [`ClusterKind::SameBehavior`].
    same_behavior: usize,
}

impl ClusterBreakdown {
    /// Returns the cluster count for a given [`ClusterKind`].
    fn count(self, kind: ClusterKind) -> usize {
        match kind {
            ClusterKind::Identical => self.identical,
            ClusterKind::NearlyIdentical => self.nearly_identical,
            ClusterKind::StructuralOnly => self.structural_only,
            ClusterKind::LooselySimilar => self.loosely_similar,
            ClusterKind::SameBehavior => self.same_behavior,
        }
    }
}

impl From<&Report> for ClusterBreakdown {
    fn from(report: &Report) -> Self {
        let mut out = Self::default();
        for cluster in &report.clusters {
            match classify(cluster) {
                ClusterKind::Identical => {
                    out.identical = out.identical.saturating_add(1);
                }
                ClusterKind::NearlyIdentical => {
                    out.nearly_identical = out.nearly_identical.saturating_add(1);
                }
                ClusterKind::StructuralOnly => {
                    out.structural_only = out.structural_only.saturating_add(1);
                }
                ClusterKind::LooselySimilar => {
                    out.loosely_similar = out.loosely_similar.saturating_add(1);
                }
                ClusterKind::SameBehavior => {
                    out.same_behavior = out.same_behavior.saturating_add(1);
                }
            }
        }
        out
    }
}

/// Renders one cluster row plus a one-line interpretation underneath.
/// The row header (`#N ● NxM copies in files`) is shared-text and uses
/// the bucket's `hybrid_title`. The interpretation line under it reads
/// the plain `evidence_sentence` so a non-specialist sees non-jargon prose
/// immediately. Technical mode tacks on the signal triple for expert
/// operators.
fn render_cluster(theme: &Theme, index: usize, cluster: &ReportCluster, technical: bool) {
    let kind = classify(cluster);
    let labels = bucket_labels(kind);
    let signal_color = bucket_colour(theme, kind);
    let files = summarise_files(&cluster.occurrences);
    if technical {
        eprintln!(
            "  {bold}#{rank:<2}{reset} {color}●{reset} {title}  [{dim}{id}{reset}] \
             {size}× copies · {nodes} AST nodes · weight {weight:.1}  \
             {dim}(s={s:.2} j={j:.2} e={e:.2}){reset}  {cyan}{files}{reset}",
            bold = theme.bold,
            reset = theme.reset,
            color = signal_color,
            title = labels.hybrid_title,
            dim = theme.dim,
            cyan = theme.cyan,
            rank = index.saturating_add(1),
            id = cluster.id.get(..8).unwrap_or(&cluster.id),
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
            "  {bold}#{rank:<2}{reset} {color}●{reset} {title} — {size}× copies in {cyan}{files}{reset}",
            bold = theme.bold,
            reset = theme.reset,
            color = signal_color,
            title = labels.hybrid_title,
            cyan = theme.cyan,
            rank = index.saturating_add(1),
            size = cluster.size,
            files = files,
        );
    }
    eprintln!(
        "       {dim}↳ {action}{reset}",
        dim = theme.dim,
        reset = theme.reset,
        action = labels.evidence_sentence,
    );
}

/// Collapses the occurrence list into `"file.ext + N more"`.
fn summarise_files(occurrences: &[ReportOccurrence]) -> String {
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

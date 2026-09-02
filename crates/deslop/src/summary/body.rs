//! Mass-only CLI summary rendering.

use deslop_core::{report::ReportCluster, report::ReportOccurrence, Report};

use super::{theme::Theme, ColorChoice};

/// Number of clusters shown in the stderr summary.
const TOP_CLUSTERS_IN_SUMMARY: usize = 10;
/// Maximum distinct file names listed inline in a cluster row.
const MAX_FILE_COUNT_SUMMARY_SEGMENTS: usize = 3;

/// Prints the summary block to stderr.
pub fn summary(color: ColorChoice, report: &Report, technical: bool) {
    let theme = Theme::pick(color);
    eprintln!();
    write_headline(&theme, report);
    write_diff_delta_line(&theme, report);
    write_hidden_line(&theme, report);
    write_cache_line(&theme, report, technical);
    write_provenance_line(&theme, report, technical);
    if report.clusters.is_empty() {
        write_clean_line(&theme, report);
        return;
    }
    write_severity_line(&theme, report);
    write_worst_offender_line(&theme, report);
    eprintln!();
    write_top_clusters_header(&theme, report, technical);
    for cluster in report.clusters.iter().take(TOP_CLUSTERS_IN_SUMMARY) {
        render_cluster(&theme, cluster, technical);
    }
    write_omitted_clusters(&theme, report);
    write_next_steps(&theme);
}

/// Top line with cluster count and total engine-authored mass.
fn write_headline(theme: &Theme, report: &Report) {
    let total_mass = report
        .clusters
        .iter()
        .fold(0_u64, |total, cluster| total.saturating_add(cluster.mass));
    eprintln!(
        "{bold}Found {clusters} groups of duplicated code{reset} across {files} file(s) {dim}(total mass {total_mass}){reset}",
        bold = theme.bold,
        dim = theme.dim,
        reset = theme.reset,
        clusters = report.clusters.len(),
        files = report.files_analysed,
    );
}

/// Diff-scoped cluster delta line.
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
        "  {bold}{newly} group(s) newly introduced by this diff, {cross_file} cross-file with untouched code{reset} {dim}({outside} untouched group(s) omitted by --only-changed){reset}",
        bold = theme.bold,
        dim = theme.dim,
        reset = theme.reset,
    );
}

/// Reports clusters removed from the visible report by engine policy.
fn write_hidden_line(theme: &Theme, report: &Report) {
    if report.clusters_hidden == 0 {
        return;
    }
    eprintln!(
        "  {dim}({hidden} more groups hidden by built-in noise filters or report policy){reset}",
        dim = theme.dim,
        reset = theme.reset,
        hidden = report.clusters_hidden,
    );
}

/// Empty-body closer.
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

/// Cache-stats line.
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

/// Embedding provenance without leaking pair measurements onto clusters.
fn write_provenance_line(theme: &Theme, report: &Report, technical: bool) {
    let Some(provenance) = report.embedding_provenance.as_ref() else {
        return;
    };
    if technical {
        eprintln!(
            "  {dim}pair admission embeddings: {provider}/{model}@{version} ({dims}-d, indexed {indexed}/{attempted}, failures {failed}){reset}",
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
            "  {dim}meaning-based pair admission enabled{reset}",
            dim = theme.dim,
            reset = theme.reset,
        );
    }
}

/// Counts engine-stamped mass rank bands.
fn write_severity_line(theme: &Theme, report: &Report) {
    let bands = ["worst", "top10", "mid", "faint"];
    let parts: Vec<String> = bands
        .iter()
        .filter_map(|band| {
            let count = report
                .clusters
                .iter()
                .filter(|cluster| cluster.rank_band == *band)
                .count();
            (count > 0).then(|| format!("{count} × {band}"))
        })
        .collect();
    eprintln!(
        "  {dim}mass severity: {parts}{reset}",
        dim = theme.dim,
        reset = theme.reset,
        parts = parts.join(" · "),
    );
}

/// Names the highest-mass cluster.
fn write_worst_offender_line(theme: &Theme, report: &Report) {
    let Some(worst) = report.clusters.first() else {
        return;
    };
    eprintln!(
        "  {bold}Highest mass:{reset} {mass} from {occurrences} occurrences × {nodes} canonical nodes in {cyan}{files}{reset}",
        bold = theme.bold,
        cyan = theme.cyan,
        reset = theme.reset,
        mass = worst.mass,
        occurrences = worst.occurrence_count,
        nodes = worst.canonical_node_count,
        files = summarise_files(&worst.occurrences),
    );
}

/// Heading for the mass-ranked list.
fn write_top_clusters_header(theme: &Theme, report: &Report, technical: bool) {
    eprintln!(
        "{bold}Highest-mass {count} groups{reset}",
        bold = theme.bold,
        reset = theme.reset,
        count = report.clusters.len().min(TOP_CLUSTERS_IN_SUMMARY),
    );
    if technical {
        eprintln!(
            "  {dim}columns: rank, id, mass, occurrences, canonical AST nodes, files{reset}",
            dim = theme.dim,
            reset = theme.reset,
        );
    }
}

/// Renders one neutral mass-only cluster row.
fn render_cluster(theme: &Theme, cluster: &ReportCluster, technical: bool) {
    let files = summarise_files(&cluster.occurrences);
    if technical {
        eprintln!(
            "  {bold}#{rank:<2}{reset} Duplicate code [{dim}{id}{reset}] · mass {mass} · {occurrences} occurrences · {nodes} AST nodes · {cyan}{files}{reset}",
            bold = theme.bold,
            reset = theme.reset,
            dim = theme.dim,
            cyan = theme.cyan,
            rank = cluster.rank,
            id = short_id(&cluster.id),
            mass = cluster.mass,
            occurrences = cluster.occurrence_count,
            nodes = cluster.canonical_node_count,
        );
    } else {
        eprintln!(
            "  {bold}#{rank:<2}{reset} Duplicate code — mass {mass}, {occurrences} occurrences in {cyan}{files}{reset}",
            bold = theme.bold,
            reset = theme.reset,
            cyan = theme.cyan,
            rank = cluster.rank,
            mass = cluster.mass,
            occurrences = cluster.occurrence_count,
        );
    }
}

/// Stable short id for display.
fn short_id(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

/// Reports rows omitted from the terminal summary.
fn write_omitted_clusters(theme: &Theme, report: &Report) {
    if report.clusters.len() <= TOP_CLUSTERS_IN_SUMMARY {
        return;
    }
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

/// Closing advice.
fn write_next_steps(theme: &Theme) {
    eprintln!();
    eprintln!(
        "{bold}Next:{reset} open the .html report to inspect occurrences or explicitly compare two endpoints.",
        bold = theme.bold,
        reset = theme.reset,
    );
}

/// Collapses the occurrence list into file names.
fn summarise_files(occurrences: &[ReportOccurrence]) -> String {
    let mut names: Vec<String> = Vec::new();
    for occurrence in occurrences {
        let name = occurrence
            .path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !names.contains(&name) {
            names.push(name);
        }
        if names.len() >= MAX_FILE_COUNT_SUMMARY_SEGMENTS {
            break;
        }
    }
    append_unshown_file_count(&names, occurrences)
}

/// Appends the number of additional files beyond the inline cap.
fn append_unshown_file_count(names: &[String], occurrences: &[ReportOccurrence]) -> String {
    let shown = names.join(", ");
    let unique_count = occurrences
        .iter()
        .map(|occurrence| occurrence.path.as_path())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    if unique_count <= names.len() {
        return shown;
    }
    format!(
        "{shown} (+{} more)",
        unique_count.saturating_sub(names.len())
    )
}

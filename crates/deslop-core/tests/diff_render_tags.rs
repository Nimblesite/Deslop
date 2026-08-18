//! Renderer-level diff tagging ([OUTPUT-SCHEMA-DIFF-TAGS],
//! [METRICS-DIFF-SCOPE], [CLI-ARG-ONLY-CHANGED]).
//!
//! Black-box over `deslop-core`'s public render surface. The E2E CLI
//! coverage lives in `crates/deslop/tests/diff_scoped_reporting.rs`;
//! this file pins the two properties the CLI test cannot see directly:
//! the exact text a tagged report renders, and the byte-identity of an
//! untagged report's rendering with the pre-diff build.

use std::path::PathBuf;

use deslop_core::{
    render::{render_html, render_text},
    report::{ActionHint, CacheStats, Report, ReportCluster, ReportOccurrence, ReportSignals},
    report_metrics::{DiffMetrics, RepoMetrics, ThresholdSource, ThresholdSummary},
};

/// Occurrence tags for the three populations the fixture models.
#[derive(Clone, Copy)]
struct Tags {
    /// `ReportOccurrence.in_diff` for the first (canonical) occurrence.
    first: Option<bool>,
    /// `ReportOccurrence.in_diff` for the second occurrence.
    second: Option<bool>,
    /// `ReportCluster.intersects_diff`.
    intersects: Option<bool>,
    /// `ReportCluster.is_newly_introduced`.
    newly: Option<bool>,
}

/// The untagged shape: every diff field absent, as on a run without
/// `--diff`.
const UNTAGGED: Tags = Tags {
    first: None,
    second: None,
    intersects: None,
    newly: None,
};

/// The mixed population: changed code cloning an untouched helper.
const MIXED: Tags = Tags {
    first: Some(true),
    second: Some(false),
    intersects: Some(true),
    newly: Some(false),
};

/// The wholly-new population: both copies arrived with the diff.
const FRESH: Tags = Tags {
    first: Some(true),
    second: Some(true),
    intersects: Some(true),
    newly: Some(true),
};

/// The legacy population: neither copy is touched by the diff.
const LEGACY: Tags = Tags {
    first: Some(false),
    second: Some(false),
    intersects: Some(false),
    newly: Some(false),
};

fn occurrence(name: &str, line: i64, in_diff: Option<bool>) -> ReportOccurrence {
    ReportOccurrence {
        path: PathBuf::from(name),
        start_byte: 0,
        end_byte: 40,
        start_line: line,
        end_line: line.saturating_add(9),
        hidden: false,
        in_diff,
    }
}

fn cluster(id: &str, first: &str, second: &str, tags: Tags) -> ReportCluster {
    ReportCluster {
        id: id.to_owned(),
        weight: 4.5,
        size: 2,
        canonical_node_count: 12,
        signals: ReportSignals {
            structural: 1.0,
            token_jaccard: 1.0,
            embedding_cos: 0.0,
            fused: 1.0,
        },
        bucket: "identical".to_owned(),
        category: "logic".to_owned(),
        occurrences: vec![
            occurrence(first, 8, tags.first),
            occurrence(second, 30, tags.second),
        ],
        occurrences_total: 2,
        occurrences_truncated: false,
        summary: "two identical copies".to_owned(),
        interpretation: "extract a shared helper".to_owned(),
        intersects_diff: tags.intersects,
        is_newly_introduced: tags.newly,
    }
}

fn metrics(diff: Option<DiffMetrics>) -> RepoMetrics {
    RepoMetrics {
        analysed_loc: 200,
        duplicated_loc: 40,
        duplication_percent: 20.0,
        clusters_total: 3,
        duplicated_files: 4,
        threshold: ThresholdSummary {
            percent: 10.0,
            breached: true,
            source: ThresholdSource::Cli,
        },
        per_file: Vec::new(),
        diff,
    }
}

fn diff_metrics() -> DiffMetrics {
    DiffMetrics {
        added_loc: 38,
        duplicated_added_loc: 24,
        duplication_percent: 63.15789473684211,
        threshold: ThresholdSummary {
            percent: 0.0,
            breached: true,
            source: ThresholdSource::Cli,
        },
    }
}

/// A three-cluster report. `tagged` switches every diff field on at
/// once, exactly as `--diff` does.
fn report(tagged: bool, outside: Option<usize>) -> Report {
    let (mixed, fresh, legacy) = if tagged {
        (MIXED, FRESH, LEGACY)
    } else {
        (UNTAGGED, UNTAGGED, UNTAGGED)
    };
    Report {
        tool_version: "test".to_owned(),
        min_nodes: 3,
        files_analysed: 6,
        clusters_hidden: 0,
        cache_stats: CacheStats::default(),
        metrics: metrics(tagged.then(diff_metrics)),
        schema_doc: "schema".to_owned(),
        action_hints: vec![ActionHint {
            pattern: "bucket=identical".to_owned(),
            recommendation: "extract".to_owned(),
        }],
        boilerplate_hints: Vec::new(),
        embedding_provenance: None,
        clusters: vec![
            cluster("aaaa1111", "src/caller.rs", "src/helper.rs", mixed),
            cluster("bbbb2222", "src/fresh_a.rs", "src/fresh_b.rs", fresh),
            cluster("cccc3333", "src/legacy_a.rs", "src/legacy_b.rs", legacy),
        ],
        clusters_outside_diff: outside,
    }
}

#[test]
fn dump_renderings() {
    println!("===TEXT-UNTAGGED===");
    println!("{}", render_text(&report(false, None)));
    println!("===TEXT-TAGGED===");
    println!("{}", render_text(&report(true, Some(1))));
    println!("===HTML-UNTAGGED-LEN===");
    println!("{}", render_html(&report(false, None), None, false).len());
    println!("===HTML-UNTAGGED===");
    println!("{}", render_html(&report(false, None), None, false));
    println!("===HTML-TAGGED===");
    println!("{}", render_html(&report(true, Some(1)), None, false));
}

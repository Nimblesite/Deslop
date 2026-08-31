//! Renderer-level diff tagging ([OUTPUT-SCHEMA-DIFF-TAGS],
//! [METRICS-DIFF-SCOPE], [CLI-ARG-ONLY-CHANGED]).
//!
//! Black-box over `deslop-core`'s public render surface. The E2E CLI
//! coverage lives in `crates/deslop/tests/diff_scoped_reporting.rs`;
//! this file pins the two properties the CLI test cannot see directly:
//! the exact text a tagged report renders, and the byte-exact
//! rendering of an untagged report — the pre-diff output a no-`--diff`
//! run must keep producing.

use std::path::PathBuf;

use deslop_core::{
    render::{render_html, render_text},
    report::{ActionHint, CacheStats, Report, ReportCluster, ReportOccurrence},
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
    let mut cluster = deslop_core::report_fixtures::fixture_cluster(
        id,
        vec![
            occurrence(first, 8, tags.first),
            occurrence(second, 30, tags.second),
        ],
    );
    cluster.weight = 4.5;
    cluster.canonical_node_count = 12;
    "csharp".clone_into(&mut cluster.language);
    "two identical copies".clone_into(&mut cluster.summary);
    "extract a shared helper".clone_into(&mut cluster.interpretation);
    cluster.intersects_diff = tags.intersects;
    cluster.is_newly_introduced = tags.newly;
    cluster
}

/// The three populations, tagged as `--diff` stamps them or fully
/// untagged. Order matters: the goldens below pin it.
fn clusters(tagged: bool) -> Vec<ReportCluster> {
    let (mixed, fresh, legacy) = if tagged {
        (MIXED, FRESH, LEGACY)
    } else {
        (UNTAGGED, UNTAGGED, UNTAGGED)
    };
    vec![
        cluster("aaaa1111", "src/caller.rs", "src/helper.rs", mixed),
        cluster("bbbb2222", "src/fresh_a.rs", "src/fresh_b.rs", fresh),
        cluster("cccc3333", "src/legacy_a.rs", "src/legacy_b.rs", legacy),
    ]
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
        folders: Vec::new(),
        diff,
    }
}

/// `gated: true` models the `--only-changed` shape, where the CLI
/// resolved `--fail-over` onto the diff scope; `false` models plain
/// `--diff`, where the diff threshold stays `none()` — the repo gate
/// governs ([METRICS-DIFF-SCOPE]).
fn diff_metrics(gated: bool) -> DiffMetrics {
    DiffMetrics {
        added_loc: 38,
        duplicated_added_loc: 24,
        duplication_percent: 63.157_894_736_842_11,
        threshold: if gated {
            ThresholdSummary {
                percent: 0.0,
                breached: true,
                source: ThresholdSource::Cli,
            }
        } else {
            ThresholdSummary::none()
        },
    }
}

fn report(
    clusters: Vec<ReportCluster>,
    diff: Option<DiffMetrics>,
    outside: Option<usize>,
) -> Report {
    Report {
        tool_version: "test".to_owned(),
        min_nodes: 3,
        files_analysed: 6,
        clusters_hidden: 0,
        cache_stats: CacheStats::default(),
        metrics: metrics(diff),
        schema_doc: "schema".to_owned(),
        action_hints: vec![ActionHint {
            pattern: "bucket=identical".to_owned(),
            recommendation: "extract".to_owned(),
        }],
        boilerplate_hints: Vec::new(),
        embedding_provenance: None,
        clusters,
        clusters_outside_diff: outside,
    }
}

/// A run without `--diff`: no diff metrics, no tags, nothing omitted.
fn untagged_report() -> Report {
    report(clusters(false), None, None)
}

/// A `--diff` run without `--only-changed`: every cluster tagged and
/// kept, `clusters_outside_diff` absent, diff threshold unresolved —
/// the repo-wide gate governs.
fn diff_report() -> Report {
    report(clusters(true), Some(diff_metrics(false)), None)
}

/// A `--diff --only-changed` run: the legacy cluster is dropped from
/// the body and counted, `clusters_total` follows the body, and the
/// diff gate governs — exactly as `apply_only_changed` plus the CLI's
/// gate rerouting produce. Repo threshold ok, diff threshold breached:
/// the state whose banner colour proves which gate governs.
fn only_changed_report() -> Report {
    let mut kept = clusters(true);
    kept.truncate(2);
    let mut filtered = report(kept, Some(diff_metrics(true)), Some(1));
    filtered.metrics.clusters_total = 2;
    filtered.metrics.threshold.breached = false;
    filtered
}

/// The opposite governing-gate direction: legacy repo debt breaches
/// the repo threshold, but the diff itself is clean — the banner must
/// read from the governing diff gate and render ok.
fn only_changed_clean_report() -> Report {
    let mut clean = only_changed_report();
    clean.metrics.threshold.breached = true;
    if let Some(diff) = clean.metrics.diff.as_mut() {
        diff.threshold.percent = 65.0;
        diff.threshold.breached = false;
    }
    clean
}

/// One rendered-page expectation: the markup to look for, how many
/// times it must appear, and why that count is the contract.
type Expectation = (&'static str, usize, &'static str);

/// Asserts every expectation against one rendered page, naming the
/// offending needle and reason on the first that misses.
fn assert_page(html: &str, expectations: &[Expectation]) {
    for (needle, times, why) in expectations {
        assert_eq!(html.matches(needle).count(), *times, "{why}: {needle}");
    }
}

/// The pre-diff text output, byte for byte. Cluster blocks carry no
/// occurrence rows — badged rows exist only under `--diff`, so this
/// golden is the byte-identity contract for every no-diff run.
const UNTAGGED_TEXT: &str = "deslop test -- 6 file(s), 3 cluster(s), 0 hidden
repo: 20.0% duplicated (40 / 200 LOC, 3 clusters across 4 files)
threshold: 10.00% (breached)
embeddings: off
-- action hints --
  [bucket=identical] extract
#1 [aaaa1111] weight=4.50 size=2 nodes=12
  two identical copies
  :: extract a shared helper
#2 [bbbb2222] weight=4.50 size=2 nodes=12
  two identical copies
  :: extract a shared helper
#3 [cccc3333] weight=4.50 size=2 nodes=12
  two identical copies
  :: extract a shared helper
";

/// A `--diff` run: the added-lines figure and one badged row per
/// occurrence; no diff-threshold verdict (the repo gate governs) and
/// no delta line (nothing was omitted).
const DIFF_TEXT: &str = "deslop test -- 6 file(s), 3 cluster(s), 0 hidden
repo: 20.0% duplicated (40 / 200 LOC, 3 clusters across 4 files)
threshold: 10.00% (breached)
diff: 63.2% of added lines duplicated (24 / 38 added LOC)
embeddings: off
-- action hints --
  [bucket=identical] extract
#1 [aaaa1111] weight=4.50 size=2 nodes=12
  two identical copies
  :: extract a shared helper
  - src/caller.rs:8-17 [in diff]
  - src/helper.rs:30-39 [existing]
#2 [bbbb2222] weight=4.50 size=2 nodes=12
  two identical copies
  :: extract a shared helper
  - src/fresh_a.rs:8-17 [in diff]
  - src/fresh_b.rs:30-39 [in diff]
#3 [cccc3333] weight=4.50 size=2 nodes=12
  two identical copies
  :: extract a shared helper
  - src/legacy_a.rs:8-17 [existing]
  - src/legacy_b.rs:30-39 [existing]
";

/// A `--diff --only-changed` run: the governing diff gate renders its
/// verdict, the delta line carries all four figures (intersecting =
/// newly + cross-file; omitted named beside them), and the repo line
/// still says 3 clusters — derived as `clusters_total +
/// clusters_outside_diff`, since `clusters_total` follows the body.
const ONLY_CHANGED_TEXT: &str = "deslop test -- 6 file(s), 2 cluster(s), 0 hidden
repo: 20.0% duplicated (40 / 200 LOC, 3 clusters across 4 files)
threshold: 10.00% (ok)
diff: 63.2% of added lines duplicated (24 / 38 added LOC)
diff threshold: 0.00% (breached)
delta: 2 cluster(s) intersect the diff — 1 newly introduced, 1 cross-file with untouched code; 1 untouched cluster(s) omitted
embeddings: off
-- action hints --
  [bucket=identical] extract
#1 [aaaa1111] weight=4.50 size=2 nodes=12
  two identical copies
  :: extract a shared helper
  - src/caller.rs:8-17 [in diff]
  - src/helper.rs:30-39 [existing]
#2 [bbbb2222] weight=4.50 size=2 nodes=12
  two identical copies
  :: extract a shared helper
  - src/fresh_a.rs:8-17 [in diff]
  - src/fresh_b.rs:30-39 [in diff]
";

/// The untagged banner, closed at the threshold verdict — pinning the
/// exact bytes forecloses any diff tail leaking into a no-diff run.
const UNTAGGED_BANNER: &str = "<p class=\"metrics-banner metrics-banner--breached\">repo: \
     20.0% duplicated (40 / 200 LOC, 3 clusters across 4 files) · threshold 10.00% \
     (breached)</p>";

/// The `--diff` banner: the added-lines figure appended, no delta
/// segment (nothing was omitted).
const DIFF_BANNER: &str = "<p class=\"metrics-banner metrics-banner--breached\">repo: \
     20.0% duplicated (40 / 200 LOC, 3 clusters across 4 files) · threshold 10.00% \
     (breached) · diff: 63.2% of added lines duplicated (24 / 38 added LOC)</p>";

/// The `--only-changed` banner: the governing diff verdict and the
/// four-figure delta appended, and the colour class read from the
/// governing diff gate — breached here although the repo gate is ok.
const ONLY_CHANGED_BANNER: &str = "<p class=\"metrics-banner metrics-banner--breached\">repo: \
     20.0% duplicated (40 / 200 LOC, 3 clusters across 4 files) · threshold 10.00% \
     (ok) · diff: 63.2% of added lines duplicated (24 / 38 added LOC) · diff threshold \
     0.00% (breached) · 1 newly introduced group(s), 1 cross-file with untouched code, \
     1 untouched group(s) omitted</p>";

/// The opposite direction: repo debt breached, diff clean — the class
/// must follow the governing diff gate and render ok.
const ONLY_CHANGED_CLEAN_BANNER: &str = "<p class=\"metrics-banner metrics-banner--ok\">repo: \
     20.0% duplicated (40 / 200 LOC, 3 clusters across 4 files) · threshold 10.00% \
     (breached) · diff: 63.2% of added lines duplicated (24 / 38 added LOC) · diff threshold \
     65.00% (ok) · 1 newly introduced group(s), 1 cross-file with untouched code, \
     1 untouched group(s) omitted</p>";

#[test]
fn untagged_report_renders_the_exact_pre_diff_bytes() {
    let report = untagged_report();
    assert_eq!(render_text(&report), UNTAGGED_TEXT);

    assert_page(
        &render_html(&report, None, false),
        &[
            (UNTAGGED_BANNER, 1, "banner ends at the repo verdict"),
            (
                "class=\"cluster-card kind-identical cat-logic\"",
                3,
                "all three cards carry the plain class list",
            ),
            (
                "<span class=\"diff-badge\">",
                0,
                "no badge element renders without --diff (the CSS rule alone is static)",
            ),
            ("facet-diff", 0, "no diff facet controls without --diff"),
        ],
    );
}

#[test]
fn diff_tagged_text_renders_the_gate_and_one_badged_row_per_occurrence() {
    assert_eq!(render_text(&diff_report()), DIFF_TEXT);
    assert_eq!(render_text(&only_changed_report()), ONLY_CHANGED_TEXT);
}

#[test]
fn diff_tagged_html_marks_banner_cards_badges_and_facets() {
    assert_page(
        &render_html(&diff_report(), None, false),
        &[
            (DIFF_BANNER, 1, "banner carries the added-lines figure"),
            (
                "class=\"cluster-card kind-identical cat-logic in-diff\"",
                2,
                "the mixed and fresh cards are marked in-diff",
            ),
            (
                "class=\"cluster-card kind-identical cat-logic\"",
                1,
                "the legacy card keeps the plain class list",
            ),
            (
                "<span class=\"diff-badge\">[in diff]</span>",
                3,
                "one in-diff badge per in-diff occurrence",
            ),
            (
                "<span class=\"diff-badge\">[existing]</span>",
                3,
                "one existing badge per untouched occurrence",
            ),
            (
                "<input type=\"checkbox\" id=\"facet-diff-touched\" \
                 class=\"facet-input facet-diff-touched\" checked>",
                1,
                "the touched facet renders checked",
            ),
            (
                "<input type=\"checkbox\" id=\"facet-diff-untouched\" \
                 class=\"facet-input facet-diff-untouched\" checked>",
                1,
                "the untouched facet renders checked",
            ),
            (">Touched by diff", 1, "facet chip label"),
            (">Untouched by diff", 1, "facet chip label"),
        ],
    );
    assert_page(
        &render_html(&only_changed_report(), None, false),
        &[(
            ONLY_CHANGED_BANNER,
            1,
            "breached diff gate governs the banner although the repo gate is ok",
        )],
    );
    assert_page(
        &render_html(&only_changed_clean_report(), None, false),
        &[(
            ONLY_CHANGED_CLEAN_BANNER,
            1,
            "clean diff gate governs the banner although the repo gate is breached",
        )],
    );
}

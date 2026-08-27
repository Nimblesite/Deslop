//! gh #467 — the two-copy floor of [CLONE-NOISE-LITERAL-VARIATION-CALLS].
//!
//! `test_orders_api.py` holds one function copy-pasted once. Only the
//! route literal differs between the two copies: the same awaited
//! `client.delete`, the same `headers={"X-API-Key": ...}` dictionary, the
//! same `assert resp.status_code == 204`. Nothing in this fixture is an
//! already-extracted helper, so the filter's own stated criterion —
//! "scaffolding has nothing left once its literals are removed"
//! (`docs/specs/noise.md`, [CLONE-NOISE-LITERAL-VARIATION-CALLS]) — is
//! not met here. Strip the literals and an awaited third-party call, a
//! header dictionary and a status assertion are all still standing, and
//! `delete_and_expect_204(client, route, api_key)` deletes the second
//! copy outright. That is duplication a reviewer would send back.
//!
//! The engine agrees, right up until the last step. It builds the
//! cluster, buckets it `nearly_identical` on saturated shape and token
//! evidence, and then the render pass throws it away:
//!
//! ```text
//! DEBUG cluster hidden from report cluster="846e93abd0adbe0c"
//!       bucket="nearly_identical" category="logic" occurrences=2
//!       structural=1.0 token_jaccard=1.0 embedding_cos=0.0
//!       content_agreement=0.9333333333333333
//! ```
//!
//! So the tool reports `duplicated_loc = 0` and `duplication_percent =
//! 0.0` for `test_orders_api.py` — twelve of its twenty-one lines are the
//! repeated unit and the file is called clean ([METRICS-REPO]). The gh
//! #71 family is four copies of this same shape; suppression at **two**
//! is not a family heuristic misfiring on a crowd, it is the shape itself
//! being unreportable. A textbook copy-paste pair is invisible.
//!
//! The fixture stages the byte-identical `settle_ledger` control every
//! noise fixture stages, for the opposite reason. This suite asserts a
//! finding is *present*, so it must also prove the run that failed to
//! produce it could still see a copy it has no doubt about — otherwise a
//! detector that had stopped producing candidates would fail this test
//! with the same message as the defect. Control ranked first, pair
//! second, both published, both counted ([RANK-SCORE], [METRICS-REPO]).

use std::{collections::BTreeSet, path::Path};

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::common::{
    signals::{
        rank_of, signal_dump, ACT_NOW_BUCKETS, ACT_NOW_FUSED, IDENTICAL_BUCKET, REUSE_FUSED,
    },
    verdict::{
        assert_cluster_mentions, assert_type1_identical_signals, duplicated_loc, loc_as_f64,
    },
    *,
};

/// The fixture: one copy-pasted pair, one byte-identical control.
const FIXTURE: &str = "python-copy-paste-pair-invisible";

/// The file holding the pair.
const PAIR_FILE: &str = "test_orders_api.py";

/// The false-negative control staged beside it.
const CONTROL: [&str; 2] = ["control_clone_a.py", "control_clone_b.py"];

/// Every file this report must attribute a duplicated line to.
const DUPLICATED_FILE_NAMES: [&str; 3] = [CONTROL[0], CONTROL[1], PAIR_FILE];

/// What a failure here is about.
const LABEL: &str = "gh #467 copy-pasted endpoint pair";

/// The `min_nodes` the gh #71 pin runs at, so this fixture is a
/// controlled truncation of that family to two members rather than a
/// different measurement of a different thing.
const MIN_NODES: u32 = 4;

/// Every file in the fixture is read and measured.
const FILES_ANALYSED: u64 = 3;

/// Lines in the pair file, and in the two control files together.
const PAIR_FILE_LOC: u64 = 21;
const CONTROL_FILES_LOC: u64 = 27;
const ANALYSED_LOC: u64 = PAIR_FILE_LOC + CONTROL_FILES_LOC;

/// The repeated unit: a six-line function, twice.
const PAIR_LOC: u64 = 12;

/// The control clone: an eight-line function, twice.
const CONTROL_LOC: u64 = 16;

/// Both findings are this report's duplication.
const EXPECTED_DUPLICATED_LOC: u64 = PAIR_LOC + CONTROL_LOC;

/// Two findings, neither of them scaffolding.
const VISIBLE_CLUSTERS: usize = 2;
const NOTHING_HIDDEN: u64 = 0;

/// The cluster the render pass names when it hides one. Read off the
/// run's own trace in the module doc above, so this pins the published
/// cluster to the suppressed one rather than to a lookalike.
const PAIR_CLUSTER_ID: &str = "846e93abd0adbe0c";
const PAIR_BUCKET: &str = "nearly_identical";
const PAIR_CATEGORY: &str = "logic";
const PAIR_SIZE: u64 = 2;
const CONTROL_SIZE: u64 = 2;

/// Where each copy sits: two six-line functions, back to back.
const PAIR_SPANS: [(u64, u64); 2] = [(8, 13), (16, 21)];

/// Rank order ([RANK-SCORE]). The byte-identical control saturates, so it
/// heads the report; the pair follows it. A finding a reader has to
/// scroll past demoted noise to reach is a finding they never read.
const CONTROL_RANK: usize = 0;
const PAIR_RANK: usize = 1;

/// Signals fixed by the fixture bytes. Shape and token evidence both
/// saturate — the two copies are the same subtree — and embeddings are
/// off, so nothing here has a band to hide inside ([FUSED-THRESHOLD]).
const PAIR_SIGNALS: [(&str, f64); 6] = [
    ("structural", 1.0),
    ("token_jaccard", 1.0),
    ("shape", 1.0),
    ("embedding_cos", 0.0),
    ("rename_consistency", 1.0),
    ("literal_fraction", 0.0),
];

/// Content agreement, observed on this exact cluster in the run's own
/// trace (`content_agreement=0.9333333333333333`): the two copies agree
/// on fourteen of fifteen normalised positions.
const PAIR_AGREEMENT: f64 = 14.0 / 15.0;

/// The repeated unit, as a human reads it. Every reported copy must carry
/// all three parts, or what was published is not the duplication.
const AWAITED_CALL: &str = "await client.delete(";
const HEADER_ARGUMENT: &str = "headers={\"X-API-Key\": test_api_key}";
const STATUS_ASSERTION: &str = "assert resp.status_code == 204";

/// Which set of figures a percentage failure is about.
const REPO_SCOPE: &str = "repo";

/// The one thing that differs between the copies.
const ORDERS_ROUTE: &str = "/api/v1/orders/";
const INVOICES_ROUTE: &str = "/api/v1/invoices/";

/// [CLONE-NOISE-LITERAL-VARIATION-CALLS] gh #467. Two copies, one
/// literal apart, no helper to reuse — reported, ranked and counted.
#[test]
fn a_copy_pasted_endpoint_pair_is_reported_beside_the_control() -> Result<()> {
    let scan_root = fixture(FIXTURE);
    let report = run_report(&scan_root, MIN_NODES)?;
    assert_the_whole_fixture_was_analysed(&report);
    let control = expect_surviving_control(&report)?;
    let pair = expect_pair(&report)?;
    assert_nothing_was_suppressed(&report);
    assert_pair_identity(pair);
    assert_pair_occurrences(pair);
    assert_pair_reports_the_copied_code(&scan_root, pair)?;
    assert_pair_signals(pair);
    assert_the_pair_is_actionable(pair);
    assert_ranking(&report, control, pair)?;
    assert_repo_metrics(&report)?;
    assert_pair_file_metrics(&report)?;
    assert_every_copied_file_carries_its_lines(&report);
    Ok(())
}

/// The run reached this fixture and parsed all of it. Asserted before
/// anything about clusters so a mistyped fixture name or an unparsed file
/// fails here, with its own message, instead of masquerading as the
/// suppression under test.
fn assert_the_whole_fixture_was_analysed(report: &Value) {
    assert_eq!(
        (
            field(report, "files_analysed").as_u64(),
            metric_field(report, "analysed_loc").as_u64(),
        ),
        (Some(FILES_ANALYSED), Some(ANALYSED_LOC)),
        "{LABEL}: {PAIR_FILE} ({PAIR_FILE_LOC} lines) and both control files \
         ({CONTROL_FILES_LOC} lines together) must all be read and measured, so \
         every verdict below is a decision the run actually made rather than a \
         file it never opened: {report:#}"
    );
}

/// The false-negative half: the byte-identical copy in this same run
/// survives, whole and saturated.
///
/// `negative_pin`'s control assertion is bound to the "control is the
/// *sole* published cluster" contract every suppression pin asserts. This
/// suite needs the opposite — the control sharing the report with the
/// finding under test — so the bucket, size and occurrence halves are
/// stated here and the full signal vector goes through the shared
/// `assert_type1_identical_signals`.
fn expect_surviving_control(report: &Value) -> Result<&Value> {
    let control = expect_cluster_spanning(report, &CONTROL)?;
    assert_eq!(
        (cluster_bucket(control), cluster_size(control)),
        (IDENTICAL_BUCKET, CONTROL_SIZE),
        "{LABEL}: `settle_ledger` is copied byte for byte into both control \
         files, so `{IDENTICAL_BUCKET}` with both copies shown is the only \
         honest verdict. Anything less and this run had stopped seeing \
         duplication at all, which is not what this test is about: {dump}",
        dump = signal_dump(control),
    );
    assert!(
        !occurrences(control).iter().any(occurrence_is_hidden),
        "{LABEL}: a byte-proven copy may not carry a hidden occurrence: {control:#}"
    );
    assert_type1_identical_signals(control, LABEL);
    Ok(control)
}

/// The defect. The pair is copy-paste duplication with no helper anywhere
/// to reuse, and the report must contain it.
fn expect_pair(report: &Value) -> Result<&Value> {
    assert!(
        cluster_spanning(report, &[PAIR_FILE]).is_some(),
        "{LABEL}: {PAIR_FILE} holds one function copy-pasted once, differing \
         only in a route literal, and this fixture contains no helper either \
         copy could have been calling. Removing the literals leaves \
         `{AWAITED_CALL}`, `{HEADER_ARGUMENT}` and `{STATUS_ASSERTION}` \
         standing, so [CLONE-NOISE-LITERAL-VARIATION-CALLS] has no scaffolding \
         to suppress and the pair must be published. Instead the report \
         contains only: {published:#?}",
        published = visible_cluster_lines(report),
    );
    cluster_spanning(report, &[PAIR_FILE])
        .ok_or_else(|| anyhow!("the pair cluster asserted above is missing"))
}

/// Nothing in this fixture is scaffolding, so the run may not hide
/// anything. The counter is what separates "never clustered" from "built,
/// bucketed, and thrown away at render time".
fn assert_nothing_was_suppressed(report: &Value) {
    assert_eq!(
        clusters_hidden(report),
        NOTHING_HIDDEN,
        "{LABEL}: a non-zero count here is the engine building the pair, \
         bucketing it `{PAIR_BUCKET}`, and the render pass discarding it — the \
         shape was found and then unreported, which is the false negative this \
         fixture exists to name: {report:#}"
    );
}

/// The cluster's own identity: the id the render pass names when it hides
/// one, the bucket it carries, the two members it holds, and the category
/// that says this is logic rather than a data table.
fn assert_pair_identity(pair: &Value) {
    assert_eq!(
        (cluster_id(pair), cluster_bucket(pair), cluster_size(pair)),
        (PAIR_CLUSTER_ID, PAIR_BUCKET, PAIR_SIZE),
        "{LABEL}: the published cluster must be the very one the run hides \
         today — same id, same bucket, both copies: {dump}",
        dump = signal_dump(pair),
    );
    assert_eq!(
        field(pair, "category").as_str(),
        Some(PAIR_CATEGORY),
        "{LABEL}: an awaited request and the assertion on its response are \
         `{PAIR_CATEGORY}`, not a data table: {pair:#}"
    );
}

/// `(path, start_line, end_line)` of every occurrence, in report order.
fn reported_spans(cluster: &Value) -> Vec<(String, u64, u64)> {
    occurrences(cluster)
        .iter()
        .map(|occurrence| {
            (
                field(occurrence, "path").as_str().unwrap_or("?").to_owned(),
                field(occurrence, "start_line").as_u64().unwrap_or_default(),
                field(occurrence, "end_line").as_u64().unwrap_or_default(),
            )
        })
        .collect()
}

/// Both copies, at the lines they actually occupy, and neither of them
/// rendered hidden — a hidden member keeps `size` whole while dropping out
/// of the report a human reads and out of every line metric.
fn assert_pair_occurrences(pair: &Value) {
    let expected: Vec<(String, u64, u64)> = PAIR_SPANS
        .iter()
        .map(|(start, end)| (PAIR_FILE.to_owned(), *start, *end))
        .collect();
    assert_eq!(
        reported_spans(pair),
        expected,
        "{LABEL}: the two copies are six-line functions back to back in \
         {PAIR_FILE}; the report must point a reader at both of them: {pair:#}"
    );
    assert!(
        !occurrences(pair).iter().any(occurrence_is_hidden),
        "{LABEL}: hiding one copy leaves a cluster that reads whole while half \
         of it has vanished from duplicated_loc: {pair:#}"
    );
}

/// What is reported is the duplication itself: both copies carry the whole
/// repeated unit, and between them the two differing routes.
fn assert_pair_reports_the_copied_code(scan_root: &Path, pair: &Value) -> Result<()> {
    let texts = assert_cluster_mentions(scan_root, pair, &[ORDERS_ROUTE, INVOICES_ROUTE])?;
    for text in &texts {
        assert!(
            text.contains(AWAITED_CALL)
                && text.contains(HEADER_ARGUMENT)
                && text.contains(STATUS_ASSERTION),
            "{LABEL}: every reported copy must carry the whole repeated unit — \
             `{AWAITED_CALL}`, `{HEADER_ARGUMENT}` and `{STATUS_ASSERTION}`. A \
             range covering less than that is not what a reader would extract: \
             {text}"
        );
    }
    Ok(())
}

/// Every signal the fixture bytes determine, plus the content agreement
/// the run's own trace records for this cluster.
fn assert_pair_signals(pair: &Value) {
    for (name, expected) in PAIR_SIGNALS {
        assert!(
            approx(signal(pair, name), expected),
            "{LABEL}: signal `{name}` must be {expected}, got {actual}. The two \
             copies are one subtree with one literal changed, so this value is \
             determined by the fixture: {dump}",
            actual = signal(pair, name),
            dump = signal_dump(pair),
        );
    }
    assert!(
        approx(signal(pair, "agreement"), PAIR_AGREEMENT),
        "{LABEL}: content agreement must be {PAIR_AGREEMENT}, got {actual}: {dump}",
        actual = signal(pair, "agreement"),
        dump = signal_dump(pair),
    );
}

/// The agent-facing half ([FUSED-THRESHOLD]). A finding published below
/// the act-now line tells an agent asking `find-similar` to go ahead and
/// write the second copy, which is the same false negative wearing a
/// different label.
fn assert_the_pair_is_actionable(pair: &Value) {
    let fused = signal(pair, "fused");
    assert!(
        ACT_NOW_BUCKETS.contains(&cluster_bucket(pair)),
        "{LABEL}: two copies one literal apart belong in an act-now bucket, got \
         {bucket}: {dump}",
        bucket = cluster_bucket(pair),
        dump = signal_dump(pair),
    );
    assert!(
        fused >= ACT_NOW_FUSED,
        "{LABEL}: fused={fused:.4} must clear the act-now line {ACT_NOW_FUSED}, \
         the stronger of the two agent-facing bars — clearing it clears the \
         reuse-bias line {REUSE_FUSED} with it. Below either, an agent asking \
         find-similar is told to go ahead and author the copy this fixture \
         stages: {dump}",
        dump = signal_dump(pair),
    );
}

/// Rank order, and that these two findings are the whole report.
fn assert_ranking(report: &Value, control: &Value, pair: &Value) -> Result<()> {
    assert_eq!(
        cluster_count(report),
        VISIBLE_CLUSTERS,
        "{LABEL}: the control and the pair are the whole of this report: {lines:#?}",
        lines = visible_cluster_lines(report),
    );
    assert_eq!(
        (rank_of(report, control)?, rank_of(report, pair)?),
        (CONTROL_RANK, PAIR_RANK),
        "{LABEL}: the byte-identical control saturates and heads the report; the \
         pair follows it. A finding a reader reaches only by scrolling past \
         demoted noise is a finding they never read: {lines:#?}",
        lines = visible_cluster_lines(report),
    );
    Ok(())
}

/// [METRICS-REPO] A percentage a reader cannot re-derive is not a
/// transparent figure, so each one asserted here is recomputed from the two
/// counts beside it instead of pasted in as a float.
fn assert_percent_re_derives(
    scope: &str,
    reported: f64,
    duplicated: u64,
    analysed: u64,
) -> Result<()> {
    let expected = 100.0 * loc_as_f64(duplicated)? / loc_as_f64(analysed)?;
    assert!(
        approx(reported, expected),
        "{LABEL}: {scope} duplication_percent must be duplicated/analysed × 100 \
         ({duplicated}/{analysed} = {expected}), got {reported}"
    );
    Ok(())
}

/// The repo totals: both findings' lines, both findings counted, all three
/// copied files named.
fn assert_repo_totals(report: &Value) {
    assert_eq!(
        (
            duplicated_loc(report),
            metric_field(report, "clusters_total").as_u64(),
            metric_field(report, "duplicated_files").as_u64(),
        ),
        (
            EXPECTED_DUPLICATED_LOC,
            u64::try_from(VISIBLE_CLUSTERS).ok(),
            u64::try_from(DUPLICATED_FILE_NAMES.len()).ok(),
        ),
        "{LABEL}: the control's {CONTROL_LOC} lines and the pair's {PAIR_LOC} are \
         both duplication in this repo; charging only the control understates \
         the gate by the whole finding: {lines:#?}",
        lines = visible_cluster_lines(report),
    );
}

/// The repo percentage, re-derived from the totals asserted above.
fn assert_repo_metrics(report: &Value) -> Result<()> {
    assert_repo_totals(report);
    let reported = metric_field(report, "duplication_percent")
        .as_f64()
        .unwrap_or(-1.0);
    assert_percent_re_derives(REPO_SCOPE, reported, EXPECTED_DUPLICATED_LOC, ANALYSED_LOC)
}

/// The `metrics.per_file` row for `name`.
fn per_file_row<'a>(report: &'a Value, name: &str) -> Result<&'a Value> {
    per_file_metrics(report)
        .iter()
        .find(|row| field(row, "path").as_str() == Some(name))
        .ok_or_else(|| anyhow!("no per-file metric row for {name}: {report:#}"))
}

/// The file-level statement of the defect. The repo total cannot say which
/// file earned a line, so a copy-pasted file being called clean is
/// invisible to it.
fn assert_pair_file_metrics(report: &Value) -> Result<()> {
    let row = per_file_row(report, PAIR_FILE)?;
    assert_eq!(
        (
            field(row, "analysed_loc").as_u64(),
            field(row, "duplicated_loc").as_u64(),
        ),
        (Some(PAIR_FILE_LOC), Some(PAIR_LOC)),
        "{LABEL}: {PAIR_LOC} of {PAIR_FILE}'s {PAIR_FILE_LOC} lines are the \
         repeated unit. Reporting 0 duplicated lines calls a copy-pasted file \
         100% clean: {row:#}"
    );
    let reported = field(row, "duplication_percent").as_f64().unwrap_or(-1.0);
    assert_percent_re_derives(PAIR_FILE, reported, PAIR_LOC, PAIR_FILE_LOC)
}

/// Which files the metrics attribute a duplicated line to — all three of
/// them, the pair file included.
fn assert_every_copied_file_carries_its_lines(report: &Value) {
    let counted: BTreeSet<String> = visible_duplicated_lines(report).into_keys().collect();
    let expected: BTreeSet<String> = DUPLICATED_FILE_NAMES
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    assert_eq!(
        counted,
        expected,
        "{LABEL}: every file holding a copy must carry duplicated lines, \
         {PAIR_FILE} included — a file dropped here is a file the duplication \
         gate reads as clean: {lines:#?}",
        lines = visible_cluster_lines(report),
    );
}

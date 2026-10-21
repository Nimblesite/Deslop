//! End-to-end assertions for [METRICS-TOTAL-INCLUDE].

use std::{fs, path::Path};

use anyhow::Result;
use serde_json::Value;

use super::support::{
    assert_contains, deslop_command, outputs_under, read_json_report, run_scan, temp_scan_dir,
    MIN_NODES_FLAG, NO_COLOR_FLAG, REPORT_OUTPUT_STEM,
};
use crate::common::{field, metric_field};

/// A long systematically-renamed pair ([CLONE-BUCKETS-ROUTING] folds it
/// to `nearly_identical`). `for`-shaped so it can never merge with the
/// `while let`-shaped similar pair below.
const NEARLY_SIDE_LEDGER: &str = r"pub fn settle_ledger(entries: &[Entry], ceiling: i64) -> Ledger {
    let mut ledger = Ledger::new();
    let mut carried = 0;
    let mut skipped = 0;
    for entry in entries {
        let amount = entry.amount();
        if amount > ceiling {
            skipped += 1;
            continue;
        }
        carried += amount;
        ledger.record(entry.id(), amount);
        if carried > ceiling {
            ledger.mark_overflow(entry.id());
            carried = ceiling;
        }
        ledger.touch(entry.id());
    }
    ledger.set_carried(carried);
    ledger.set_skipped(skipped);
    ledger
}
";

/// The ledger under a one-for-one rename of every identifier — the
/// other side of the `nearly_identical` relation.
const NEARLY_SIDE_JOURNAL: &str = r"pub fn reconcile_journal(records: &[Record], cap: i64) -> Journal {
    let mut journal = Journal::new();
    let mut accrued = 0;
    let mut dropped = 0;
    for record in records {
        let value = record.value();
        if value > cap {
            dropped += 1;
            continue;
        }
        accrued += value;
        journal.record(record.id(), value);
        if accrued > cap {
            journal.mark_overflow(record.id());
            accrued = cap;
        }
        journal.touch(record.id());
    }
    journal.set_accrued(accrued);
    journal.set_dropped(dropped);
    journal
}
";

/// One side of a copied `while let` drain loop whose second side
/// changes several operations (fresh report, an added `else` arm, a
/// `finish()` exit) on top of the rename — edits past the
/// nearly-identical band, so the pair folds to `loosely_similar`.
const SIMILAR_SIDE_DRAIN: &str = r"pub fn drain_queue(queue: &mut Vec<Task>, budget: i64) -> Summary {
    let mut summary = Summary::new();
    let mut spent = 0;
    while let Some(task) = queue.pop() {
        let cost = task.cost();
        if spent + cost > budget {
            summary.deferred += 1;
            continue;
        }
        spent += cost;
        summary.completed += 1;
        if task.is_large() {
            summary.large += 1;
        }
        summary.earned += task.reward();
    }
    summary.spent = spent;
    summary
}
";

/// The drain loop under a rename plus real operation edits — the
/// `loosely_similar` side of the fixture.
const SIMILAR_SIDE_INBOX: &str = r"pub fn empty_inbox(inbox: &mut Vec<Mail>, allowance: i64) -> Report {
    let mut report = Report::fresh(allowance);
    let mut used = 0;
    while let Some(mail) = inbox.pop() {
        let weight = mail.weight();
        if used + weight > allowance {
            report.postponed += 1;
            continue;
        }
        used += weight;
        report.handled += 1;
        if mail.is_heavy() {
            report.heavy += 1;
        } else {
            report.skipped += 1;
        }
        report.scored += mail.points();
    }
    report.used = used;
    report.finish()
}
";

/// A short byte-identical pair — the `identical` relation, deliberately
/// lighter than both other pairs so the fixture's mass order and kind
/// order disagree.
const IDENTICAL_CHECKSUM: &str = r"pub fn checksum(values: &[i64]) -> i64 {
    let mut hash = 7;
    for value in values {
        hash = hash * 31 + value;
        if hash > 1000000 { hash = hash % 1000003; }
    }
    hash
}
";

/// File names the three relations are seeded into.
const NEARLY_FILES: [&str; 2] = ["ledger.rs", "journal.rs"];
/// `SIMILAR_SIDE_DRAIN` is one line shorter than its edited sibling.
const SIMILAR_FILES: [(&str, &str); 2] = [
    ("drain.rs", SIMILAR_SIDE_DRAIN),
    ("inbox.rs", SIMILAR_SIDE_INBOX),
];
const IDENTICAL_FILES: [&str; 2] = ["sum_a.rs", "sum_b.rs"];

/// Analysed-line counts of each seeded file, hand-derived from the
/// constants above and pinned by the assertions that follow.
const NEARLY_ANALYSED_LINES_PER_FILE: u64 = 22;
const SIMILAR_DRAIN_LINES: u64 = 19;
const SIMILAR_INBOX_LINES: u64 = 21;
const IDENTICAL_LINES_PER_FILE: u64 = 8;
/// 2 × 22 nearly-identical + 19 + 21 similar + 2 × 8 identical lines.
const CORPUS_ANALYSED_LOC: u64 = NEARLY_ANALYSED_LINES_PER_FILE * NEARLY_FILES.len() as u64
    + SIMILAR_DRAIN_LINES
    + SIMILAR_INBOX_LINES
    + IDENTICAL_LINES_PER_FILE * IDENTICAL_FILES.len() as u64;
/// Default include set (`identical` + `nearly_identical`):
/// 2 × 22 + 2 × 8 duplicated lines.
const DEFAULT_TOTAL_DUPLICATED_LOC: u64 = NEARLY_ANALYSED_LINES_PER_FILE
    * NEARLY_FILES.len() as u64
    + IDENTICAL_LINES_PER_FILE * IDENTICAL_FILES.len() as u64;
const DEFAULT_TOTAL_PERCENT: f64 = 60.0;
/// The excluded `loosely_similar` pair still covers its own lines with
/// its own per-kind figures over the same repo denominator.
const SIMILAR_KIND_DUPLICATED_LOC: u64 = SIMILAR_DRAIN_LINES + SIMILAR_INBOX_LINES;
const SIMILAR_KIND_PERCENT: f64 = 40.0;
const NEARLY_KIND_DUPLICATED_LOC: u64 = 44;
const NEARLY_KIND_PERCENT: f64 = 44.0;
const IDENTICAL_KIND_DUPLICATED_LOC: u64 = 16;
const IDENTICAL_KIND_PERCENT: f64 = 16.0;
/// Node floor high enough that only the three whole-function pairs
/// cluster — no intra-file fragments join the report.
pub(crate) const TOTAL_INCLUDE_MIN_NODES: &str = "30";

/// Seeds the three-relation corpus into a scan root. `pub(crate)` so
/// the per-kind threshold suites in `cli::thresholds` drive the same
/// corpus — one fixture, many gates.
pub(crate) fn seed_three_relation_corpus(scan_root: &Path) -> Result<()> {
    fs::write(scan_root.join(NEARLY_FILES[0]), NEARLY_SIDE_LEDGER)?;
    fs::write(scan_root.join(NEARLY_FILES[1]), NEARLY_SIDE_JOURNAL)?;
    for (name, source) in SIMILAR_FILES {
        fs::write(scan_root.join(name), source)?;
    }
    for name in IDENTICAL_FILES {
        fs::write(scan_root.join(name), IDENTICAL_CHECKSUM)?;
    }
    Ok(())
}

/// Seeds the three-relation corpus into a fresh scan root and returns
/// its parsed JSON report.
fn run_total_include_corpus() -> Result<Value> {
    let (tmp, scan_root) = temp_scan_dir("src")?;
    seed_three_relation_corpus(&scan_root)?;
    let out = outputs_under(tmp.path());
    run_scan(
        &scan_root,
        &tmp.path().join(REPORT_OUTPUT_STEM),
        &[MIN_NODES_FLAG, TOTAL_INCLUDE_MIN_NODES],
    )?;
    read_json_report(&out.json)
}

/// Asserts one `metrics.per_kind` row carries the expected figures.
fn assert_per_kind_row(row: &Value, kind: &str, dup: u64, percent: f64, included: bool) {
    assert_eq!(field(row, "kind").as_str(), Some(kind), "per_kind row kind");
    assert_eq!(
        field(row, "duplicated_loc").as_u64(),
        Some(dup),
        "per_kind {kind} duplicated_loc"
    );
    let pct = field(row, "duplication_percent").as_f64().unwrap_or(-1.0);
    assert!(
        (pct - percent).abs() < 0.01,
        "per_kind {kind} percent: got {pct}, want {percent}"
    );
    assert_eq!(
        field(row, "included_in_total").as_bool(),
        Some(included),
        "per_kind {kind} included_in_total"
    );
}

// Implements [METRICS-TOTAL-INCLUDE]: with no configuration the repo
// total counts only `identical` and `nearly_identical` clusters; the
// excluded kinds keep per-kind figures but leave the total, per-file
// and folder numerators untouched.
#[test]
fn default_total_counts_only_identical_and_nearly_identical() -> Result<()> {
    let json = run_total_include_corpus()?;

    // Fixture validity: all three relations must be reported, or the
    // assertions below would pass against a corpus that proves nothing.
    let reported: Vec<String> = crate::common::clusters(&json)
        .iter()
        .map(crate::common::cluster_kind)
        .map(str::to_owned)
        .collect();
    for expected in [
        crate::common::IDENTICAL_KIND,
        crate::common::NEARLY_IDENTICAL_KIND,
        crate::common::LOOSELY_SIMILAR_KIND,
    ] {
        assert!(
            reported.iter().any(|kind| kind == expected),
            "corpus must report a {expected} cluster, got {reported:?}: {json:#}"
        );
    }

    // The headline figures count only the default include set.
    assert_eq!(
        metric_field(&json, "analysed_loc").as_u64(),
        Some(CORPUS_ANALYSED_LOC),
        "every seeded line is analysed: {json:#}"
    );
    assert_eq!(
        metric_field(&json, "duplicated_loc").as_u64(),
        Some(DEFAULT_TOTAL_DUPLICATED_LOC),
        "[METRICS-TOTAL-INCLUDE] default total counts identical + nearly_identical only: {json:#}"
    );
    let pct = metric_field(&json, "duplication_percent")
        .as_f64()
        .unwrap_or(-1.0);
    assert!(
        (pct - DEFAULT_TOTAL_PERCENT).abs() < 0.01,
        "default total percent: got {pct}, want {DEFAULT_TOTAL_PERCENT}: {json:#}"
    );

    // The wire names the include set so clients never re-derive it.
    let includes: Vec<String> = metric_field(&json, "total_includes")
        .as_array()
        .map(|kinds| {
            kinds
                .iter()
                .filter_map(|kind| kind.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        includes,
        vec![
            crate::common::IDENTICAL_KIND.to_owned(),
            crate::common::NEARLY_IDENTICAL_KIND.to_owned(),
        ],
        "[METRICS-TOTAL-INCLUDE] the report states which kinds the total counts"
    );

    assert_default_total_per_kind(&json)?;
    assert_default_total_per_file(&json);
    Ok(())
}

fn assert_default_total_per_kind(json: &Value) -> Result<()> {
    let per_kind = metric_field(json, "per_kind")
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        per_kind.len(),
        3,
        "one per_kind row per reported clone kind: {per_kind:?}"
    );
    let kinds: Vec<&str> = per_kind
        .iter()
        .map(|row| field(row, "kind").as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        kinds,
        [
            crate::common::IDENTICAL_KIND,
            crate::common::NEARLY_IDENTICAL_KIND,
            crate::common::LOOSELY_SIMILAR_KIND,
        ],
        "per_kind rows follow the category display order, Similar below Nearly identical"
    );
    let [identical_row, nearly_row, similar_row] = per_kind.as_slice() else {
        anyhow::bail!("expected 3 per_kind rows: {per_kind:?}");
    };
    assert_per_kind_row(
        identical_row,
        crate::common::IDENTICAL_KIND,
        IDENTICAL_KIND_DUPLICATED_LOC,
        IDENTICAL_KIND_PERCENT,
        true,
    );
    assert_per_kind_row(
        nearly_row,
        crate::common::NEARLY_IDENTICAL_KIND,
        NEARLY_KIND_DUPLICATED_LOC,
        NEARLY_KIND_PERCENT,
        true,
    );
    assert_per_kind_row(
        similar_row,
        crate::common::LOOSELY_SIMILAR_KIND,
        SIMILAR_KIND_DUPLICATED_LOC,
        SIMILAR_KIND_PERCENT,
        false,
    );
    Ok(())
}

fn assert_default_total_per_file(json: &Value) {
    let per_file = metric_field(json, "per_file")
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut by_path: std::collections::HashMap<String, u64> = per_file
        .iter()
        .map(|row| {
            (
                field(row, "path").as_str().unwrap_or_default().to_owned(),
                field(row, "duplicated_loc").as_u64().unwrap_or(0),
            )
        })
        .collect();
    for (name, _) in SIMILAR_FILES {
        assert_eq!(
            by_path.remove(name),
            Some(0),
            "excluded kind leaves the per-file numerator: {per_file:?}"
        );
    }
    for name in NEARLY_FILES {
        assert_eq!(
            by_path.remove(name),
            Some(NEARLY_ANALYSED_LINES_PER_FILE),
            "included kinds keep their per-file lines: {per_file:?}"
        );
    }
    for name in IDENTICAL_FILES {
        assert_eq!(
            by_path.remove(name),
            Some(IDENTICAL_LINES_PER_FILE),
            "included kinds keep their per-file lines: {per_file:?}"
        );
    }
    assert!(
        by_path.is_empty(),
        "no unclaimed per_file rows remain: {by_path:?}"
    );
}

// Implements [METRICS-TOTAL-INCLUDE]: naming every clone kind in
// `[total] include` reproduces the pre-change total exactly — the
// include set partitions the same line projection, so nothing about
// the figure itself moved, only which kinds feed it.
#[test]
fn include_all_kinds_reproduces_the_all_clones_total() -> Result<()> {
    const ALL_KINDS_INCLUDE: &str = "[total]\ninclude = [\"identical\", \"nearly_identical\", \"loosely_similar\", \"same_behavior\"]\n";
    let (tmp, scan_root) = temp_scan_dir("src")?;
    seed_three_relation_corpus(&scan_root)?;
    fs::write(scan_root.join(".deslop.toml"), ALL_KINDS_INCLUDE)?;
    let out = outputs_under(tmp.path());
    run_scan(
        &scan_root,
        &tmp.path().join(REPORT_OUTPUT_STEM),
        &[MIN_NODES_FLAG, TOTAL_INCLUDE_MIN_NODES],
    )?;
    let json = read_json_report(&out.json)?;
    assert_eq!(
        metric_field(&json, "duplicated_loc").as_u64(),
        Some(CORPUS_ANALYSED_LOC),
        "every line of every pair is duplicated under the full include set: {json:#}"
    );
    let pct = metric_field(&json, "duplication_percent")
        .as_f64()
        .unwrap_or(-1.0);
    assert!(
        (pct - 100.0).abs() < 0.01,
        "include-all reproduces the pre-change 100%: got {pct}"
    );
    for row in metric_field(&json, "per_kind")
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        assert_eq!(
            field(&row, "included_in_total").as_bool(),
            Some(true),
            "every reported kind is included: {row:?}"
        );
    }
    Ok(())
}

/// Runs the corpus with `[total] include` pinned to `body` and asserts
/// the run fails with the runtime error code.
fn assert_include_rejected(body: &str) -> Result<()> {
    let (tmp, scan_root) = temp_scan_dir("src")?;
    seed_three_relation_corpus(&scan_root)?;
    fs::write(scan_root.join(".deslop.toml"), body)?;
    let mut cmd = deslop_command(&scan_root, &tmp.path().join(REPORT_OUTPUT_STEM))?;
    let _assertion = cmd
        .args([MIN_NODES_FLAG, TOTAL_INCLUDE_MIN_NODES, NO_COLOR_FLAG])
        .assert()
        .code(1);
    Ok(())
}

// Implements [CLONE-BUCKETS-STRUCTURAL-ONLY]: shape-only is never
// duplication, so `[total] include` refuses it instead of silently
// counting informational lines.
#[test]
fn include_structural_only_is_rejected() -> Result<()> {
    assert_include_rejected("[total]\ninclude = [\"identical\", \"structural_only\"]\n")
}

// Implements [METRICS-TOTAL-INCLUDE]: an unknown kind spelling is a
// config error, not a silent no-op — a typo must not empty the total.
#[test]
fn include_unknown_kind_is_rejected() -> Result<()> {
    assert_include_rejected("[total]\ninclude = [\"identical\", \"exact\"]\n")
}

// Implements [METRICS-TOTAL-INCLUDE]: the human report says which
// kinds the total counts, and marks every excluded kind's group so a
// reader never wonders why its lines are missing from the headline.
#[test]
fn report_text_and_html_mark_the_include_set_and_excluded_kinds() -> Result<()> {
    const TOTAL_COUNTS_LINE: &str = "total counts: Identical code, Nearly identical code";
    const EXCLUDED_CLASS: &str = "clone-group--excluded";
    const EXCLUDED_MARKER: &str = " — not counted in total";
    const INCLUDED_MARKER: &str = "Identical code — 1 group(s)";
    let (tmp, scan_root) = temp_scan_dir("src")?;
    seed_three_relation_corpus(&scan_root)?;
    let out = outputs_under(tmp.path());
    run_scan(
        &scan_root,
        &tmp.path().join(REPORT_OUTPUT_STEM),
        &[MIN_NODES_FLAG, TOTAL_INCLUDE_MIN_NODES],
    )?;
    let text = fs::read_to_string(&out.txt)?;
    assert_contains(
        &text,
        TOTAL_COUNTS_LINE,
        "the text report names the include set",
    );
    let html = fs::read_to_string(&out.html)?;
    assert_contains(
        &html,
        TOTAL_COUNTS_LINE,
        "the HTML banner names the include set",
    );
    // The similar group is the excluded one: greyed class plus words.
    let similar_header = format!(
        "clone-group clone-group--loosely-similar {EXCLUDED_CLASS}\">\
         <summary title=\"Type-3 clone with larger edits\">Similar code — 1 group(s){EXCLUDED_MARKER}</summary>",
    );
    assert_contains(
        &html,
        &similar_header,
        "the excluded kind's group is greyed and says so",
    );
    // Included kinds carry neither the class nor the marker.
    assert!(!html.contains(&format!(
        "clone-group clone-group--identical {EXCLUDED_CLASS}"
    )));
    let identical_header = format!("{INCLUDED_MARKER}</summary>");
    assert!(!html.contains(&format!("{identical_header}{EXCLUDED_MARKER}")));
    // Shape-only was never a candidate, so it is never marked excluded.
    assert!(!html.contains("clone-group--structural-only clone-group--excluded"));
    Ok(())
}

use super::support::*;

/// Writes the cross-file clone-pair fixture into a fresh temp dir, runs
/// `deslop` with the given `--min-nodes` value, asserts the run
/// succeeded, and returns the fixture's analysed-line count alongside the
/// parsed JSON report. The temp dir is dropped on return — the report is
/// already fully materialised in the returned `Value`.
fn run_clone_pair(min_nodes: &str) -> Result<(u64, Value)> {
    let (tmp, scan_root) = temp_scan_dir("src")?;
    let analysed = write_clone_pair(&scan_root)?;
    let out = outputs_under(tmp.path());
    run_scan(
        &scan_root,
        &tmp.path().join("report"),
        &[MIN_NODES_FLAG, min_nodes],
    )?;
    let json = read_json_report(&out.json)?;
    Ok((analysed, json))
}

#[test]
fn metrics_zero_on_empty_corpus() -> Result<()> {
    let (tmp, scan_root) = temp_scan_dir("empty")?;
    let out = outputs_under(tmp.path());
    run_scan(&scan_root, &tmp.path().join("report"), &[])?;
    let json = read_json_report(&out.json)?;
    assert_eq!(metric_field(&json, "analysed_loc").as_u64(), Some(0));
    assert_eq!(metric_field(&json, "duplicated_loc").as_u64(), Some(0));
    assert_eq!(metric_field(&json, "clusters_total").as_u64(), Some(0));
    assert_eq!(metric_field(&json, "duplicated_files").as_u64(), Some(0));
    let pct = metric_field(&json, "duplication_percent")
        .as_f64()
        .unwrap_or(-1.0);
    assert!((0.0..=0.0001).contains(&pct), "percent must be 0: {pct}");
    assert_eq!(threshold_field(&json, "source").as_str(), Some("none"));
    assert!(
        metric_field(&json, "per_file")
            .as_array()
            .is_some_and(Vec::is_empty),
        "empty corpus yields no per_file rows"
    );
    Ok(())
}

// Implements [METRICS-REPO]: the `per_file` breakdown sums back to the
// repo aggregate, carries each file's own exact percentage, and arrives
// sorted worst-first on the wire.
#[test]
fn metrics_per_file_breakdown_matches_repo_totals() -> Result<()> {
    let (analysed, json) = run_clone_pair("8")?;
    let per_file = metric_field(&json, "per_file")
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        per_file.len(),
        2,
        "both fixture files must appear in per_file: {per_file:?}"
    );
    let analysed_sum: u64 = per_file
        .iter()
        .map(|entry| field(entry, "analysed_loc").as_u64().unwrap_or(0))
        .sum();
    assert_eq!(
        analysed_sum, analysed,
        "per_file analysed_loc must sum to the repo total: {per_file:?}"
    );
    let repo_dup = metric_field(&json, "duplicated_loc").as_u64().unwrap_or(0);
    let dup_sum: u64 = per_file
        .iter()
        .map(|entry| field(entry, "duplicated_loc").as_u64().unwrap_or(0))
        .sum();
    assert_eq!(
        dup_sum, repo_dup,
        "per_file duplicated_loc must sum to the repo total: {per_file:?}"
    );
    for entry in &per_file {
        assert_per_file_entry(entry);
    }
    let first = field(
        per_file.first().unwrap_or(&Value::Null),
        "duplication_percent",
    )
    .as_f64()
    .unwrap_or(0.0);
    let last = field(
        per_file.last().unwrap_or(&Value::Null),
        "duplication_percent",
    )
    .as_f64()
    .unwrap_or(0.0);
    assert!(
        first >= last,
        "per_file must be sorted worst-first: {per_file:?}"
    );
    Ok(())
}

/// Issue #286: `per_file[].path` is documented as relative to the scan
/// root — the same contract every occurrence path already honours — but
/// was emitted absolute. That inflates every JSON report by the length of
/// the scan root times the file count, and writes the user's home
/// directory into a file agents copy between machines.
#[test]
fn metrics_per_file_paths_are_scan_root_relative() -> Result<()> {
    let (_analysed, json) = run_clone_pair("8")?;
    let mut paths: Vec<String> = metric_field(&json, "per_file")
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| field(entry, "path").as_str().map(str::to_owned))
        .collect();
    paths.sort();
    assert_eq!(
        paths,
        vec!["Alpha.cs".to_owned(), "Beta.cs".to_owned()],
        "per_file paths must be scan-root-relative, exactly as occurrence paths are"
    );
    Ok(())
}

/// Asserts one `per_file` row: a `.cs` path, non-zero duplication for
/// this cross-file clone fixture, and a percentage computed against the
/// file's own analysed-line denominator.
fn assert_per_file_entry(entry: &Value) {
    let dup = field(entry, "duplicated_loc").as_u64().unwrap_or(0);
    let file_analysed = field(entry, "analysed_loc").as_u64().unwrap_or(0);
    assert!(dup > 0, "both clone files must duplicate: {entry:?}");
    assert!(
        field(entry, "path")
            .as_str()
            .is_some_and(|path| path.contains(".cs")),
        "every FileMetric carries a source path: {entry:?}"
    );
    let dup32 = u32::try_from(dup).unwrap_or(u32::MAX);
    let analysed32 = u32::try_from(file_analysed).unwrap_or(u32::MAX);
    let expected = if analysed32 == 0 {
        0.0
    } else {
        f64::from(dup32) / f64::from(analysed32) * 100.0
    };
    let pct = field(entry, "duplication_percent").as_f64().unwrap_or(-1.0);
    assert!(
        (pct - expected).abs() < 0.01,
        "per-file percent uses the file's own denominator: got {pct}, want {expected}: {entry:?}"
    );
}

// --- [METRICS-TOTAL-INCLUDE] --------------------------------------------
// Which clone kinds count toward the repo total is a configuration
// choice, not a fixed engine fact. The default include set is the two
// byte-level kinds — `identical` and `nearly_identical` — so a repo's
// headline duplication figure describes copy-paste, not loose family
// resemblance. Excluded kinds keep their clusters and their per-kind
// figures; their lines simply leave the total, per-file and folder
// numerators.

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
    assert_eq!(
        by_path.remove(SIMILAR_FILES[0].0),
        Some(0),
        "excluded kind leaves the per-file numerator: {per_file:?}"
    );
    assert_eq!(
        by_path.remove(SIMILAR_FILES[1].0),
        Some(0),
        "excluded kind leaves the per-file numerator: {per_file:?}"
    );
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
        "clone-group clone-group--loosely-similar {EXCLUDED_CLASS}\" open>\
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

// Implements [METRICS-REPO]: duplicated_loc on a hand-counted fixture
// matches the lines covered by at least two non-hidden occurrences.
#[test]
fn metrics_match_hand_counted_fixture() -> Result<()> {
    let (analysed, json) = run_clone_pair("8")?;
    let metrics = field(&json, "metrics").clone();
    assert_eq!(
        metric_field(&json, "analysed_loc").as_u64(),
        Some(analysed),
        "analysed_loc mismatch: {metrics}"
    );
    let dup = metric_field(&json, "duplicated_loc").as_u64().unwrap_or(0);
    assert!(dup > 0, "duplicated_loc must exceed zero: {metrics}");
    assert!(
        dup <= analysed,
        "duplicated_loc {dup} cannot exceed analysed {analysed}",
    );
    let dup_files = metric_field(&json, "duplicated_files")
        .as_u64()
        .unwrap_or(0);
    assert!(
        dup_files >= 2,
        "both fixture files should contribute: {metrics}"
    );
    let clusters = field(&metrics, "clusters_total").as_u64().unwrap_or(0);
    assert!(clusters >= 1, "at least one cluster expected: {metrics}");
    Ok(())
}

// Implements [METRICS-REPO]: hidden occurrences (report_hide) do not
// count toward duplicated_loc. Hiding one of a two-file cross-file
// clone pair must shrink the metric and drop the hidden file from
// `duplicated_files`.
#[test]
fn metrics_exclude_hidden_occurrences() -> Result<()> {
    // Baseline without any hide policy.
    let (tmp_plain, plain_root) = temp_scan_dir("src")?;
    let _analysed = write_clone_pair(&plain_root)?;
    let plain_out = outputs_under(tmp_plain.path());
    run_scan(
        &plain_root,
        &tmp_plain.path().join("report"),
        &[MIN_NODES_FLAG, MIN_NODES_VALUE],
    )?;
    let plain_metrics = field(&read_json_report(&plain_out.json)?, "metrics").clone();
    let plain_dup = field(&plain_metrics, "duplicated_loc")
        .as_u64()
        .unwrap_or(0);
    let plain_files = field(&plain_metrics, "duplicated_files")
        .as_u64()
        .unwrap_or(0);
    assert!(
        plain_dup > 0 && plain_files >= 2,
        "baseline must cover both files: {plain_metrics}"
    );

    // With Alpha.cs report_hidden: metric shrinks, hidden file drops
    // out of `duplicated_files`.
    let (tmp, scan_root) = temp_scan_dir("src")?;
    let _analysed = write_clone_pair(&scan_root)?;
    fs::write(
        scan_root.join(".deslop.toml"),
        "[defaults]\nreport_hide = [\"**/Alpha.cs\"]\n",
    )?;
    let out = outputs_under(tmp.path());
    run_scan(
        &scan_root,
        &tmp.path().join("report"),
        &[MIN_NODES_FLAG, MIN_NODES_VALUE],
    )?;
    let metrics = field(&read_json_report(&out.json)?, "metrics").clone();
    let hidden_dup = field(&metrics, "duplicated_loc").as_u64().unwrap_or(0);
    let hidden_files = field(&metrics, "duplicated_files").as_u64().unwrap_or(0);
    assert!(
        hidden_dup < plain_dup,
        "hiding Alpha.cs must shrink duplicated_loc: plain={plain_dup} hidden={hidden_dup}: {metrics}",
    );
    assert!(
        hidden_files <= 1,
        "hidden files must not appear in duplicated_files: {metrics}"
    );
    Ok(())
}

// Implements [METRICS-REPO]: overlapping sibling-extension ranges count
// once per line. Two files with two clone pairs at different sizes must
// produce duplicated_loc <= lines in the files, never 2x that.
#[test]
fn metrics_deduplicate_overlapping_sibling_ranges() -> Result<()> {
    let (analysed, json) = run_clone_pair("4")?;
    let metrics = field(&json, "metrics").clone();
    let dup = field(&metrics, "duplicated_loc").as_u64().unwrap_or(0);
    assert!(
        dup <= analysed,
        "duplicated_loc {dup} must never exceed analysed {analysed} — \
         sibling-extension windows must be deduplicated per file: {metrics}"
    );
    Ok(())
}

// Implements [EXIT-CODES]: --fail-over 0.0 is breached by any
// duplication and the CLI exits 3 with the report on disk.
#[test]
fn fail_over_cli_exits_three_on_breach() -> Result<()> {
    let (tmp, scan_root) = temp_scan_dir("src")?;
    let _analysed = write_clone_pair(&scan_root)?;
    let out = outputs_under(tmp.path());
    let mut cmd = deslop_command(&scan_root, &tmp.path().join("report"))?;
    let _assertion = cmd
        .args([
            MIN_NODES_FLAG,
            MIN_NODES_VALUE,
            "--fail-over",
            "0",
            NO_COLOR_FLAG,
        ])
        .assert()
        .code(3);
    assert!(
        out.json.exists(),
        "report must land on disk before exit 3: {}",
        out.json.display()
    );
    let json = read_json_report(&out.json)?;
    assert_eq!(threshold_field(&json, "source").as_str(), Some("cli"));
    assert_eq!(threshold_field(&json, "breached").as_bool(), Some(true));
    Ok(())
}

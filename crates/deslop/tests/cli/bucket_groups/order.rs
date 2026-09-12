//! [CLONE-KIND-LABELS] The category display order, on the rendered report.
//!
//! The kind table in `docs/specs/taxonomy.md` is the display order:
//! Identical, Nearly identical, Same behavior, Similar, then shape-only.
//! [CLONE-BUCKETS-STRUCTURAL-ONLY] strengthens the tail of it — shape-only
//! "is always the last category, below Similar, regardless of its size or
//! number of matches".
//!
//! Order by mass would be a different order. A reader who opens the report
//! and sees "Similar code" above "Identical code" cannot tell whether the
//! categories are ranked by relation strength or by size, and the strongest
//! evidence in the repository is no longer the first thing on the page.
//! This module pins the rendered order against a corpus built so the two
//! orders disagree.

use super::*;
use crate::common::{cluster_kind, clusters, LOOSELY_SIMILAR_TITLE, SAME_BEHAVIOR_TITLE};

/// A long renamed pair. Both sides run the same accumulate-with-ceiling
/// loop over the same operations; only the names differ, so the pair
/// folds to `nearly_identical` ([CLONE-BUCKETS-ROUTING]). It is long
/// enough to outrank the short exact pair below on mass.
const HEAVY_RENAMED_A: &str = r"pub fn settle_ledger(entries: &[Entry], ceiling: i64) -> Ledger {
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

/// The same work under a systematic rename: every identifier, parameter
/// and type is substituted one-for-one, while the operations called stay
/// the same. [CLONE-BUCKETS-NORTH-STAR] — "a systematic rename or
/// parameter substitution is still a copy" — puts this in Nearly
/// identical, and its length puts it above the exact pair on mass.
const HEAVY_RENAMED_B: &str = r"pub fn reconcile_journal(records: &[Record], cap: i64) -> Journal {
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

/// A short byte-identical pair — `identical` by [CLONE-BUCKETS-IDENTICAL],
/// and deliberately far lighter than the renamed pair above, so ranking by
/// mass and ordering by category disagree.
const LIGHT_IDENTICAL: &str = r"pub fn checksum(values: &[i64]) -> i64 {
    let mut hash = 7;
    for value in values {
        hash = hash * 31 + value;
    }
    hash
}
";

/// Files the three relations are seeded into.
const HEAVY_FILES: [(&str, &str); 2] = [
    ("ledger.rs", HEAVY_RENAMED_A),
    ("journal.rs", HEAVY_RENAMED_B),
];
const LIGHT_FILES: [&str; 2] = ["sum_a.rs", "sum_b.rs"];
const SHAPE_ONLY_FILE: &str = "shapes.rs";
const MIXED_DIR: &str = "category_order";

/// The opening of one kind's collapsible group, which is what fixes its
/// position on the page.
const GROUP_OPEN: &str = "<details class=\"clone-group";

/// [CLONE-KIND-LABELS] The category display order, strongest relation
/// first and the informational non-clone last.
const CATEGORY_DISPLAY_ORDER: [&str; 5] = [
    IDENTICAL_TITLE,
    NEARLY_IDENTICAL_TITLE,
    SAME_BEHAVIOR_TITLE,
    LOOSELY_SIMILAR_TITLE,
    STRUCTURAL_ONLY_TITLE,
];

/// The kind titles the report opened a group for, in the order they
/// appear down the page. Read from the rendered group headers, so the
/// assertion sees exactly what a reader scrolling the report sees.
fn group_titles_in_page_order(html: &str) -> Vec<String> {
    html.split(GROUP_OPEN)
        .skip(1)
        .filter_map(|group| {
            let (_, after_summary_tag) = group.split_once("\">")?;
            let (title, _) = after_summary_tag.split_once(" — ")?;
            Some(title.to_owned())
        })
        .collect()
}

/// Seeds the exact, renamed and shape-only relations into one scan root
/// and renders every report surface.
fn render_mixed_report() -> Result<(String, Value)> {
    let (tmp, scan_root, out) = seeded_scan(MIXED_DIR, |root| {
        for (name, source) in HEAVY_FILES {
            fs::write(root.join(name), source)?;
        }
        for name in LIGHT_FILES {
            fs::write(root.join(name), LIGHT_IDENTICAL)?;
        }
        fs::write(root.join(SHAPE_ONLY_FILE), super::information::SHAPE_ONLY_DISJOINT)?;
        Ok(())
    })?;
    let mut cmd = deslop_command(&scan_root, &tmp.path().join(REPORT_OUTPUT_STEM))?;
    let _assertion = cmd
        .args([MIN_NODES_FLAG, MIN_NODES_VALUE, NO_COLOR_FLAG])
        .assert()
        .success();
    Ok((fs::read_to_string(&out.html)?, read_json_report(&out.json)?))
}

// [CLONE-KIND-LABELS] [CLONE-BUCKETS-STRUCTURAL-ONLY] The report lists
// categories in relation-strength order, not in mass order.
#[test]
fn html_groups_follow_the_category_display_order_not_the_mass_ranking() -> Result<()> {
    let (html, json) = render_mixed_report()?;

    // The corpus must actually disagree with the display order, or the
    // assertion below proves nothing: the heavy renamed pair has to
    // outrank the light exact pair on mass ([RANK-MASS-SUM]).
    let ranked: Vec<&str> = clusters(&json).iter().map(cluster_kind).collect();
    assert!(
        ranked.contains(&IDENTICAL_KIND) && ranked.contains(&NEARLY_IDENTICAL_KIND),
        "the corpus must fold both an exact and a renamed relation: {json:#}"
    );
    let first_identical = ranked.iter().position(|kind| *kind == IDENTICAL_KIND);
    let first_renamed = ranked.iter().position(|kind| *kind == NEARLY_IDENTICAL_KIND);
    assert!(
        first_renamed < first_identical,
        "this corpus is built so mass order and category order disagree: the \
         long renamed pair must outrank the short exact pair, got {ranked:?} \
         in {json:#}"
    );

    // The page, however, leads with the strongest relation.
    let rendered = group_titles_in_page_order(&html);
    assert!(
        rendered.len() >= 3,
        "the corpus renders an exact, a renamed and an informational group, \
         got {rendered:?} in: {html}"
    );
    let expected: Vec<&str> = CATEGORY_DISPLAY_ORDER
        .into_iter()
        .filter(|title| rendered.iter().any(|shown| shown == title))
        .collect();
    assert_eq!(
        rendered, expected,
        "[CLONE-KIND-LABELS] the kind table is the category display order; \
         the report must not reorder categories by mass"
    );
    assert_eq!(
        rendered.first().map(String::as_str),
        Some(IDENTICAL_TITLE),
        "byte-identical code is the strongest evidence in the repository and \
         leads the report, however light it is: {rendered:?}"
    );
    assert_eq!(
        rendered.last().map(String::as_str),
        Some(STRUCTURAL_ONLY_TITLE),
        "[CLONE-BUCKETS-STRUCTURAL-ONLY] shape-only is always the last \
         category, below Similar, regardless of its size: {rendered:?}"
    );

    // Ordering is presentation: it moves no duplication figure.
    assert_eq!(
        metric_field(&json, "clusters_total").as_u64(),
        Some(ranked.iter().filter(|kind| **kind != STRUCTURAL_ONLY_KIND).count() as u64),
        "only clones are counted as clone groups, whatever order they render \
         in: {json:#}"
    );
    Ok(())
}

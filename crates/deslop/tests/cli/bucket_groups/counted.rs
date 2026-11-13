//! [METRICS-TOTAL-COUNTED] Every kind group says the same way whether its
//! lines are in the headline figure.
//!
//! The Duplication panel marked "Similar code" as excluded and left
//! "Same shape, different content" unmarked and expanded, though neither
//! counts (gh #633). A group either counts or it does not: one marker,
//! one collapse rule, and the kinds that count come first.

use anyhow::Context;

use super::*;
use crate::{
    cli::total_include::{seed_three_relation_corpus, TOTAL_INCLUDE_MIN_NODES},
    common::LOOSELY_SIMILAR_TITLE,
};

/// Only Similar code counts: the reverse of the default include set.
const SIMILAR_ONLY_INCLUDE: &str = "[total]\ninclude = [\"loosely_similar\"]\n";
const NOT_COUNTED_CLASS: &str = " clone-group--excluded";
const NOT_COUNTED_MARKER: &str = " — not counted in total";
const SIMILAR_TAXONOMY: &str = "Type-3 clone with larger edits";
const IDENTICAL_TAXONOMY: &str = "Type-1 exact clone";
const NEARLY_IDENTICAL_TAXONOMY: &str = "Type-2 / close Type-3 clone";
const SHAPE_ONLY_TAXONOMY: &str = "Informational non-clone";
const ONE_GROUP: usize = 1;

/// The header of a kind group the total does not count: greyed, marked
/// in words, and collapsed.
pub(super) fn not_counted_expander(css_suffix: &str, taxonomy: &str, title: &str) -> String {
    format!(
        "<details class=\"clone-group clone-group--{css_suffix}{NOT_COUNTED_CLASS}\">\
         <summary title=\"{taxonomy}\">{title} — {ONE_GROUP} group(s){NOT_COUNTED_MARKER}</summary>"
    )
}

/// Byte offset of `needle` in `html`, failing with the marker it sought.
fn position(html: &str, needle: &str) -> Result<usize> {
    html.find(needle)
        .with_context(|| format!("the report must contain {needle}"))
}

/// With only Similar code in the include set, its group is open, unmarked
/// and first; the two stronger kinds are marked, collapsed and after it.
#[test]
fn groups_the_total_counts_lead_and_the_rest_are_marked_and_collapsed() -> Result<()> {
    let (tmp, scan_root) = temp_scan_dir("src")?;
    seed_three_relation_corpus(&scan_root)?;
    fs::write(scan_root.join(".deslop.toml"), SIMILAR_ONLY_INCLUDE)?;
    let out = outputs_under(tmp.path());
    run_scan(
        &scan_root,
        &tmp.path().join(REPORT_OUTPUT_STEM),
        &[MIN_NODES_FLAG, TOTAL_INCLUDE_MIN_NODES],
    )?;
    let html = fs::read_to_string(&out.html)?;
    let similar = position(
        &html,
        &kind_expander(
            "loosely-similar",
            SIMILAR_TAXONOMY,
            LOOSELY_SIMILAR_TITLE,
            ONE_GROUP,
        ),
    )?;
    let identical = position(
        &html,
        &not_counted_expander("identical", IDENTICAL_TAXONOMY, IDENTICAL_TITLE),
    )?;
    let nearly = position(
        &html,
        &not_counted_expander(
            "nearly-identical",
            NEARLY_IDENTICAL_TAXONOMY,
            NEARLY_IDENTICAL_TITLE,
        ),
    )?;
    assert!(
        similar < identical && identical < nearly,
        "counted kinds lead, the rest keep display order: {similar} {identical} {nearly}"
    );
    Ok(())
}

/// The shape-only group header, as every report must write it.
pub(super) fn shape_only_expander() -> String {
    not_counted_expander(
        "structural-only",
        SHAPE_ONLY_TAXONOMY,
        STRUCTURAL_ONLY_TITLE,
    )
}

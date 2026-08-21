//! [PIPELINE-NORMALIZE-AST] — an operator is behaviour, and two
//! functions that differ only in their operators are not duplicates.
//!
//! Normalisation walked `named_children` alone. In every grammar here
//! the operator of a binary expression is an *anonymous* token, so
//! `alpha + beta` and `alpha - beta` normalised to the same subtree with
//! the same identifier frontier and the same literals: nothing
//! downstream had any evidence they differed. The pair rendered
//! `structural = 1.00`, `token_jaccard = 1.00`, `agreement = 1.00` and
//! `fused = 1.00` — the engine's strongest possible claim, made about
//! code that computes a different answer.
//!
//! That is the [FUSED-THRESHOLD] act-now line, so a `find-similar`
//! consumer is told to reuse one where the other is meant, and an agent
//! following `docs/snippets/agents-md-recipe.md` deletes a subtraction
//! in favour of an addition. Sign errors, inverted comparisons and
//! inverted boolean guards are exactly the defects that survive review.
//!
//! Three families are pinned, because three different grammar
//! productions carry the token: arithmetic (`binary_operator`),
//! comparison (`comparison_operator`) and boolean
//! (`boolean_operator`).
//!
//! The contract is *not* "never cluster them". Two functions that share
//! a shape and differ in an operator may well be worth a reader's
//! attention. The contract is that the report must not claim the
//! content is duplicated: the pair must stay out of the act-now
//! buckets, and its rendered confidence must stay under the act-now
//! line.
//!
//! # Why this fixture cannot pass by going blind
//!
//! `control_alpha.py` / `control_beta.py` hold a byte-identical
//! function. Any normalisation change wide enough to stop the operator
//! families clustering also has to leave that pair `identical` and
//! saturated, asserted in the same run.

use serde_json::Value;

mod common;
use crate::common::{signals::*, *};

/// Node floor low enough that each four-statement function body is a
/// candidate window.
const MIN_NODES: u32 = 8;

/// The three operator families, as `(label, left file, right file)`.
const FAMILIES: [(&str, &str, &str); 3] = [
    ("arithmetic + / -", "arithmetic_add.py", "arithmetic_sub.py"),
    ("comparison == / !=", "comparison_eq.py", "comparison_ne.py"),
    ("boolean and / or", "boolean_and.py", "boolean_or.py"),
];

/// The byte-identical pair that must survive whatever separates the
/// families above.
const CONTROL: [&str; 2] = ["control_alpha.py", "control_beta.py"];

/// Renders the fixture.
fn render() -> Result<Value> {
    run_report(&fixture("operator-drift"), MIN_NODES)
}

/// Every visible cluster as `id [bucket] fused files`.
fn published(report: &Value) -> Vec<String> {
    clusters(report)
        .iter()
        .map(|cluster| {
            format!(
                "{id} [{bucket}] fused={fused:.4} {files:?}",
                id = cluster_id(cluster),
                bucket = cluster_bucket(cluster),
                fused = signal(cluster, "fused"),
                files = occurrence_files(cluster),
            )
        })
        .collect()
}

// The defect: an operator-only difference must not be published as
// duplicated content, in any of the three grammar productions that
// carry the token.
#[test]
fn an_operator_only_difference_never_reaches_the_act_now_line() -> Result<()> {
    let report = render()?;
    for (label, left, right) in FAMILIES {
        let Some(cluster) = cluster_spanning(&report, &[left, right]) else {
            continue;
        };
        let dump = signal_dump(cluster);
        assert!(
            !ACT_NOW_BUCKETS.contains(&cluster_bucket(cluster)),
            "{label}: {left} and {right} compute different answers. A \
             cluster in an act-now bucket tells a `find-similar` consumer \
             to write one where the other is meant — {dump}"
        );
        assert!(
            signal(cluster, "fused") < ACT_NOW_FUSED,
            "{label}: rendered confidence {fused:.4} is at or above the \
             act-now line of {ACT_NOW_FUSED}; the engine is making its \
             strongest claim about code whose behaviour differs — {dump}",
            fused = signal(cluster, "fused"),
        );
        assert!(
            signal(cluster, "agreement") < 1.0,
            "{label}: `agreement` is the measured proof that the members \
             share their content, and these members do not — a saturated \
             agreement here is the measurement itself going blind to the \
             operator — {dump}"
        );
    }
    Ok(())
}

// The false-negative control, in the same run: a real byte-identical
// clone still ranks, still says `identical`, and still saturates.
#[test]
fn the_byte_identical_control_survives_in_the_same_run() -> Result<()> {
    let report = render()?;
    let control = expect_cluster_spanning(&report, &CONTROL)?;
    let dump = signal_dump(control);
    assert_eq!(
        cluster_bucket(control),
        "identical",
        "the control is copied byte for byte — {dump}"
    );
    assert_eq!(
        cluster_size(control),
        2,
        "both copies of the control must be shown — {dump}"
    );
    assert!(
        approx(signal(control, "fused"), 1.0) && approx(signal(control, "agreement"), 1.0),
        "byte-proven duplication saturates confidence and agreement; a fix \
         that lowered every score has distinguished nothing — {dump}"
    );
    Ok(())
}

// Neither operator family may be published as the report's worst
// offender while a real clone sits in the same run.
#[test]
fn the_real_clone_outranks_every_operator_family() -> Result<()> {
    let report = render()?;
    let control = expect_cluster_spanning(&report, &CONTROL)?;
    assert_eq!(
        cluster_id(clusters(&report).first().unwrap_or(&Value::Null)),
        cluster_id(control),
        "the one real duplication in this corpus must rank first: \
         {published:#?}",
        published = published(&report),
    );
    Ok(())
}

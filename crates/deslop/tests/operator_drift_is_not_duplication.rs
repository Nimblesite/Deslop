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

use crate::common::{signals::*, *};

/// Node floor low enough that each four-statement function body is a
/// candidate window.
const MIN_NODES: u32 = 8;

/// The three operator families, as
/// `(label, left file, right file, is published)`.
///
/// The fourth column is the family's **pinned disposition**, and it is
/// what stops this suite passing by disappearance. Measured on the
/// fixture, all three families reach the report as nothing at all, with
/// `clusters_hidden` at 0 — so "no cluster" is the current, deliberate
/// answer and is asserted as such. Skipping a missing family instead, as
/// this test did, made a recall hole and a correct exclusion look
/// identical: every operator assertion below was unreachable and the
/// suite was green while pinning nothing.
///
/// A family that starts publishing flips its column and fails here, which
/// is the point: whether these pairs should be shown at all is a judgement
/// for a person, and the signal assertions below still bound what may be
/// claimed if one ever is.
const FAMILIES: [Family; 3] = [
    Family {
        label: "arithmetic + / -",
        left: "arithmetic_add.py",
        right: "arithmetic_sub.py",
        left_operator: "__op__+",
        right_operator: "__op__-",
        is_published: false,
    },
    Family {
        label: "comparison == / !=",
        left: "comparison_eq.py",
        right: "comparison_ne.py",
        left_operator: "__op__==",
        right_operator: "__op__!=",
        is_published: false,
    },
    Family {
        label: "boolean and / or",
        left: "boolean_and.py",
        right: "boolean_or.py",
        left_operator: "__op__and",
        right_operator: "__op__or",
        is_published: false,
    },
];

/// One operator family: the pair, the operator leaf each member must
/// normalise to, and whether the report publishes the pair.
struct Family {
    /// Human label used in every assertion message.
    label: &'static str,
    /// The member holding the first operator.
    left: &'static str,
    /// The member holding the second.
    right: &'static str,
    /// The normalised leaf `left` must carry, and `right` must not.
    left_operator: &'static str,
    /// The normalised leaf `right` must carry, and `left` must not.
    right_operator: &'static str,
    /// The pinned disposition: whether a cluster spans the pair.
    is_published: bool,
}

/// The byte-identical pair that must survive whatever separates the
/// families above.
const CONTROL: [&str; 2] = ["control_alpha.py", "control_beta.py"];

/// Every file the fixture holds: three operator pairs plus the control.
///
/// Asserted per run so "the family published nothing" can only ever mean
/// the pair was analysed and excluded. The control proves the *run* was
/// still detecting; this proves these particular files reached the
/// pipeline at all, which a control in other files cannot say.
const FIXTURE_FILE_COUNT: u64 = 8;

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
//
// Every family is judged in the same run as the byte-identical control,
// and the control is asserted *here* rather than only in its own test.
// Absence is a legitimate disposition for these pairs — they compute
// different answers, so publishing nothing is a correct answer — but
// absence alone proves nothing on its own: a detector that had stopped
// producing candidates, a filter widened until it ate real code, or a
// normalisation change that dropped the whole fixture would all show the
// same empty result. The control is what separates "considered and
// excluded" from "never looked". Before this, the loop `continue`d on a
// missing cluster and the three families it exists to pin were asserted
// against nothing at all; measured on the fixture, all three were absent
// and `clusters_hidden` was 0, so the test was green while pinning
// nothing.
#[test]
fn an_operator_only_difference_never_reaches_the_act_now_line() -> Result<()> {
    let report = render()?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FIXTURE_FILE_COUNT),
        "every operator pair must reach the pipeline before its absence from \
         the report can mean anything: {published:#?}",
        published = published(&report),
    );
    let control = expect_cluster_spanning(&report, &CONTROL)?;
    assert_eq!(
        cluster_bucket(control),
        "identical",
        "the byte-identical control must still be published as duplication in \
         this very run — without it, every assertion below is satisfied just \
         as well by a detector that produced no candidates: {published:#?}",
        published = published(&report),
    );
    assert!(
        approx(signal(control, "agreement"), 1.0),
        "the control's agreement must stay saturated in this run — a \
         separation that lowered every score has distinguished nothing: {dump}",
        dump = signal_dump(control),
    );
    for family in FAMILIES {
        let Family {
            label,
            left,
            right,
            is_published,
            ..
        } = family;
        let found = cluster_spanning(&report, &[left, right]);
        assert_eq!(
            found.is_some(),
            is_published,
            "{label}: {left} and {right} changed disposition. The pinned answer \
             is {is_published}, the run says {actual}. Either the pair started \
             being published — read the signals below and decide whether that \
             is right — or it stopped, which is a recall hole this suite exists \
             to catch: {published:#?}",
            actual = found.is_some(),
            published = published(&report),
        );
        let Some(cluster) = found else { continue };
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

/// The `--debug-ast` dump for one fixture file, as the exact sequence of
/// normalised node kinds it names.
///
/// The dump prints one node per line as `<indent><kind> [start..end]`, so
/// the kind is everything between the indent and the span. Reading it
/// this way is what makes the assertions below exact: a `contains` test
/// for `"__op__+"` is also satisfied by `__op__++` and `__op__+=`, and a
/// grammar that started emitting the wrong one of those would have gone
/// unnoticed while the fixture stayed green.
fn debug_ast_kinds(file_name: &str) -> Result<Vec<String>> {
    let output = assert_cmd::Command::cargo_bin("deslop")?
        .arg("--debug-ast")
        .arg(fixture("operator-drift").join(file_name))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    Ok(String::from_utf8(output)?
        .lines()
        .filter_map(|line| line.trim_start().split_once(" ["))
        .map(|(kind, _span)| kind.to_owned())
        .collect())
}

/// Every operator leaf one file normalises to, in dump order.
fn operator_leaves(file_name: &str) -> Result<Vec<String>> {
    Ok(debug_ast_kinds(file_name)?
        .into_iter()
        .filter(|kind| kind.starts_with(OPERATOR_KIND_PREFIX))
        .collect())
}

/// The namespace every operator leaf is emitted behind.
const OPERATOR_KIND_PREFIX: &str = "__op__";

/// Operator occurrences each family member carries. Every fixture body
/// is four statements over one operator, so this is also the proof that
/// *every* occurrence normalised — one surviving occurrence would
/// satisfy a mere presence check while the other three were dropped.
const OPERATOR_LEAVES_PER_MEMBER: usize = 4;

// The operator-specific non-vacuity proof, on the pairs themselves.
//
// The disposition pin above says each family publishes nothing, and the
// control says the run was still detecting — but neither shows that these
// particular files exercised *operator* normalisation. They would both
// hold just as well if operators had stopped reaching the digest
// altogether, and that is the original defect: with the operator dropped,
// `alpha + beta` and `alpha - beta` normalise to the same subtree.
//
// This reads the two members' normalised trees directly and pins each
// one's operator leaves *exactly*: the file carries its own spelling,
// four times, and carries no other operator at all. It is the whole
// mechanism the fixture exists for, asserted where it happens rather
// than inferred from the report.
#[test]
fn each_family_member_normalises_to_its_own_operator_leaf() -> Result<()> {
    for family in FAMILIES {
        for (file, own) in [
            (family.left, family.left_operator),
            (family.right, family.right_operator),
        ] {
            let leaves = operator_leaves(file)?;
            let expected: Vec<String> =
                std::iter::repeat_n(own.to_owned(), OPERATOR_LEAVES_PER_MEMBER).collect();
            assert_eq!(
                leaves,
                expected,
                "{label}: {file} must normalise to exactly \
                 {OPERATOR_LEAVES_PER_MEMBER} x {own} and nothing else under \
                 {OPERATOR_KIND_PREFIX}. Fewer means the operator stopped \
                 reaching the digest and this file now hashes the same as its \
                 sibling; a different spelling means normalisation is telling \
                 the reader about a token nobody wrote.",
                label = family.label,
            );
        }
    }
    Ok(())
}

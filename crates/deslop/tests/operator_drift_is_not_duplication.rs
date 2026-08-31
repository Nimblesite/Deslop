//! [PIPELINE-NORMALIZE-AST] — an operator is behaviour, and two
//! functions that differ only in their operators are not duplicates.
//!
//! Normalisation walked `named_children` alone. In every grammar here
//! the operator of a binary expression is an *anonymous* token, so
//! `alpha + beta` and `alpha - beta` normalised to the same subtree with
//! the same identifier frontier and the same literals: nothing
//! downstream had any evidence they differed. The pair rendered
//! `structural = 1.00`, `token_jaccard = 1.00` and `pair_agreement =
//! 1.00` — the engine's strongest possible claim, made about
//! code that computes a different answer.
//!
//! That is a duplicate verdict, so a `find-similar`
//! consumer is told to reuse one where the other is meant, and an agent
//! following `docs/snippets/agents-md-recipe.md` deletes a subtraction
//! in favour of an addition. Sign errors, inverted comparisons and
//! inverted boolean guards are exactly the defects that survive review.
//!
//! Four families are pinned. Three cover the grammar productions that
//! carry the token — arithmetic (`binary_operator`), comparison
//! (`comparison_operator`) and boolean (`boolean_operator`) — and none
//! of them clusters once the operator reaches the digest. The fourth,
//! `ledger`, is a twelve-line body differing in exactly one `+`/`-`,
//! which is what a real sign error looks like. Its byte-identical regions
//! still cluster, but no published occurrence pair may cover the changed
//! operator line.
//!
//! The contract is *not* "never cluster them". Two functions that share
//! a shape and differ in an operator may well be worth a reader's
//! attention. The contract is that the report must not claim the
//! content is duplicated: the pair must stay out of the explicit
//! duplicate admission, and explicit pair evidence must remain honest.
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

/// The operator families and one source line on which their computations
/// disagree. A file-pair lookup is insufficient: the ledger files also
/// contain genuine byte-identical subregions, and mistaking one of those
/// for the operator-bearing enclosure condemned correct duplication.
const FAMILIES: [Family; 4] = [
    Family {
        label: "arithmetic + / -",
        left: "arithmetic_add.py",
        right: "arithmetic_sub.py",
        left_leaves: &["__op__+", "__op__+", "__op__+", "__op__+"],
        right_leaves: &["__op__-", "__op__-", "__op__-", "__op__-"],
        changed_line: 2,
    },
    Family {
        label: "comparison == / !=",
        left: "comparison_eq.py",
        right: "comparison_ne.py",
        left_leaves: &["__op__==", "__op__==", "__op__==", "__op__=="],
        right_leaves: &["__op__!=", "__op__!=", "__op__!=", "__op__!="],
        changed_line: 2,
    },
    Family {
        label: "boolean and / or",
        left: "boolean_and.py",
        right: "boolean_or.py",
        left_leaves: &["__op__and", "__op__and", "__op__and", "__op__and"],
        right_leaves: &["__op__or", "__op__or", "__op__or", "__op__or"],
        changed_line: 2,
    },
    Family {
        label: "ledger + / - inside a shared body",
        left: "ledger_credit.py",
        right: "ledger_debit.py",
        left_leaves: &["__op__*", "__op__+", "__op__+", "__op__/", "__op__-"],
        right_leaves: &["__op__*", "__op__-", "__op__+", "__op__/", "__op__-"],
        changed_line: 6,
    },
];

/// One operator family: the pair, the operator leaves each member must
/// normalise to, and a source line carrying the changed computation.
struct Family {
    /// Human label used in every assertion message.
    label: &'static str,
    /// The member holding the first operator.
    left: &'static str,
    /// The member holding the second.
    right: &'static str,
    /// Every operator leaf `left` normalises to, in dump order.
    left_leaves: &'static [&'static str],
    /// Every operator leaf `right` normalises to, in dump order.
    right_leaves: &'static [&'static str],
    /// A line whose operator differs between the two members.
    changed_line: u64,
}

/// The byte-identical pair that must survive whatever separates the
/// families above.
const CONTROL: [&str; 2] = ["control_alpha.py", "control_beta.py"];

/// Every file the fixture holds: four operator pairs plus the control.
/// Asserted per run so "the family published nothing" can only ever mean
/// the pair was analysed and excluded. The control proves the *run* was
/// still detecting; this proves these particular files reached the
/// pipeline at all, which a control in other files cannot say.
const FIXTURE_FILE_COUNT: u64 = 10;

/// Renders the fixture.
fn render() -> Result<Value> {
    run_report(&fixture("operator-drift"), MIN_NODES)
}

/// Every visible cluster as `id mass files`.
fn published(report: &Value) -> Vec<String> {
    clusters(report)
        .iter()
        .map(|cluster| {
            format!(
                "{id} mass={mass} {files:?}",
                id = cluster_id(cluster),
                mass = field(cluster, "mass").as_u64().unwrap_or(0),
                files = occurrence_files(cluster),
            )
        })
        .collect()
}

/// Whether one occurrence covers `line` in the named fixture file.
fn occurrence_covers(occurrence: &Value, file: &str, line: u64) -> bool {
    let path_matches = field(occurrence, "path")
        .as_str()
        .and_then(|path| std::path::Path::new(path).file_name())
        .is_some_and(|name| name == file);
    let start = field(occurrence, "start_line").as_u64().unwrap_or(0);
    let end = field(occurrence, "end_line").as_u64().unwrap_or(0);
    path_matches && start <= line && line <= end
}

/// Whether a published cluster pairs both sides of a family's changed line.
fn covers_operator_pair(cluster: &Value, family: &Family) -> bool {
    [family.left, family.right].iter().all(|file| {
        occurrences(cluster)
            .iter()
            .any(|occurrence| occurrence_covers(occurrence, file, family.changed_line))
    })
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
fn an_operator_only_difference_never_claims_duplication() -> Result<()> {
    let report = render()?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(FIXTURE_FILE_COUNT),
        "every operator pair must reach the pipeline before its absence from \
         the report can mean anything: {published:#?}",
        published = published(&report),
    );
    let control = expect_cluster_spanning(&report, &CONTROL)?;
    assert!(
        has_verbatim_pair(&fixture("operator-drift"), control)?,
        "the byte-identical control must still be published as duplication in \
         this very run — without it, every assertion below is satisfied just \
         as well by a detector that produced no candidates: {published:#?}",
        published = published(&report),
    );
    assert_structural_only_contract(control, "operator-drift control");
    for family in FAMILIES {
        let offenders: Vec<&Value> = clusters(&report)
            .iter()
            .filter(|cluster| covers_operator_pair(cluster, &family))
            .collect();
        assert!(
            offenders.is_empty(),
            "{label}: {left} and {right} compute different answers on line \
             {line}. No duplicate cluster may pair occurrences covering both \
             changed operators: {offenders:#?}\nall clusters: {published:#?}",
            label = family.label,
            left = family.left,
            right = family.right,
            line = family.changed_line,
            published = published(&report),
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
    assert!(
        has_verbatim_pair(&fixture("operator-drift"), control)?,
        "the control is copied byte for byte — {dump}"
    );
    assert_eq!(
        cluster_size(control),
        2,
        "both copies of the control must be shown — {dump}"
    );
    Ok(())
}

// A genuine exact subregion may outrank the small control. Every cluster
// ahead of it must be byte-identical and must exclude the ledger's changed
// operator line; otherwise ranking would still elevate the false positive.
#[test]
fn every_cluster_ahead_of_the_control_is_proven_duplication() -> Result<()> {
    let report = render()?;
    let control = expect_cluster_spanning(&report, &CONTROL)?;
    let control_rank = rank_of(&report, control)?;
    let scan_root = fixture("operator-drift");
    let ledger = &FAMILIES[3];
    for cluster in clusters(&report).iter().take(control_rank) {
        assert_eq!(
            distinct_texts(&scan_root, cluster)?.len(),
            1,
            "a cluster may outrank the control only with byte-identical \
             source proof: {}",
            signal_dump(cluster),
        );
        assert!(
            !covers_operator_pair(cluster, ledger),
            "the changed ledger operator must never enter a higher-ranked \
             duplicate cluster: {}",
            signal_dump(cluster),
        );
    }
    Ok(())
}

/// The `--debug-ast` dump for one fixture file, as the exact sequence of
/// normalised node kinds it names.
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

// The operator-specific non-vacuity proof, on the pairs themselves.
//
// The disposition pin above says what each family reaches the report as,
// and the control says the run was still detecting — but neither shows
// that these particular files exercised *operator* normalisation. They
// would both hold just as well if operators had stopped reaching the
// digest altogether, and that is the original defect: with the operator
// dropped, `alpha + beta` and `alpha - beta` normalise to the same
// subtree.
//
// This reads each member's normalised tree directly and pins its
// operator leaves *exactly*, in dump order. Exactly, because a presence
// check passes while three of four occurrences are dropped, and because
// `contains("__op__+")` is also satisfied by `__op__++` and `__op__+=`.
// The two sides are then required to differ, which is what makes the
// pair an operator-drift pair at all: identical leaf lists would mean
// the fixture had stopped posing the question.
#[test]
fn each_family_member_normalises_to_its_own_operator_leaves() -> Result<()> {
    for family in FAMILIES {
        for (file, expected) in [
            (family.left, family.left_leaves),
            (family.right, family.right_leaves),
        ] {
            assert_eq!(
                operator_leaves(file)?,
                expected,
                "{label}: {file} must normalise to exactly these operator \
                 leaves under {OPERATOR_KIND_PREFIX}, in this order. Fewer \
                 means an operator stopped reaching the digest and this file \
                 now hashes closer to its sibling; a different spelling means \
                 normalisation is telling the reader about a token nobody \
                 wrote.",
                label = family.label,
            );
        }
        assert_ne!(
            family.left_leaves,
            family.right_leaves,
            "{label}: the two members carry the same operator leaves, so \
             the pair no longer differs in an operator and the family \
             proves nothing about operator drift",
            label = family.label,
        );
    }
    Ok(())
}

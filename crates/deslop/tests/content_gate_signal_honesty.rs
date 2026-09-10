//! [FUSED-CONTENT-GATE] — the gate may correct a signal it can prove,
//! and may not publish one it did not measure (gh #431).
//!
//! The mass-only wire cutover removed the cluster `signals` block, the
//! routing bucket, and the `evidence_verdict` sentence: admission
//! evidence is pair-scoped and cluster surfaces carry cluster facts and
//! mass only ([PIPELINE-CLUSTER-CLOSURE]). The fabrication this suite
//! exists to catch — `content_gated_signals` rewriting `token_jaccard`
//! to `1.0` for a `nearly_identical` cluster on a Merkle argument that
//! held at digest equality and nowhere else — is therefore impossible
//! by construction on the current wire. The suite pins the migration
//! instead, at the same strength:
//!
//! 1. **Recall** — every pair this fixture exists to report is still
//!    **admitted** and reported, with the same rank ordering and the
//!    wire mass formula `canonical_node_count × (occurrence_count − 1)`
//!    ([RANK-MASS-SUM]). A cutover that silently dropped the
//!    near-saturated Type-3 pair or the digest-equal control is caught
//!    exactly as the old signal assertions would have caught a
//!    silently-demoted cluster. The accessor pair is the same contract
//!    read from the other side: it is the gh #460 false positive, the
//!    aligned core refuses it ([FUSED-SHARED-SUBTREE-CORE]), and this
//!    suite pins its absence beside a control that stays visible in the
//!    same run.
//! 2. **No fabricated evidence surface** — no cluster on the wire may
//!    carry `signals`, `bucket`, `category`, `evidence_verdict`, or any
//!    other pair-only or presentation field. Reintroducing a cluster
//!    signal that a gate could fabricate fails these negative pins the
//!    moment it lands.
//!
//! `ledger_alpha.py` / `ledger_beta.py` are a long Type-3 pair whose one
//! control-flow node changes from `if` to `while`, keeping their
//! measured structural overlap inside `[0.99, 1.0)` — the band the old
//! routing tolerance would have sat through. The byte-identical control
//! in `content-gate-unsaturated` proves the digest-equal population
//! separately.

use deslop_core::report::{PairClassification, PairComparison, PairEndpoint};
use serde_json::Value;

use crate::common::negative_pin::{
    assert_control_is_the_only_published_cluster, assert_family_hidden_with_control,
};
use crate::common::{signals::*, *};

/// Node floor for the small control and accessor fixtures.
const MIN_NODES: u32 = 8;

/// Node floor that admits the 307-node Type-3 function roots while excluding
/// exact repeated statement windows, so the assertion cannot select a nested
/// digest-equal pair instead of the near-saturated pair under test.
const SATURATION_BAND_MIN_NODES: u32 = 200;

/// The pair that lands inside the `[0.99, 1.0)` band: one control-flow
/// node apart, so their normalised subtrees differ and no digest is shared.
const SATURATION_BAND_PAIR: [&str; 2] = ["ledger_alpha.py", "ledger_beta.py"];

/// The byte-identical control in the same fixture: both axes saturate,
/// the gate runs, and `pair_agreement = 1.00` genuinely corroborates.
const SATURATED_CONTROL_PAIR: [&str; 2] = ["control_alpha.rs", "control_beta.rs"];

/// The pair that reproduces gh #460: two unrelated tree-sitter field
/// accessors — different node kind, different field, different body —
/// whose only shared authored logic is the grammar-mandated accessor
/// idiom. The aligned core pairs `return value` against
/// `collect_identifiers(left, source, out)` and finds an inconsistent
/// rename, so the rescue refuses the pair
/// ([FUSED-SHARED-SUBTREE-CORE]). The pair is held CLEARLY OUT in
/// `corpus/register/deslop.json`; this suite pins its absence from the
/// fixture report and the refusal on the pair wire.
const ACCESSOR_PAIR: [&str; 2] = ["accessor_argument.rs", "accessor_assignment.rs"];

const EXACT_SCORE: f64 = 1.0;

const SHAPE_SATURATION_FLOOR: f64 = 0.99;

/// The duplication this fixture publishes: the byte-identical control,
/// seven lines in each of its two copies. Measured, never hand-counted —
/// `metrics.duplicated_loc` of the fixture report.
const CONTROL_DUPLICATED_LOC: u64 = 14;

/// The control leads the report: it is the only finding, and a
/// byte-identical copy outranks anything a fixture can stage against it.
const CONTROL_RANK: u64 = 1;

/// `canonical_node_count × (occurrence_count − 1)` for the control.
const CONTROL_MASS: u64 = 32;

/// Normalised nodes in the control's canonical extent.
const CONTROL_CANONICAL_NODES: u64 = 32;

/// Both copies of the control are visible members.
const CONTROL_OCCURRENCES: u64 = 2;

/// Label the accessor family carries in every assertion below.
const ACCESSOR_LABEL: &str = "gh #460 accessor idiom";

/// Label the byte-identical control carries.
const CONTROL_LABEL: &str = "byte-identical control";

/// The whole `argument_payload` function, lines 6–13 of its file.
const ACCESSOR_ARGUMENT_FN: (&str, usize, usize) = ("accessor_argument.rs", 161, 375);

/// The whole `assignment_targets` function, lines 6–12 of its file.
const ACCESSOR_ASSIGNMENT_FN: (&str, usize, usize) = ("accessor_assignment.rs", 166, 417);

/// `node.kind() == "keyword_argument"` — one of the two one-line spans
/// the pre-correction build published as a duplicate of the other.
const ACCESSOR_ARGUMENT_KIND: (&str, usize, usize) = ("accessor_argument.rs", 222, 255);

/// `node.kind() == "assignment"` — the other half of that published pair.
const ACCESSOR_ASSIGNMENT_KIND: (&str, usize, usize) = ("accessor_assignment.rs", 255, 282);

/// The refusal every accessor comparison must reach, with the measured
/// axes that produce it ([FUSED-CONTENT-GATE], gh #460).
struct RefusedPair {
    /// Measured ordered structural overlap.
    structural: f64,
    /// Measured token Jaccard.
    token_jaccard: f64,
    /// Measured raw-content agreement.
    agreement: f64,
    /// Measured consistent-renaming support.
    rename_consistency: f64,
    /// Whether saturated normalised evidence demanded content support.
    content_required: bool,
    /// Whether every applicable content guard passed.
    content_ok: bool,
    /// The presentation classification, when the pair earns one.
    classification: Option<PairClassification>,
    /// The engine's own sentence about the refusal.
    explanation: &'static str,
}

/// The two whole authored functions: unsaturated shape, so the content
/// guard is never asked, and the pair falls at the LSH-only floors.
const ACCESSOR_FUNCTIONS: RefusedPair = RefusedPair {
    structural: 0.689_655_172_413_793_1,
    token_jaccard: 0.453_125,
    agreement: 0.3,
    rename_consistency: 0.0,
    content_required: false,
    content_ok: true,
    classification: None,
    explanation: "rejected: pair fails the LSH-only guards",
};

/// The two one-line `node.kind()` tests the pre-correction build
/// published: shape and tokens both saturate at 1.00, and the content
/// guard is what refuses them. This is the gh #431 fabrication surface
/// asserted at the pair wire — a pair whose shape agrees completely is
/// still not told its content agreed.
const ACCESSOR_KIND_TESTS: RefusedPair = RefusedPair {
    structural: 1.0,
    token_jaccard: 1.0,
    agreement: 0.666_666_666_666_666_6,
    rename_consistency: 0.333_333_333_333_333_3,
    content_required: true,
    content_ok: false,
    classification: Some(PairClassification::StructuralOnly),
    explanation: "rejected: saturated normalised evidence lacks required pair content support",
};

/// One fixture span as the endpoint the pair wire names it by.
fn endpoint(scan_root: &std::path::Path, span: (&str, usize, usize)) -> PairEndpoint {
    PairEndpoint {
        path: scan_root.join(span.0),
        start_byte: span.1,
        end_byte: span.2,
    }
}

/// Compares two named fixture spans through the pair wire, without
/// deriving either endpoint from a cluster — the pair under test is
/// refused, so no cluster names it.
fn compare_spans(
    scan_root: &std::path::Path,
    left: (&str, usize, usize),
    right: (&str, usize, usize),
) -> Result<PairComparison> {
    compare_endpoints(
        scan_root,
        MIN_NODES,
        endpoint(scan_root, left),
        endpoint(scan_root, right),
    )
}

/// Asserts one pair's measured axes and its refusal, exactly.
fn assert_refused(comparison: &PairComparison, expected: &RefusedPair, label: &str) {
    assert_refused_axes(comparison, expected, label);
    assert_refused_verdict(comparison, expected, label);
    assert!(
        !comparison.evidence.admitted,
        "{label}: two unrelated accessors must never be an admitted edge: {comparison:#?}"
    );
}

/// The four measured axes the pair wire publishes for this pair.
fn assert_refused_axes(comparison: &PairComparison, expected: &RefusedPair, label: &str) {
    let evidence = &comparison.evidence;
    assert_pair_metric(evidence.structural, expected.structural, label);
    assert_pair_metric(evidence.token_jaccard, expected.token_jaccard, label);
    assert_pair_metric(evidence.agreement, expected.agreement, label);
    assert_pair_metric(
        evidence.rename_consistency,
        expected.rename_consistency,
        label,
    );
}

/// The verdict those axes produce, and the refusal itself.
fn assert_refused_verdict(comparison: &PairComparison, expected: &RefusedPair, label: &str) {
    let evidence = &comparison.evidence;
    assert_eq!(
        (
            evidence.content_required,
            evidence.content_ok,
            evidence.classification,
            evidence.explanation.as_str(),
        ),
        (
            expected.content_required,
            expected.content_ok,
            expected.classification,
            expected.explanation,
        ),
        "{label}: the pair wire must publish the measured verdict: {comparison:#?}"
    );
}

/// Both accessor comparisons, each pinned on the saturation band it sits
/// in and on its exact refusal. The whole functions never saturate, so
/// the content guard is never asked; the one-line kind tests do, and the
/// guard is what refuses them.
fn assert_both_accessor_pairs_are_refused(scan_root: &std::path::Path) -> Result<()> {
    let functions = compare_spans(scan_root, ACCESSOR_ARGUMENT_FN, ACCESSOR_ASSIGNMENT_FN)?;
    assert!(
        functions.evidence.structural < SHAPE_SATURATION_FLOOR,
        "the whole accessor functions must stay unsaturated: {functions:#?}"
    );
    assert_refused(
        &functions,
        &ACCESSOR_FUNCTIONS,
        "the whole accessor functions",
    );
    let fragments = compare_spans(scan_root, ACCESSOR_ARGUMENT_KIND, ACCESSOR_ASSIGNMENT_KIND)?;
    assert!(
        fragments.evidence.structural >= SHAPE_SATURATION_FLOOR,
        "the one-line kind tests must saturate: {fragments:#?}"
    );
    assert_refused(&fragments, &ACCESSOR_KIND_TESTS, "the one-line kind tests");
    Ok(())
}

/// The exact published facts of the byte-identical control.
fn assert_control_facts(control: &Value, report: &Value) {
    assert_eq!(
        (
            field(control, "rank").as_u64(),
            field(control, "mass").as_u64(),
            field(control, "canonical_node_count").as_u64(),
            field(control, "occurrence_count").as_u64(),
        ),
        (
            Some(CONTROL_RANK),
            Some(CONTROL_MASS),
            Some(CONTROL_CANONICAL_NODES),
            Some(CONTROL_OCCURRENCES),
        ),
        "the byte-identical control must keep its exact published facts: {report:#}"
    );
}

/// Renders the unsaturated-gate fixture once per assertion below, at the
/// same [`MIN_NODES`] floor the ledger corpus uses. Only the
/// byte-identical control clusters; the accessor pair is refused, and
/// this suite pins that refusal.
fn render_unsaturated() -> Result<Value> {
    run_report(&fixture("content-gate-unsaturated"), MIN_NODES)
}

/// Compares the exact pair named by the fixture contract.
fn explicit_pair(
    scan_root: &std::path::Path,
    min_nodes: u32,
    cluster: &Value,
    files: [&str; 2],
) -> Result<PairComparison> {
    compare_pair(
        scan_root,
        min_nodes,
        occurrence_for_file(cluster, files[0])?,
        occurrence_for_file(cluster, files[1])?,
    )
}

// The pair this fixture exists for: a Type-3 near-miss inside the
// [0.99, 1.0) band. The old defect published it carrying a fabricated
// `token_jaccard = 1.00` and a `shape = 1.00` derived from it, because
// the routing tolerance `STRUCTURAL_SATURATION_FLOOR` (0.99) let the
// gate correct signals it had not measured. With the cluster signals
// block gone, the fabrication surface is gone; what the test pins is
// that the band pair still admits, reports with the exact wire mass,
// and no pair-only field has crept back onto the cluster.
#[test]
fn the_content_gate_publishes_no_token_jaccard_it_did_not_measure() -> Result<()> {
    let scan_root = fixture("content-gate-saturation-band");
    let report = run_report(&scan_root, SATURATION_BAND_MIN_NODES)?;
    let ledger = clusters(&report)
        .iter()
        .find(|cluster| {
            SATURATION_BAND_PAIR
                .iter()
                .all(|file| cluster_file_set(cluster).contains(*file))
        })
        .ok_or_else(|| {
            anyhow::anyhow!("expected the ledger pair admitted on the report: {report:#}")
        })?;
    assert_structural_only_contract(ledger, "ledger closure");
    assert_no_pair_surface_on_cluster(ledger, "ledger pair");
    let comparison = explicit_pair(
        &scan_root,
        SATURATION_BAND_MIN_NODES,
        ledger,
        SATURATION_BAND_PAIR,
    )?;
    let evidence = &comparison.evidence;
    assert!(
        (SHAPE_SATURATION_FLOOR..EXACT_SCORE).contains(&evidence.structural),
        "fixture must exercise the near-saturated structural band: {comparison:#?}"
    );
    assert!(
        evidence.token_jaccard < EXACT_SCORE,
        "pair comparison must publish measured Jaccard, never a fabricated saturation: {comparison:#?}"
    );
    assert!(
        evidence.content_required,
        "near-saturated shape must require pair content: {comparison:#?}"
    );
    assert!(
        evidence.content_ok,
        "the renamed ledger pair must clear pair content: {comparison:#?}"
    );
    assert!(!evidence.admitted, "the explicit ledger pair lacks the LSH-only floor and must not be an edge: {comparison:#?}");
    assert_eq!(evidence.classification, None);
    assert_eq!(
        evidence.explanation,
        "rejected: pair fails the LSH-only guards"
    );
    assert!(
        !clusters(&report)
            .iter()
            .any(|cluster| cluster.get("signals").is_some()),
        "no cluster on the report may carry a signals block — the fabrication \
         path for the gh #431 defect is a cluster signal: {report:#}"
    );
    Ok(())
}

// The other half of the contract: the digest-equal population the old
// correction served must keep admitting. A byte-identical control still
// reports, ranked ahead of every approximate pair in the same fixture,
// with the wire mass formula.
#[test]
fn a_digest_equal_pair_keeps_its_saturated_signals() -> Result<()> {
    let scan_root = fixture("content-gate-unsaturated");
    let report = run_report(&scan_root, MIN_NODES)?;
    let control = expect_cluster_spanning(&report, &SATURATED_CONTROL_PAIR)?;
    assert_structural_only_contract(control, "byte-identical control");
    assert_no_pair_surface_on_cluster(control, "byte-identical control");
    assert_eq!(
        field(control, "rank").as_u64().unwrap_or(0),
        1,
        "the byte-identical control must lead the fixture's report: {report:#}"
    );
    let comparison = explicit_pair(&scan_root, MIN_NODES, control, SATURATED_CONTROL_PAIR)?;
    let evidence = &comparison.evidence;
    assert_pair_metric(
        evidence.structural,
        EXACT_SCORE,
        "control structural overlap",
    );
    assert_pair_metric(evidence.token_jaccard, EXACT_SCORE, "control token Jaccard");
    assert_pair_metric(evidence.agreement, EXACT_SCORE, "control content agreement");
    assert_pair_metric(
        evidence.rename_consistency,
        EXACT_SCORE,
        "control rename consistency",
    );
    assert!(
        evidence.content_required && evidence.content_ok && evidence.admitted,
        "the exact control must clear every pair gate: {comparison:#?}"
    );
    assert_eq!(evidence.classification, Some(PairClassification::Identical));
    Ok(())
}

// [FUSED-CONTENT-GATE] gh #460, gh #532 — the report may not tell a
// reader that content evidence corroborated a match whose evidence did
// not corroborate it. The corrected engine states that contract twice
// over, and this test asserts both statements.
//
// At the report: the match is never published. The two accessors share
// only the grammar-mandated field-read idiom, the aligned core refuses
// them ([FUSED-SHARED-SUBTREE-CORE]), and the corroborated control in
// the same run is published untouched. The absence is asserted as a
// decision rather than a scan that never looked —
// `assert_family_hidden_with_control` demands both accessor files carry
// analysed lines, and that the control leads the report whole.
//
// At the pair wire: both spans still resolve, so both are measured.
// The whole functions never reach saturation, so the content guard is
// not asked and the pair falls at the LSH-only floors. The two one-line
// `node.kind()` tests the pre-correction build published *do* saturate,
// at 1.00 shape and 1.00 tokens — and are refused by the content guard
// with `content_ok = false`. That is the gh #431 fabrication surface
// asserted directly: a pair whose shape agrees completely is still not
// told its content agreed.
#[test]
fn a_cluster_whose_evidence_did_not_corroborate_is_not_told_it_agreed() -> Result<()> {
    let scan_root = fixture("content-gate-unsaturated");
    let report = run_report(&scan_root, MIN_NODES)?;
    assert_family_hidden_with_control(
        &report,
        ACCESSOR_LABEL,
        &ACCESSOR_PAIR,
        &SATURATED_CONTROL_PAIR,
    )?;
    for cluster in clusters(&report) {
        assert_no_pair_surface_on_cluster(cluster, "published cluster");
    }
    assert_both_accessor_pairs_are_refused(&scan_root)
}

// The other half of the contract, asserted in the same run: with the gh
// #460 pair correctly refused, the corroborated control is the whole of
// this report's duplication. The two comparisons the old test made —
// control mass against accessor mass, control rank against accessor rank
// — no longer have a second subject, so the contract is stated against
// the control's own published figures and the report's totals, which a
// demoted or duplicated control fails just as loudly.
#[test]
fn the_corroborated_control_is_the_whole_of_this_reports_duplication() -> Result<()> {
    let report = render_unsaturated()?;
    let control = expect_cluster_spanning(&report, &SATURATED_CONTROL_PAIR)?;
    assert_structural_only_contract(control, CONTROL_LABEL);
    assert_no_pair_surface_on_cluster(control, CONTROL_LABEL);
    assert_control_facts(control, &report);
    assert_control_is_the_only_published_cluster(
        &report,
        CONTROL_LABEL,
        &SATURATED_CONTROL_PAIR,
        CONTROL_DUPLICATED_LOC,
    )
}

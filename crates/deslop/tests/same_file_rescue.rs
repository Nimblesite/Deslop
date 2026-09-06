//! [FUSED-SHARED-SUBTREE-SAME-FILE] A near-miss rescued inside one file
//! is held to the same echo rule as one rescued across files.
//!
//! Two sibling classes in one file each hold a byte-identical method.
//! The classes measure high overlap *because of that method*; a rescue
//! that admitted them would hand [PIPELINE-CLUSTER-SUBSUME] a wider,
//! byte-divergent view that encloses the exact one and replaces it. The
//! finding is the method, at its own lines, in both classes.
//!
//! [FUSED-CANDIDATE-BUCKET-STAR] A same-file bucket pairs every member
//! with the member that sorts first and with nothing else. When the
//! first member is the one that *differs*, the byte-identical pair
//! behind it is never a candidate, so one unrelated sibling deletes an
//! exact duplicate from the report. Recall must not depend on what else
//! happens to share the shape: the copied pair is the same finding with
//! or without the sibling standing in front of it.

use std::{collections::BTreeSet, ops::RangeInclusive};

use anyhow::Result;

use deslop_test_support::{write_csharp_star_shadow_fixture, CSHARP_COPIED_BODY};

use crate::common::signals::{
    assert_no_pair_surface_on_cluster, assert_structural_only_contract, has_verbatim_pair,
};
use crate::common::verdict::{assert_reported, expect_only_finding_is_the_pair};
use crate::common::*;

/// One file, two classes, one method copied byte for byte between them.
const SIBLING_CLASS_FIXTURE: &str = "csharp-same-file-class-echo";
/// The only file the fixture holds.
const SIBLING_CLASS_FILE: &str = "Ledgers.cs";
/// The 1-based lines of `AlphaLedger.Reconcile`.
const ALPHA_RECONCILE_LINES: RangeInclusive<u64> = 7..=22;
/// The 1-based lines of `BetaLedger.Reconcile`, the byte-identical copy.
const BETA_RECONCILE_LINES: RangeInclusive<u64> = 31..=46;
/// A floor above the one-line `WithinCeiling` accessors and the field
/// declarations, so the method pair is the only duplication in reach.
const SIBLING_CLASS_MIN_NODES: u32 = 20;
/// Files the fixture holds; a one-file scan must still analyse it.
const SIBLING_CLASS_FILE_COUNT: u64 = 1;
/// The two copies are byte-identical, so they slice to one text.
const SIBLING_CLASS_DISTINCT_TEXTS: usize = 1;
/// What a suppression of this pair would prove.
const SIBLING_CLASS_WHY: &str =
    "one method copied byte for byte between two sibling classes in one file \
     is the finding, at its own lines in both classes. Publishing the classes \
     instead would widen a byte-divergent view over the exact one; publishing \
     nothing would mean the same-file echo rule refused the copy itself.";

#[test]
fn sibling_classes_wrapping_one_exact_method_publish_the_method() -> Result<()> {
    let scan_root = fixture(SIBLING_CLASS_FIXTURE);
    let report = run_report(&scan_root, SIBLING_CLASS_MIN_NODES)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(SIBLING_CLASS_FILE_COUNT),
        "the one file must be analysed: {report:#}"
    );
    // [FUSED-SHARED-SUBTREE-ECHO] The class pair shares nothing beyond
    // the method it wraps, so it is refused and cannot widen the finding.
    let cluster = expect_only_finding_is_the_pair(
        &scan_root,
        &report,
        SIBLING_CLASS_FILE,
        &[ALPHA_RECONCILE_LINES, BETA_RECONCILE_LINES],
        SIBLING_CLASS_DISTINCT_TEXTS,
        SIBLING_CLASS_WHY,
    )
    .map(|_| ())
    .and_then(|()| {
        clusters(&report)
            .first()
            .ok_or_else(|| anyhow::anyhow!("one cluster asserted above"))
    })?;
    // [PIPELINE-CLUSTER-EXACT-SCOPE] Both occurrences are the authored
    // method, and a byte-for-byte copy is a verbatim pair.
    assert!(
        has_verbatim_pair(&scan_root, cluster)?,
        "{SIBLING_CLASS_WHY} a byte-for-byte copy is a verbatim pair: {cluster:#}"
    );
    assert_structural_only_contract(cluster, SIBLING_CLASS_FIXTURE);
    assert_no_pair_surface_on_cluster(cluster, SIBLING_CLASS_FIXTURE);
    Ok(())
}

/// The one file the star-shadow fixture holds.
const STAR_SHADOW_FILE: &str = "Rates.cs";
/// A floor above the individual statements, so the copied method is the
/// only duplication the scan can reach.
const STAR_SHADOW_MIN_NODES: u32 = 12;
/// `ApplyDelta` and `ApplyEpsilon` with `ApplyAlpha` written ahead of
/// them — the sibling that shares their shape and none of their bytes.
const SHADOWED_COPY_LINES: RangeInclusive<u64> = 11..=17;
/// `ApplyEpsilon` in the same variant.
const SHADOWED_PASTE_LINES: RangeInclusive<u64> = 19..=25;
/// `ApplyDelta` with no sibling ahead of it.
const ALONE_COPY_LINES: RangeInclusive<u64> = 3..=9;
/// `ApplyEpsilon` in that variant.
const ALONE_PASTE_LINES: RangeInclusive<u64> = 11..=17;
/// `ApplyAlpha`, which is nobody's duplicate and must reach no cluster.
const SIBLING_LINES: RangeInclusive<u64> = 3..=9;
/// The sibling's own name, which may not appear in the finding either.
const SIBLING_METHOD_NAME: &str = "ApplyAlpha";
/// The declaration names differ, so the two occurrences are two texts.
const STAR_SHADOW_DISTINCT_TEXTS: usize = 2;
/// Files each variant holds; a one-file scan must still analyse it.
const STAR_SHADOW_FILE_COUNT: u64 = 1;
/// The two declaration names over the one copied body. Both must be
/// reported: the copy-paste is visible only in the bodies, because the
/// names the reader would compare are exactly what differs.
const COPIED_METHOD_NAMES: [&str; 2] = ["ApplyDelta", "ApplyEpsilon"];

/// [FUSED-CANDIDATE-BUCKET-STAR] An exact same-file duplicate stays
/// reported when a differing sibling of the same shape is written ahead
/// of it. The two scans below hold the same copied method; the only
/// difference is the sibling, which duplicates nothing.
#[test]
fn a_shape_sibling_may_not_hide_an_exact_same_file_copy() -> Result<()> {
    let alone = assert_copied_pair_published(false, &[ALONE_COPY_LINES, ALONE_PASTE_LINES])?;
    let shadowed =
        assert_copied_pair_published(true, &[SHADOWED_COPY_LINES, SHADOWED_PASTE_LINES])?;
    assert_eq!(
        alone, shadowed,
        "the sibling changes no byte of the copied method, so it must not \
         change the reported occurrences either"
    );
    Ok(())
}

/// Scans one variant of the star-shadow fixture and asserts the copied
/// pair is the report: one cluster, both methods at `spans`, the copied
/// body in both occurrence texts, and no line outside the pair counted as
/// duplicated. Returns the reported texts sorted, so the caller can hold
/// both variants to the same finding.
fn assert_copied_pair_published(
    with_sibling: bool,
    spans: &[RangeInclusive<u64>],
) -> Result<Vec<String>> {
    let workspace = tempfile::tempdir()?;
    let scan_root = workspace.path().join("src");
    write_csharp_star_shadow_fixture(&scan_root, with_sibling)?;
    let report = run_report(&scan_root, STAR_SHADOW_MIN_NODES)?;
    assert_eq!(
        field(&report, "files_analysed").as_u64(),
        Some(STAR_SHADOW_FILE_COUNT),
        "the one file must be analysed (with_sibling={with_sibling}): {report:#}"
    );
    let why = star_shadow_why(with_sibling);
    let mut texts = expect_only_finding_is_the_pair(
        &scan_root,
        &report,
        STAR_SHADOW_FILE,
        spans,
        STAR_SHADOW_DISTINCT_TEXTS,
        &why,
    )?;
    texts.sort();
    for text in &texts {
        assert!(
            text.contains(CSHARP_COPIED_BODY),
            "{why} each occurrence must report the copied body itself, not a \
             fragment of it: {text}"
        );
    }
    assert_reported(&texts, &COPIED_METHOD_NAMES, &why);
    if with_sibling {
        let duplicated: BTreeSet<u64> = spans.iter().cloned().flatten().collect();
        assert!(
            SIBLING_LINES
                .clone()
                .all(|line| !duplicated.contains(&line)),
            "{why} the sibling duplicates nothing and none of its lines may \
             be counted: {report:#}"
        );
        for text in &texts {
            assert!(
                !text.contains(SIBLING_METHOD_NAME),
                "{why} the sibling may not be pulled into the finding: {text}"
            );
        }
    }
    Ok(texts)
}

/// What a suppression of this scan would prove. The two variants fail for
/// different reasons, so each names its own.
fn star_shadow_why(with_sibling: bool) -> String {
    let shared = "two byte-identical method bodies in one file are a copy-paste \
                  duplicate and the report's whole finding";
    if with_sibling {
        format!(
            "{shared}. Hiding them when a differing sibling of the same shape is \
             written above them proves recall depends on write order: the bucket \
             pairs every member with the member that sorts first, and that member \
             is the one that differs."
        )
    } else {
        format!(
            "{shared}. Hiding them with nothing else in the file proves the detector went blind."
        )
    }
}

/// Two methods in one file that differ in nothing but their literals.
const MANY_HOLES_FIXTURE: &str = "csharp-merge-manyholes";
/// The only file the fixture holds.
const MANY_HOLES_FILE: &str = "Sprawl.cs";
/// `Sprawl.ApplyStandard` — six `Set` calls and a `Commit`.
const MANY_HOLES_STANDARD_LINES: RangeInclusive<u64> = 3..=12;
/// `Sprawl.ApplyPremium`, the copy, at twelve different literals.
const MANY_HOLES_PREMIUM_LINES: RangeInclusive<u64> = 14..=23;
/// The same floor the rest of the same-file band is measured at.
const MANY_HOLES_MIN_NODES: u32 = 12;
/// Both member names; the pair is the finding, so both must be reported.
const MANY_HOLES_METHOD_NAMES: [&str; 2] = ["ApplyStandard", "ApplyPremium"];
/// Twelve literals differ, so the two declarations are never one text.
const MANY_HOLES_DISTINCT_TEXTS: usize = 2;
/// What a suppression of this pair would prove.
const MANY_HOLES_WHY: &str =
    "two methods that share every identifier and every call, and substitute \
     consistently at all twelve literal positions, are one parameterised \
     method. Hiding them proves the same-file promote floor judged a \
     literal-only copy on the one axis its own edit demolishes.";

/// [FUSED-CONTENT-GATE] A same-file pair is routed on
/// `support = max(agreement, rename_consistency)` against
/// `content_gate.promote_floor`. `ApplyStandard` and `ApplyPremium` share
/// every identifier position and every call, and substitute consistently
/// at all twelve literal positions; their measured `agreement` is 0.567
/// and their `rename_consistency` is 0.0, so the gate refuses them and
/// the file publishes nothing.
///
/// The two halves of that reading cannot both be right. [TECH-PMATCH-BAKER]
/// makes `rename_consistency` the Type-2 discriminator over *"the
/// identifier positions the bijection must explain plus every aligned
/// literal position"* — and here every identifier position is preserved
/// and every literal substitutes. Reporting `0.0` for that pair leaves
/// `max(agreement, rename_consistency)` reading agreement alone, so a
/// literal-only copy is judged on the one axis its own edit demolishes.
///
/// [AUTOFIX-MERGE-GATE] independently calls this pair a duplication: the
/// merge gate refuses it for *twelve distinct substitutions exceeding the
/// budget*, which is a statement about a clone too parameterised to merge
/// mechanically, not about two unrelated methods. `too_many_holes_refuse`
/// reaches that verdict through a synthetic plan, so it holds even while
/// the detector publishes nothing to plan over.
#[test]
fn a_literal_only_copy_inside_one_file_is_a_finding() -> Result<()> {
    let scan_root = fixture(MANY_HOLES_FIXTURE);
    let report = run_report(&scan_root, MANY_HOLES_MIN_NODES)?;
    let texts = expect_only_finding_is_the_pair(
        &scan_root,
        &report,
        MANY_HOLES_FILE,
        &[MANY_HOLES_STANDARD_LINES, MANY_HOLES_PREMIUM_LINES],
        MANY_HOLES_DISTINCT_TEXTS,
        MANY_HOLES_WHY,
    )?;
    assert_reported(&texts, &MANY_HOLES_METHOD_NAMES, MANY_HOLES_WHY);
    Ok(())
}

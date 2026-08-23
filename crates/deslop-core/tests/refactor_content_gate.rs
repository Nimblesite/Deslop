//! [FUSION-CONTENT-GATE] × [AUTOFIX-EXTRACT-PRECONDITIONS] rule 1: the
//! refactor gates decide on the *measured* content evidence, never on
//! the bucket label alone (gh #344).
//!
//! Shape saturation is not evidence of duplication. An anchor-poor
//! scaffolding family and a corroborated Type-2 rename both render
//! `structural = 1.00, token_jaccard = 1.00`; only `agreement` and
//! `rename_consistency` tell them apart. Offering a shared-helper
//! refactor on the first would fold two unrelated methods into one, so
//! the preconditions refuse it and say why in the measured numbers,
//! rendered by the single shared `render::signals` formatter every
//! surface uses.
//!
//! Both directions are pinned here, because a gate that only ever
//! refuses is as wrong as one that only ever allows.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, ensure, Context, Result};
use deslop_core::{
    buckets::{lacks_content_support, CONTENT_SUPPORT_FLOOR},
    refactor::{
        self,
        consolidate::{compute_consolidation_plan, ConsolidationOutcome},
        preconditions,
    },
    render::signals::plain_explanation,
    report::ReportCluster,
    wire_generated::MergeVerdict,
};

use crate::common::{analyse_refactor_fixture as analyse, fixture, merge::merge_plans};

/// The single ranked cluster of a two-occurrence fixture. Every fixture
/// used here is deliberately one duplication, so a second cluster means
/// the fixture drifted and the assertions below would be measuring
/// something else.
fn sole_cluster(fixture_name: &str) -> Result<ReportCluster> {
    let report = analyse(&fixture(fixture_name))?;
    ensure!(
        report.clusters.len() == 1,
        "{fixture_name} must report exactly one cluster, got {}",
        report.clusters.len()
    );
    report
        .clusters
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("{fixture_name} reported no cluster"))
}

/// Measured content support — the stronger of the two independent
/// populations, exactly as [FUSION-CONTENT-GATE] routes on it.
fn support(cluster: &ReportCluster) -> f64 {
    cluster
        .signals
        .agreement
        .max(cluster.signals.rename_consistency)
}

/// The single-file fixture's source bytes.
fn source_of(fixture_name: &str, file_name: &str) -> Result<Vec<u8>> {
    fs::read(fixture(fixture_name).join(file_name)).context("fixture source")
}

/// The sole merge plan of a single-file fixture.
fn sole_merge_plan(
    fixture_name: &str,
    file_name: &str,
) -> Result<deslop_core::wire_generated::MergePlan> {
    let plans = merge_plans(fixture_name, file_name)?;
    ensure!(
        plans.len() == 1,
        "{fixture_name} must yield exactly one merge plan, got {}",
        plans.len()
    );
    plans
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("{fixture_name} yielded no merge plan"))
}

/// A same-file family whose two methods share only a shape — unrelated
/// identifiers, differing loop stride — must not reach any
/// shared-helper action, and the refusal must state the measured
/// evidence that convicted it.
///
/// Before #344 the bucket pre-filter admitted it: `structural_only` is
/// an exact-structural bucket, the two occurrences sit in one file and
/// their ranges are disjoint, so `eligible_ranges` handed the merge
/// engine two blocks that agree on 18% of their raw content.
#[test]
fn shape_only_same_file_family_is_refused_with_the_measured_reason() -> Result<()> {
    let cluster = sole_cluster("csharp-shape-only-samefile")?;
    ensure!(
        cluster.bucket == "structural_only",
        "the family must route structural_only, got {}",
        cluster.bucket
    );
    ensure!(
        cluster.signals.structural >= 0.99 && cluster.signals.token_jaccard >= 0.99,
        "the shape must saturate for the gate to apply: structural={:.2} token_jaccard={:.2}",
        cluster.signals.structural,
        cluster.signals.token_jaccard
    );
    ensure!(
        (0.17..0.19).contains(&cluster.signals.agreement),
        "measured byte agreement is ~0.18, got {:.3}",
        cluster.signals.agreement
    );
    ensure!(
        cluster.signals.rename_consistency == 0.0,
        "no corroborated rename explains the differences, got {:.3}",
        cluster.signals.rename_consistency
    );
    ensure!(
        support(&cluster) < CONTENT_SUPPORT_FLOOR,
        "support {:.3} must sit below the {CONTENT_SUPPORT_FLOOR} floor",
        support(&cluster)
    );
    ensure!(
        lacks_content_support(cluster.signals),
        "the shared predicate must convict this family"
    );

    ensure!(
        preconditions::eligible_ranges(&cluster).is_none(),
        "a shape-only family must not yield rewritable ranges"
    );
    let source = source_of("csharp-shape-only-samefile", "Scaffold.cs")?;
    let parser = refactor::parser_for_path(Path::new("Scaffold.cs"))
        .ok_or_else(|| anyhow!("C# parser registered"))?;
    ensure!(
        refactor::compute_plan(&cluster, &source, parser.as_ref())?.is_none(),
        "no verbatim extract may be offered for a shape-only family"
    );

    let plan = sole_merge_plan("csharp-shape-only-samefile", "Scaffold.cs")?;
    let MergeVerdict::AiOrHuman { reason } = &plan.verdict else {
        return Err(anyhow!(
            "a shape-only family must never merge mechanically: {:?}",
            plan.verdict
        ));
    };
    ensure!(
        plan.workspace_edit.is_none(),
        "a refusal carries no workspace edit"
    );
    ensure!(
        reason.contains("shapes match") && reason.contains("content"),
        "the reason must name the content gate, got {reason}"
    );
    ensure!(
        reason.contains(&plain_explanation(cluster.signals)),
        "the reason must quote the shared render::signals explanation \
         `{}`, got {reason}",
        plain_explanation(cluster.signals)
    );
    ensure!(
        reason.contains("agreement 0.18") && reason.contains("rename 0.00"),
        "the reason must carry the measured evidence verbatim, got {reason}"
    );
    Ok(())
}

/// The cross-file half of the same defect: `consolidation_candidate` is
/// the LSP's offer screen for "replace every copy with one canonical
/// definition". Offering that on two methods that share only a shape
/// would delete a live method and repoint its callers at unrelated
/// code — the single most destructive action the surface can take.
#[test]
fn shape_only_cross_file_family_is_not_a_consolidation_candidate() -> Result<()> {
    let cluster = sole_cluster("csharp-shape-only-crossfile")?;
    ensure!(
        cluster.bucket == "structural_only",
        "the family must route structural_only, got {}",
        cluster.bucket
    );
    ensure!(
        deslop_core::report::distinct_visible_path_count(&cluster) == 2,
        "the family must span two files for the consolidation screen to apply"
    );
    ensure!(
        (0.16..0.18).contains(&cluster.signals.agreement)
            && cluster.signals.rename_consistency == 0.0,
        "measured evidence is ~0.17 agreement and no rename proof, got {:.3} / {:.3}",
        cluster.signals.agreement,
        cluster.signals.rename_consistency
    );
    ensure!(
        !preconditions::consolidation_candidate(&cluster),
        "a shape-only cross-file family must not be offered consolidation"
    );

    let mut sources: HashMap<PathBuf, Vec<u8>> = HashMap::new();
    for occurrence in &cluster.occurrences {
        let bytes = fs::read(fixture("csharp-shape-only-crossfile").join(&occurrence.path))
            .context("fixture source")?;
        let _replaced = sources.insert(occurrence.path.clone(), bytes);
    }
    let parser = refactor::parser_for_path(Path::new("LedgerPosting.cs"))
        .ok_or_else(|| anyhow!("C# parser registered"))?;
    let outcome = compute_consolidation_plan(&cluster, &sources, parser.as_ref())
        .map_err(|error| anyhow!("consolidation failed: {error}"))?;
    let ConsolidationOutcome::Refused(reason) = outcome else {
        return Err(anyhow!(
            "consolidating a shape-only family would delete a live method"
        ));
    };
    ensure!(
        reason.contains("shapes match") && reason.contains(&plain_explanation(cluster.signals)),
        "the consolidation refusal must carry the measured explanation, got {reason}"
    );
    Ok(())
}

/// The allow direction. The verbatim extract fixture is a real
/// duplication: 96% of its raw collapsed content agrees, so the gate
/// vouches for it and every existing precondition still decides the
/// outcome.
#[test]
fn content_proven_clone_still_reaches_the_verbatim_extract() -> Result<()> {
    let cluster = sole_cluster("csharp-extract-type1")?;
    ensure!(
        cluster.bucket == "nearly_identical",
        "the clone routes nearly_identical, got {}",
        cluster.bucket
    );
    ensure!(
        (0.94..0.97).contains(&cluster.signals.agreement),
        "measured byte agreement is ~0.96, got {:.3}",
        cluster.signals.agreement
    );
    ensure!(
        support(&cluster) >= CONTENT_SUPPORT_FLOOR,
        "support {:.3} must clear the {CONTENT_SUPPORT_FLOOR} floor",
        support(&cluster)
    );
    ensure!(
        !lacks_content_support(cluster.signals),
        "the shared predicate must vouch for a measured clone"
    );

    let ranges = preconditions::eligible_ranges(&cluster)
        .ok_or_else(|| anyhow!("a content-proven single-file clone must yield ranges"))?;
    ensure!(
        ranges.len() == 2,
        "both occurrences stay rewritable, got {}",
        ranges.len()
    );
    let source = source_of("csharp-extract-type1", "InvoiceMath.cs")?;
    let parser = refactor::parser_for_path(Path::new("InvoiceMath.cs"))
        .ok_or_else(|| anyhow!("C# parser registered"))?;
    let plan = refactor::compute_plan(&cluster, &source, parser.as_ref())?
        .ok_or_else(|| anyhow!("the content-proven clone must still extract"))?;
    ensure!(
        plan.edits.len() == 3,
        "two call sites plus the helper insertion, got {}",
        plan.edits.len()
    );
    ensure!(
        plan.method_name == format!("ExtractedFromCluster_{}", &cluster.id[..6]),
        "the helper name derives from the cluster id, got {} for id {}",
        plan.method_name,
        cluster.id
    );
    Ok(())
}

/// The gate must route on `max(agreement, rename_consistency)`, never
/// on agreement alone. This fixture is a textbook Type-2 rename: its
/// pooled byte agreement (0.64) sits *below* the floor, and only the
/// corroborated rename proof (0.84) keeps it eligible. Collapsing the
/// two populations into one would refuse the most valuable clone class
/// there is.
#[test]
fn proven_rename_survives_the_gate_that_agreement_alone_would_fail() -> Result<()> {
    let cluster = sole_cluster("csharp-extract-type2")?;
    ensure!(
        cluster.signals.structural >= 0.99 && cluster.signals.token_jaccard >= 0.99,
        "the shape saturates here exactly as it does for the scaffolding family"
    );
    ensure!(
        cluster.signals.agreement < CONTENT_SUPPORT_FLOOR,
        "pooled agreement {:.3} must sit below the floor for this test to bite",
        cluster.signals.agreement
    );
    ensure!(
        cluster.signals.rename_consistency >= 0.99,
        "a textbook Type-2 rename is corroborated at 1.0 — every authored \
         position differs and every difference is a consistent rename \
         ([PIPELINE-NORMALIZE-AST-OPERATOR] keeps shared operators out of \
         the ratio), got {:.3}",
        cluster.signals.rename_consistency
    );
    ensure!(
        !lacks_content_support(cluster.signals),
        "the rename population alone must vouch for the cluster"
    );
    let ranges = preconditions::eligible_ranges(&cluster)
        .ok_or_else(|| anyhow!("a proven rename must stay eligible for the merge engine"))?;
    ensure!(ranges.len() == 2, "both renamed sites stay rewritable");

    let plan = sole_merge_plan("csharp-extract-type2", "RateMath.cs")?;
    let MergeVerdict::AiOrHuman { reason } = &plan.verdict else {
        return Err(anyhow!(
            "this fixture refuses on control flow, not on content"
        ));
    };
    ensure!(
        reason.contains("transfers control"),
        "the refusal must still come from the safety rules, not the content gate, got {reason}"
    );
    Ok(())
}

/// Byte-proven `identical` clusters are exempt: [CLONE-BUCKETS-IDENTICAL]
/// already compared their raw source bytes, which is strictly stronger
/// evidence than the collapsed-leaf measurement. The cross-file
/// consolidation offer must survive.
#[test]
fn byte_proven_identical_family_stays_a_consolidation_candidate() -> Result<()> {
    let cluster = sole_cluster("csharp-extract-crossfile")?;
    ensure!(
        cluster.bucket == "identical",
        "the family is byte-proven identical, got {}",
        cluster.bucket
    );
    ensure!(
        deslop_core::report::distinct_visible_path_count(&cluster) == 2,
        "the family spans two files"
    );
    ensure!(
        preconditions::consolidation_candidate(&cluster),
        "a byte-proven cross-file family must keep its consolidation offer"
    );
    Ok(())
}

/// The tree's *genuine* duplication fixtures — Type-1 copies, Type-2
/// renames, Type-3 near-misses, and every merge / extract /
/// consolidate fixture the refactor suites drive — across C#, Rust,
/// Python, Dart, JavaScript and TypeScript.
///
/// Fixtures that produce no cluster at the refactor suites' `min_nodes`
/// are deliberately absent: a fixture that clusters nothing would make
/// the sweep below vacuous.
const GENUINE_CLONE_FIXTURES: [&str; 38] = [
    "csharp-type1",
    "csharp-type3",
    "javascript-type3",
    "typescript-type3",
    "ts-type3-reorder",
    "js-type3-guard",
    "js-type3-stmt",
    "js-type2-loop",
    "ts-type2-loop",
    "js-type1-identical",
    "ts-type1-identical",
    "type2-rename-few-literals",
    "rust-small",
    "rust-issue-176-verbatim-copy",
    "python-issue-104-genuine-copy",
    "python-issue-133-genuine-copy",
    "dart-forwarding-business-pair",
    "csharp-merge-rename",
    "csharp-merge-identhole",
    "csharp-merge-leafgap",
    "csharp-merge-defaults",
    "csharp-merge-drift",
    "csharp-merge-typeconflict",
    "csharp-merge-return",
    "csharp-merge-readafter",
    "csharp-merge-writtenhole",
    "csharp-merge-writtencontext",
    "csharp-merge-operatordrift",
    "dart-merge-leafgap",
    "dart-merge-writtencontext",
    "rust-merge-leafgap",
    "rust-consolidate",
    "rust-consolidate-drift",
    "rust-extract-type1",
    "rust-extract-attrs",
    "python-extract-type1",
    "python-extract-module",
    "csharp-extract-freevars",
];

/// The over-refusal guard, swept across every genuine-duplication
/// fixture in the tree.
///
/// A safety gate that refuses too much is not safe, it is blind: the
/// user stops being offered the refactors this product exists to offer,
/// and nothing tells them why. So the gate is measured against the
/// whole corpus of real clones rather than against one hand-picked
/// happy path — not one of them may be convicted, in any language, at
/// any clone type.
///
/// The cluster-count assertion is what keeps this honest. Without it a
/// fixture that quietly stopped clustering would turn its leg of the
/// sweep into an assertion about nothing.
#[test]
fn no_genuine_clone_fixture_is_convicted_by_the_content_gate() -> Result<()> {
    let mut clusters_checked = 0_usize;
    for name in GENUINE_CLONE_FIXTURES {
        let report = analyse(&fixture(name))?;
        ensure!(
            !report.clusters.is_empty(),
            "{name} must still report a duplication for this sweep to assert anything"
        );
        for cluster in &report.clusters {
            ensure!(
                !lacks_content_support(cluster.signals),
                "{name} cluster {} is a genuine clone the content gate convicted: \
                 bucket={} structural={:.2} token_jaccard={:.2} agreement={:.3} \
                 rename_consistency={:.3}",
                cluster.id,
                cluster.bucket,
                cluster.signals.structural,
                cluster.signals.token_jaccard,
                cluster.signals.agreement,
                cluster.signals.rename_consistency
            );
            ensure!(
                preconditions::content_refusal(cluster).is_none(),
                "{name} cluster {} must keep its refactor actions",
                cluster.id
            );
            clusters_checked = clusters_checked.saturating_add(1);
        }
    }
    ensure!(
        clusters_checked >= GENUINE_CLONE_FIXTURES.len(),
        "the sweep must judge at least one cluster per fixture, judged {clusters_checked}"
    );
    Ok(())
}

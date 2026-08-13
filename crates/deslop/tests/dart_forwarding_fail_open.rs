//! [RANK-STRUCTURAL-ONLY-FORWARDING] The forwarding proof must fail open.
//!
//! This predicate deletes findings. Everything it cannot read must
//! therefore keep its cluster visible, and the two ways it could
//! over-reach both get a control here.
//!
//! **Computation dressed as a wrapper.** `dart-forwarding-fail-open`
//! holds two one-statement methods that each contain a call — the same
//! surface shape as the meilisearch REST family. Between the input and
//! the call sits a multiplication and an addition. Any rule counting
//! statements, or counting calls, hides them. The allowlist does not
//! contain arithmetic, so the proof fails and the liftable pair stays on
//! the report.
//!
//! **A copy-paste bug inside a real family.**
//! `dart-forwarding-duplicate-route` holds five wrappers that all prove
//! forwarding. Two of them DELETE the same route, so one of those calls
//! is dead or misaimed. Their declarations differ only in the method
//! name, so the reported windows never compare equal and a window-level
//! distinctness check passes straight over the bug. Comparing the proven
//! *bodies* is what sees it, and one shared body disqualifies the
//! suppression for the whole family.
//!
//! **Business calls wearing the wrapper shape.**
//! `dart-forwarding-business-pair` holds two sibling pairs that each
//! make one allowlisted call and differ only in an integer literal —
//! the exact surface of the meilisearch wrappers. But the calls go to a
//! *sibling helper on the same class*, not to a collaborator the class
//! holds, so parameterising the helper call lifts the pair. A proof
//! that accepts any call, wherever it goes, hides both; forwarding must
//! mean handing the data to collaborator state — a field or parameter
//! receiver. The pairs vary an int rather than a string because
//! same-callee *string*-literal variation is already suppressed by
//! design ([CLONE-NOISE-LITERAL-VARIATION-CALLS]); what is pinned here
//! is the reach the forwarding proof added beyond that filter.

use anyhow::Result;

mod common;
use crate::common::{verdict::*, *};

#[test]
fn one_statement_bodies_that_compute_are_not_forwarding() -> Result<()> {
    let scan_root = fixture("dart-forwarding-fail-open");
    let report = run_report(&scan_root, 12)?;

    let cluster = expect_sole_cluster(
        &report,
        "two one-statement Dart methods that multiply and add are liftable \
         logic, not API scaffolding. Hiding them proves the forwarding \
         allowlist leaked arithmetic — or that a statement count stood in for \
         the proof.",
    )?;
    assert_single_file_cluster(cluster, 2, "Calc.dart");
    let _texts = assert_cluster_mentions(&scan_root, cluster, &["scaledDomestic", "scaledExport"])?;
    assert_duplicated_loc_at_least(&report, 4);
    Ok(())
}

#[test]
fn same_class_helper_calls_are_not_forwarding() -> Result<()> {
    let scan_root = fixture("dart-forwarding-business-pair");
    let report = run_report(&scan_root, 12)?;

    let visible = expect_visible_only(
        &report,
        2,
        "both business pairs call a sibling helper — parameterisable logic, \
         not API scaffolding. The bound-result pair is the regression pin: \
         the branch's forwarding proof hid it because it accepted any call \
         without proving a collaborator receiver. The renamed arrow pair is \
         the visibility boundary: content evidence keeps a consistent rename \
         visible as nearly_identical on either side of the fix.",
    );
    let mut texts = Vec::new();
    for cluster in visible {
        assert_single_file_cluster(cluster, 2, "Pricing.dart");
        texts.extend(occurrence_texts(&scan_root, cluster)?);
    }
    for name in [
        "quarterlyFee",
        "annualCharge",
        "standardTotal",
        "premiumTotal",
    ] {
        assert!(
            texts.iter().any(|text| text.contains(name)),
            "{name} is half of a liftable pair and must be reported: {texts:#?}"
        );
    }
    assert_duplicated_loc_at_least(&report, 4);
    Ok(())
}

#[test]
fn wrappers_sharing_a_body_keep_the_family_visible() -> Result<()> {
    let scan_root = fixture("dart-forwarding-duplicate-route");
    let report = run_report(&scan_root, 12)?;

    let cluster = expect_sole_cluster(
        &report,
        "two of these five wrappers DELETE the same route — one call is dead \
         or misaimed. Hiding the family erases a real finding, and the \
         reported windows cannot show it because the method names differ.",
    )?;
    assert_single_file_cluster(cluster, 5, "Api.dart");
    let texts = assert_cluster_mentions(
        &scan_root,
        cluster,
        &[
            "resetAlpha",
            "resetBeta",
            "resetGamma",
            "resetDelta",
            "resetEpsilon",
        ],
    )?;
    let duplicate_route = texts
        .iter()
        .filter(|text| text.contains("/indexes/dup/settings"))
        .count();
    assert_eq!(
        duplicate_route, 2,
        "both same-route wrappers must be among the reported occurrences — \
         they are the reason this family is not noise: {texts:#?}"
    );
    assert_duplicated_loc_at_least(&report, 10);
    Ok(())
}

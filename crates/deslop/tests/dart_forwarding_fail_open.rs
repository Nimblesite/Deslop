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
use serde_json::Value;

mod common;
use crate::common::*;

#[test]
fn one_statement_bodies_that_compute_are_not_forwarding() -> Result<()> {
    let scan_root = fixture("dart-forwarding-fail-open");
    let report = run_report(&scan_root, 12)?;

    let visible = clusters(&report);
    assert_eq!(
        visible.len(),
        1,
        "two one-statement Dart methods that multiply and add are liftable \
         logic, not API scaffolding. Hiding them proves the forwarding \
         allowlist leaked arithmetic — or that a statement count stood in for \
         the proof. report={report:#}"
    );
    let cluster = visible
        .first()
        .ok_or_else(|| anyhow::anyhow!("the visible cluster asserted above is missing"))?;
    assert_eq!(
        cluster_size(cluster),
        2,
        "both methods are occurrences of the one cluster: {cluster:#}"
    );
    assert_eq!(
        occurrence_files(cluster),
        vec!["Calc.dart", "Calc.dart"],
        "single-file pair by construction: {cluster:#}"
    );
    let texts = occurrence_texts(&scan_root, cluster)?;
    assert!(
        texts.iter().any(|text| text.contains("scaledDomestic"))
            && texts.iter().any(|text| text.contains("scaledExport")),
        "both computing methods must be reported: {texts:#?}"
    );
    assert_eq!(
        clusters_hidden(&report),
        0,
        "nothing here proves forwarding, so nothing may be hidden: {report:#}"
    );
    let duplicated_loc = report
        .pointer("/metrics/duplicated_loc")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    assert!(
        duplicated_loc >= 4,
        "the duplicated bodies must count toward the metrics: \
         duplicated_loc={duplicated_loc}, report={report:#}"
    );
    Ok(())
}

#[test]
fn same_class_helper_calls_are_not_forwarding() -> Result<()> {
    let scan_root = fixture("dart-forwarding-business-pair");
    let report = run_report(&scan_root, 12)?;

    let visible = clusters(&report);
    assert_eq!(
        visible.len(),
        2,
        "both business pairs call a sibling helper — parameterisable logic, \
         not API scaffolding. The bound-result pair is the regression pin: \
         the branch's forwarding proof hid it because it accepted any call \
         without proving a collaborator receiver. The renamed arrow pair is \
         the visibility boundary: content evidence keeps a consistent rename \
         visible as nearly_identical on either side of the fix. \
         report={report:#}"
    );
    for cluster in visible {
        assert_eq!(
            cluster_size(cluster),
            2,
            "each pair is one cluster of two occurrences: {cluster:#}"
        );
        assert_eq!(
            occurrence_files(cluster),
            vec!["Pricing.dart", "Pricing.dart"],
            "single-file pairs by construction: {cluster:#}"
        );
    }
    let texts: Vec<String> = visible
        .iter()
        .map(|cluster| occurrence_texts(&scan_root, cluster))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
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
    assert_eq!(
        clusters_hidden(&report),
        0,
        "no body here reaches collaborator state, so nothing may be hidden: {report:#}"
    );
    let duplicated_loc = report
        .pointer("/metrics/duplicated_loc")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    assert!(
        duplicated_loc >= 4,
        "both visible pairs contribute duplicated lines: \
         duplicated_loc={duplicated_loc}, report={report:#}"
    );
    Ok(())
}

#[test]
fn wrappers_sharing_a_body_keep_the_family_visible() -> Result<()> {
    let scan_root = fixture("dart-forwarding-duplicate-route");
    let report = run_report(&scan_root, 12)?;

    let visible = clusters(&report);
    assert_eq!(
        visible.len(),
        1,
        "two of these five wrappers DELETE the same route — one call is dead \
         or misaimed. Hiding the family erases a real finding, and the \
         reported windows cannot show it because the method names differ. \
         report={report:#}"
    );
    let cluster = visible
        .first()
        .ok_or_else(|| anyhow::anyhow!("the visible cluster asserted above is missing"))?;
    assert_eq!(
        cluster_size(cluster),
        5,
        "the whole family stays visible, not just the offending pair: {cluster:#}"
    );
    assert_eq!(
        occurrence_files(cluster),
        vec!["Api.dart"; 5],
        "single-file family by construction: {cluster:#}"
    );

    let texts = occurrence_texts(&scan_root, cluster)?;
    let duplicate_route = texts
        .iter()
        .filter(|text| text.contains("/indexes/dup/settings"))
        .count();
    assert_eq!(
        duplicate_route, 2,
        "both same-route wrappers must be among the reported occurrences — \
         they are the reason this family is not noise: {texts:#?}"
    );
    for name in [
        "resetAlpha",
        "resetBeta",
        "resetGamma",
        "resetDelta",
        "resetEpsilon",
    ] {
        assert!(
            texts.iter().any(|text| text.contains(name)),
            "{name} must be reported: {texts:#?}"
        );
    }

    assert_eq!(
        clusters_hidden(&report),
        0,
        "one shared body disqualifies the suppression for the family: {report:#}"
    );
    let duplicated_loc = report
        .pointer("/metrics/duplicated_loc")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    assert!(
        duplicated_loc >= 10,
        "the visible family contributes duplicated lines: \
         duplicated_loc={duplicated_loc}, report={report:#}"
    );
    Ok(())
}
